//! Where somebody has read to among the events: text the daemon hands
//! out and takes back as it was (`api::cursor`). Nobody outside reads it
//! or adds to it, so that what it is made of can change: one journal's
//! number today, a place in each project's journal when there are those.

use reqwest::StatusCode;
use serde_json::{Value, json};

use crate::{Rig, soon};

/// The ids and titles of the cards the stream says were added, once it
/// has said `count` of them.
async fn added(response: &mut reqwest::Response, count: usize) -> Vec<(String, String)> {
    let mut text = String::new();
    loop {
        let frames: Vec<(String, String)> = text
            .split("\n\n")
            .filter(|frame| frame.contains("data:"))
            .filter_map(|frame| {
                let line =
                    |prefix: &str| frame.lines().find_map(|line| line.strip_prefix(prefix)).map(str::trim);
                let event: Value = serde_json::from_str(line("data:")?).ok()?;
                Some((line("id:")?.to_string(), event["data"]["title"].as_str()?.to_string()))
            })
            .collect();
        if frames.len() >= count && text.ends_with("\n\n") {
            return frames;
        }
        let chunk =
            soon("more of the event stream", response.chunk()).await.unwrap().expect("the stream ended");
        text.push_str(&String::from_utf8_lossy(&chunk));
    }
}

/// The titles of the cards `path` says were added, oldest first.
async fn titles(rig: &Rig, path: &str) -> Vec<String> {
    let found = rig.read(path).await;
    let mut titles: Vec<String> = found["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["data"]["title"].as_str().unwrap().to_string())
        .collect();
    titles.reverse();
    titles
}

#[tokio::test]
async fn where_the_events_have_got_to_is_text_handed_out_and_taken_back_as_it_was() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    rig.send("/api/board/shop/items", json!({ "title": "Before" })).await;

    // What was read comes with the place to go on from. It is text, and
    // no number: there is nothing in it to add to. The number of the last
    // event there was is beside it, as it always was.
    let read = rig.read("/api/events/recent?topics=board.item.created").await;
    assert_eq!(read["events"].as_array().map(Vec::len), Some(1), "{read}");
    let cursor = read["cursor"].as_str().unwrap_or_else(|| panic!("a place, as text: {read}")).to_string();
    assert!(cursor.parse::<f64>().is_err(), "{cursor}");
    let last = read["last_seq"].as_u64().unwrap();
    // The fleet's reading hands out one of the same kind.
    let fleet = rig.read("/api/fleet").await;
    assert!(
        fleet["cursor"].as_str().is_some_and(|place| place.parse::<f64>().is_err()),
        "{}",
        fleet["cursor"]
    );

    rig.send("/api/board/shop/items", json!({ "title": "First" })).await;
    rig.send("/api/board/shop/items", json!({ "title": "Second" })).await;

    // Handed back as it was, it reads what was said after it and nothing
    // from before: asked of what is kept, and followed.
    let after = |place: &str| format!("/api/events/recent?topics=board.item.created&after={place}");
    assert_eq!(titles(&rig, &after(&cursor)).await, ["First", "Second"]);
    let follow = |since: String| {
        let address = format!("{}/api/events?topics=board.item.created&since={since}", rig.url);
        reqwest::Client::new().get(address).bearer_auth(&rig.token).send()
    };
    let mut stream = follow(cursor.clone()).await.unwrap();
    assert_eq!(stream.status(), StatusCode::OK);
    let heard = added(&mut stream, 2).await;
    assert_eq!([heard[0].1.as_str(), heard[1].1.as_str()], ["First", "Second"]);
    drop(stream);
    // Each frame of the stream says the place after it the same way, and
    // that too is handed back as it was, in the address as in the header
    // a browser sends by itself.
    assert!(heard.iter().all(|(id, _)| id.parse::<f64>().is_err()), "{heard:?}");
    let mut stream = follow(heard[0].0.clone()).await.unwrap();
    assert_eq!(added(&mut stream, 1).await[0].1, "Second");
    drop(stream);
    assert_eq!(titles(&rig, &after(&heard[0].0)).await, ["Second"]);

    // An event's number is still taken, and means what it did: what was
    // said after the event of that number. It is what was handed out
    // before there was a place, and what a page left open still sends.
    assert_eq!(titles(&rig, &after(&last.to_string())).await, ["First", "Second"]);
    let mut stream = follow(last.to_string()).await.unwrap();
    assert_eq!(added(&mut stream, 2).await.len(), 2);
    drop(stream);

    // Anything else is no place, and whoever wrote it is told so and what
    // to hand back.
    for path in ["/api/events/recent?after=yesterday", "/api/events?since=at-first"] {
        let (status, refused) = rig.get(path).await;
        let message = refused["error"]["message"].as_str().unwrap_or_default();
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}: {refused}");
        assert!(
            message.contains("is not a place among the events") && message.contains("hand back a `cursor`"),
            "{path}: {refused}"
        );
    }
    // What a browser sends by itself and does not read is passed over, as
    // it was: the stream is opened from now.
    let odd = reqwest::Client::new()
        .get(format!("{}/api/events", rig.url))
        .bearer_auth(&rig.token)
        .header("last-event-id", "yesterday")
        .send()
        .await
        .unwrap();
    assert_eq!(odd.status(), StatusCode::OK);
}

/// The frames of the stream until one that `until` takes, each as its
/// kind (`message` where it says none) and what it carries.
async fn frames(
    response: &mut reqwest::Response,
    until: impl Fn(&str, &Value) -> bool,
) -> Vec<(String, Value)> {
    let mut text = String::new();
    loop {
        let frames: Vec<(String, Value)> = text
            .split("\n\n")
            .filter(|frame| frame.contains("data:"))
            .filter_map(|frame| {
                let line =
                    |prefix: &str| frame.lines().find_map(|line| line.strip_prefix(prefix)).map(str::trim);
                let data: Value = serde_json::from_str(line("data:")?).ok()?;
                Some((line("event:").unwrap_or("message").to_string(), data))
            })
            .collect();
        if text.ends_with("\n\n") && frames.iter().any(|(kind, data)| until(kind, data)) {
            return frames;
        }
        let chunk =
            soon("more of the event stream", response.chunk()).await.unwrap().expect("the stream ended");
        text.push_str(&String::from_utf8_lossy(&chunk));
    }
}

#[tokio::test]
async fn a_stream_gone_on_from_a_place_before_what_was_passed_on_says_first_that_it_is_behind() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    let cursor = rig.read("/api/events/recent?limit=1").await["cursor"].as_str().unwrap().to_string();
    // A run nothing hears: what the runner says of it is passed on as it
    // is said, and kept nowhere.
    let source =
        "blueprint: once\nstate: { done: false }\nsteps:\n  keep:\n    do: set\n    with: { done: true }\n";
    let applied = rig.send("/api/apply", json!({ "source": source, "key": "once", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let follow = |since: Option<&str>| {
        let since = since.map(|since| format!("&since={since}")).unwrap_or_default();
        let address = format!("{}/api/events?topics=board.item.created{since}", rig.url);
        reqwest::Client::new().get(address).bearer_auth(&rig.token).send()
    };

    // Gone on from before it, the stream says first that it is behind,
    // and which topics may be missing; then it goes on as it always did.
    let mut stream = follow(Some(&cursor)).await.unwrap();
    rig.send("/api/board/shop/items", json!({ "title": "After" })).await;
    let heard = frames(&mut stream, |_, data| data["data"]["title"] == "After").await;
    let (kind, said) = &heard[0];
    assert_eq!(kind, "behind", "{heard:?}");
    let topics = said["topics"].as_array().unwrap();
    assert!(topics.contains(&json!("run.finished")) && topics.contains(&json!("step.started")), "{said}");
    assert!(heard[1..].iter().all(|(kind, _)| kind == "message"), "{heard:?}");
    drop(stream);

    // Followed from now, nothing was missed and nothing says so; the
    // stream says first where now is, in a frame with nothing in it, so
    // that a browser that has to try again goes on from there.
    let mut stream = follow(None).await.unwrap();
    let opening = soon("the stream's first frame", stream.chunk()).await.unwrap().expect("the stream ended");
    let opening = String::from_utf8_lossy(&opening).to_string();
    let place = opening.lines().find_map(|line| line.strip_prefix("id:")).map(str::trim).unwrap_or_default();
    assert!(!place.is_empty() && place.parse::<f64>().is_err() && !opening.contains("data:"), "{opening}");
    rig.send("/api/board/shop/items", json!({ "title": "Now" })).await;
    let heard = frames(&mut stream, |_, data| data["data"]["title"] == "Now").await;
    assert!(heard.iter().all(|(kind, _)| kind == "message"), "{heard:?}");
}
