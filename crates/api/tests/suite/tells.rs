//! What a blueprint tells somebody (`inbox.tell`, `talk::telling`), and
//! what a fleet tells of what waits for a person (`std/told`).

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{asked, key, person};

/// A text that tells whom it is handed, about what it is handed.
const NIGHTLY: &str = r#"blueprint: nightly
permissions: [inbox]
params:
  to: { type: list }
  about: { type: string, default: "" }
  says: { type: string, default: "The night's build is green." }
steps:
  tell:
    do: inbox.tell
    with: { to: "${params.to}", says: "${params.says}", why: built, about: "${params.about}" }
outputs:
  told: ${steps.tell.out.told}
  passed: ${steps.tell.out.passed}
"#;

/// What `token`'s box holds: its lines, and how many wait unread.
pub(crate) async fn inbox(rig: &Rig, token: &str) -> (Vec<Value>, u64) {
    let (status, found) = rig.call(Method::GET, "/api/inbox", Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{found}");
    (found["lines"].as_array().unwrap().clone(), found["unread"].as_u64().unwrap())
}

/// What each line of `token`'s box says, the latest told first.
pub(crate) async fn says(rig: &Rig, token: &str) -> Vec<String> {
    let (lines, _) = inbox(rig, token).await;
    lines.iter().map(|line| line["says"].as_str().unwrap_or_default().to_string()).collect()
}

/// `token` applies `NIGHTLY` under `key` and waits for its steps: the answer.
async fn nightly(rig: &Rig, token: &str, key: &str, params: Value) -> Value {
    let body = json!({ "source": NIGHTLY, "key": key, "params": params, "wait": true });
    let (status, applied) = rig.call(Method::POST, "/api/apply", Some(token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    applied
}

#[tokio::test]
async fn a_step_tells_whoever_it_names_as_its_applier_once_wherever_the_daemon_stops_and_the_line_is_its_owners()
 {
    let mut owned = Rig::new().await;
    let rig = &owned;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let (mira, nell) = (person(rig, "Mira").await, person(rig, "Nell").await);
    // One the factory does not start, which holds its own inbox; and one
    // it starts, which has none yet.
    let invited = rig.send("/api/agent/invite", json!({ "name": "scout" })).await;
    let joining = json!({ "invitation": invited["invitation"] });
    let joined = rig.call(Method::POST, "/api/agent/join", None, Some(joining)).await;
    let scout = joined.1["token"].as_str().expect("an agent that joined").to_string();
    rig.send("/api/agents", json!({ "name": "worker" })).await;

    // 1: Mira's text tells her, Nell by her handle, and both agents.
    let to = json!(["applier", "@nell", "scout", "agent:worker"]);
    let applied = nightly(rig, &mira.token, "first", json!({ "to": to, "about": "in_build" })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let gave = &applied["finished"]["outputs"];
    assert_eq!(gave["told"], json!([mira.actor, nell.actor, "agent:scout"]), "{applied}");
    assert_eq!(gave["passed"], json!(["agent:worker"]), "{applied}");
    let speaker = rig.read("/api/system/speaker").await["name"].as_str().unwrap().to_string();
    for token in [&mira.token, &nell.token, &scout] {
        let (lines, unread) = inbox(rig, token).await;
        assert_eq!((lines.len(), unread), (1, 1), "{lines:?}");
        let line = &lines[0];
        assert_eq!((&line["kind"], &line["says"]), (&json!("told"), &json!("The night's build is green.")));
        assert_eq!(
            (&line["about"], &line["place"], &line["why"]),
            (&json!("in_build"), &json!("in_build"), &json!("built"))
        );
        assert_eq!(line["from"], json!(format!("bot:{speaker}")));
        assert_eq!(line.get("said"), None, "a told line is no message's: {line}");
    }

    // 2: nobody else reads a line of it: not the factory's owner, who
    // was not told, by listing or by its id.
    let hers = inbox(rig, &nell.token).await.0[0]["id"].as_str().unwrap().to_string();
    assert_eq!(inbox(rig, &rig.token).await, (Vec::new(), 0));
    for does in ["read", "unread", "away"] {
        let theirs = asked(rig, &rig.token, Method::POST, &format!("/api/inbox/{hers}/{does}"), None).await;
        assert_eq!(theirs.0, StatusCode::NOT_FOUND, "{}", theirs.1);
    }
    // One line by its id, for a page of its own: its owner's, and to
    // anybody else, the factory's owner included, one that is not there
    // in the words one that never was is refused in.
    let (status, one) = rig.call(Method::GET, &format!("/api/inbox/{hers}"), Some(&nell.token), None).await;
    assert_eq!(status, StatusCode::OK, "{one}");
    assert_eq!(one, inbox(rig, &nell.token).await.0[0], "the line as the box lists it");
    for token in [&rig.token, &mira.token] {
        let theirs = asked(rig, token, Method::GET, &format!("/api/inbox/{hers}"), None).await;
        let never = asked(rig, token, Method::GET, "/api/inbox/inb_never", None).await;
        assert_eq!(theirs.0, StatusCode::NOT_FOUND, "{}", theirs.1);
        assert_eq!((theirs.0, theirs.1.replace(&hers, "inb_never")), never);
    }
    // Read, then made to wait unread again by its owner, who alone is told;
    // asked again, nothing more is said.
    let mark = rig.mark();
    rig.call(Method::POST, &format!("/api/inbox/{hers}/read"), Some(&nell.token), Some(json!({}))).await;
    let (status, again) = rig
        .call(Method::POST, &format!("/api/inbox/{hers}/unread"), Some(&nell.token), Some(json!({})))
        .await;
    assert_eq!(status, StatusCode::OK, "{again}");
    assert_eq!((&again["line"]["read"], &again["unread"]), (&Value::Null, &json!(1)), "{again}");
    let said = rig.event("inbox.unread", |event| event.seq > mark).await;
    assert_eq!(said.data, json!({ "owner": nell.actor, "lines": [hers], "place": "in_build", "unread": 1 }));
    let twice = rig
        .call(Method::POST, &format!("/api/inbox/{hers}/unread"), Some(&nell.token), Some(json!({})))
        .await;
    assert_eq!(twice.1["unread"], 1);
    assert_eq!(rig.count_of("inbox.unread").await, 1, "the same asked again says nothing");
    let (status, one) = rig.call(Method::GET, &format!("/api/inbox/{hers}"), Some(&nell.token), None).await;
    assert_eq!((status, &one["read"]), (StatusCode::OK, &Value::Null), "{one}");
    // Put away, a line is read by its id all the same, saying so.
    let mine = inbox(rig, &mira.token).await.0[0]["id"].as_str().unwrap().to_string();
    rig.call(Method::POST, &format!("/api/inbox/{mine}/away"), Some(&mira.token), Some(json!({}))).await;
    let (status, one) = rig.call(Method::GET, &format!("/api/inbox/{mine}"), Some(&mira.token), None).await;
    assert!(status == StatusCode::OK && one["away"].is_i64(), "{one}");

    // 3: a key of Mira's that was not given the box may apply the text,
    // and its step is refused: nobody is told.
    let narrow = key(rig, &mira.token, &["blueprints", "channels", "records"]).await;
    let applied = nightly(rig, &narrow, "narrow", json!({ "to": ["@nell"], "says": "From a key." })).await;
    assert_eq!(applied["finished"]["status"], "failed", "{applied}");
    assert!(applied.to_string().contains("may not use `inbox:write`"), "{applied}");
    // And a word that names nobody is refused with nobody told.
    let applied =
        nightly(rig, &mira.token, "nobody", json!({ "to": ["@nell", "@ghost"], "says": "To a ghost." }))
            .await;
    assert_eq!(applied["finished"]["status"], "failed", "{applied}");
    assert!(applied.to_string().contains("`@ghost` names nobody to tell"), "{applied}");
    assert_eq!(says(rig, &nell.token).await, ["The night's build is green."]);

    // 4: the daemon goes as the line is told, the last thing its journal
    // keeps, with the step that told it not yet written as done. Back,
    // the step is done again, finds its line, and tells nothing twice.
    let told = rig.count_of("inbox.told").await;
    let journal = rig.node().app.journal.clone();
    journal.shut_after(|event| event.topic == "inbox.told");
    let body =
        json!({ "source": NIGHTLY, "key": "stopped", "params": { "to": ["@nell"], "says": "Told once." } });
    // Whoever asks anything of a journal that has shut may never be answered.
    tokio::select! {
        _ = rig.call(Method::POST, "/api/apply", Some(&mira.token), Some(body)) => {}
        _ = journal.shut() => {}
    }
    journal.shut().await;
    owned.restart().await;
    let rig = &owned;
    let again = rig
        .event("step.started", |event| event.data["target"] == "inbox.tell" && event.data["resumed"] == true)
        .await;
    let step = again.seq;
    rig.event("run.finished", move |event| event.seq > step && event.data["status"] == "ok").await;
    assert_eq!(says(rig, &nell.token).await, ["Told once.", "The night's build is green."]);
    assert_eq!(rig.count_of("inbox.told").await, told + 1);
}

/// A node whose fleet has stand-ins of its own: `writer` asks, of a
/// card that says to ask about something, until it is told its question
/// was answered, and does any other card; `critic` approves, unless a
/// file `says-<its name>` in the folder handed back says otherwise.
async fn rig() -> (Rig, std::path::PathBuf) {
    let mut at = None;
    let rig = Rig::with("api-tells", |config, dir| {
        let does = ["append", "CHANGES.md", "changed", "say", "I changed a file.", "say", "DECISION: done"];
        let asks = ["say", "Which total: with tax, or without?", "say", "DECISION: ask"];
        let mut writes = vec!["read", "where", "it has been answered", "("];
        writes.extend(does);
        writes.extend([")", "where", "Ask about", "("]);
        writes.extend(asks);
        writes.push(")");
        writes.extend(does);
        let name = format!("%{}%", brand::env_var("SLOT"));
        let says = dir.join(format!("says-{name}")).to_string_lossy().into_owned();
        let looks =
            ["swallow", "says", &says, "say", "It does what the card asks.", "say", "DECISION: approve"];
        config.adapters.push(("writer".into(), crate::standin::argv(&writes)));
        config.adapters.push(("critic".into(), crate::standin::argv(&looks)));
        at = Some(dir.to_path_buf());
    })
    .await;
    (Rig::owned(rig).await, at.unwrap())
}

/// Waits until `owner` is told of something that is running after
/// `after` in the journal, which is what a failure is about: the event.
async fn told_of_running(rig: &Rig, owner: &str, after: u64) -> std::sync::Arc<kernel::Event> {
    let owner = owner.to_string();
    rig.event("inbox.told", move |event| {
        let about = event.data["about"].as_str().unwrap_or_default();
        event.seq > after && event.data["owner"] == owner.as_str() && about.starts_with("in_")
    })
    .await
}

/// The lines of `token`'s box that are there for `why`, the latest told
/// first: what each says, and how often it was told.
pub(crate) async fn lines(rig: &Rig, token: &str, why: &str) -> Vec<(String, u64)> {
    let (lines, _) = inbox(rig, token).await;
    let of = lines.iter().filter(|line| line["why"] == why);
    of.map(|line| (line["says"].as_str().unwrap_or_default().to_string(), line["count"].as_u64().unwrap()))
        .collect()
}

/// Why the line of `token`'s box that says `card` is left says it is:
/// what follows the card and that it is left for a person.
async fn left_of(rig: &Rig, token: &str, card: &str) -> String {
    let lines = lines(rig, token, "left").await;
    let begins = format!("{card} is left for a person: ");
    let found = lines.iter().find_map(|(says, _)| says.strip_prefix(&begins));
    found.unwrap_or_else(|| panic!("no line says {card} is left with why: {lines:?}")).to_string()
}

/// Waits until somebody has been told `times` lines of what was said
/// on `card`, a line each time a question was said there naming them.
async fn named(rig: &Rig, card: &str, times: usize) {
    let mut told = rig.follow("inbox.told");
    crate::soon(&format!("{times} lines of what was said on {card}"), async {
        loop {
            let all =
                journal::EventQuery { topics: "inbox.told".into(), limit: 10_000, ..Default::default() };
            let said = rig.said(all).await;
            let of =
                said.iter().filter(|event| event.data["kind"] == "commented" && event.data["place"] == card);
            if of.count() >= times {
                return;
            }
            told.next().await.unwrap();
        }
    })
    .await
}

/// Says something on a card, as the factory's owner.
async fn answer(rig: &Rig, id: &str, body: &str) {
    rig.send(&format!("/api/records/{id}/comments"), json!({ "body": body })).await;
}

/// Waits until a card's agent has asked after `after` in the journal:
/// where the journal had got to as it was said.
async fn asks(rig: &Rig, id: &str, after: u64) -> u64 {
    let card = id.to_string();
    rig.event("card.asked", move |event| event.seq > after && event.data["card"] == card.as_str()).await.seq
}

/// Waits until what tells has done with what was said at `said`, by
/// its handler `flow`: the run that event set off is over.
pub(crate) async fn heard(rig: &Rig, flow: &'static str, said: u64) {
    rig.event("run.finished", move |event| {
        event.seq > said && event.data["blueprint"] == "std/told" && event.data["flow"] == flow
    })
    .await;
}

/// What a card's journey came to: how much was said, how many runs
/// and instances were begun, and how much of it was of what tells.
#[derive(Debug)]
struct Journey {
    said: usize,
    runs: usize,
    instances: usize,
    of_what_tells: usize,
}

/// A card that asks nothing, made ready, worked, approved, merged and
/// tidied away: what that came to.
async fn journey(rig: &Rig, title: &str) -> Journey {
    let before = rig.mark();
    let id = crate::fleets::add(rig, title, &[]).await;
    crate::lanes::move_card(rig, &id, "ready", None).await;
    let done = crate::lanes::moved(rig, &id, "done", before).await;
    rig.event("run.finished", move |event| event.seq > done && event.data["flow"] == "on.tidied").await;
    let all =
        journal::EventQuery { topics: "**".into(), after: before, limit: 100_000, ..Default::default() };
    let said = rig.said(all).await;
    let told = rig.read("/api/instances?project=shop&live=true").await;
    let told = told["instances"].as_array().unwrap().iter().find(|one| one["blueprint"] == "std/told");
    let told = told.expect("what tells is attached to the fleet")["id"].as_str().unwrap().to_string();
    Journey {
        said: said.len(),
        runs: said.iter().filter(|event| event.topic == "run.started").count(),
        instances: said.iter().filter(|event| event.topic == "instance.started").count(),
        of_what_tells: said
            .iter()
            .filter(|event| {
                event.source.instance.as_deref() == Some(told.as_str())
                    || event.data["blueprint"] == "std/told"
            })
            .count(),
    }
}

#[tokio::test]
async fn a_fleet_tells_whoever_its_rows_say_of_what_waits_for_a_person_once_wherever_the_daemon_stops() {
    use crate::fleets::{add, shop};
    use crate::lanes::move_card;

    let (mut owned, at) = rig().await;
    let rig = &owned;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let grace = person(rig, "Grace").await;
    let dir = shop(rig, json!({})).await;

    // 1: the fleet keeps who is told what as a table of its project,
    // which sits beside the board and starts with whoever applied it.
    rig.event("record.created", |event| event.data["collection"] == "tells").await;
    let tables = rig.read("/api/collections?project=shop").await;
    let table = tables["collections"].as_array().unwrap().iter().find(|one| one["name"] == "tells").unwrap();
    assert_eq!(table["beside"], "board", "{table}");
    let rows = rig.read("/api/collections/shop/tells/records").await;
    let rows: Vec<(&Value, &Value)> = rows["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (&row["data"]["title"], &row["data"]["whom"]))
        .collect();
    assert_eq!(rows, [(&json!("applier"), &json!("applier"))]);

    // A card that waits for nobody costs nothing more for there being
    // somebody to tell: worked, approved and merged, with nothing of
    // what tells run for it and nothing said by it.
    let plain = journey(rig, "Add a shelf").await;
    assert_eq!(plain.of_what_tells, 0, "{plain:?}");
    assert!(plain.runs > 0 && plain.instances > 0 && plain.said > plain.runs, "{plain:?}");

    // 2: a card whose agent asks says its question on the card, as its
    // agent, naming the owner, who applied the fleet: being named is what
    // tells them, a line of their box that opens the card at it; and
    // nobody else is told, and nothing of it is told twice.
    crate::fleets::says(&at, "shop-reviewer-1", "The total is wrong.", "send_back");
    let id = add(rig, "Ask about the cart", &[]).await;
    move_card(rig, &id, "ready", None).await;
    let first = asks(rig, &id, 0).await;
    heard(rig, "on.asked", first).await;
    let question = "shop-worker-1 said: @ada Which total: with tax, or without?";
    crate::lanes::history(rig, &id, question).await;
    named(rig, &id, 1).await;
    let (mine, unread) = inbox(rig, &rig.token).await;
    assert_eq!((mine.len(), unread), (1, 1), "{mine:?}");
    assert_eq!((&mine[0]["kind"], &mine[0]["place"]), (&json!("commented"), &json!(id)), "{mine:?}");
    assert_eq!(mine[0]["from"], "agent:shop-worker-1", "said by whoever asked: {mine:?}");
    assert_eq!(mine[0]["on_record"]["body"], "@ada Which total: with tax, or without?");
    assert_eq!(lines(rig, &rig.token, "asked").await, [], "no line of its own beside the one being named is");
    let me = mine[0]["owner"].as_str().unwrap().to_string();
    assert_eq!(inbox(rig, &grace.token).await, (Vec::new(), 0));
    let read = format!("/api/inbox/{}/read", mine[0]["id"].as_str().unwrap());
    assert_eq!(rig.call(Method::POST, &read, Some(&rig.token), None).await.0, StatusCode::OK);

    // 3: a second row tells Grace of cards that are left and of what
    // fails, and of nothing else. Answered, the card is worked, looked
    // at and sent back, and its agent asks again: said on the card again,
    // naming the owner, who has a second line, and Grace nothing.
    let lead = json!({ "fields": { "title": "lead", "whom": "@grace", "of": ["left", "failed"] } });
    rig.send("/api/collections/shop/tells/records", lead).await;
    answer(rig, &id, "Without tax.").await;
    let second = asks(rig, &id, first).await;
    heard(rig, "on.asked", second).await;
    crate::lanes::history_with(rig, &id, question, 2).await;
    named(rig, &id, 2).await;
    assert_eq!(inbox(rig, &rig.token).await.1, 1, "the second waits unread");
    assert_eq!(inbox(rig, &grace.token).await, (Vec::new(), 0));

    // 4: the daemon goes as the question is said on the card a third
    // time, the last thing its journal keeps. Back, the step that said it
    // is done again and says nothing twice, and the owner is told of it once.
    let journal = rig.node().app.journal.clone();
    journal.shut_after(move |event| event.topic == "record.commented" && event.data["once"].is_string());
    // Whoever asks anything of a journal that has shut may never be answered.
    tokio::select! {
        _ = answer(rig, &id, "Without tax, as was said.") => {}
        _ = journal.shut() => {}
    }
    journal.shut().await;
    owned.restart().await;
    let rig = &owned;
    let third = asks(rig, &id, second).await;
    heard(rig, "on.asked", third).await;
    crate::lanes::history_with(rig, &id, question, 3).await;
    named(rig, &id, 3).await;
    let all = journal::EventQuery { topics: "record.commented".into(), limit: 1_000, ..Default::default() };
    let questions = rig.said(all).await.iter().filter(|event| event.data["once"].is_string()).count();
    assert_eq!(questions, 3, "each asking said once");
    assert_eq!(inbox(rig, &rig.token).await.0.len(), 3);

    // 5: answered once more it is worked and looked at, and having been
    // sent back as often as it may be it is left for a person: which
    // the owner and Grace are both told, each in a line of their own.
    answer(rig, &id, "Without.").await;
    let its = id.clone();
    let left = rig.event("card.left", move |event| event.data["card"] == its.as_str()).await.seq;
    heard(rig, "on.left", left).await;
    let sentence =
        format!("{id} was sent back 2 times, which is as often as it may be: it is left for a person.");
    assert_eq!(lines(rig, &rig.token, "left").await, [(sentence.clone(), 1)]);
    assert_eq!(lines(rig, &grace.token, "left").await, [(sentence, 1)]);
    assert_eq!(
        lines(rig, &rig.token, "named").await.len(),
        3,
        "each question and the leaving are a line each"
    );

    // 6: something of the fleet fails: a card whose place cannot be
    // made fails its round, which the fleet says where it reports and
    // both are told, about what failed.
    let other = add(rig, "Add a till", &[]).await;
    std::fs::create_dir_all(dir.join(".lanes")).unwrap();
    std::fs::write(dir.join(".lanes").join(&other), "in the way\n").unwrap();
    let before = rig.mark();
    move_card(rig, &other, "ready", None).await;
    told_of_running(rig, &grace.actor, before).await;
    told_of_running(rig, &me, before).await;
    for token in [&rig.token, &grace.token] {
        let failures = lines(rig, token, "failed").await;
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert!(failures[0].0.starts_with("Something in the fleet for shop failed: "), "{failures:?}");
        assert!(failures[0].0.contains("could not be made"), "{failures:?}");
    }
    // A round that fails leaves its card, which is said of the card
    // too, by the run that asked for the round: a line about the card.
    let its = other.clone();
    let left = rig.event("card.left", move |event| event.data["card"] == its.as_str()).await;
    assert!(left.data["why"].as_str().unwrap().contains("could not be made"), "{left:?}");
    heard(rig, "on.left", left.seq).await;
    for token in [&rig.token, &grace.token] {
        let of_it = left_of(rig, token, &other).await;
        assert!(of_it.contains("could not be made"), "{of_it}");
    }

    // 6b: a card nobody decided is left by those who looked at it,
    // with what they said.
    std::fs::write(at.join("says-shop-reviewer-1"), "I am not sure what to make of this.\n").unwrap();
    let lamp = add(rig, "Add a lamp", &[]).await;
    move_card(rig, &lamp, "ready", None).await;
    let its = lamp.clone();
    let left = rig.event("card.left", move |event| event.data["card"] == its.as_str()).await;
    heard(rig, "on.left", left.seq).await;
    for token in [&rig.token, &grace.token] {
        let of_it = left_of(rig, token, &lamp).await;
        assert!(of_it.contains("neither approved it nor sent it back"), "{of_it}");
    }

    // 6c: and one its round leaves, what is laid in the round having
    // sent it back as often as that may: a command of the project's
    // that never passes, for cards that carry a label.
    let strict = json!({
        "title": "Strict", "stage": "work", "order": 15, "does": "std/run", "command": "exit 1",
        "labelled": "strict", "rounds": 1,
    });
    crate::fleets::add_row(rig, "segments", strict).await;
    let shelf = rig
        .send("/api/board/shop/items", json!({ "title": "Add a strict shelf", "labels": ["strict"] }))
        .await;
    let shelf = shelf["id"].as_str().unwrap().to_string();
    move_card(rig, &shelf, "ready", None).await;
    let its = shelf.clone();
    let left = rig.event("card.left", move |event| event.data["card"] == its.as_str()).await;
    heard(rig, "on.left", left.seq).await;
    assert_eq!(left_of(rig, &rig.token, &shelf).await, left.data["why"].as_str().unwrap());
    assert!(!left_of(rig, &grace.token, &shelf).await.is_empty());
    // Each leaving was said once, and each is a line of its own.
    for card in [&id, &other, &lamp, &shelf] {
        let all = journal::EventQuery { topics: "card.left".into(), limit: 1_000, ..Default::default() };
        let said = rig.said(all).await.iter().filter(|event| event.data["card"] == card.as_str()).count();
        assert_eq!(said, 1, "{card}");
    }
    assert_eq!(lines(rig, &rig.token, "left").await.len(), 4);

    // 7: a row taken away and a row emptied tell nobody: with neither,
    // a card that asks says its question on the card naming nobody and
    // saying so, and where the fleet reports; and no box hears of it.
    let rows = rig.read("/api/collections/shop/tells/records").await;
    for row in rows["records"].as_array().unwrap() {
        let id = row["id"].as_str().unwrap();
        let (status, answer) = match row["data"]["title"] == "lead" {
            true => rig.call(Method::DELETE, &format!("/api/records/{id}"), Some(&rig.token), None).await,
            false => {
                let emptied = json!({ "fields": { "whom": "" } });
                rig.call(Method::PATCH, &format!("/api/records/{id}"), Some(&rig.token), Some(emptied)).await
            }
        };
        assert_eq!(status, StatusCode::OK, "{answer}");
    }
    let tellings = rig.count_of("inbox.told").await;
    std::fs::remove_file(dir.join(".lanes").join(&other)).unwrap();
    let quiet = add(rig, "Ask about the till", &[]).await;
    move_card(rig, &quiet, "ready", None).await;
    let asked = asks(rig, &quiet, 0).await;
    heard(rig, "on.asked", asked).await;
    let nobody = "Nobody is asked this: no row of the project's `tells` names anybody to tell of a question.";
    let history = crate::lanes::history(rig, &quiet, &format!("shop-worker-1 said: Which total: with tax, or without?\n\n{nobody} Answer it in its Comments; and say who is asked from now on with `{cli} record add shop tells title=lead whom=@<somebody> of=asked`.", cli = brand::CLI)).await;
    assert!(!history.is_empty());
    let channel = crate::lanes::said(rig).await;
    assert!(
        channel.contains(&format!("{quiet}: shop-worker-1 asked something, and nobody is asked it: ")),
        "{channel}"
    );
    assert_eq!(rig.count_of("inbox.told").await, tellings);

    // And with nobody to tell, a card's journey is as free of it.
    std::fs::remove_file(at.join("says-shop-reviewer-1")).unwrap();
    let untold = journey(rig, "Add a counter").await;
    assert_eq!(untold.of_what_tells, 0, "{untold:?}");
}

#[tokio::test]
async fn a_question_is_said_on_its_card_naming_whom_the_rows_say_and_a_reply_there_is_its_answer() {
    use crate::fleets::{add, shop};
    use crate::lanes::move_card;

    let (owned, _) = rig().await;
    let rig = &owned;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let grace = person(rig, "Grace").await;
    shop(rig, json!({})).await;
    rig.event("record.created", |event| event.data["collection"] == "tells").await;

    // 1: the row that tells whoever applied the fleet is made to name
    // Grace: the next question is put to her, and to nobody else.
    let rows = rig.read("/api/collections/shop/tells/records").await;
    let row = rows["records"][0]["id"].as_str().unwrap().to_string();
    let to_grace = json!({ "fields": { "whom": "@grace" } });
    let (status, changed) =
        rig.call(Method::PATCH, &format!("/api/records/{row}"), Some(&rig.token), Some(to_grace)).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    let id = add(rig, "Ask about the cart", &[]).await;
    move_card(rig, &id, "ready", None).await;
    let asked = asks(rig, &id, 0).await;
    heard(rig, "on.asked", asked).await;
    crate::lanes::history(rig, &id, "shop-worker-1 said: @grace Which total: with tax, or without?").await;
    named(rig, &id, 1).await;
    let (hers, unread) = inbox(rig, &grace.token).await;
    assert_eq!((hers.len(), unread), (1, 1), "{hers:?}");
    assert_eq!((&hers[0]["kind"], &hers[0]["place"]), (&json!("commented"), &json!(id)), "{hers:?}");
    assert_eq!(inbox(rig, &rig.token).await, (Vec::new(), 0), "whoever applied the fleet is named no more");
    let card = rig.read(&format!("/api/records/{id}")).await;
    assert_eq!(card["data"]["question"], "Which total: with tax, or without?", "{card}");

    // 2: Grace answers by saying something on the card, as anything is
    // said there: it is the answer, the question comes off the card and
    // its agent is given it again with what she said.
    let mark = rig.mark();
    let reply = json!({ "body": "Without tax." });
    let (status, said) =
        rig.call(Method::POST, &format!("/api/records/{id}/comments"), Some(&grace.token), Some(reply)).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    let its = id.clone();
    let answered =
        rig.event("card.answered", move |event| event.seq > mark && event.data["card"] == its.as_str()).await;
    assert_eq!(
        (&answered.data["body"], &answered.data["asker"]),
        (&json!("Without tax."), &json!("shop-worker-1"))
    );
    let its = id.clone();
    rig.event("agent.turn.started", move |event| {
        event.seq > answered.seq && event.data["thread"] == format!("lane:{its}").as_str()
    })
    .await;
    let card = rig.read(&format!("/api/records/{id}")).await;
    assert_eq!(card["data"]["question"], "", "{card}");
}

/// The fleet's text as it shipped before it told anybody: what ships
/// now without what it attaches, under the version it then said.
fn before_it_told() -> String {
    let now = include_str!("../../../../blueprints/std/fleet.bp");
    let (head, rest) = now.split_once("# What waits for a person is told").expect("what the fleet attaches");
    let (attached, tail) = rest.split_once("\nstate:\n").expect("the fleet's state");
    assert!(attached.contains("uses:") && !tail.contains("uses:"), "{attached}");
    let before = format!("{head}state:\n{tail}");
    let said = before.lines().find(|line| line.starts_with("version: ")).expect("its version");
    before.replace(said, "version: 13")
}

/// The live instances of the shop that run `blueprint`: the version each says.
async fn running(rig: &Rig, blueprint: &str) -> Vec<String> {
    let listed = rig.read("/api/instances?project=shop&live=true").await;
    let of = listed["instances"].as_array().unwrap().iter().filter(|one| one["blueprint"] == blueprint);
    of.map(|one| one["version"].as_str().unwrap_or("none").to_string()).collect()
}

/// `token` applies the shop's fleet, saying `more` beside what a fleet
/// is always started with: what the daemon answered.
async fn fleet_as(rig: &Rig, token: &str, more: &[&str]) -> Value {
    let mut request = json!({
        "blueprint": "std/fleet", "project": "shop", "params": crate::fleets::params(json!({})), "wait": true,
    });
    for word in more {
        request[*word] = json!(true);
    }
    let (status, applied) = rig.call(Method::POST, "/api/apply", Some(token), Some(request)).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    applied
}

/// A card that says to ask about `what`, made ready: its id, once its
/// agent has asked and what looks after the board's cards has put the
/// question on it.
async fn asking(rig: &Rig, what: &str) -> String {
    let id = crate::fleets::add(rig, &format!("Ask about {what}"), &[]).await;
    crate::lanes::move_card(rig, &id, "ready", None).await;
    let asked = asks(rig, &id, 0).await;
    rig.event("run.finished", move |event| {
        event.seq > asked && event.data["blueprint"] == "std/cards" && event.data["flow"] == "on.asked"
    })
    .await;
    id
}

#[tokio::test]
async fn a_fleet_from_before_tells_nobody_until_it_is_applied_again_taking_what_ships_by_somebody_who_may_tell()
 {
    // A node that reads the fleet's text from before in place of what it
    // ships, its texts said to be held, as a factory from before holds them.
    let mut at = None;
    let rig = Rig::with("api-tells-before", |config, dir| {
        let std = dir.join("before").join("std");
        std::fs::create_dir_all(&std).unwrap();
        std::fs::write(std.join(brand::blueprint_file("fleet")), before_it_told()).unwrap();
        config.blueprints_dir = Some(dir.join("before"));
        let writes = ["say", "Which total: with tax, or without?", "say", "DECISION: ask"];
        config.adapters.push(("writer".into(), crate::standin::argv(&writes)));
        config.adapters.push(("critic".into(), crate::standin::argv(&["say", "DECISION: approve"])));
        at = Some(dir.to_path_buf());
    })
    .await;
    let rig = Rig::owned(rig).await;
    let held = json!({ "come": "held" });
    let (status, answer) = rig.call(Method::POST, "/api/system/texts", Some(&rig.token), Some(held)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    // A key as one made before there was an inbox: all a person holds, but that.
    let scopes: Vec<&str> =
        kernel::principal::MEMBER_SCOPES.iter().copied().filter(|scope| *scope != "inbox").collect();
    let old = key(&rig, &rig.token, &scopes).await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;

    // 1: applied by that key, the fleet from before runs its own text:
    // it attaches nothing, keeps no table of who is told, and a card
    // that asks tells nobody.
    fleet_as(&rig, &old, &[]).await;
    assert_eq!(running(&rig, "std/fleet").await, ["13"]);
    assert!(running(&rig, "std/told").await.is_empty());
    asking(&rig, "the cart").await;
    let tables = rig.read("/api/collections?project=shop").await;
    assert!(tables["collections"].as_array().unwrap().iter().all(|one| one["name"] != "tells"), "{tables}");
    assert_eq!(rig.count_of("inbox.told").await, 0);

    // 2: the program is replaced, and the fleet stays as it was until it
    // is applied again saying to take what ships. Taken by the key from
    // before, which may tell nobody, what tells cannot say whom: it says
    // so where the fleet reports, once, with what to do, and stops; the
    // fleet goes on, nothing in it having failed, with nobody told.
    std::fs::remove_dir_all(at.unwrap().join("before").join("std")).unwrap();
    assert_eq!(running(&rig, "std/fleet").await, ["13"]);
    let before = rig.mark();
    fleet_as(&rig, &old, &["take"]).await;
    // What ships now, whatever its number is.
    let ships = include_str!("../../../../blueprints/std/fleet.bp")
        .lines()
        .find_map(|line| line.strip_prefix("version: "))
        .expect("its version");
    assert_eq!(running(&rig, "std/fleet").await, [ships]);
    let untold = |project: &'static str| {
        move |event: &kernel::Event| {
            let body = event.data["body"].as_str().unwrap_or_default();
            body.starts_with(&format!("Nobody is told of what waits for a person in {project}: "))
        }
    };
    let said = rig.event("channel.message", move |event| event.seq > before && untold("shop")(event)).await;
    let body = said.data["body"].as_str().unwrap();
    assert!(body.contains("takes `inbox:write`") && body.contains("saying `--force`"), "{body}");
    rig.event("instance.stopped", move |event| event.seq > before && event.data["blueprint"] == "std/told")
        .await;
    asking(&rig, "the till").await;
    assert!(running(&rig, "std/told").await.is_empty());
    assert_eq!(rig.count_of("inbox.told").await, 0);
    let said_in_channels = || async {
        let all =
            journal::EventQuery { topics: "channel.message".into(), limit: 10_000, ..Default::default() };
        rig.said(all).await
    };
    let messages = said_in_channels().await;
    assert_eq!(messages.iter().filter(|event| untold("shop")(event)).count(), 1);
    let failed = |event: &&kernel::Event| event.data["body"].as_str().unwrap_or_default().contains("failed");
    assert_eq!(messages.iter().filter(failed).count(), 0, "{messages:?}");
    assert_eq!(rig.count_of("instance.failed").await, 0);

    // 3: applied again by the owner, who may, it tells: the table starts
    // with whoever applied it, and the next card that asks is in their box.
    fleet_as(&rig, &rig.token, &["force"]).await;
    let told = include_str!("../../../../blueprints/std/told.bp")
        .lines()
        .find_map(|line| line.strip_prefix("version: "))
        .expect("its version");
    assert_eq!(running(&rig, "std/told").await, [told]);
    rig.event("record.created", |event| event.data["collection"] == "tells").await;
    let id = asking(&rig, "the shelf").await;
    named(&rig, &id, 1).await;
    let (mine, _) = inbox(&rig, &rig.token).await;
    assert_eq!(mine.len(), 1, "{mine:?}");
    assert_eq!(mine[0]["on_record"]["body"], "@ada Which total: with tax, or without?", "{mine:?}");
    assert_eq!(rig.count_of("inbox.told").await, 1);

    // 4: a fleet first applied by such a key says the same as it comes
    // up, where it will report, and works its cards with nobody told.
    rig.send("/api/projects", json!({ "name": "Yard" })).await;
    rig.board("yard").await;
    let mut params = crate::fleets::params(json!({}));
    params["project"] = json!("yard");
    let fleet = json!({ "blueprint": "std/fleet", "project": "yard", "params": params, "wait": true });
    let (status, applied) = rig.call(Method::POST, "/api/apply", Some(&old), Some(fleet)).await;
    assert_eq!((status, &applied["finished"]["status"]), (StatusCode::OK, &json!("ok")), "{applied}");
    rig.event("channel.message", untold("yard")).await;
    let yard = rig.send("/api/board/yard/items", json!({ "title": "Ask about the gate" })).await;
    let yard = yard["id"].as_str().unwrap().to_string();
    crate::lanes::move_card(&rig, &yard, "ready", None).await;
    asks(&rig, &yard, 0).await;
    let messages = said_in_channels().await;
    assert_eq!(messages.iter().filter(|event| untold("yard")(event)).count(), 1);
    assert_eq!(rig.count_of("inbox.told").await, 1);
    assert_eq!(rig.count_of("instance.failed").await, 0);
}

#[tokio::test]
async fn a_cards_plan_that_waits_and_a_card_left_where_cards_are_planned_are_told_whoever_applied_the_fleet()
{
    use crate::fleets::{add, add_row, says, shop};
    const ARCHITECT: &str = "shop-architect-1";

    // A fleet with an architect and a stage that plans the cards in
    // `draft` and sends a planned card nowhere.
    let (rig, at) = crate::fleets::rig().await;
    shop(&rig, json!({})).await;
    let architect =
        json!({ "title": "architect", "role": "architect", "adapter": "critic", "permission": "read" });
    add_row(&rig, "agents", architect).await;
    let plan = json!({ "title": "plan", "column": "draft", "does": "plan", "role": "architect", "pass": "" });
    add_row(&rig, "stages", plan).await;
    rig.event("record.created", |event| event.data["collection"] == "tells").await;

    // 1: planned, the card stays where it is for a person to accept its
    // plan by moving it on: which whoever applied the fleet is told.
    says(&at, ARCHITECT, "One piece: the cart.", "propose");
    let id = add(&rig, "Add a cart", &[]).await;
    let its = id.clone();
    let planned = rig.event("card.planned", move |event| event.data["card"] == its.as_str()).await;
    assert_eq!(planned.data["waits"], true, "{planned:?}");
    heard(&rig, "on.drafted", planned.seq).await;
    let accept = "Move it on to accept it, or say on it what should be otherwise.";
    assert_eq!(
        lines(&rig, &rig.token, "plan").await,
        [(format!("{id}: {ARCHITECT} has said how it would be done, and the plan waits. {accept}"), 1)]
    );

    // 2: what the project lays after a plan leaves a card for a person
    // where it does not pass: said the same way, by the stage.
    let checked =
        json!({ "title": "Checked", "stage": "plan", "order": 15, "does": "std/run", "command": "exit 1" });
    add_row(&rig, "segments", checked).await;
    let till = add(&rig, "Add a till", &[]).await;
    let its = till.clone();
    let left = rig.event("card.left", move |event| event.data["card"] == its.as_str()).await;
    heard(&rig, "on.left", left.seq).await;
    assert_eq!(left_of(&rig, &rig.token, &till).await, left.data["why"].as_str().unwrap());
    assert_eq!(lines(&rig, &rig.token, "left").await.len(), 1);
}
