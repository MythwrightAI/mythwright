//! Whoever follows a page of a wiki is told in their box when it changes
//! (`talk::followed`): one line a page, raised by a reply, a revision, a
//! change of how it stands and a move, by anybody but themselves; never of
//! a page they may not read; and, named in a reply, by that line alone.
//! Stopped anywhere in the telling, told once.

use kernel::Event;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::found::Grain;
use crate::kept::{Person, person};
use crate::observed::node_with_stand_ins;
use crate::simulation::{Came, Place, Play, Scenario, simulate};

fn id(page: &Value) -> String {
    page["id"].as_str().unwrap().to_string()
}

/// `who` follows the page `page`.
async fn follows(rig: &Rig, who: &Person, page: &str) {
    let path = format!("/api/topics/{page}/follow");
    let (status, followed) = rig.call(Method::POST, &path, Some(&who.token), Some(json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{followed}");
}

/// The lines of `token`'s box, every one, of `kind`.
async fn lines(rig: &Rig, token: &str, kind: &str) -> Vec<Value> {
    let (status, found) = rig.call(Method::GET, "/api/inbox?all=true", Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{found}");
    found["lines"].as_array().unwrap().iter().filter(|line| line["kind"] == kind).cloned().collect()
}

/// Waits until `owner`'s line of `page` is raised the `count`th time, after `mark`.
async fn raised(rig: &Rig, owner: &str, page: &str, count: u64) {
    let (owner, page) = (owner.to_string(), page.to_string());
    rig.event("inbox.told", move |event| {
        event.data["owner"] == owner.as_str()
            && event.data["kind"] == "followed"
            && event.data["place"] == page.as_str()
            && event.data["count"] == count
    })
    .await;
}

/// Changes `fields` of the page `page` as `token`, as any record is.
async fn set(rig: &Rig, token: &str, page: &str, fields: Value) {
    let (status, changed) = rig
        .call(Method::PATCH, &format!("/api/records/{page}"), Some(token), Some(json!({ "fields": fields })))
        .await;
    assert_eq!(status, StatusCode::OK, "{changed}");
}

/// Replies `body` on `page` as `token`: the number it was said at.
async fn reply(rig: &Rig, token: &str, page: &str, body: &str) -> u64 {
    let path = format!("/api/topics/{page}/replies");
    let (status, said) = rig.call(Method::POST, &path, Some(token), Some(json!({ "body": body }))).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    said["seq"].as_u64().unwrap()
}

/// What a follower's one line of a page says: who did what (the history
/// line's event), the words of a reply, and how often it was raised.
fn said(line: &Value) -> (Value, Value, Value, Value) {
    let on = &line["on_record"];
    (on["who"].clone(), on["topic"].clone(), on["body"].clone(), line["count"].clone())
}

#[tokio::test]
async fn a_follower_is_told_of_each_change_of_a_page_by_one_line_that_opens_it_at_what_changed() {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let nell = person(&rig, "Nell").await;
    let handbook = id(&rig.send("/api/topics", json!({ "title": "Handbook" })).await);
    let page = id(&rig.send("/api/topics", json!({ "title": "Deploying", "under": handbook })).await);
    let other = id(&rig.send("/api/topics", json!({ "title": "Releases" })).await);
    follows(&rig, &nell, &page).await;

    // A reply: one line, about the page, opening it at the reply.
    let first = reply(&rig, &rig.token, &page, "Which tag do we deploy?").await;
    raised(&rig, &nell.actor, &page, 1).await;
    let told = lines(&rig, &nell.token, "followed").await;
    assert_eq!(told.len(), 1, "{told:?}");
    let line = &told[0];
    assert_eq!(
        (&line["about"], &line["place"], &line["why"]),
        (&json!(page), &json!(page), &json!("follows"))
    );
    assert_eq!(line["on_record"]["seq"], json!(rig.number(first)), "{line}");
    assert_eq!(
        (&line["on_record"]["title"], &line["on_record"]["path"]),
        (&json!("Deploying"), &json!(["Handbook"]))
    );
    assert_eq!(
        said(line),
        (json!("Ada"), json!("record.commented"), json!("Which tag do we deploy?"), json!(1))
    );
    assert!(line["on_record"].get("did").is_none(), "a reply says itself: {line}");
    // Read, it waits for nobody until the page changes again.
    let (status, _) = rig
        .call(
            Method::POST,
            &format!("/api/inbox/{}/read", line["id"].as_str().unwrap()),
            Some(&nell.token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Each of these raises the same line, unread again, saying what changed last.
    set(&rig, &rig.token, &page, json!({ "body": "Tag, then push." })).await;
    raised(&rig, &nell.actor, &page, 2).await;
    let line = lines(&rig, &nell.token, "followed").await[0].clone();
    assert_eq!((said(&line).1, line["read"].clone()), (json!("topic.revised"), Value::Null), "{line}");
    assert_eq!(line["on_record"]["did"], json!("Revised the page"), "what it did, in place of words");
    // Tracked as a question, it is answered by a reply; closed, opened again, and its status taken away.
    set(&rig, &rig.token, &page, json!({ "status": "open" })).await;
    raised(&rig, &nell.actor, &page, 3).await;
    let line = lines(&rig, &nell.token, "followed").await[0].clone();
    assert_eq!(line["on_record"]["did"], json!("Marked it open"), "{line}");
    let answer = json!({ "reply": first });
    rig.send(&format!("/api/topics/{page}/answer"), answer).await;
    raised(&rig, &nell.actor, &page, 4).await;
    set(&rig, &rig.token, &page, json!({ "status": "closed" })).await;
    raised(&rig, &nell.actor, &page, 5).await;
    set(&rig, &rig.token, &page, json!({ "status": "open" })).await;
    raised(&rig, &nell.actor, &page, 6).await;
    set(&rig, &rig.token, &page, json!({ "status": null })).await;
    raised(&rig, &nell.actor, &page, 7).await;
    let line = lines(&rig, &nell.token, "followed").await[0].clone();
    assert_eq!(
        (said(&line).1, line["on_record"]["did"].clone()),
        (json!("topic.untracked"), json!("Removed its status")),
        "{line}"
    );
    rig.send(&format!("/api/topics/{page}/move"), json!({ "under": other })).await;
    raised(&rig, &nell.actor, &page, 8).await;
    let told = lines(&rig, &nell.token, "followed").await;
    assert_eq!(told.len(), 1, "{told:?}");
    assert_eq!((said(&told[0]).1, said(&told[0]).3), (json!("topic.moved"), json!(8)));
    assert_eq!(told[0]["on_record"]["did"], json!("Moved it"));
    assert_eq!(told[0]["on_record"]["path"], json!(["Releases"]), "where it is now");

    // Following a page follows none above or under it: the wiki it was in, changed, tells her nothing,
    // and a page under hers neither; hers changed after them is the next she hears of.
    let under = id(&rig.send("/api/topics", json!({ "title": "Rolling back", "under": page })).await);
    set(&rig, &rig.token, &handbook, json!({ "body": "All of it." })).await;
    set(&rig, &rig.token, &under, json!({ "body": "Revert the tag." })).await;
    set(&rig, &rig.token, &page, json!({ "summary": "Tag, then push." })).await;
    raised(&rig, &nell.actor, &page, 9).await;
    let told = lines(&rig, &nell.token, "followed").await;
    assert_eq!(told.iter().map(|line| line["place"].clone()).collect::<Vec<_>>(), [json!(page)]);
    // No words in what is said of her box.
    let event = rig.event("inbox.told", |event| event.data["kind"] == "followed").await;
    assert!(!event.data.to_string().contains("Which tag"), "{:?}", event.data);
}

#[tokio::test]
async fn a_follower_is_not_told_of_their_own_change_and_a_flurry_of_revisions_is_one_line() {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let nell = person(&rig, "Nell").await;
    let page = id(&rig.send("/api/topics", json!({ "title": "Deploying" })).await);
    follows(&rig, &nell, &page).await;

    // Her own reply and revisions, and a key of hers, tell her nothing.
    reply(&rig, &nell.token, &page, "I will write it up.").await;
    set(&rig, &nell.token, &page, json!({ "body": "Draft." })).await;
    let key = crate::kept::key(&rig, &nell.token, &["records", "inbox"]).await;
    set(&rig, &key, &page, json!({ "body": "Draft, again." })).await;
    // Somebody else's flurry of revisions is one line, raised by each.
    for body in ["One.", "One, two.", "One, two, three."] {
        set(&rig, &rig.token, &page, json!({ "body": body })).await;
    }
    raised(&rig, &nell.actor, &page, 3).await;
    let told = lines(&rig, &nell.token, "followed").await;
    assert_eq!(told.len(), 1, "{told:?}");
    assert_eq!(said(&told[0]), (json!("Ada"), json!("topic.revised"), json!(""), json!(3)));
    let from = told[0]["from"].as_str().unwrap();
    assert!(from.starts_with("user:") && from != nell.actor, "{from}");
    // Once she follows no longer, nothing more.
    let path = format!("/api/topics/{page}/follow");
    rig.call(Method::POST, &path, Some(&nell.token), Some(json!({ "follows": false }))).await;
    set(&rig, &rig.token, &page, json!({ "body": "Four." })).await;
    reply(&rig, &rig.token, &page, "@nell is this right?").await;
    // Named, she is told as anybody named is: that is after the revision, which told her nothing.
    let told = nell.actor.clone();
    rig.event("inbox.told", move |event| {
        event.data["owner"] == told.as_str() && event.data["kind"] == "commented"
    })
    .await;
    assert_eq!(lines(&rig, &nell.token, "followed").await[0]["count"], json!(3));
}

#[tokio::test]
async fn a_follower_named_in_a_reply_is_told_by_one_line_and_somebody_named_who_follows_not_by_theirs() {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let (nell, mira) = (person(&rig, "Nell").await, person(&rig, "Mira").await);
    let page = id(&rig.send("/api/topics", json!({ "title": "Deploying" })).await);
    follows(&rig, &nell, &page).await;
    reply(&rig, &rig.token, &page, "@nell and @mira, which tag?").await;
    let told = mira.actor.clone();
    rig.event("inbox.told", move |event| event.data["owner"] == told.as_str()).await;
    raised(&rig, &nell.actor, &page, 1).await;
    // Nell follows the page: her line of it, saying the words that name her, and nothing more.
    let (status, all) = rig.call(Method::GET, "/api/inbox?all=true", Some(&nell.token), None).await;
    assert_eq!(status, StatusCode::OK);
    let kinds: Vec<&Value> = all["lines"].as_array().unwrap().iter().map(|line| &line["kind"]).collect();
    assert_eq!(kinds, [&json!("followed")], "{all}");
    assert_eq!(all["lines"][0]["on_record"]["body"], json!("@nell and @mira, which tag?"));
    assert_eq!(all["unread"], json!(1));
    // Mira does not: she is named, as anybody is.
    assert_eq!(lines(&rig, &mira.token, "commented").await.len(), 1);
    assert!(lines(&rig, &mira.token, "followed").await.is_empty());
}

/// The topics' table declared again, kept to whoever holds `private:wiki`.
const KEPT: &str = r#"
blueprint: kept-topics
permissions: [admin, projects, records]
steps:
  topics:
    do: collection.ensure
    with:
      project: "@"
      name: topics
      model: topic
      reads: private:wiki
      fields:
        title: { type: text, required: true }
        body: { type: document }
        summary: { type: document }
        status: { type: choice, of: [open, answered, closed, standing] }
        workspace: { type: text, quiet: true, hidden: true }
        project: { type: text, quiet: true, hidden: true }
        followers: { type: list, quiet: true, hidden: true }
        answer: { type: text, quiet: true }
        parent: { type: text, quiet: true, hidden: true }
      history:
        record.created: "{actor} made it"
        record.updated: "{actor} changed its {changed}"
        record.removed: "{actor} took it away"
        record.commented: "{actor} replied: {body}"
        topic.*: ""
"#;

#[tokio::test]
async fn a_follower_who_may_no_longer_read_the_pages_is_told_nothing_and_their_line_is_passed_over() {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let (nell, ola) = (person(&rig, "Nell").await, person(&rig, "Ola").await);
    let page = id(&rig.send("/api/topics", json!({ "title": "Deploying" })).await);
    follows(&rig, &nell, &page).await;
    follows(&rig, &ola, &page).await;
    reply(&rig, &rig.token, &page, "Tag, then push.").await;
    raised(&rig, &nell.actor, &page, 1).await;
    raised(&rig, &ola.actor, &page, 1).await;

    // The topics are kept to whoever holds `private:wiki`, which Ola is given and Nell is not.
    let ola_id = ola.actor.trim_start_matches("user:");
    let given = format!("/api/accounts/{ola_id}/given/private:wiki");
    let (status, answer) = rig.call(Method::PUT, &given, Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let applied = rig.send("/api/apply", json!({ "source": KEPT, "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let (status, _) = rig.call(Method::GET, &format!("/api/topics/{page}"), Some(&nell.token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "she may not read it now");

    // Changed, it tells Ola, and nothing of it is Nell's: no line told, and the one she had passed over.
    reply(&rig, &rig.token, &page, "The secret tag is v9.").await;
    raised(&rig, &ola.actor, &page, 2).await;
    let (status, all) = rig.call(Method::GET, "/api/inbox?all=true", Some(&nell.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!((all["lines"].clone(), all["unread"].clone()), (json!([]), json!(0)), "{all}");
    let ola_line = &lines(&rig, &ola.token, "followed").await[0];
    assert_eq!(ola_line["on_record"]["body"], json!("The secret tag is v9."));
    // Told once, before; her line and Ola's are told in one write, so nothing of hers is still to come.
    let query = journal::EventQuery { topics: "inbox.told".into(), limit: 100, ..Default::default() };
    let told = rig.node().app.journal.read(query).await.unwrap();
    let hers = told.iter().filter(|event| event.data["owner"] == nell.actor.as_str()).count();
    assert_eq!(hers, 1);
}

/// What the owner says on a page Nell follows.
const SAID: &str = "Tag, then push.";

/// The owner replies on a page Nell follows, which tells her.
#[derive(Clone)]
struct Followed;

impl Scenario for Followed {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-followers").await;
        rig.send("/api/tier", json!({ "tier": "premium" })).await;
        let nell = person(&rig, "Nell").await;
        let page = id(&rig.send("/api/topics", json!({ "title": "Deploying" })).await);
        follows(&rig, &nell, &page).await;
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("page", page);
        play.notes.insert("nell", nell.token);
        play.notes.insert("actor", nell.actor);
        play
    }

    /// Looking first, as a person does: a reply the daemon went away in
    /// the middle of is in the page's history, and is not said again.
    async fn act(&self, _: usize, play: &mut Play) {
        let page = play.note("page").to_string();
        if replies(play, &page).await.is_empty() {
            play.rig.send(&format!("/api/topics/{page}/replies"), json!({ "body": SAID })).await;
        }
    }

    fn heard(&self, _: usize, play: &Play, event: &Event) -> bool {
        event.topic == "inbox.told"
            && event.data["owner"] == play.note("actor")
            && event.data["kind"] == "followed"
    }

    async fn came_to(&self, play: &Play) -> Came {
        let page = play.note("page").to_string();
        let (_, found) =
            play.rig.call(Method::GET, "/api/inbox?all=true", Some(play.note("nell")), None).await;
        let lines = found["lines"].as_array().into_iter().flatten();
        let boxed: Value = lines
            .map(|line| {
                json!([line["kind"], line["place"], line["count"], line["on_record"]["body"], line["read"]])
            })
            .collect();
        Came::default().with("said", json!(replies(play, &page).await)).with("boxed", boxed)
    }
}

/// The words of each reply on the page, as its history keeps them.
async fn replies(play: &Play, page: &str) -> Vec<Value> {
    let history = play.rig.read(&format!("/api/records/{page}/history")).await;
    let lines = history["history"].as_array().into_iter().flatten();
    lines
        .filter(|line| line["topic"] == "record.commented")
        .map(|line| line["data"]["body"].clone())
        .collect()
}

/// As the reply is said, as its line of history is written, and as whoever follows the page is told.
fn in_the_telling(place: &Place) -> bool {
    matches!(place.topic.as_str(), "record.commented" | "collection.history" | "inbox.told")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_follower_is_told_once_of_a_reply_wherever_the_daemon_is_stopped_in_the_telling() {
    let found =
        simulate("a follower told of a reply", Followed, Grain::asked_or(Grain::Where(in_the_telling))).await;
    found.holds();
}
