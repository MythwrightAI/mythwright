//! A change an agent asks for and may not make, approved by a person,
//! stopped anywhere in the asking and the approving and started again
//! (`told::asks`, `docs/approvals.md`).
//!
//! An agent of a project asks, from a channel, for a cell of a record
//! that takes `admin`; whoever could make it is told by a line of their
//! inbox, and the owner approves it, which makes the change as the agent
//! and tells the agent in the channel. The ask's row and what is said of
//! it are one write, and so are the change and its approval; the lines
//! and the telling back are owed by the row until they are done, and a
//! start finishes them. Wherever the daemon goes, the change is made
//! once, the owner has one line of it, told once, and the agent is told
//! once. Declined, nothing is made, and the agent is told why, once.

use kernel::Event;
use serde_json::{Value, json};

use crate::found::Grain;
use crate::observed::node_with_stand_ins;
use crate::simulation::{Came, Place, Play, Scenario, simulate};

const SETTINGS: &str = "blueprint: settings
steps:
  table:
    do: collection.ensure
    with:
      project: shop
      name: settings
      fields:
        title: { type: text }
        note: { type: text, takes: admin }
      history:
        record.updated: \"{actor} changed its {changed}\"
        record.asked: \"{actor} asked to change it: {why}\"
        record.ask.declined: \"{actor} declined a change of it[: {why}]\"
";

/// The credential an agent's program reads each time it asks the factory anything.
fn token_of(play: &Play, agent: &str) -> String {
    let file = play.rig.dir.join(format!("data/slots/{agent}/token"));
    std::fs::read_to_string(file).unwrap().trim().to_string()
}

/// The ask, and then the owner approving it, or declining it with why.
#[derive(Clone)]
struct Decided {
    approve: bool,
}

impl Scenario for Decided {
    const ACTS: usize = 2;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-approving").await;
        rig.send("/api/projects", json!({ "name": "Shop" })).await;
        let applied =
            rig.send("/api/apply", json!({ "source": SETTINGS, "project": "shop", "wait": true })).await;
        assert_eq!(applied["finished"]["status"], "ok", "{applied}");
        let row = json!({ "fields": { "title": "Front door" } });
        rig.send("/api/collections/shop/settings/records", row).await;
        rig.send("/api/channels", json!({ "name": "talk", "project": "shop" })).await;
        let helper = json!({ "name": "helper", "adapter": "builtin", "project": "shop", "channel": "talk" });
        rig.send("/api/agents", helper).await;
        Play::on(rig, &at).await
    }

    /// Asking again what waits is the ask there is, and approving what
    /// is approved is as it was decided: so an act the daemon went away
    /// in the middle of is done again as it is.
    async fn act(&self, nth: usize, play: &mut Play) {
        match nth {
            0 => {
                let token = token_of(play, "helper");
                let asked = json!({
                    "record": "shop-settings-1", "set": { "note": "loud" }, "why": "hard to hear", "channel": "talk",
                });
                let (status, came) =
                    play.rig.call(reqwest::Method::POST, "/api/asked", Some(&token), Some(asked)).await;
                assert_eq!(status, reqwest::StatusCode::OK, "{came}");
                play.notes.insert("ask", came["ask"]["ask"].as_str().unwrap().to_string());
            }
            _ => {
                let how = if self.approve { "approve" } else { "decline" };
                let path = format!("/api/asked/{}/{how}", play.note("ask"));
                let (status, came) = play.rig.post(&path, json!({ "why": "not on a Friday" })).await;
                assert_eq!(status, reqwest::StatusCode::OK, "{came}");
            }
        }
    }

    fn heard(&self, nth: usize, _: &Play, event: &Event) -> bool {
        match nth {
            0 => event.topic == "inbox.told",
            _ => event.topic == "channel.message" && body(&event.data).contains("d your change"),
        }
    }

    async fn came_to(&self, play: &Play) -> Came {
        let rig = &play.rig;
        let record = rig.read("/api/records/shop-settings-1").await;
        let asks = rig.read("/api/asked").await;
        let asks: Vec<Value> =
            asks["asked"].as_array().into_iter().flatten().map(|ask| ask["status"].clone()).collect();
        let inbox = rig.read("/api/inbox?all=true").await;
        let lines: Vec<Value> = inbox["lines"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|line| json!([line["says"], line["about"], line["count"]]))
            .collect();
        let talk = rig.read("/api/channels/talk/messages").await;
        let told =
            talk["messages"].as_array().into_iter().flatten().filter(|one| body(one).contains("your change"));
        let history = rig.read("/api/records/shop-settings-1/history").await;
        let history: Vec<Value> =
            history["history"].as_array().into_iter().flatten().map(|line| line["said"].clone()).collect();
        Came::default()
            .with("note", record["data"]["note"].clone())
            .with("asks", json!(asks))
            .with("lines", json!(lines))
            .with("told", json!(told.map(body).collect::<Vec<_>>()))
            .with("history", json!(history))
    }
}

fn body(data: &Value) -> &str {
    data["body"].as_str().unwrap_or_default()
}

/// Where an ask is kept, told, approved, made and told back.
fn in_the_asking(place: &Place) -> bool {
    place.topic.starts_with("record.ask")
        || ["inbox.told", "record.updated", "channel.message"].contains(&place.topic.as_str())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_change_asked_for_and_approved_is_made_once_and_told_once_wherever_the_daemon_is_stopped_in_it() {
    let grain = Grain::asked_or(Grain::Where(in_the_asking));
    simulate("a change asked for and approved", Decided { approve: true }, grain).await.holds();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_change_asked_for_and_declined_makes_nothing_and_is_told_once_wherever_the_daemon_is_stopped_in_it()
{
    let grain = Grain::asked_or(Grain::Where(in_the_asking));
    simulate("a change asked for and declined", Decided { approve: false }, grain).await.holds();
}
