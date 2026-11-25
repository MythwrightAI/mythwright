//! `instances`, `instance` and `apply --instance`: which text each
//! instance runs, which are behind the one now there, and applying one
//! again as it is, from the terminal.

use kernel::Source;
use serde_json::json;

use crate::{Rig, soon};

/// Starts a worker when it is asked for one; none of its steps names it.
const WATCHER: &str = "\
blueprint: watcher
on:
  wanted:
    event: shop.wanted
    steps:
      go: { spawn: worker, as: the-worker, with: { room: general } }
";

fn worker(version: u32, claims: &str) -> String {
    format!(
        "blueprint: worker\nversion: {version}\nparams:\n  room: {{ type: string }}\nsteps:\n  claim: {{ do: emit, with: {{ \
         topic: {claims} }} }}\non:\n  poke:\n    event: shop.poke\n    do: noop\n"
    )
}

/// The row of `instances` that is about the instance with this key, and
/// the lines set in under it.
fn about(listed: &str, key: &str) -> (String, Vec<String>) {
    let mut lines = listed.lines().skip_while(|line| !line.contains(key));
    let row = lines.next().unwrap_or_else(|| panic!("no `{key}` in:\n{listed}")).to_string();
    (row, lines.take_while(|line| line.starts_with("    ")).map(str::to_string).collect())
}

#[tokio::test]
async fn the_terminal_says_which_text_each_instance_runs_and_which_are_behind_and_applies_one_again_as_it_is()
{
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let blueprints = rig.placed("shop");
    std::fs::create_dir_all(&blueprints).unwrap();
    let file = |name: &str| blueprints.join(brand::blueprint_file(name));
    let first = worker(1, "shop.claimed");
    std::fs::write(file("watcher"), WATCHER).unwrap();
    std::fs::write(file("worker"), &first).unwrap();
    rig.run(&["apply", file("watcher").to_str().unwrap(), "--project", "shop"]).await.ok();

    let journal = &rig.node.as_ref().unwrap().app.journal;
    let mut active = journal.follow_every(Default::default(), "instance.active");
    journal.emit("shop.wanted", Source::default(), json!({})).await.unwrap();
    soon("the watcher and the worker to settle", async {
        for _ in 0..2 {
            active.next().await.unwrap();
        }
    })
    .await;

    let short = |text: &str| blueprint::revision(text)[..8].to_string();
    let listed = rig.run(&["instances"]).await.ok();
    assert_eq!(listed.lines().count(), 2, "a row each and nothing under them:\n{listed}");
    let (row, under) = about(&listed, "the-worker");
    assert!(row.contains(&format!("  worker   the-worker  active  {} v1  ", short(&first))), "{listed}");
    assert!(under.is_empty() && !listed.contains("behind"), "{listed}");
    let (row, _) = about(&listed, "watcher");
    assert!(row.contains(&format!("  active  {}  ", short(WATCHER))), "one that says no version:\n{listed}");
    let id = about(&listed, "the-worker").0.split_whitespace().next().unwrap().to_string();

    // Somebody saves the worker's file with other steps.
    let second = worker(2, "shop.claimed_again");
    std::fs::write(file("worker"), &second).unwrap();
    let listed = rig.run(&["instances"]).await.ok();
    let (row, under) = about(&listed, "the-worker");
    assert!(row.contains(&format!("  active  {} v1 behind  ", short(&first))), "{listed}");
    let why = "    The text of `worker` now there begins otherwise than the one this runs: its steps, its \
               params, or something else it says at the top. It runs that text only when it is itself \
               applied again, which runs its steps again.";
    // And what moved between the two, in words, for whoever decides whether to take it.
    let moved = "    worker 1 to 2: the step `claim` is changed";
    assert_eq!(under, [why, moved], "{listed}");
    assert!(about(&listed, "watcher").1.is_empty(), "{listed}");
    let one = rig.run(&["instance", &id]).await.ok();
    assert!(one.contains(&format!("\n  text {} v1 behind\n{why}\n{moved}\n", short(&first))), "{one}");

    // Applied again as it is: its steps run again, in the text now there.
    let again = rig.run(&["apply", "--instance", &id]).await.ok();
    assert!(
        again.contains(&format!("instance {id} updated, its steps run again; run run_"))
            && again.contains(" ok in "),
        "{again}"
    );
    let listed = rig.run(&["instances"]).await.ok();
    let (row, under) = about(&listed, "the-worker");
    assert!(row.contains(&format!("  active  {} v2  ", short(&second))) && under.is_empty(), "{listed}");
    assert_eq!(listed.lines().count(), 2, "and it is the same instance:\n{listed}");
    let same = rig.run(&["apply", "--instance", &id]).await.ok();
    assert!(same.contains("is already running this; nothing to do"), "{same}");

    // Saved again with only what it does later changed: applied again, it
    // takes that where it is, and the terminal says nothing was run.
    let third = second.replace("event: shop.poke", "event: shop.prod").replace("version: 2", "version: 3");
    std::fs::write(file("worker"), &third).unwrap();
    let listed = rig.run(&["instances"]).await.ok();
    let (_, under) = about(&listed, "the-worker");
    assert!(
        under.len() == 2
            && under[0].ends_with("when what started it, or it itself, is applied again.")
            && under[1] == "    worker 2 to 3: the handler `poke` is changed",
        "{listed}"
    );
    let taken = rig.run(&["apply", "--instance", &id]).await.ok();
    assert_eq!(
        taken.trim(),
        format!("instance {id} took the text now there where it is; none of its steps ran again")
    );
    let listed = rig.run(&["instances"]).await.ok();
    let (row, under) = about(&listed, "the-worker");
    assert!(row.contains(&format!("  active  {} v3  ", short(&third))) && under.is_empty(), "{listed}");

    let nothing = rig.run(&["apply", "--instance", "in_nothing"]).await;
    assert!(
        nothing.code != 0 && nothing.err.contains("there is no instance `in_nothing`"),
        "{}",
        nothing.err
    );
}

#[tokio::test]
async fn applying_what_is_still_at_work_says_so_and_names_the_run_that_is_going() {
    let rig = Rig::signed_in().await;
    let dir = crate::scratch("patient");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(brand::blueprint_file("patient"));
    let source = "\
blueprint: patient
steps:
  begin: { do: emit, with: { topic: shop.begun } }
  hold: { do: wait, with: { event: shop.go } }
";
    std::fs::write(&file, source).unwrap();
    let file = file.to_str().unwrap();
    let journal = &rig.node.as_ref().unwrap().app.journal;
    let mut holding = journal.follow_every(Default::default(), "step.started");

    let first = rig.run(&["apply", file, "--no-wait"]).await.ok();
    let run = first.trim().rsplit(' ').next().unwrap().to_string();
    assert!(first.contains(" started; run run_"), "{first}");
    soon("it to be waiting", async {
        while holding.next().await.unwrap().source.step.as_deref() != Some("hold") {}
    })
    .await;

    // Asked for again and not waited for: it is said to be at it, with the
    // run that is going.
    let again = rig.run(&["apply", file, "--no-wait"]).await.ok();
    let said = format!("is already in the middle of this, so nothing was started over; run {run}");
    assert!(again.trim().ends_with(&said), "{again}");

    // Let go, it ends as the one run it ever was.
    let mut finished = journal.follow_every(Default::default(), "run.finished");
    journal.emit("shop.go", Source::default(), json!({})).await.unwrap();
    let ended = soon("it to end", finished.next()).await.unwrap();
    assert_eq!(
        (ended.source.run.as_deref(), ended.data["status"].as_str()),
        (Some(run.as_str()), Some("ok"))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn applying_says_what_the_check_would_have_warned_of() {
    let rig = Rig::signed_in().await;
    let dir = crate::scratch("warned");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(brand::blueprint_file("pool"));
    // A handler that tries to wake itself, and one nothing wakes.
    let source = "\
blueprint: pool
on:
  look:
    event: pool.look
    steps:
      again: { do: emit, with: { topic: pool.look } }
  never: { event: pool.nobody_says_this, do: noop }
";
    std::fs::write(&file, source).unwrap();
    let applied = rig.run(&["apply", file.to_str().unwrap()]).await;
    assert_eq!(applied.code, 0, "a warning refuses nothing: {}", applied.err);
    assert!(applied.out.contains(" started; run run_"), "{}", applied.out);
    assert!(
        applied.err.contains("warning: line 3: `look` says `pool.look`, which is what it listens for"),
        "{}",
        applied.err
    );
    assert!(
        applied.err.contains("warning: line 7: nothing that can be seen here says `pool.nobody_says_this`"),
        "{}",
        applied.err
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn an_event_is_said_from_the_terminal_and_a_handler_hears_it() {
    let rig = Rig::signed_in().await;
    let dir = crate::scratch("said");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(brand::blueprint_file("shop"));
    let source = "\
blueprint: shop
state: { last: none, total: 0 }
on:
  paid:
    event: shop.paid
    do: set
    with: { last: \"${ event.data.order }\", total: ${ event.data.total } }
";
    std::fs::write(&file, source).unwrap();
    rig.run(&["apply", file.to_str().unwrap()]).await.ok();
    let journal = &rig.node.as_ref().unwrap().app.journal;
    let mut handled = journal.follow_every(Default::default(), "run.finished");

    let said = rig.run(&["emit", "shop.paid", "order=o-17", "total=40"]).await.ok();
    assert!(said.starts_with("said shop.paid ("), "{said}");
    soon("the handler", async { while handled.next().await.unwrap().data["flow"] != "on.paid" {} }).await;
    let one = rig.run(&["instances"]).await.ok();
    let id = one.split_whitespace().next().unwrap().to_string();
    let shown = rig.run(&["instance", &id]).await.ok();
    assert!(shown.contains(r#""last":"o-17""#) && shown.contains(r#""total":40"#), "{shown}");

    // What the system says of itself is not for saying.
    let refused = rig.run(&["emit", "instance.failed", "error=no"]).await;
    assert!(
        refused.code != 0 && refused.err.contains("only the system may publish under"),
        "{}",
        refused.err
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn a_service_that_signs_is_described_listed_and_taken_away_from_the_command_line() {
    let rig = Rig::signed_in().await;
    let nobody = rig.run(&["caller", "list"]).await.ok();
    assert!(nobody.contains("no service outside is let in by its signature"), "{nobody}");

    let kept = rig.dir.join("signing-secret.txt");
    std::fs::write(&kept, "s3cr3t-of-the-payments-service").unwrap();
    rig.run(&["secret", "set", "pay-signing", "--file", kept.to_str().unwrap()]).await.ok();
    // Said with something missing, it says what, and where one is shown whole.
    let short = rig.run(&["caller", "set", "payments", "--topic", "shop.paid"]).await;
    assert!(short.code != 0 && short.err.contains("missing `--secret`"), "{}", short.err);
    let described = [
        "caller",
        "set",
        "payments",
        "--topic",
        "shop.paid",
        "--secret",
        "pay-signing",
        "--header",
        "Pay-Signature",
        "--prefix",
        "v1=",
        "--time-prefix",
        "t=",
        "--signed",
        "{time}.{body}",
    ];
    let made = rig.run(&described).await.ok();
    assert!(made.contains("/in/payments") && made.contains("said as `shop.paid`"), "{made}");
    assert!(!made.contains("s3cr3t"), "{made}");

    let listed = rig.run(&["caller", "list"]).await.ok();
    for part in ["payments", "says shop.paid", "signs with pay-signing", "good for 5m", "/in/payments"] {
        assert!(listed.contains(part), "{part}: {listed}");
    }
    // One that would prove nothing is refused in the daemon's words.
    let unsigned = rig
        .run(&[
            "caller",
            "set",
            "loose",
            "--topic",
            "shop.paid",
            "--secret",
            "pay-signing",
            "--header",
            "X-Sig",
            "--time-header",
            "X-Time",
        ])
        .await;
    assert!(unsigned.code != 0 && unsigned.err.contains("a time that was not signed"), "{}", unsigned.err);

    let gone = rig.run(&["caller", "remove", "payments"]).await.ok();
    assert!(gone.contains("its address answers nobody"), "{gone}");
    let nobody = rig.run(&["caller", "list"]).await.ok();
    assert!(nobody.contains("no service outside"), "{nobody}");
}
