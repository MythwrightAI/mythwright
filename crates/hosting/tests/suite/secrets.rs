//! Secrets: a value a program needs, kept by the factory and handed to the
//! program a step or a service runs, in its environment and nowhere else.
//! It is not in the blueprint, the database, an event or a log.

use conductor::RunStatus;
use hosting::Status;
use serde_json::json;

use crate::standin::{Hold, cmd};
use crate::{Rig, line, step};

const VALUE: &str = "sk_live_abc123";

/// Puts what the secret should be where the program of a test can compare
/// it with what it was handed: in a file of the test's own, so that the
/// value is in nothing the factory was given to write down.
fn expecting(rig: &Rig) {
    std::fs::write(rig.path("expected"), VALUE).unwrap();
}

/// Everything the factory wrote down, as text: every event, every step of
/// every run, and what it keeps of its services.
async fn written_down(rig: &Rig) -> String {
    let mut all = String::new();
    for event in rig.events("**").await {
        all.push_str(&serde_json::to_string(&event).unwrap());
    }
    for instance in rig.engine.instances(conductor::InstanceFilter::default()).await.unwrap() {
        all.push_str(&serde_json::to_string(&instance).unwrap());
        for run in rig.engine.runs(&instance.id, 50).await.unwrap() {
            all.push_str(&serde_json::to_string(&rig.engine.run(&run.id).await.unwrap()).unwrap());
        }
    }
    let kept: Vec<String> = rig
        .journal
        .db()
        .call(|connection| {
            let mut statement = connection.prepare("SELECT spec || proc FROM services")?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            rows.collect()
        })
        .await
        .unwrap();
    all.push_str(&kept.join(""));
    all
}

#[tokio::test]
async fn a_secret_is_handed_to_the_program_a_step_runs_and_is_written_nowhere_else() {
    let rig = Rig::new().await;
    rig.hosting.secrets().unwrap().set("stripe", VALUE).unwrap();
    expecting(&rig);
    // It goes on only where what it was handed is what was expected.
    let pays = cmd(&["holds", "expected", "%STRIPE_KEY%", "say", "paid in %REGION%"]);
    let source = format!(
        "
blueprint: charge
permissions: [shell, secrets]
steps:
  pay:
    do: shell
    env:
      STRIPE_KEY: {{ secret: stripe }}
      REGION: eu
    with:
      cmd: {pays}
"
    );
    let run = rig.run(&source, json!({})).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(step(&run, "pay").output["stdout"], "paid in eu");
    assert!(!written_down(&rig).await.contains(VALUE), "the value is in nothing that was written down");
    // It is listed by its name, and that is all that is ever said of it.
    let kept = rig.hosting.secrets().unwrap().list();
    assert_eq!(kept.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["stripe"]);
}

#[tokio::test]
async fn a_param_says_which_secret_a_program_is_handed_and_the_empty_text_says_none() {
    let rig = Rig::new().await;
    rig.hosting.secrets().unwrap().set("stripe", VALUE).unwrap();
    expecting(&rig);
    // Where the variable is not there at all it says so; where it is, it
    // goes on only if it holds what was expected.
    let pays =
        cmd(&["lacking", "STRIPE_KEY", "handed nothing", "holds", "expected", "%STRIPE_KEY%", "say", "paid"]);
    let source = &format!(
        "
blueprint: charge
permissions: [shell, secrets]
params:
  key: {{ type: string, default: \"\" }}
steps:
  pay:
    do: shell
    env:
      STRIPE_KEY: {{ secret: \"${{params.key}}\" }}
    with:
      cmd: {pays}
"
    );
    let told = rig.run(source, json!({ "key": "stripe" })).await;
    assert_eq!(step(&told, "pay").output["stdout"], "paid", "{:?}", told.error);
    // Told none, the variable is not there at all: not empty, and not an
    // error about a secret with no name.
    let untold = rig.run(source, json!({})).await;
    assert_eq!(untold.status, RunStatus::Ok, "{:?}", untold.error);
    assert_eq!(step(&untold, "pay").output["stdout"], "handed nothing");
    assert!(!written_down(&rig).await.contains(VALUE), "the value is in nothing that was written down");
}

#[tokio::test]
async fn a_secret_that_is_not_there_fails_the_step_that_wants_it_and_says_what_to_do() {
    let rig = Rig::new().await;
    let source = "
blueprint: charge
permissions: [shell, secrets]
steps:
  pay: { do: shell, env: { STRIPE_KEY: { secret: stripe } }, with: { run: echo paid } }
";
    let run = rig.run(source, json!({})).await;
    assert_eq!(run.status, RunStatus::Failed);
    let why = step(&run, "pay").error.clone().unwrap_or_default();
    assert!(why.contains("there is no secret called `stripe`") && why.contains("secret set stripe"), "{why}");
}

#[tokio::test]
async fn a_blueprint_that_does_not_say_it_uses_secrets_is_not_handed_one() {
    let rig = Rig::new().await;
    rig.hosting.secrets().unwrap().set("stripe", VALUE).unwrap();
    let source = "
blueprint: sly
permissions: [shell]
steps:
  pay: { do: shell, env: { STRIPE_KEY: { secret: stripe } }, with: { run: echo $STRIPE_KEY } }
";
    let refused = rig.engine.apply(rig.request(source, json!({}))).await.unwrap_err();
    assert!(
        refused.message.contains("this uses `secrets:use`, which is not listed under `permissions`"),
        "{}",
        refused.message
    );
}

#[tokio::test]
async fn a_service_is_given_its_secret_each_time_it_is_started_and_what_is_kept_of_it_is_the_name() {
    let rig = Rig::new().await;
    rig.hosting.secrets().unwrap().set("stripe", VALUE).unwrap();
    expecting(&rig);
    // It says it has what was expected, ends badly the first time it is
    // started, and goes on the second.
    let serves = cmd(&["holds", "expected", "%STRIPE_KEY%", "say", "has it", "first", "again", "3", "idle"]);
    let source = format!(
        "
blueprint: checkout
permissions: [fleet, shell, secrets]
steps:
  serve:
    do: service.start
    env:
      STRIPE_KEY: {{ secret: stripe }}
    with:
      name: checkout
      restart_backoff: 10ms
      cmd: {serves}
"
    );
    let mut lines = rig.lines();
    let applied = rig.apply(&source, json!({})).await;
    assert_eq!(rig.finished(&applied).await.status, RunStatus::Ok);
    // Started, it has it; ended and started again, it has it again.
    line(&mut lines, "has it").await;
    rig.event("service.exited", |_| true).await;
    line(&mut lines, "has it").await;
    assert_eq!(rig.hosting.services()[0].status, Status::Healthy);
    assert!(
        !written_down(&rig).await.contains(VALUE),
        "what is kept of the service names the secret and no more"
    );

    rig.engine.stop(&applied.instance).await.unwrap();
    rig.event("service.stopped", |_| true).await;
}

#[tokio::test]
async fn a_secret_is_sealed_where_it_is_kept_and_opens_only_as_itself_and_only_here() {
    let rig = Rig::new().await;
    let kept = rig.hosting.secrets().unwrap();
    kept.set("stripe", VALUE).unwrap();
    assert_eq!(kept.get("stripe").unwrap(), VALUE);

    // What is on the disk is not the value, nor anything it can be read from.
    let file = rig.path(".secrets").join("stripe");
    let on_disk = std::fs::read(&file).unwrap();
    assert!(!String::from_utf8_lossy(&on_disk).contains(VALUE));
    assert!(on_disk.starts_with(b"sealed 1\n"), "{:?}", &on_disk[..12.min(on_disk.len())]);
    // Set again to the same thing, it is sealed afresh: two files of one
    // value are not alike.
    kept.set("stripe", VALUE).unwrap();
    assert_ne!(std::fs::read(&file).unwrap(), on_disk);

    // Put under another name, it does not open: one secret is not made to
    // stand in for another by moving files.
    std::fs::copy(&file, rig.path(".secrets").join("mail")).unwrap();
    let moved = kept.get("mail").unwrap_err();
    assert!(moved.message.contains("does not open"), "{}", moved.message);
    assert!(!moved.message.contains(VALUE));

    // Nor does another installation open it, though it has the file.
    let other = Rig::new().await;
    std::fs::create_dir_all(other.path(".secrets")).unwrap();
    std::fs::copy(&file, other.path(".secrets").join("stripe")).unwrap();
    let theirs = other.hosting.secrets().unwrap().get("stripe").unwrap_err();
    assert!(theirs.message.contains("sealed by another installation"), "{}", theirs.message);

    // One written as it is, by a build that did not seal or by hand, is
    // read as it is, and is sealed where it is once it has been read.
    std::fs::write(rig.path(".secrets").join("old"), "plain-as-it-was").unwrap();
    assert_eq!(kept.get("old").unwrap(), "plain-as-it-was");
    assert!(std::fs::read(rig.path(".secrets").join("old")).unwrap().starts_with(b"sealed 1\n"));
    assert_eq!(kept.get("old").unwrap(), "plain-as-it-was");
}

#[tokio::test]
async fn a_projects_environment_keeps_its_secrets_apart_from_the_factorys_and_each_opens_only_as_itself() {
    let rig = Rig::new().await;
    let kept = rig.hosting.secrets().unwrap();
    kept.set_of("shop", "prod", "stripe", VALUE).unwrap();
    assert_eq!(kept.get_of("shop", "prod", "stripe").unwrap(), VALUE);
    assert!(kept.has_of("shop", "prod", "stripe") && !kept.has_of("shop", "dev", "stripe"));

    // Never one of the factory's: not listed with them, and not found by
    // a name a blueprint could say, worked out or not.
    assert!(kept.list().is_empty(), "{:?}", kept.list().iter().map(|one| &one.name).collect::<Vec<_>>());
    for said in ["shop/prod/stripe", "@projects", "stripe"] {
        let refused = kept.get(said).unwrap_err();
        assert!(!refused.message.contains(VALUE), "{said}: {}", refused.message);
    }
    assert!(kept.set("shop/prod/stripe", "x").is_err());

    // Sealed under where it is: put in another environment's place, or
    // another project's, it does not open.
    let file = rig.path(".secrets").join("@projects").join("shop").join("prod").join("stripe");
    let on_disk = std::fs::read(&file).unwrap();
    assert!(on_disk.starts_with(b"sealed 1\n") && !String::from_utf8_lossy(&on_disk).contains(VALUE));
    for (project, environment) in [("shop", "dev"), ("blog", "prod")] {
        let there = rig.path(".secrets").join("@projects").join(project).join(environment);
        std::fs::create_dir_all(&there).unwrap();
        std::fs::copy(&file, there.join("stripe")).unwrap();
        let moved = kept.get_of(project, environment, "stripe").unwrap_err();
        assert!(moved.message.contains("does not open"), "{project}/{environment}: {}", moved.message);
    }

    // One the list names and the folder does not have says what to do.
    let missing = kept.get_of("shop", "staging", "stripe").unwrap_err();
    assert!(missing.message.contains("has no value for it"), "{}", missing.message);
    assert!(
        kept.set_of("../up", "prod", "x", "y").is_err() && kept.set_of("shop", "Prod", "x", "y").is_err()
    );

    // Taken away one by one, an environment at a time, and a project's all.
    assert!(kept.remove_of("shop", "prod", "stripe").unwrap());
    assert!(!kept.remove_of("shop", "prod", "stripe").unwrap());
    kept.set_of("shop", "prod", "stripe", VALUE).unwrap();
    kept.forget_environment("shop", "dev").unwrap();
    assert!(kept.has_of("shop", "prod", "stripe") && !kept.has_of("shop", "dev", "stripe"));
    kept.forget_project("shop").unwrap();
    assert!(!rig.path(".secrets").join("@projects").join("shop").exists());
    assert!(rig.path(".secrets").join("@projects").join("blog").exists(), "another project's is left");
}

/// Everything in the folder programs started here write to, as text: what
/// each printed, as it lies on the disk.
fn left_on_the_disk(rig: &Rig) -> String {
    let mut all = String::new();
    for entry in std::fs::read_dir(rig.path(".run")).into_iter().flatten().flatten() {
        all.push_str(&String::from_utf8_lossy(&std::fs::read(entry.path()).unwrap_or_default()));
    }
    all
}

const MARK: &str = "[the secret `stripe`]";

#[tokio::test]
async fn a_program_that_prints_what_it_was_handed_has_the_secrets_name_written_down_and_never_its_value() {
    let rig = Rig::new().await;
    rig.hosting.secrets().unwrap().set("stripe", VALUE).unwrap();
    // It says its key where it prints, where it complains, and in the last
    // line it fails with, which is what is said of the failure.
    let leaks = cmd(&[
        "say",
        "paying with %STRIPE_KEY% now",
        "err",
        "refused: %STRIPE_KEY% is not a key",
        "exit",
        "3",
    ]);
    let source = format!(
        "
blueprint: leaks
permissions: [shell, secrets]
steps:
  pay:
    do: shell
    env:
      STRIPE_KEY: {{ secret: stripe }}
    with:
      cmd: {leaks}
"
    );
    let mut lines = rig.lines();
    let run = rig.run(&source, json!({})).await;
    assert_eq!(run.status, RunStatus::Failed);
    let pay = step(&run, "pay");
    // What it gave, what is said of how it failed, and each line of its log.
    assert!(
        pay.error.as_deref().unwrap_or_default().contains(&format!("refused: {MARK} is not a key")),
        "{pay:?}"
    );
    line(&mut lines, &format!("paying with {MARK} now")).await;
    line(&mut lines, &format!("refused: {MARK} is not a key")).await;
    let kept = written_down(&rig).await;
    assert!(!kept.contains(VALUE), "the value is in nothing that was written down");
    assert!(kept.contains(MARK));
    assert!(!left_on_the_disk(&rig).contains(VALUE), "nor in what the program's output lay in");
}

#[tokio::test]
async fn a_step_handed_a_secret_is_over_when_whatever_holds_its_output_has_said_its_last() {
    let rig = Rig::new().await;
    rig.hosting.secrets().unwrap().set("stripe", VALUE).unwrap();
    // What the program leaves behind is held twice, and each time comes
    // to the test by a connection of its own and waits there until it is
    // let go: so nothing waits to be opened, a holding is one program's
    // and no other's, and however the test ends, what was left behind
    // finds its connection closed and goes.
    let hold = Hold::new().await;
    let at = hold.address();
    // The program ends at once, and leaves behind something that still
    // holds its output and says one thing more each time it is let go.
    let last = "the last of it, with %STRIPE_KEY%";
    let lingers = cmd(&[
        "also", "(", "held", &at, "first", "say", "more", "held", &at, "second", "err", last, ")", "say",
        "early",
    ]);
    let source = format!(
        "
blueprint: lingers
permissions: [shell, secrets]
steps:
  pay:
    do: shell
    env:
      STRIPE_KEY: {{ secret: stripe }}
    with:
      cmd: {lingers}
"
    );
    let mut lines = rig.lines();
    let applied = rig.apply(&source, json!({})).await;
    // The program itself is over by the time what it said has been read.
    // What it left behind then says something more: its output grows
    // while its end is being waited for, which is when the waiting used to
    // be let go of and begun again as if nothing more were to come.
    line(&mut lines, "early").await;
    let first = hold.came().await;
    assert_eq!(first.who, "first");
    first.go().await;
    line(&mut lines, "more").await;
    let second = hold.came().await;
    assert_eq!(second.who, "second");
    second.go().await;
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(step(&run, "pay").output["stderr"], format!("the last of it, with {MARK}"));
}

/// Ends what a test's services run when the test is over, however it
/// ended. A test that starts a service stops the instance that owns it,
/// and then there is nothing left for this to do; one that fails before
/// it gets there would leave a program running that nobody ends, since a
/// factory leaves its services running on purpose. A service is ended by
/// its wrapper's group, and only while that process still says it is
/// that service's: a number alone may be another program's by then.
struct Ended<'a>(&'a Rig);

impl Drop for Ended<'_> {
    fn drop(&mut self) {
        for service in self.0.hosting.services() {
            let said = std::fs::read(format!("/proc/{}/environ", service.pid)).unwrap_or_default();
            let its_own = format!("svc.{}.", service.name);
            // Where a group is reached by its number (Unix), the number
            // has to be shown to be this service's still. Where it is a
            // hold this process has (Windows), nothing but what this
            // process started is ended by letting go of it.
            let still_its = said.windows(its_own.len()).any(|part| part == its_own.as_bytes());
            if service.pid > 1 && (still_its || cfg!(windows)) {
                // Said, so that what a failed test printed shows what was ended for it.
                eprintln!(
                    "the test was over with `{}` still running; its group {} was ended",
                    service.name, service.pid
                );
                let _ = hosting::group::stop(service.pid, true);
            }
        }
    }
}

#[tokio::test]
async fn a_service_that_prints_what_it_was_handed_has_a_log_without_it_on_the_disk_too() {
    let rig = Rig::new().await;
    let _ended = Ended(&rig);
    rig.hosting.secrets().unwrap().set("stripe", VALUE).unwrap();
    let paid = r#"::event {"topic": "shop.paid", "with": "%STRIPE_KEY%"}"#;
    let serves = cmd(&["say", "listening, key %STRIPE_KEY%", "say", paid, "idle"]);
    let source = format!(
        "
blueprint: checkout
permissions: [fleet, shell, secrets]
steps:
  serve:
    do: service.start
    env:
      STRIPE_KEY: {{ secret: stripe }}
    with:
      name: checkout
      restart: never
      cmd: {serves}
"
    );
    let mut lines = rig.lines();
    let applied = rig.apply(&source, json!({})).await;
    assert_eq!(rig.finished(&applied).await.status, RunStatus::Ok);
    line(&mut lines, &format!("listening, key {MARK}")).await;
    // What it says as an event of its own carries the name, and not the value.
    let paid = rig.event("shop.paid", |_| true).await;
    assert_eq!(paid.data["with"], MARK);
    // Its log is a file it goes on writing to: that file never held the value.
    let disk = left_on_the_disk(&rig);
    assert!(disk.contains(&format!("listening, key {MARK}")) && !disk.contains(VALUE), "{disk}");
    assert!(!written_down(&rig).await.contains(VALUE));

    rig.engine.stop(&applied.instance).await.unwrap();
    rig.event("service.stopped", |_| true).await;
}

#[test]
fn what_was_handed_is_taken_out_whole_and_line_by_line_and_a_short_word_is_left() {
    use hosting::unsaid::{Unsaid, mark};
    let armoured = "-----BEGIN KEY-----\nMIIBVgIBADANBgkq\nhkiG9w0BAQEFAASC\n-----END KEY-----";
    let unsaid = Unsaid::of([
        (VALUE.to_string(), mark("stripe")),
        // One that holds another is taken whole, whichever was said first.
        (format!("{VALUE}-test"), mark("stripe-test")),
        (armoured.to_string(), mark("deploy")),
        // Too short to be told from ordinary words: taking it out of every
        // line would say more of it than leaving it does.
        ("on".to_string(), mark("flag")),
    ]);
    assert_eq!(unsaid.from("nothing of it here, only on and off"), "nothing of it here, only on and off");
    assert_eq!(
        unsaid.from(&format!("key={VALUE} and {VALUE}-test")),
        "key=[the secret `stripe`] and [the secret `stripe-test`]"
    );
    // A key of several lines is printed a line at a time.
    assert_eq!(unsaid.from("  MIIBVgIBADANBgkq"), "  [the secret `deploy`]");
    assert_eq!(unsaid.from(armoured), "[the secret `deploy`]");
    let mut given = json!({ "stdout": format!("a {VALUE}"), "rows": [{ "said": VALUE }], "code": 3 });
    unsaid.from_all(&mut given);
    assert_eq!(given, json!({ "stdout": "a [the secret `stripe`]", "rows": [{ "said": MARK }], "code": 3 }));
    assert!(Unsaid::default().is_empty() && !unsaid.is_empty());
}
