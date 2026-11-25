//! The `offline` gate: nothing the product does leaves the machine unasked.
//!
//! People are told that this product sends nothing anywhere unless they
//! point it somewhere (an agent's model, a hub they join, a command in a
//! blueprint). This gate is what holds it to that, for the code here and for
//! every library linked into it.
//!
//! The daemon's and the command line's tests, which between them drive the
//! whole product, are run in a place with no network: the machine's own
//! loopback, and one more interface that leads nowhere and is the route to
//! everywhere else. Anything sent towards the outside goes to that
//! interface and is counted. A name being looked up, an update check, a
//! library reporting home: each is at least one packet. The gate fails if
//! the tests fail there, or if the count is not zero.
//!
//! This is the one time those tests are run (the `test` gate leaves them
//! to this one), so they are run there as they would be anywhere: by
//! whoever ran the gate and not by the place's administrator, each from
//! its crate's own folder. Where no such place can be made they are run
//! all the same, with the network there, and the gate says that it could
//! not count.

use std::process::{Command, Stdio};

use crate::gates::{GateResult, Scope, failures};
use crate::repo::Repo;
use crate::suites::{self, Program, WHOLE_PRODUCT};

/// Run inside a new network namespace, as its administrator. `$1` and `$2`
/// are who ran the gate, as numbers; after them comes this program and
/// what it is told, to run the tests (`suites::inside`).
const INSIDE: &str = r#"
ip link set lo up || exit 90
# An interface that goes nowhere, with no address of its own making, so that
# the system itself sends nothing through it.
ip link add trap0 type dummy || exit 90
ip link set dev trap0 addrgenmode none || exit 90
ip addr add 198.18.0.1/15 dev trap0 || exit 90
ip link set trap0 up || exit 90
ip route add default dev trap0 || exit 90
ip -6 route add default dev trap0 || exit 90
echo "trap-set"
# The tests are handed back to whoever ran the gate, who can change nothing
# of this place: as its administrator a test may read and write what the
# person it stands in for could not.
as=""
if [ -n "$1" ] && unshare --map-user="$1" --map-group="$2" true 2>/dev/null; then
  as="unshare --map-user=$1 --map-group=$2"
  echo "as-themselves"
fi
shift 2
$as "$@"
status=$?
echo "sent-outside: $(ip -s link show trap0 | awk '/TX:/ { getline; print $2 }')"
exit $status
"#;

/// Who is running the gate, as the system numbers them.
#[cfg(unix)]
fn whoever() -> Option<(u32, u32)> {
    use std::os::unix::fs::MetadataExt;
    let own = std::fs::metadata("/proc/self").ok()?;
    Some((own.uid(), own.gid()))
}

#[cfg(not(unix))]
fn whoever() -> Option<(u32, u32)> {
    None
}

/// Runs the programs with the network there, for a system that cannot cut
/// it off: they are tests before they are a count.
fn in_the_open(repo: &Repo, programs: &[Program], why: &str) -> GateResult {
    let outcome = suites::run_all(repo, programs);
    if outcome.problems.is_empty() {
        Ok(format!("skipped: {why}; the tests ran with the network there. {}", outcome.timed))
    } else {
        Err(outcome.problems)
    }
}

pub fn offline(repo: &Repo, scope: &Scope) -> GateResult {
    // The same build as the `test` gate's, so that nothing is compiled a
    // second way for this one; its part of what was built is these.
    let programs: Vec<Program> = suites::built(repo, scope)?
        .into_iter()
        .filter(|program| WHOLE_PRODUCT.contains(&program.krate.as_str()))
        .collect();
    if programs.is_empty() {
        return match scope {
            Scope::Everything => Err(vec!["found no test programs to run with the network cut off".into()]),
            Scope::Crates(_) => Ok("nothing that runs changed".into()),
        };
    }
    crate::leftover::held(repo, || apart(repo, &programs))
}

/// Runs the programs where there is no network, or says why they were run
/// with it there.
fn apart(repo: &Repo, programs: &[Program]) -> GateResult {
    if !cfg!(target_os = "linux") {
        return in_the_open(
            repo,
            programs,
            "cutting the network off for one command is done with Linux namespaces",
        );
    }
    let (user, group) = whoever()
        .map_or((String::new(), String::new()), |(user, group)| (user.to_string(), group.to_string()));
    // The tests are run in that place by this program, started again there.
    let Ok(this) = std::env::current_exe() else {
        return in_the_open(repo, programs, "this program could not find itself to run the tests there");
    };
    let mut command = Command::new("unshare");
    command.args(["--map-root-user", "--net", "sh", "-c", INSIDE, "sh", &user, &group]);
    command.arg(this).args(suites::asked(programs));
    let child = command
        .current_dir(&repo.root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let done = match child.map_err(|err| err.to_string()).and_then(suites::follow) {
        Ok(done) => done,
        Err(err) => {
            return in_the_open(
                repo,
                programs,
                &format!("`unshare` is needed to cut the network off ({err})"),
            );
        }
    };
    let said = |what: &str| done.lines.iter().any(|(_, line)| line == what);
    if !said("trap-set") {
        let text = done.text();
        let reason = text.lines().rev().find(|line| !line.trim().is_empty()).unwrap_or("no reason given");
        return in_the_open(
            repo,
            programs,
            &format!("this system would not set up a place with no network ({})", reason.trim()),
        );
    }
    let outcome = suites::heard(&done);
    let timed = outcome.timed;
    let sent: Option<u64> = done
        .lines
        .iter()
        .find_map(|(_, line)| line.strip_prefix("sent-outside: "))
        .and_then(|count| count.trim().parse().ok());
    if !done.ok {
        let mut lines = vec![
            "these fail, run with no network (`cargo test -p api -p cli` runs them with it there):"
                .to_string(),
        ];
        // What the runner said is wrong, or, where it said nothing (it
        // could not be started there), whatever was printed.
        if outcome.problems.is_empty() {
            lines.extend(failures(&done.text()));
        } else {
            lines.extend(outcome.problems);
        }
        return Err(lines);
    }
    let whose = if said("as-themselves") {
        ""
    } else {
        " (as the place's administrator: this system's `unshare` hands a program to nobody else)"
    };
    match sent {
        Some(0) => Ok(format!("ran with no network{whose}; nothing was sent off the machine. {timed}")),
        Some(count) => Err(vec![
            format!("{count} packets were sent towards the outside while the tests ran with no network"),
            "something here, or a library linked in, reached for another machine without being asked".into(),
            "to see what, run a test program from `cargo test --no-run -p api -p cli` under `strace -f -e trace=connect,sendto`"
                .into(),
        ]),
        None => Err(vec!["could not read how much was sent towards the outside".into()]),
    }
}
