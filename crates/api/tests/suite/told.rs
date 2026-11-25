//! What a run keeps of the event that set it off is read by whoever may
//! be told that event, and by nobody else.
//!
//! A handler hears what whoever applied its text may read, and its run
//! keeps the whole event and what each step gave. Whoever may read runs
//! is not thereby somebody who may read what the event was about: a
//! record of a table kept private, said to those who hold what it is
//! kept under. So a run is shown by the one question an event is handed
//! out by ([`journal::Journal::may_tell`], and the topic's own scope),
//! wherever a run leaves the daemon: listed, read alone, with its
//! instance, and by a step's log.
//!
//! What the runner says of such a run as it goes (that it started, each
//! line a step prints, why a step or the run failed) takes what the
//! event that set it off took, so it is handed out by the same question
//! wherever events are: to whoever follows what is said, and to a
//! handler.
//!
//! And that question is asked in one place: three tests read the source
//! for anything that reads what an event takes for itself, for a route
//! that hands a run out without asking, and for something the runner
//! says of a run as if no event had set it off.

use std::path::Path;

use futures_util::StreamExt;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::{Followed, Rig, soon};

/// Two tables, one kept private, and a handler that hears a record of
/// either changed: it reads the record, says a line and fails a step
/// that is let fail, so that what the event held is in what a step gave,
/// in a step's log and in why a step failed.
const WATCH: &str = r#"
blueprint: watch
permissions: [projects, records]
steps:
  moods:
    do: collection.ensure
    with:
      name: moods
      reads: private:journal
      fields: { title: { type: text, required: true }, went: { type: choice, of: [well, badly] } }
  guides:
    do: collection.ensure
    with:
      name: guides
      fields: { title: { type: text, required: true }, went: { type: choice, of: [well, badly] } }
on:
  changed:
    event: record.updated
    for: { project: life }
    steps:
      read: { do: record.get, with: { id: "${event.data.id}" } }
      said: { do: log, with: { message: "it went ${event.data.chose.went} for ${steps.read.out.data.title}" } }
      wrong: { do: fail, allow_failure: true, with: { message: "nothing is wrong with ${steps.read.out.data.title}" } }
"#;

/// What is of the private record and nowhere else: its id, which what is
/// said of it carries, and what it is called, which a step read.
const MARKED: [&str; 2] = ["life-moods-1", "quince"];

pub(crate) async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "reader", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

fn said(body: &Value) -> String {
    body["error"]["message"].as_str().unwrap_or_default().to_string()
}

/// The run among `runs` that the change of `record` set off.
fn of<'a>(runs: &'a Value, run: &str) -> &'a Value {
    let all = runs["runs"].as_array().unwrap();
    all.iter().find(|one| one["id"] == run).unwrap_or_else(|| panic!("the run `{run}` is listed: {runs}"))
}

#[tokio::test]
async fn a_run_set_off_by_something_kept_private_is_shown_whole_only_to_whoever_may_be_told_what_set_it_off()
{
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Life" })).await;
    let applied = rig.send("/api/apply", json!({ "source": WATCH, "project": "life", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let instance = applied["instance"].as_str().unwrap().to_string();
    for table in ["moods", "guides"] {
        let fields = json!({ "fields": { "title": "Saturday", "went": "well" } });
        rig.send(&format!("/api/collections/life/{table}/records"), fields).await;
    }

    // The owner changes a record of each, and the handler has a run for each.
    let mut ended = rig.follow("run.finished");
    for (record, title, went) in [("life-moods-1", "quince", "badly"), ("life-guides-1", "bread", "badly")] {
        let change = json!({ "fields": { "title": title, "went": went } });
        let (status, changed) =
            rig.call(Method::PATCH, &format!("/api/records/{record}"), Some(&rig.token), Some(change)).await;
        assert_eq!(status, StatusCode::OK, "{changed}");
    }
    let mut handled = 0;
    while handled < 2 {
        let event = soon("the handler's two runs to end", ended.next()).await.unwrap();
        handled += usize::from(event.data["flow"] == "on.changed");
    }
    let whole = rig.read(&format!("/api/runs?instance={instance}")).await;
    let run_of = |record: &str| -> String {
        let all = whole["runs"].as_array().unwrap();
        let one = all.iter().find(|one| one["event"]["data"]["id"] == record);
        one.unwrap_or_else(|| panic!("the owner reads the run for `{record}`: {whole}"))["id"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let (kept, open) = (run_of("life-moods-1"), run_of("life-guides-1"));

    // The owner reads all of it, by each way a run is read.
    let listed = rig.read("/api/runs?limit=100").await;
    let alone = rig.read(&format!("/api/runs/{kept}")).await;
    let with_its_instance = rig.read(&format!("/api/instances/{instance}")).await;
    for (way, answer) in [
        ("listed", of(&listed, &kept)),
        ("alone", &alone),
        ("with its instance", of(&with_its_instance, &kept)),
    ] {
        assert_eq!(answer["event"]["data"]["id"], "life-moods-1", "{way}: {answer}");
        assert!(answer.get("kept_from").is_none(), "{way}: {answer}");
    }
    assert_eq!(alone["steps"][0]["output"]["data"]["title"], "quince", "{alone}");
    assert!(alone["steps"][2]["error"].as_str().unwrap().contains("quince"), "{alone}");
    let log = rig.read(&format!("/api/runs/{kept}/steps/said/log")).await;
    assert!(log.to_string().contains("it went badly for quince"), "{log}");

    // A person of the factory who was not given the journal: who may read
    // runs, and may not be told of a record of the moods.
    let member = key(&rig, kernel::principal::MEMBER_SCOPES).await;
    let ask = |path: String| {
        let (rig, member) = (&rig, member.clone());
        async move { rig.call(Method::GET, &path, Some(&member), None).await }
    };
    let (status, listed) = ask("/api/runs?limit=100".to_string()).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    let (status, of_instance) = ask(format!("/api/runs?instance={instance}")).await;
    assert_eq!(status, StatusCode::OK, "{of_instance}");
    let (status, alone) = ask(format!("/api/runs/{kept}")).await;
    assert_eq!(status, StatusCode::OK, "{alone}");
    let (status, with_its_instance) = ask(format!("/api/instances/{instance}")).await;
    assert_eq!(status, StatusCode::OK, "{with_its_instance}");
    for (way, answer) in [
        ("the runs there are", &listed),
        ("the instance's runs", &of_instance),
        ("the run alone", &alone),
        ("the instance with its runs", &with_its_instance),
    ] {
        for marked in MARKED {
            assert!(!answer.to_string().contains(marked), "`{marked}` is read in {way}: {answer}");
        }
    }
    // The run is still there for them: when, which handler, how it ended
    // and by which steps, and that something of it is kept from them.
    for (way, answer) in [
        ("listed", of(&listed, &kept)),
        ("of its instance", of(&of_instance, &kept)),
        ("alone", &alone),
        ("with its instance", of(&with_its_instance, &kept)),
    ] {
        assert_eq!(answer["flow"], "on.changed", "{way}: {answer}");
        assert_eq!(answer["status"], "ok", "{way}: {answer}");
        assert_eq!(answer["event"]["topic"], "record.updated", "{way}: {answer}");
        assert!(answer["started"].as_i64().is_some_and(|at| at > 0), "{way}: {answer}");
        assert_eq!(answer["kept_from"], true, "{way}: {answer}");
        assert!(answer["event"].get("data").is_none(), "{way}: {answer}");
    }
    let steps: Vec<(&str, &str)> = alone["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|step| (step["step"].as_str().unwrap(), step["status"].as_str().unwrap()))
        .collect();
    assert_eq!(steps, [("read", "ok"), ("said", "ok"), ("wrong", "failed")], "{alone}");
    for step in alone["steps"].as_array().unwrap() {
        assert!(step["output"].is_null() && step["error"].is_null(), "{step}");
    }
    // A step's log of it is refused as the log of a run that is not there.
    let (status, log) = ask(format!("/api/runs/{kept}/steps/said/log")).await;
    let (missing, none) = ask("/api/runs/run_nowhere".to_string()).await;
    assert_eq!((status, missing), (StatusCode::NOT_FOUND, StatusCode::NOT_FOUND), "{log} {none}");
    assert_eq!(said(&log), said(&none).replace("run_nowhere", &kept), "{log}");

    // What was set off by something they may be told of is as it was.
    let (_, theirs) = ask(format!("/api/runs/{open}")).await;
    assert_eq!(theirs["event"]["data"]["id"], "life-guides-1", "{theirs}");
    assert_eq!(theirs["steps"][0]["output"]["data"]["title"], "bread", "{theirs}");
    assert!(theirs["steps"][2]["error"].as_str().unwrap().contains("bread"), "{theirs}");
    assert!(theirs.get("kept_from").is_none(), "{theirs}");
    assert_eq!(of(&listed, &open)["event"]["data"]["chose"]["went"], "badly", "{listed}");
    let (status, log) = ask(format!("/api/runs/{open}/steps/said/log")).await;
    assert!(status == StatusCode::OK && log.to_string().contains("it went badly for bread"), "{log}");
    // And so is a run that no event set off: what they applied themselves.
    let says =
        "blueprint: says\nsteps:\n  say: { do: log, with: { message: plain } }\noutputs:\n  said: plain\n";
    let asked = json!({ "source": says, "project": "life", "wait": true });
    let (status, own) = rig.call(Method::POST, "/api/apply", Some(&member), Some(asked)).await;
    assert_eq!((status, &own["finished"]["outputs"]["said"]), (StatusCode::OK, &json!("plain")), "{own}");
    assert!(own["finished"].get("kept_from").is_none(), "{own}");

    // Given what the moods are kept under, a key reads the run as the owner does.
    let holds: Vec<&str> =
        kernel::principal::MEMBER_SCOPES.iter().copied().chain(["private:journal"]).collect();
    let trusted = key(&rig, &holds).await;
    let (status, alone) = rig.call(Method::GET, &format!("/api/runs/{kept}"), Some(&trusted), None).await;
    assert_eq!(status, StatusCode::OK, "{alone}");
    assert_eq!(alone["event"]["data"]["id"], "life-moods-1", "{alone}");
    assert_eq!(alone["steps"][0]["output"]["data"]["title"], "quince", "{alone}");
    let path = format!("/api/runs/{kept}/steps/said/log");
    let (status, log) = rig.call(Method::GET, &path, Some(&trusted), None).await;
    assert!(status == StatusCode::OK && log.to_string().contains("it went badly for quince"), "{log}");
}

/// The same two tables, and three handlers. Two hear a record of either
/// changed: one says a line of it; the other fails, having been tried
/// again, with what is done as its step starts saying a line and what is
/// done as it fails failing too, so that everything the runner says of a
/// step and of a run has the record's words in it. The third hears a
/// run of its own tree fail.
const HEARD: &str = r#"
blueprint: heard
permissions: [projects, records]
steps:
  moods:
    do: collection.ensure
    with:
      name: moods
      reads: private:journal
      fields: { title: { type: text, required: true }, went: { type: choice, of: [well, badly] } }
  guides:
    do: collection.ensure
    with:
      name: guides
      fields: { title: { type: text, required: true }, went: { type: choice, of: [well, badly] } }
on:
  changed:
    event: record.updated
    for: { project: life }
    steps:
      read: { do: record.get, with: { id: "${event.data.id}" } }
      said: { do: log, with: { message: "it went ${event.data.chose.went} for ${steps.read.out.data.title}" } }
  broke:
    event: record.updated
    for: { project: life }
    steps:
      read: { do: record.get, with: { id: "${event.data.id}" } }
      lost:
        do: fail
        with: { message: "all is lost for ${steps.read.out.data.title}" }
        retry: { max: 1, backoff: 0s }
        on_start:
          - { do: log, with: { message: "beginning with ${steps.read.out.data.title}" } }
        on_fail:
          - { do: record.get, with: { id: "life-nowhere-${steps.read.out.data.title}" } }
  troubled:
    event: run.failed
    scope: own
    steps:
      noted: { do: log, with: { message: "a run of this tree failed" } }
"#;

/// What somebody else applies, to be told of every failure there is.
const LISTENS: &str = r#"
blueprint: listens
steps:
  nothing: { do: noop }
on:
  failed:
    event: run.failed
    scope: fleet
    steps:
      told: { do: log, with: { message: "a run failed: ${event.data.error}" } }
  stepped:
    event: step.failed
    scope: fleet
    steps:
      told: { do: log, with: { message: "a step failed: ${event.data.error}" } }
"#;

/// What is said, where it is kept, once everything a test did is at
/// rest: whoever follows what is said has heard all of it by then.
pub(crate) const RESTED: &str = "heard.rested";

/// Waits for a run of each of `flows` to have ended.
async fn ended(runs: &mut Followed, mut flows: Vec<&str>) {
    while !flows.is_empty() {
        let event = soon("a run of each handler to end", runs.next()).await.unwrap();
        if let Some(at) = flows.iter().position(|flow| event.data["flow"] == *flow) {
            flows.swap_remove(at);
        }
    }
}

/// What is said from now on, as whoever holds `token` is told it.
pub(crate) async fn following(rig: &Rig, token: &str) -> reqwest::Response {
    let stream = reqwest::Client::new()
        .get(format!("{}/api/events", rig.url))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_eq!(stream.status(), StatusCode::OK);
    stream
}

/// Every event `stream` hands over until it has said [`RESTED`] `times`.
pub(crate) async fn told_until_rested(stream: reqwest::Response, times: usize) -> Vec<Value> {
    let mut stream = stream.bytes_stream();
    let rested = format!("\"topic\":\"{RESTED}\"");
    let heard = soon("what was said once everything was at rest", async {
        let mut heard = String::new();
        while heard.matches(&rested).count() < times {
            heard.push_str(&String::from_utf8_lossy(&stream.next().await.unwrap().unwrap()));
        }
        heard
    })
    .await;
    let events = heard.lines().filter_map(|line| line.strip_prefix("data:"));
    events.map(|event| serde_json::from_str(event.trim()).unwrap_or_else(|_| panic!("{event}"))).collect()
}

/// What the runner says of a run and of its steps, and what a step
/// prints.
const OF_A_RUN: [&str; 4] = ["run.", "step.", "hook.", "log."];

#[tokio::test]
async fn what_a_run_says_of_itself_is_told_only_to_whoever_may_be_told_what_set_it_off() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Life" })).await;
    let applied = rig.send("/api/apply", json!({ "source": HEARD, "project": "life", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let instance = applied["instance"].as_str().unwrap().to_string();
    for table in ["moods", "guides"] {
        let fields = json!({ "fields": { "title": "Saturday", "went": "well" } });
        rig.send(&format!("/api/collections/life/{table}/records"), fields).await;
    }
    // A person of the factory who was not given the journal, who has
    // asked to be told of every failure there is, and follows what is
    // said; and the owner, following too.
    let member = key(&rig, kernel::principal::MEMBER_SCOPES).await;
    let asked = json!({ "source": LISTENS, "project": "life", "wait": true });
    let (status, theirs) = rig.call(Method::POST, "/api/apply", Some(&member), Some(asked)).await;
    assert_eq!((status, &theirs["finished"]["status"]), (StatusCode::OK, &json!("ok")), "{theirs}");
    let theirs = theirs["instance"].as_str().unwrap().to_string();
    let before = rig.read("/api/events/recent?limit=1").await["cursor"].as_str().unwrap().to_string();
    let (to_the_member, to_the_owner) = (following(&rig, &member).await, following(&rig, &rig.token).await);

    // A record of the private table is changed, and everything that
    // sets off has ended; then one of the other, and the same.
    let mut runs = rig.follow("run.finished");
    let change = async |record: &str, title: &str| {
        let change = json!({ "fields": { "title": title, "went": "badly" } });
        let (status, changed) =
            rig.call(Method::PATCH, &format!("/api/records/{record}"), Some(&rig.token), Some(change)).await;
        assert_eq!(status, StatusCode::OK, "{changed}");
    };
    change("life-moods-1", "quince").await;
    ended(&mut runs, vec!["on.changed", "on.broke", "on.troubled"]).await;
    change("life-guides-1", "bread").await;
    ended(&mut runs, vec!["on.changed", "on.broke", "on.troubled", "on.failed", "on.stepped"]).await;
    let journal = rig.node().app.journal.clone();
    let every: Vec<_> = journal.of_project("life").into_iter().chain([journal]).collect();
    for journal in &every {
        journal.emit(RESTED, kernel::Source::actor("system"), json!({})).await.unwrap();
    }

    // The runs the private record set off: the two that heard it change,
    // and the one that heard the second of them fail.
    let whole = rig.read(&format!("/api/runs?instance={instance}")).await;
    let all = whole["runs"].as_array().unwrap();
    let set_off = |by: &dyn Fn(&Value) -> bool| -> Vec<String> {
        all.iter()
            .filter(|run| by(&run["event"]))
            .map(|run| run["id"].as_str().unwrap().to_string())
            .collect()
    };
    let mut kept = set_off(&|event| event["data"]["id"] == "life-moods-1");
    assert_eq!(kept.len(), 2, "{whole}");
    // A handler of the same tree still hears such a run fail: it hears
    // as whoever applied it, who may be told.
    let heard_fail = set_off(&|event| {
        event["topic"] == "run.failed" && kept.iter().any(|run| event["source"]["run"] == run.as_str())
    });
    assert_eq!(heard_fail.len(), 1, "a handler of the same tree hears the run fail: {whole}");
    kept.extend(heard_fail);
    let of_kept = |event: &Value| kept.iter().any(|run| event["source"]["run"] == run.as_str());
    let of_a_run = |event: &Value| {
        let topic = event["topic"].as_str().unwrap_or_default();
        OF_A_RUN.iter().any(|family| topic.starts_with(family))
    };

    // The owner is told all of it: each thing the runner says of a step
    // and of a run, with the record's words where they are in it.
    let owner = told_until_rested(to_the_owner, every.len()).await;
    for (topic, words) in [
        ("run.started", ""),
        ("step.started", ""),
        ("log.line", "it went badly for quince"),
        ("log.line", "beginning with quince"),
        ("step.retrying", "all is lost for quince"),
        ("hook.failed", "life-nowhere-quince"),
        ("step.failed", "all is lost for quince"),
        ("step.finished", ""),
        ("run.failed", "all is lost for quince"),
        ("run.finished", "all is lost for quince"),
    ] {
        let said =
            |event: &&Value| event["topic"] == topic && of_kept(event) && event.to_string().contains(words);
        assert!(
            owner.iter().any(|event| said(&event)),
            "the owner is told `{topic}` with `{words}`: {owner:?}"
        );
    }
    // Each says that hearing it takes what hearing of the record does.
    for event in owner.iter().filter(|event| of_a_run(event) && of_kept(event)) {
        assert_eq!(event["source"]["takes"], "private:journal", "{event}");
    }
    let recent = format!("/api/events/recent?limit=1000&after={before}");
    let kept_too = rig.read(&recent).await;
    let kept_too = kept_too["events"].as_array().unwrap();
    assert!(kept_too.iter().any(|event| event["topic"] == "run.failed" && of_kept(event)), "{kept_too:?}");

    // The member is told none of it: nothing with the record's words in
    // it, and nothing the runner says of those runs at all; as it is
    // said, and read back afterwards.
    let (status, read_back) = rig.call(Method::GET, &recent, Some(&member), None).await;
    assert_eq!(status, StatusCode::OK, "{read_back}");
    let member_told = told_until_rested(to_the_member, every.len()).await;
    for (way, told) in
        [("as it is said", &member_told), ("read back", read_back["events"].as_array().unwrap())]
    {
        for (event, marked) in told.iter().flat_map(|event| MARKED.map(|marked| (event, marked))) {
            assert!(!event.to_string().contains(marked), "`{marked}` is told to the member, {way}: {event}");
        }
        for event in told {
            assert!(!(of_a_run(event) && of_kept(event)), "the member is told of a kept run, {way}: {event}");
        }
        // What the other record set off is theirs to hear, as it was.
        for (topic, words) in
            [("log.line", "it went badly for bread"), ("run.failed", "all is lost for bread")]
        {
            if way == "read back" && topic == "log.line" {
                continue;
            }
            let said = |event: &&Value| event["topic"] == topic && event.to_string().contains(words);
            assert!(told.iter().any(|event| said(&event)), "the member is told `{topic}`, {way}: {told:?}");
        }
    }

    // Nor are their handlers handed any of it: each has heard the other
    // record's run fail, and nothing of the private one's.
    let handled = rig.read(&format!("/api/runs?instance={theirs}")).await;
    let handled = handled["runs"].as_array().unwrap();
    let heard = |flow: &str, words: &str| {
        handled
            .iter()
            .any(|run| run["flow"] == flow && run["event"]["data"]["error"].to_string().contains(words))
    };
    assert!(heard("on.failed", "bread") && heard("on.stepped", "bread"), "{handled:?}");
    for run in handled {
        assert!(!of_kept(&run["event"]), "a handler of the member's was handed what a kept run said: {run}");
        assert!(!run.to_string().contains("quince"), "{run}");
    }
    // And the run of the handler that heard the kept run fail is shown to
    // them as a run set off by something kept from them is.
    let (status, listed) =
        rig.call(Method::GET, &format!("/api/runs?instance={instance}"), Some(&member), None).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    for marked in MARKED {
        assert!(!listed.to_string().contains(marked), "`{marked}` is read in the runs listed: {listed}");
    }
    for run in &kept {
        assert_eq!(of(&listed, run)["kept_from"], true, "{listed}");
    }
}

/// Every line of this program's own source, under `folders` from the
/// top, that is no comment: the file, which line and what it says. A
/// crate's tests are not read: they hand nothing to anybody.
pub(crate) fn lines_of(folders: &[&str]) -> Vec<(String, usize, String)> {
    let top = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut folders: Vec<_> = folders.iter().map(|folder| top.join(folder)).collect();
    let (mut lines, mut looked) = (Vec::new(), 0);
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if name != "tests" && name != "target" && name != "node_modules" {
                    folders.push(path);
                }
                continue;
            }
            if !name.ends_with(".rs") {
                continue;
            }
            looked += 1;
            let file = path.strip_prefix(&top).unwrap().to_string_lossy().replace('\\', "/");
            for (line, says) in std::fs::read_to_string(&path).unwrap().lines().enumerate() {
                let code = says.trim();
                if !code.starts_with("//") {
                    lines.push((file.clone(), line + 1, code.to_string()));
                }
            }
        }
    }
    assert!(looked > 40, "looked at only {looked} files: is this still where they are?");
    lines
}

/// Holds the lines that have one of `needles` in them to a list: each is
/// let through by `passes`, or is on `listed` (a file, the words on the
/// line, how many such lines the file has, and the reason), and every
/// line of the list is there as often as it says. What is wrong, a line
/// each.
pub(crate) fn held_to(
    lines: &[(String, usize, String)],
    needles: &[&str],
    passes: impl Fn(&str) -> bool,
    listed: &[(&str, &str, usize, &str)],
) -> Vec<String> {
    let (mut wrong, mut found) = (Vec::new(), vec![0; listed.len()]);
    for (file, line, says) in lines {
        if !needles.iter().any(|needle| says.contains(needle)) || passes(says) {
            continue;
        }
        match listed.iter().position(|(at, words, ..)| at == file && says.contains(words)) {
            Some(at) => found[at] += 1,
            None => wrong.push(format!("{file}:{line}: {says}")),
        }
    }
    for ((file, words, times, _), found) in listed.iter().zip(&found) {
        if found != times {
            wrong.push(format!("`{words}` in {file} is listed as there {times} times and is there {found}"));
        }
    }
    wrong
}

/// Where what an event takes is read, or a `Source` taken apart for it,
/// and why each is no second answer to who may be told.
const READS_WHAT_IT_TAKES: &[(&str, &str, usize, &str)] = &[
    (
        "crates/api/src/followers.rs",
        "takes: event.source.takes.clone(),",
        1,
        "what tells whoever follows something keeping what a change took with it, so that the one question is \
         asked of it as each follower is told, at once or in a digest later (`followers::may`, which asks \
         `Journal::may_tell_as`): it hands nothing over itself",
    ),
    (
        "crates/journal/src/told.rs",
        "event.source.takes.as_deref()",
        1,
        "the one question itself: `Journal::may_tell`, which everything that hands an event to somebody asks",
    ),
    (
        "crates/journal/src/events.rs",
        "event.source.takes,",
        1,
        "the journal writing an event down, with what it takes beside it: nobody is handed anything by that",
    ),
    (
        "crates/kernel/src/event.rs",
        "Source { takes: Some(scope.to_string()), ..self }",
        1,
        "saying what an event takes (`Source::taking`), which is not reading it",
    ),
    (
        "crates/conductor/src/run.rs",
        "Ok(source) => source.takes,",
        1,
        "the runner reading what the event that set a run off took, from the event as the run keeps it, to say \
         what it says of that run as taking the same (`run::took`): it is said again and decides nothing, and \
         whoever is then handed what was said is asked the one question",
    ),
];

#[test]
fn what_an_event_takes_is_read_by_the_one_question_and_by_nothing_that_hands_an_event_over_for_itself() {
    let lines = lines_of(&["crates", "xtask", "desktop", "examples"]);
    let apart = |says: &str| says.contains("Source {") && says.contains("takes");
    let reads: Vec<_> =
        lines.into_iter().filter(|(_, _, says)| says.contains("source.takes") || apart(says)).collect();
    let wrong = held_to(&reads, &[""], |_| false, READS_WHAT_IT_TAKES);
    assert!(
        wrong.is_empty(),
        "these read what an event takes (`source.takes`) for themselves. Who an event may be handed to is one \
         question, `Journal::may_tell` (`crates/journal/src/told.rs`), with what its topic takes beside it \
         (`App::may_hear`, `Engine::hears`): something that takes a scope it is handed, or is kept to the \
         members of something, is answered there and nowhere else, so that a new kind of keeping is one \
         change. Ask it; what reads `takes` and hands nothing to anybody is listed in `READS_WHAT_IT_TAKES` \
         with the reason.\n{}",
        wrong.join("\n")
    );
}

/// What reads a run, or a step's log, from the runner.
const READS_A_RUN: &[&str] = &[
    "engine.run(",
    "engine.runs(",
    ".recent_runs(",
    ".wait_run(",
    ".step_log(",
    "logs().tail(\"runs\"",
    "RunInfo",
    "StepInfo",
];

/// Where a run is read from the runner and not handed, on that line,
/// through `handed::run` or `handed::runs`: and why nothing of it leaves
/// the daemon unasked.
const READ_AND_NOT_HANDED_THERE: &[(&str, &str, usize, &str)] = &[
    (
        "crates/api/src/searched.rs",
        "let mut runs = app.engine.recent_runs(200, project.map(str::to_string)).await?;",
        1,
        "the latest runs, read for a search to find one by its id: only the id and what it is a run of are \
         answered, never what set it off or what its steps gave, and its page then reads it through `handed::run`",
    ),
    (
        "crates/api/src/searched.rs",
        "runs.extend(app.engine.run(whole).await.ok());",
        1,
        "the run a search names whole, read as the latest are, for its id and what it is a run of alone",
    ),
    (
        "crates/api/src/routes/begun.rs",
        "let Ok(run) = app.engine.run(&run).await else { continue };",
        1,
        "the run an asking began, read to ask whether whoever asks may be told what set it off (`tells_of`), \
         and handed to nobody: only its id is answered, which its page then reads through `handed::run`",
    ),
    (
        "crates/api/src/routes/work.rs",
        "(Some(instance), _) => app.engine.runs(&instance, limit).await?,",
        1,
        "the runs of one instance, handed over by `handed::runs` in the line that answers",
    ),
    (
        "crates/api/src/routes/work.rs",
        "(None, 0 | 1) => app.engine.recent_runs(limit, named.pop()).await?,",
        1,
        "the runs there are, or one project's, handed over by `handed::runs` in the line that answers",
    ),
    (
        "crates/api/src/routes/work.rs",
        "all.extend(app.engine.recent_runs(limit, Some(project)).await?);",
        1,
        "the runs of each of several projects, put together and handed over by `handed::runs` in the line \
         that answers",
    ),
    (
        "crates/api/src/routes/work.rs",
        "let of = app.engine.run(&run).await.ok();",
        1,
        "read to ask whether its step's log may be read (`tells_of`), and whether its log was let go of after \
         so many days (when it ended), and handed to nobody",
    ),
    (
        "crates/api/src/routes/deploys.rs",
        "let runs = app.engine.runs(&instance.id, 20).await.ok()?;",
        1,
        "the newest run of an environment's deploy, of which only its id, its flow and when it began are taken: \
         how it went is read again through `handed::run` in `with_ran`",
    ),
    (
        "crates/api/src/routes/deploys.rs",
        "let runs = app.engine.recent_runs(last::RUNS, Some(project.to_string())).await.ok()?;",
        1,
        "a project's latest runs, read for what each of its environments last ran (`under`), for whoever may \
         read blueprints, of which only the id, the instance, when it began and how it went are taken, which \
         `handed::run` hands whoever may read runs whatever set the run off",
    ),
    (
        "crates/api/src/routes/work.rs",
        "app.engine.step_log(&run, &step,",
        1,
        "a step's log, read once `tells_of` has been asked of its run, in the lines before",
    ),
    (
        "crates/api/src/routes/work.rs",
        "Some(tail) => app.engine.logs().tail(\"runs\", &format!(\"{run}.{step}\"), tail.min(10_000)),",
        1,
        "a step's log read from its end, once `tells_of` has been asked of its run, in the lines before",
    ),
    (
        "crates/setup/src/byhand.rs",
        "let finished = engine.wait_run(&run).await?;",
        1,
        "an agent's text applied for whoever asked, read for how it ended: why it failed is said to whoever \
         applied it, which is the run's own applier",
    ),
    (
        "crates/setup/src/usual.rs",
        "let finished = engine.wait_run(&run).await",
        1,
        "`std/project` applied as a project is set up: read for how it ended and what it gave out, which is said again in the answer to whoever applied it",
    ),
    (
        "crates/setup/src/given.rs",
        "let finished = engine.wait_run(run).await",
        1,
        "each row of what a project starts with being applied by `project.set_up`: read for how it ended, by the daemon itself",
    ),
    (
        "crates/api/src/loaded.rs",
        "let read = app.engine.wait_run(run).await?;",
        1,
        "how the run an environment was loaded by came out, said of the environment as whoever may read the \
         project is told: its status, when it ended, what its blueprint gave out as `address` and `cards`, and \
         why it failed, which `project.environment.ran` and the environment's page say already to them",
    ),
    (
        "crates/gateway/src/actions.rs",
        "finished = engine.wait_run(run) => finished?,",
        1,
        "a step that applies a blueprint and waits: the run of an instance's own steps, which no event set \
         off, and what it gave is that step's to give",
    ),
];

#[test]
fn a_run_leaves_the_daemon_only_through_what_asks_whether_its_reader_may_be_told_what_set_it_off() {
    // Everything above the runner, but what hands a run out itself.
    let above = ["api", "setup", "folders", "gateway", "agents", "deciding"];
    let above = above.into_iter().chain(["studio", "hosting", "comparisons", "blocks", "cli"]);
    let folders: Vec<String> = above.map(|one| format!("crates/{one}/src")).collect();
    let folders: Vec<&str> = folders.iter().map(String::as_str).chain(["desktop"]).collect();
    let mut lines = lines_of(&folders);
    lines.retain(|(file, ..)| file != "crates/api/src/handed.rs");
    let handed = |says: &str| says.contains("handed::run(") || says.contains("handed::runs(");
    let wrong = held_to(&lines, READS_A_RUN, handed, READ_AND_NOT_HANDED_THERE);
    assert!(
        wrong.is_empty(),
        "these read a run, or a step's log, from the runner, and do not hand it over through `handed::run` or \
         `handed::runs` (`crates/api/src/handed.rs`) on the same line. A run keeps the event that set it off \
         and what its steps gave, which whoever may read runs may not be told: hand it out through one of \
         those, read a step's log only where `tells_of` says so, and list what is read for the daemon's own \
         use in `READ_AND_NOT_HANDED_THERE` with the reason.\n{}",
        wrong.join("\n")
    );
    // And the routes that hand a run over are there to be held: a list
    // that found nothing would hold nothing.
    let asked = lines.iter().filter(|(_, _, says)| handed(says)).count();
    assert!(asked >= 5, "only {asked} lines hand a run over through `handed`: is this still how it is done?");
}

/// Where the runner makes where an event came from as taking nothing,
/// and why each is nothing the runner says, for itself, of a run that an
/// event may have set off.
const SAID_AS_TAKING_NOTHING: &[(&str, &str, usize, &str)] = &[
    (
        "crates/conductor/src/run.rs",
        "takes: None,",
        1,
        "`RunCx::source` itself: where what a blueprint has a run say comes from",
    ),
    (
        "crates/conductor/src/run.rs",
        "self.source(step).taking(self.event.took.as_deref())",
        1,
        "`RunCx::about` itself: the same, made to take what the event that set the run off took, which was \
         read once as the run's context was made (`run::Fired`)",
    ),
    (
        "crates/conductor/src/intrinsic.rs",
        "let source = cx.source((!call.step.is_empty()).then_some(call.step));",
        1,
        "an `emit` step, and what a `wait` says once it is listening: what a text has its run say is its own to \
         say, as whoever applied it, who was let say that topic",
    ),
    (
        "crates/conductor/src/child.rs",
        "cx.engine.way_for(&start, &cx.source(Some(&step.id)), &mut labels)",
        1,
        "a step starting something: who asks, handed to whatever answers for comparisons, which says under it \
         only that a call went on with a kept text (`instance.call_kept`): the text's doing, and no word of a run",
    ),
    (
        "crates/conductor/src/child.rs",
        "Some((cx.source(Some(&step.id)), asker)),",
        1,
        "the same step applying what it starts: said under it is `instance.call_kept` and nothing else",
    ),
    (
        "crates/conductor/src/action.rs",
        "takes: None,",
        1,
        "`ActionCx::source` itself: where an action's own events come from, in every crate that has actions",
    ),
    (
        "crates/conductor/src/action.rs",
        "self.inner.engine.kept_in(&data).emit(topic, self.source(), data)",
        1,
        "an action saying an event of its own (`ActionCx::emit`), for the blueprint that asked it and as \
         whoever applied that",
    ),
    (
        "crates/conductor/src/carry.rs",
        "Source { instance: Some(live.id.clone()), run: Some(run.to_string()), ..Source::default() };",
        1,
        "the end of a run that was to carry an instance over to a text and never came to: no event sets such \
         a run off",
    ),
    (
        "crates/conductor/src/engine.rs",
        "..Source::default()",
        2,
        "an instance's own (`instance_source`, and that it started): what is said of an instance is not said \
         of one run",
    ),
    (
        "crates/conductor/src/settle.rs",
        "..Source::default()",
        1,
        "that an instance taken down as a conductor starts is over: the instance's own",
    ),
    (
        "crates/conductor/src/taking.rs",
        "self.emit(\"system.texts\", Source::default(), said).await;",
        1,
        "how the factory takes texts being changed: no instance's and no run's",
    ),
    (
        "crates/conductor/src/letgo.rs",
        "self.emit(\"system.logs\", Source::default(), said).await;",
        2,
        "how long the detail of runs' steps is kept, or where it is written, being said: no instance's and no run's",
    ),
    (
        "crates/conductor/src/letgo.rs",
        "self.emit(\"system.logs.let_go\", Source::default(), json!(done)).await;",
        1,
        "that what was old of many runs was let go of: no one run's, and it says nothing a run kept",
    ),
];

#[test]
fn what_the_runner_says_of_a_run_for_itself_is_said_as_taking_what_the_event_that_set_it_off_took() {
    let lines = lines_of(&["crates/conductor/src", "crates/runs/src"]);
    let plain = [".source(", "takes: None", "Source::default()"];
    let wrong = held_to(&lines, &plain, |_| false, SAID_AS_TAKING_NOTHING);
    assert!(
        wrong.is_empty(),
        "these make where an event comes from as taking nothing, in the runner. What the runner itself says of \
         a run and of its steps (that it started, a step beginning, ending, failing, tried again or passed over, \
         a line one prints, a hook that could not be done, how the run ended) is said with `RunCx::about`, which \
         takes what the event that set the run off took; a run found with nobody to end it is ended saying the \
         same (`run::took` of the event its row keeps). Said with `RunCx::source`, or with a source made by \
         hand, a handler's lines and failures are told to whoever may hear that runs go on, whatever its event \
         was kept from them. What a text has a run say (an `emit` step, an action's own event, what a step \
         starts) and what is said of an instance is listed in `SAID_AS_TAKING_NOTHING` with the reason.\n{}",
        wrong.join("\n")
    );
}
