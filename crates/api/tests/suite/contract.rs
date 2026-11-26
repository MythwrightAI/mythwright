//! The contract between blueprints and this program, held both ways: what
//! it offers against what it offered before, what it emits against what
//! it says it emits, and a handler against what its events carry.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use api::contract::{self, Asked};
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// Writes a blueprint among the project's own.
async fn put(rig: &Rig, name: &str, content: &str) {
    let body = json!({ "path": format!("@blueprints/{}", brand::blueprint_file(name)), "content": content });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// What was asked of the kept contract, by how the test was run.
fn asked() -> Asked {
    match (std::env::var("BREAK_CONTRACT"), std::env::var_os("WRITE_CONTRACT")) {
        (Ok(why), _) => Asked::Break(why),
        (_, Some(_)) => Asked::Write,
        _ => Asked::Hold,
    }
}

fn today() -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    contract::day(now as i64)
}

/// The gate. What this program offers is written as the kept file is and
/// compared; what differs is said as something added or something broken,
/// with the command that writes it down.
#[tokio::test]
async fn the_contract_kept_with_the_code_is_what_this_program_offers() {
    let rig = Rig::fresh().await;
    if let Err(why) = contract::hold(&repo().join("contract"), &rig.node().offers(), &asked(), &today()) {
        panic!("{why}");
    }
}

/// The same for what each package that ships brings, kept in the
/// package's own folder: the application names no package, in a file of
/// its own as anywhere else.
#[test]
fn the_contract_kept_with_each_package_is_what_the_package_brings() {
    let packages = repo().join("extensions");
    let brought = contract::brought(&packages);
    assert!(!brought.is_empty(), "the packages that ship are found in {}", packages.display());
    for (package, offered) in brought {
        let dir = packages.join(&package).join("contract");
        if asked() != Asked::Hold {
            std::fs::create_dir_all(&dir).unwrap();
        }
        if let Err(why) = contract::hold(&dir, &offered, &asked(), &today()) {
            panic!("the package `{package}` (extensions/{package}/contract):\n{why}");
        }
    }
}

const BEFORE: &str = "\
action thing.make
  needs things:write
  open
  takes name string needed
  takes size int
  gives id

event thing.made
  carries id string always
  carries size int
";

fn differs(now: &str) -> contract::Differences {
    contract::differences(BEFORE, now)
}

#[test]
fn something_added_is_not_something_a_blueprint_could_break_on() {
    let now = BEFORE
        .replace("  takes size int\n", "  takes size int\n  takes colour string\n")
        .replace("  gives id\n", "  gives id\n  gives made\n")
        .replace("  carries size int\n", "  carries size int always\n  carries by string\n")
        + "\naction thing.remove\n  takes id string needed\n\nevent thing.removed\n  carries id string always\n";
    let found = differs(&now);
    assert!(found.broken.is_empty(), "{:?}", found.broken);
    for added in ["`colour`, a new argument", "`made`, newly given out", "`by`, newly carried"] {
        assert!(found.added.iter().any(|line| line.contains(added)), "{added} in {:?}", found.added);
    }
    assert!(found.added.contains(&"event thing.made: `size` is always there now".to_string()));
    assert!(found.added.contains(&"action thing.remove, new".to_string()));
    assert!(found.added.contains(&"event thing.removed, new".to_string()));
    // Written in another order and under another heading, it is the same.
    assert!(contract::differences(BEFORE, &format!("# another heading\n{BEFORE}")).is_empty());
}

/// What an argument says is written inside it is read by whoever shows a
/// blueprint and checked by nothing, so no blueprint written before
/// breaks on it being said, said otherwise or no longer said.
#[test]
fn what_an_argument_says_is_written_inside_it_is_never_something_a_blueprint_could_break_on() {
    let with = BEFORE.replace(
        "  takes size int\n",
        "  takes size int\n  within size.* each\n  within size.*.kind string\n  within size.*.kind.plain word\n",
    );
    let cases: &[(&str, &str, &str)] = &[
        (BEFORE, &with, "action thing.make: `size.*.kind` is said of what it takes within: string"),
        (BEFORE, &with, "action thing.make: `size.*.kind.plain` is said of what it takes within: word"),
        (&with, BEFORE, "action thing.make: `size.*` is no longer said of what it takes within"),
        (
            &with,
            &with.replace("size.*.kind string", "size.*.kind any"),
            "action thing.make: `size.*.kind` is said to be any within, and was string",
        ),
    ];
    for (was, is, said) in cases {
        let found = contract::differences(was, is);
        assert!(found.broken.is_empty(), "{:?}", found.broken);
        assert!(found.added.contains(&said.to_string()), "`{said}` in {:?}", found.added);
    }
}

#[test]
fn each_thing_a_blueprint_written_before_could_break_on_is_said() {
    let cases: &[(&str, &str, &str, &str)] = &[
        ("action thing.make\n", "action thing.build\n", "action thing.make", "was taken away"),
        ("  takes size int\n", "", "action thing.make: `size`", "is no longer an argument"),
        ("  takes size int\n", "  takes size string\n", "action thing.make: `size`", "was int and is string"),
        ("  takes size int\n", "  takes size int needed\n", "action thing.make: `size`", "must now be given"),
        (
            "  gives id\n",
            "  takes colour string needed\n  gives id\n",
            "action thing.make: `colour`",
            "is a new argument that must be given",
        ),
        ("  gives id\n", "", "action thing.make: `id`", "is no longer given out"),
        (
            "  needs things:write\n",
            "  needs things:admin\n",
            "action thing.make",
            "needs `things:admin` now, and needed `things:write`",
        ),
        ("  open\n", "", "action thing.make", "no longer takes arguments beyond those listed"),
        ("event thing.made\n", "event thing.done\n", "event thing.made", "is no longer emitted"),
        ("  carries size int\n", "", "event thing.made: `size`", "is no longer carried"),
        (
            "  carries size int\n",
            "  carries size string\n",
            "event thing.made: `size`",
            "was int and is string",
        ),
        (
            "  carries id string always\n",
            "  carries id string\n",
            "event thing.made: `id`",
            "is no longer always there",
        ),
        (
            "event thing.made\n",
            "event thing.made\n  passing\n",
            "event thing.made",
            "is no longer kept, so no handler hears it",
        ),
    ];
    for (was, is, what, became) in cases {
        let found = differs(&BEFORE.replace(was, is));
        assert!(
            found.broken.contains(&(what.to_string(), became.to_string())),
            "{was:?} to {is:?} should say `{what} {became}`, and said {:?}",
            found.broken
        );
    }
}

#[test]
fn what_was_broken_is_written_down_only_when_it_is_said_to_be_meant_and_why() {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("contract-{}", &kernel::id::secret()[..12]));
    std::fs::create_dir_all(&dir).unwrap();
    let kept = || std::fs::read_to_string(dir.join(contract::OFFERED)).unwrap_or_default();
    let broken = || std::fs::read_to_string(dir.join(contract::BROKEN)).unwrap_or_default();

    // Nothing kept yet: everything is new, and is written only when asked.
    let refused = contract::hold(&dir, BEFORE, &Asked::Hold, "2026-11-12").unwrap_err();
    assert!(refused.contains("WRITE_CONTRACT=1 cargo test -p api the_contract"), "{refused}");
    assert!(refused.contains("action thing.make, new"), "{refused}");
    assert_eq!(kept(), "");
    contract::hold(&dir, BEFORE, &Asked::Write, "2026-11-12").unwrap();
    assert_eq!(kept(), BEFORE);
    contract::hold(&dir, BEFORE, &Asked::Hold, "2026-11-12").unwrap();

    // Something taken away is not written by asking for what was added to be.
    let without = BEFORE.replace("  takes size int\n", "");
    for asked in [Asked::Hold, Asked::Write] {
        let refused = contract::hold(&dir, &without, &asked, "2026-11-13").unwrap_err();
        assert!(refused.contains("This breaks blueprints written for the program as it was"), "{refused}");
        assert!(refused.contains("action thing.make: `size` is no longer an argument"), "{refused}");
        assert!(refused.contains("BREAK_CONTRACT=\"why\" cargo test -p api the_contract"), "{refused}");
        assert_eq!(kept(), BEFORE, "nothing is written");
    }
    // Nor by saying it is meant without saying why.
    let refused = contract::hold(&dir, &without, &Asked::Break("1".into()), "2026-11-13").unwrap_err();
    assert!(refused.contains("say why"), "{refused}");
    assert_eq!(broken(), "");

    // Said to be meant, with why: it is written, and what was broken is on the list.
    let why = "A thing has one size.";
    contract::hold(&dir, &without, &Asked::Break(why.into()), "2026-11-13").unwrap();
    assert_eq!(kept(), without);
    assert_eq!(broken(), format!("2026-11-13\taction thing.make: `size`\tis no longer an argument\t{why}\n"));
    // A second break is added under the first; nothing is taken off the list.
    let less = without.replace("  gives id\n", "");
    contract::hold(&dir, &less, &Asked::Break("It gives nothing out.".into()), "2026-11-14").unwrap();
    assert_eq!(broken().lines().count(), 2);
    assert!(broken().starts_with("2026-11-13\t"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_day_a_break_is_written_under_is_the_day_it_is() {
    assert_eq!(contract::day(0), "1970-01-01");
    assert_eq!(contract::day(951_782_400_000), "2000-02-29");
    assert_eq!(contract::day(1_794_441_600_000), "2026-11-12");
    assert_eq!(contract::day(1_794_527_999_999), "2026-11-12");
}

/// Where an event is emitted with its topic not written at the place: the
/// file, how many such places it has, and the topics they can be. A new
/// one has to be named here, with its topics described like any other.
const NOT_WRITTEN_THERE: &[(&str, usize, &[&str])] = &[
    // One of two, by whether the slot was there.
    ("agents/src/ensured.rs", 1, &["agent.slot.created", "agent.slot.updated"]),
    // The same of one invited from outside.
    ("agents/src/external.rs", 1, &["agent.slot.created", "agent.slot.updated"]),
    // What is said of a turn in the write of what it says: the one place
    // that says it there, with whose the turn is read from its row.
    ("agents/src/together.rs", 1, &["agent.turn.queued", "agent.turn.started", "agent.turn.finished"]),
    // An event said from outside, with a key and by a caller that signed: a
    // blueprint's own topic, never one of these families, which both refuse.
    ("api/src/routes/outside.rs", 2, &[]),
    // What an action says for itself: written where the action is.
    ("conductor/src/action.rs", 1, &[]),
    // The conductor's own, written where each is said.
    ("conductor/src/engine.rs", 1, &[]),
    // An `emit` step: a blueprint's own topic, never one of these families.
    ("conductor/src/intrinsic.rs", 1, &[]),
    // What the runner says of its own steps and runs, made ready kept or
    // passed on by what could hear it (`own_say`); and that a run began
    // or was taken up again, said with its row where it is passed on.
    ("conductor/src/owed.rs", 2, &["run.started", "run.resumed"]),
    // The ways a step's event is said (`RunCx::say`: by what could hear
    // it, or passed on for an instance that keeps nothing), each topic
    // written where it is said.
    ("conductor/src/run.rs", 3, &[]),
    ("gateway/src/hub.rs", 1, &["device.connected", "device.disconnected", "device.changed"]),
    // What a service says of itself, written where each is said; and
    // what its program says of its own in a line it prints, which is a
    // blueprint's own topic and never one of these families.
    ("hosting/src/service.rs", 2, &[]),
    // What is passed on and not kept, by itself or by the journal that
    // keeps what it names: whatever whoever passes it on handed it.
    ("journal/src/together.rs", 2, &[]),
    // The Trash's own two ways out of it (`take_out`).
    ("projects/src/trashed.rs", 1, &["trash.restored", "trash.gone"]),
    // What the runner says in the write of what it says, each topic
    // written where the event is made ready (`to_say`, the third of
    // these): said there in the write, passed on for an instance that
    // keeps nothing, or said by itself where nothing is written beside it,
    // each kept or passed on as it was made ready.
    ("runs/src/together.rs", 6, &[]),
    // An event said by hand to a sandbox, a factory of its own: whatever
    // topic whoever runs the test says, in the sandbox's own journal.
    ("sandbox/src/inside.rs", 1, &[]),
    ("studio/src/history.rs", 1, &["collection.history"]),
    ("studio/src/records.rs", 1, &["record.commented"]),
    // What is said of a record as it goes into the Trash or comes back,
    // made ready by `Say::new` where its topic is written, the pages that
    // go with a page among it.
    ("studio/src/trash.rs", 4, &[]),
    // What is said of a record in the write that keeps it, and after it
    // for a table the journal's write does not reach: each topic is
    // written where the event is made ready (`Say::new`, the last of
    // the names below).
    ("studio/src/whole.rs", 4, &[]),
    // What is said of lines that were read, made to wait unread again or
    // put away, by the one thing that says each with how many then wait.
    ("talk/src/inbox.rs", 1, &["inbox.read", "inbox.unread", "inbox.away"]),
    // A topic's own cells written through its own doors, each saying which.
    ("topics/src/lib.rs", 1, &["topic.followed"]),
    // What a topic's status came to, in the write that keeps its row.
    (
        "topics/src/revised.rs",
        1,
        &["topic.answered", "topic.closed", "topic.opened", "topic.standing", "topic.untracked"],
    ),
];

/// The names something is emitted through. The topic is the first thing
/// in quotes after one of them, when it is written there; `announce` is
/// what hosting and the gateway say a service's and a device's through,
/// `say` what a run says a step's through, `to_say`, `own_say` and
/// `Say::new` what make ready an event that is said in the write of what
/// it says, and `pass_on` what says one that is passed on and not kept.
const SAID_THROUGH: &[&str] = &[
    ".emit(",
    ".pass_on(",
    ".own_say(",
    ".emit_here(",
    ".emit_ephemeral(",
    ".emit_last(",
    ".announce(",
    ".say(",
    ".say_here(",
    "to_say(",
    "Say::new(",
];

#[tokio::test]
async fn every_event_this_program_emits_is_described() {
    let rig = Rig::new().await;
    let offered = rig.read("/api/actions").await;
    let described: BTreeSet<String> = offered["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["topic"].as_str().unwrap().into())
        .collect();
    assert!(described.len() > 80, "the events are handed out with the actions: {}", described.len());

    let mut written: BTreeMap<String, String> = BTreeMap::new();
    let mut elsewhere: BTreeMap<String, usize> = BTreeMap::new();
    let crates = repo().join("crates");
    let mut files = Vec::new();
    let mut folders = vec![crates.clone()];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            match path.is_dir() {
                true if path.file_name().is_some_and(|name| name != "tests" && name != "target") => {
                    folders.push(path);
                }
                false if path.extension().is_some_and(|extension| extension == "rs") => files.push(path),
                _ => {}
            }
        }
    }
    for file in files {
        let name = file.strip_prefix(&crates).unwrap().to_string_lossy().replace('\\', "/");
        let text = std::fs::read_to_string(&file).unwrap();
        for through in SAID_THROUGH {
            for (at, _) in text.match_indices(through) {
                let after = &text[at + through.len()..];
                let head: String = after.chars().take(160).collect();
                // `announce(&service, "service.started", …)`: past what it is said of.
                let head = head.trim_start();
                let head = match head.strip_prefix('&') {
                    Some(rest) => rest.split_once(',').map_or("", |(_, rest)| rest).trim_start(),
                    None => head,
                };
                match head.strip_prefix('"').and_then(|rest| rest.split_once('"')) {
                    Some((topic, _)) => {
                        written.insert(topic.to_string(), name.clone());
                    }
                    // Something else called `announce`, which says no topic.
                    None if *through == ".announce(" => {}
                    None => *elsewhere.entry(name.clone()).or_default() += 1,
                }
            }
        }
    }
    assert!(written.len() > 70, "the search still finds what is emitted: {}", written.len());
    for (topic, file) in &written {
        assert!(
            described.contains(topic),
            "`{topic}` is emitted in crates/{file} and is not described: add it to that crate's `events.rs`"
        );
    }
    let named: BTreeMap<String, usize> =
        NOT_WRITTEN_THERE.iter().map(|(file, places, _)| (file.to_string(), *places)).collect();
    assert_eq!(
        elsewhere, named,
        "the places an event is emitted with its topic not written there have changed. Each is named in \
         NOT_WRITTEN_THERE with the topics it can be, so that those are held to being described too"
    );
    for (file, _, topics) in NOT_WRITTEN_THERE {
        for topic in *topics {
            assert!(described.contains(*topic), "`{topic}` (crates/{file}) is not described");
        }
    }
    // And every one described is one this program has a place to emit.
    let known: BTreeSet<&str> = written
        .keys()
        .map(String::as_str)
        .chain(NOT_WRITTEN_THERE.iter().flat_map(|(_, _, topics)| topics.iter().copied()))
        .collect();
    for topic in &described {
        assert!(known.contains(topic.as_str()), "`{topic}` is described and nothing emits it");
    }
}

const HEARS: &str = "
blueprint: hears
permissions: [channels]
params:
  card: {}
on:
  moved:
    event: board.item.moved
    for: { id: \"${params.card}\" }
    when: ${ event.data.to == \"ready\" }
    do: channel.post
    with: { channel: general, body: \"${event.data.title} went to ${event.data.to}: ${ event.data.note ?? '' }\" }
";

async fn problems(rig: &Rig, source: &str) -> Vec<(String, String, String)> {
    let checked = rig.send("/api/blueprints/check", json!({ "source": source, "project": "shop" })).await;
    let text = |one: &Value, key: &str| one[key].as_str().unwrap_or_default().to_string();
    checked["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| (text(one, "severity"), text(one, "code"), text(one, "message")))
        .collect()
}

#[tokio::test]
async fn a_handler_is_held_to_the_events_there_are_and_to_what_they_carry() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    assert_eq!(
        problems(&rig, HEARS).await,
        vec![],
        "a handler that names what its event carries checks clean"
    );

    // A field under `for:` that the event does not carry: no event would be its.
    let found = problems(&rig, &HEARS.replace("for: { id:", "for: { card:")).await;
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].0.as_str(), found[0].1.as_str()), ("error", "unknown-field"));
    assert!(
        found[0].2.starts_with(
            "`for:` names `card`, which `board.item.moved` does not carry, so no event would ever be this \
             handler's; it carries: "
        ),
        "{}",
        found[0].2
    );
    assert!(
        found[0].2.contains("from, given, id, late, note, project, through, title, to"),
        "{}",
        found[0].2
    );

    // One read in `when:`, and one read in a step.
    let found = problems(&rig, &HEARS.replace("event.data.to == ", "event.data.too == ")).await;
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].2.starts_with(
            "`event.data.too` is not something `board.item.moved` carries; did you mean `to`?; it carries: "
        ),
        "{}",
        found[0].2
    );
    let found = problems(&rig, &HEARS.replace("${event.data.title} went", "${event.data.name} went")).await;
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].2.starts_with("`event.data.name` is not something `board.item.moved` carries"));

    // A topic that is nearly one of this program's.
    let found = problems(&rig, &HEARS.replace("event: board.item.moved", "event: board.item.move")).await;
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, "unknown-event");
    assert_eq!(found[0].2, "there is no event called `board.item.move`; did you mean `board.item.moved`?");
    let found =
        problems(&rig, &HEARS.replace("event: board.item.moved", "event: board.cards.shuffled")).await;
    assert!(
        found[0].2.starts_with("there is no event called `board.cards.shuffled`; under `board.` there are: "),
        "{}",
        found[0].2
    );
    assert!(found[0].2.contains("board.item.created"), "{}", found[0].2);

    // One that is passed on and not kept is heard by no handler.
    let passing = "blueprint: lines\non:\n  line:\n    event: log.line\n    do: noop\n";
    let found = problems(&rig, passing).await;
    assert_eq!(found[0].1, "unheard-event", "{found:?}");
    assert_eq!(
        found[0].2,
        "`log.line` is passed on to whoever is watching and is not kept, so no handler hears it"
    );
}

#[tokio::test]
async fn a_handler_on_a_group_of_topics_may_read_what_any_of_them_carries() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let group = HEARS.replace("event: board.item.moved", "event: board.item.*");
    // `to` and `note` are a move's alone, and `id` is every one's.
    assert_eq!(problems(&rig, &group).await, vec![]);
    let found = problems(&rig, &group.replace("event.data.to == ", "event.data.column == ")).await;
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].2.starts_with("`event.data.column` is not something `board.item.*` carries; it carries: "),
        "{}",
        found[0].2
    );
    let found = problems(&rig, &group.replace("event: board.item.*", "event: board.thing.*")).await;
    assert!(found[0].2.starts_with("`board.thing.*` takes no event there is; under `board.` there are: "));
    // A group that takes every family takes what nobody can list: nothing is held.
    let all =
        group.replace("event: board.item.*", "event: \"**\"").replace("event.data.to", "event.data.anything");
    assert_eq!(problems(&rig, &all).await, vec![]);
    // An instance's state is its blueprint's own, under names of its own.
    let state = "blueprint: states\non:\n  changed:\n    event: instance.state\n    when: ${ event.data.rounds > 2 }\n    do: noop\n";
    assert_eq!(problems(&rig, state).await, vec![]);
}

#[tokio::test]
async fn a_handler_that_says_its_event_comes_from_outside_is_not_held_to_a_blueprint_saying_it() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    // A payment taken, said by a service elsewhere with a key or by its
    // signature: no blueprint says it, and none has to.
    let paid = "
blueprint: takings
on:
  paid:
    event: shop.paid
    outside: true
    when: ${ event.data.total > 0 }
    do: noop
";
    assert_eq!(problems(&rig, paid).await, vec![], "what it carries is the sender's to choose");
    // Without the word, the check says what it always said, and now how
    // to answer it.
    let found = problems(&rig, &paid.replace("    outside: true\n", "")).await;
    assert_eq!((found.len(), found[0].1.as_str()), (1, "unsaid-topic"), "{found:?}");
    assert!(found[0].2.contains("say so with `outside: true`"), "{}", found[0].2);

    // What the system says of itself is never said from outside.
    let ours =
        paid.replace("event: shop.paid", "event: instance.failed").replace("event.data.total > 0", "true");
    let found = problems(&rig, &ours).await;
    assert_eq!((found[0].0.as_str(), found[0].1.as_str()), ("error", "outside-ours"), "{found:?}");
    assert!(found[0].2.starts_with("`instance.failed` is the system's own to say"), "{}", found[0].2);
    // And it is a word about an event: a timer is said by nobody.
    let timer = "blueprint: tick\non:\n  often:\n    every: 5m\n    outside: true\n    do: noop\n";
    let found = problems(&rig, timer).await;
    assert_eq!((found[0].0.as_str(), found[0].1.as_str()), ("error", "outside-what"), "{found:?}");
    let wrong = paid.replace("outside: true", "outside: sometimes");
    let found = problems(&rig, &wrong).await;
    assert!(found.iter().any(|one| one.2 == "`outside` must be true or false"), "{found:?}");
}

const SAYS: &str = "
blueprint: says
steps:
  tell:
    do: emit
    with: { topic: shop.restocked, sku: b-12, left: 40 }
";

const LISTENS: &str = "
blueprint: listens
on:
  restocked:
    event: shop.restocked
    for: { sku: b-12 }
    when: ${ event.data.left > 10 }
    do: noop
";

#[tokio::test]
async fn what_blueprints_say_to_each_other_is_held_to_what_the_one_that_says_it_writes() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    // Nobody says it yet: that may be right, and is said, not refused.
    let found = problems(&rig, LISTENS).await;
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].0.as_str(), found[0].1.as_str()), ("warning", "unsaid-topic"));
    assert!(
        found[0].2.starts_with(
            "nothing that can be seen here says `shop.restocked`: no blueprint emits it. If something \
             outside does, say so with `outside: true`; if not, this handler never runs."
        ),
        "{}",
        found[0].2
    );

    put(&rig, "says", SAYS).await;
    assert_eq!(
        problems(&rig, LISTENS).await,
        vec![],
        "the project's own blueprint says it, with these fields"
    );

    let found = problems(&rig, &LISTENS.replace("for: { sku:", "for: { item:")).await;
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].2.starts_with("`for:` names `item`, which `shop.restocked` does not carry"),
        "{}",
        found[0].2
    );
    assert!(found[0].2.ends_with("it carries: left, sku"), "{}", found[0].2);
    let found = problems(&rig, &LISTENS.replace("event.data.left", "event.data.count")).await;
    assert!(found[0].2.starts_with("`event.data.count` is not something `shop.restocked` carries"));
    // A slip in the topic is pointed at what is said.
    let found = problems(&rig, &LISTENS.replace("event: shop.restocked", "event: shop.restoked")).await;
    assert_eq!(found[0].1, "unsaid-topic");
    assert!(found[0].2.ends_with("Did you mean `shop.restocked`?"), "{}", found[0].2);

    // One that says it itself is read for it too, and a blueprint that
    // writes what cannot be read off its text leaves nothing to hold to.
    let both = format!(
        "{}on:\n  heard:\n    event: shop.restocked\n    when: ${{ event.data.nope }}\n    do: noop\n",
        SAYS.replace("blueprint: says", "blueprint: both")
    );
    let found = problems(&rig, &both).await;
    assert!(
        found[0].2.starts_with("`event.data.nope` is not something `shop.restocked` carries"),
        "{found:?}"
    );
    let open = SAYS
        .replace("with: { topic: shop.restocked, sku: b-12, left: 40 }", "with: \"${ params.said }\"")
        .replace("steps:", "params:\n  said: {}\nsteps:");
    put(&rig, "says", &open).await;
    assert_eq!(problems(&rig, &LISTENS.replace("event.data.left", "event.data.count")).await.len(), 1);
}

/// The row of the notes' table for one event: its topic, what it is, and
/// what it carries, with `?` after what is there only some of the time.
fn row(event: &Value) -> String {
    let text = |key: &str| event[key].as_str().unwrap_or_default();
    let mut carries: Vec<String> = event["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| {
            let sometimes = if field["always"] == true { "" } else { "?" };
            format!("`{}`{sometimes}", field["name"].as_str().unwrap())
        })
        .collect();
    if event["open"] == true {
        carries.push("names of its emitter's own".into());
    }
    let carries = if carries.is_empty() { "nothing".to_string() } else { carries.join(", ") };
    let kept = if event["kept"] == false { " Passed on, not kept." } else { "" };
    format!("| `{}` | {}{kept} | {carries} |", text("topic"), text("summary"))
}

#[tokio::test]
async fn the_notes_list_every_event_as_it_is_described() {
    let rig = Rig::new().await;
    let offered = rig.read("/api/actions").await;
    let rows: Vec<String> = offered["events"].as_array().unwrap().iter().map(row).collect();
    let docs = std::fs::read_to_string(repo().join("docs/blueprints.md")).unwrap();
    let missing: Vec<&String> = rows.iter().filter(|row| !docs.contains(row.as_str())).collect();
    assert!(
        missing.is_empty(),
        "docs/blueprints.md is out of step with the events as they are described. Its table (under \
         *Events*) should have these rows:\n{}",
        missing.iter().map(|row| row.as_str()).collect::<Vec<_>>().join("\n")
    );
    let listed = docs.lines().filter(|line| line.starts_with("| `") && line.matches(" | ").count() == 2);
    let stale: Vec<&str> = listed
        .filter(|line| {
            let topic = line.trim_start_matches("| `").split('`').next().unwrap_or_default();
            blueprint::events::own(topic) && topic.contains('.') && !rows.iter().any(|row| row == line)
        })
        .collect();
    assert!(
        stale.is_empty(),
        "docs/blueprints.md lists events that are not as described:\n{}",
        stale.join("\n")
    );
}
