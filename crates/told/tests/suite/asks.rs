//! A change somebody asks for that they may not make themselves: made at
//! once where they may, kept and told to whoever could make it where they
//! may not, approved (the change and its approval one write, made as the
//! asker) or declined, said back to the asker where they asked, and
//! nothing made once its record moved on or it waited its time.

use std::time::Duration;

use kernel::{ErrorKind, Principal};
use serde_json::{Map, Value, json};
use studio::CredentialKind;
use studio::collections::Spec;
use told::{Asks, Came};

use crate::{Rig, agent};

fn data(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

/// The helper, an agent that may write records and nothing more.
fn helper() -> Principal {
    agent("helper", &["projects", "records", "channels"])
}

/// A project with a table of workers, whose `permission` set to `full`
/// takes `admin`, which a member does not hold, and one row of it.
async fn workers(rig: &Rig) -> String {
    rig.studio.projects.create("Shop", None, &rig.ada).await.unwrap();
    // The helper can act, with what it holds: a change is made as it only while it can.
    let scopes = Some(helper().scopes);
    rig.studio.identity.issue(CredentialKind::Agent, "helper", "helper", scopes).await.unwrap();
    let fields = json!({
        "title": { "type": "text", "required": true },
        "count": { "type": "number" },
        "permission": { "type": "choice", "of": ["edit", "full"], "takes": { "full": "admin" } },
    });
    let spec = Spec::read(&fields, &json!(null), None).unwrap();
    rig.studio.collections.ensure("shop", "workers", spec, None, &rig.ada).await.unwrap();
    let row = data(json!({ "title": "Worker", "count": 1, "permission": "edit" }));
    rig.studio.records.add("shop", "workers", row, &rig.ada, None).await.unwrap().id
}

fn asks(rig: &Rig) -> Asks {
    Asks::new(&rig.journal, &rig.studio)
}

async fn asked(rig: &Rig, by: &Principal, row: &str, fields: Value, place: Value) -> told::Ask {
    match asks(rig).ask(by, row, data(fields), "it needs to run the tests", None, place).await.unwrap() {
        Came::Asked(ask) => ask,
        other => panic!("expected it to be asked, got {other:?}"),
    }
}

#[tokio::test]
async fn a_change_the_asker_may_make_is_made_at_once_as_theirs_and_nothing_is_kept() {
    let rig = Rig::new().await;
    let row = workers(&rig).await;
    let came =
        asks(&rig).ask(&helper(), &row, data(json!({ "count": 2 })), "", None, json!({})).await.unwrap();
    let Came::Made(record) = came else { panic!("expected it to be made, got {came:?}") };
    assert_eq!(record.data["count"], json!(2));
    let updated = rig.events("record.updated").await;
    assert_eq!(updated.last().unwrap().source.actor.as_deref(), Some("agent:helper"));
    assert!(updated.last().unwrap().data.get("approved_by").is_none());
    assert!(rig.events("record.asked").await.is_empty(), "nothing was asked");
    // Asked for what is so already, nothing happens.
    let again =
        asks(&rig).ask(&helper(), &row, data(json!({ "count": 2 })), "", None, json!({})).await.unwrap();
    assert!(matches!(again, Came::Nothing(_)), "{again:?}");
}

#[tokio::test]
async fn a_change_the_asker_may_not_make_is_kept_unmade_and_told_to_whoever_could_make_it_and_asked_again_is_the_same()
 {
    let rig = Rig::new().await;
    let row = workers(&rig).await;
    let ask = asked(&rig, &helper(), &row, json!({ "permission": "full" }), json!({})).await;
    assert_eq!((ask.status.as_str(), ask.named.clone()), ("waiting", vec![format!("user:{}", rig.ada.id)]));
    assert_eq!(ask.cells, json!([{ "name": "permission", "before": "edit", "after": "full" }]));
    let (_, kept) = rig.studio.collections.record(&row).await.unwrap();
    assert_eq!(kept.data["permission"], json!("edit"), "nothing changes until it is approved");
    // Ada, who holds `admin`, is told; Grace, who does not, is not.
    let lines = rig.studio.channels.inbox.list(&rig.ada, false).await.unwrap();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0].from, "agent:helper");
    assert!(rig.studio.channels.inbox.list(&rig.grace, false).await.unwrap().is_empty());
    let told = rig.events("record.asked").await;
    assert_eq!(told.len(), 1);
    assert_eq!(told[0].data["cells"], json!(["permission"]), "the cells' names, and never their values");
    // A turn begun again asks the same words again, and is answered with the ask there is.
    let again = asked(&rig, &helper(), &row, json!({ "permission": "full" }), json!({})).await;
    assert_eq!(again.id, ask.id);
    assert_eq!(rig.events("record.asked").await.len(), 1);
}

#[tokio::test]
async fn approved_it_is_made_as_the_asker_with_who_approved_it_and_approving_again_changes_nothing() {
    let rig = Rig::new().await;
    let row = workers(&rig).await;
    let ask = asked(&rig, &helper(), &row, json!({ "permission": "full" }), json!({})).await;
    // Grace could not make it, so she may not approve it, nor may the asker.
    let refused = asks(&rig).decide(&rig.grace, &ask.id, true, "").await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied, "{refused:?}");
    let refused = asks(&rig).decide(&helper(), &ask.id, true, "").await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied, "{refused:?}");

    let approved = asks(&rig).decide(&rig.ada, &ask.id, true, "").await.unwrap();
    assert_eq!(approved.status, "approved");
    let (_, kept) = rig.studio.collections.record(&row).await.unwrap();
    assert_eq!(kept.data["permission"], json!("full"));
    let updated = rig.events("record.updated").await;
    let made = updated.last().unwrap();
    assert_eq!(made.source.actor.as_deref(), Some("agent:helper"), "credited to whoever asked");
    assert_eq!(made.data["approved_by"], json!(format!("user:{}", rig.ada.id)));
    assert_eq!(made.data["asked"], json!(ask.id));
    let decided: Vec<String> =
        rig.events("record.ask.*").await.into_iter().map(|event| event.topic).collect();
    assert_eq!(decided, ["record.ask.approved", "record.ask.decided"]);
    // Pressed again, it is as it was decided, and nothing more is made or said.
    let again = asks(&rig).decide(&rig.ada, &ask.id, true, "").await.unwrap();
    assert_eq!(again.status, "approved");
    assert_eq!(rig.events("record.updated").await.len(), updated.len());
    assert_eq!(rig.events("record.ask.*").await.len(), 2);
}

#[tokio::test]
async fn declined_nothing_is_made_and_the_asker_is_told_in_the_thread_it_asked_from_once() {
    let rig = Rig::new().await;
    let row = workers(&rig).await;
    let channel = rig.studio.channels.create("work", Some("shop"), "", &rig.ada).await.unwrap();
    let place = json!({ "channel": channel.id });
    let ask = asked(&rig, &helper(), &row, json!({ "permission": "full" }), place).await;
    let declined = asks(&rig).decide(&rig.ada, &ask.id, false, "not on a Friday").await.unwrap();
    assert_eq!(declined.status, "declined");
    let (_, kept) = rig.studio.collections.record(&row).await.unwrap();
    assert_eq!(kept.data["permission"], json!("edit"));
    // What is owed is done once, however often a start looks.
    asks(&rig).settle().await.unwrap();
    let reader = talk::Channels::daemon("a test reads what the bot said");
    let said = rig.studio.channels.messages(&reader, &channel.id, None, 50).await.unwrap();
    let told: Vec<&str> = said.iter().map(|message| message.body.as_str()).collect();
    assert_eq!(told, [format!("@helper Ada declined your change of {row}: not on a Friday.").as_str()]);
    // One that may no longer write there is told in its box instead.
    rig.studio.identity.revoke_subject(CredentialKind::Agent, "helper").await.unwrap();
    let place = json!({ "channel": channel.id });
    let ask = asked(&rig, &helper(), &row, json!({ "permission": "full" }), place).await;
    asks(&rig).decide(&rig.ada, &ask.id, false, "still no").await.unwrap();
    assert_eq!(rig.studio.channels.messages(&reader, &channel.id, None, 50).await.unwrap().len(), 1);
    let boxed = rig.studio.channels.inbox.list(&agent("helper", &["inbox"]), false).await.unwrap();
    let says: Vec<_> = boxed.iter().filter_map(|line| line.says.as_deref()).collect();
    assert_eq!(says, [format!("Ada declined your change of {row}: still no.").as_str()]);
}

#[tokio::test]
async fn an_ask_whose_record_moved_on_or_that_waited_its_time_makes_nothing() {
    let rig = Rig::new().await;
    let row = workers(&rig).await;
    let ask = asked(&rig, &helper(), &row, json!({ "permission": "full" }), json!({})).await;
    rig.clock.advance(Duration::from_millis(10));
    rig.studio.records.update(&row, data(json!({ "count": 3 })), &rig.ada, None).await.unwrap();
    let stale = asks(&rig).decide(&rig.ada, &ask.id, true, "").await.unwrap();
    assert_eq!(stale.status, "stale", "changed since it was asked");
    let (_, kept) = rig.studio.collections.record(&row).await.unwrap();
    assert_eq!(kept.data["permission"], json!("edit"));

    let late = asked(&rig, &helper(), &row, json!({ "permission": "full" }), json!({})).await;
    rig.clock.advance(Duration::from_millis(kernel::approvals::LASTS_MS as u64));
    let expired = asks(&rig).decide(&rig.ada, &late.id, true, "").await.unwrap();
    assert_eq!(expired.status, "expired");
}

#[tokio::test]
async fn an_asker_has_at_most_so_many_asks_waiting_at_once() {
    let rig = Rig::new().await;
    workers(&rig).await;
    for n in 0..kernel::approvals::MOST_WAITING {
        let row = data(json!({ "title": format!("Worker {n}"), "permission": "edit" }));
        let row = rig.studio.records.add("shop", "workers", row, &rig.ada, None).await.unwrap().id;
        asked(&rig, &helper(), &row, json!({ "permission": "full" }), json!({})).await;
    }
    let row = data(json!({ "title": "One more", "permission": "edit" }));
    let row = rig.studio.records.add("shop", "workers", row, &rig.ada, None).await.unwrap().id;
    let refused =
        asks(&rig).ask(&helper(), &row, data(json!({ "permission": "full" })), "", None, json!({})).await;
    assert_eq!(refused.unwrap_err().kind, ErrorKind::Conflict);
}

#[tokio::test]
async fn a_change_nobody_here_could_make_is_refused_and_a_change_refused_for_other_than_scopes_is_refused_as_it_was()
 {
    let rig = Rig::new().await;
    let row = workers(&rig).await;
    // A value the field does not offer is refused as the change would be, and nothing is kept.
    let refused = asks(&rig)
        .ask(&helper(), &row, data(json!({ "permission": "root" })), "", None, json!({}))
        .await
        .unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid, "{refused:?}");
    // Somebody who may not read the records asks nothing.
    let blind = agent("blind", &["projects"]);
    let refused = asks(&rig)
        .ask(&blind, &row, data(json!({ "permission": "full" })), "", None, json!({}))
        .await
        .unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied, "{refused:?}");
    assert!(rig.events("record.asked").await.is_empty());
}

#[tokio::test]
async fn nothing_is_made_as_an_asker_that_cannot_act_and_one_taken_away_has_its_asks_withdrawn_telling_nobody()
 {
    let rig = Rig::new().await;
    let row = workers(&rig).await;
    let first = asked(&rig, &helper(), &row, json!({ "permission": "full" }), json!({})).await;
    rig.studio.identity.revoke_subject(CredentialKind::Agent, "helper").await.unwrap();
    let refused = asks(&rig).decide(&rig.ada, &first.id, true, "").await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict, "{refused:?}");
    assert!(refused.message.starts_with("helper cannot act now"), "{refused:?}");
    let withdrawn = asks(&rig).withdraw_all_of("agent:helper", "helper was taken away").await.unwrap();
    assert_eq!(withdrawn, 1);
    let ask = asks(&rig).one(&first.id).await.unwrap();
    assert_eq!(
        (ask.status.as_str(), &ask.decided.unwrap()["why"]),
        ("withdrawn", &json!("helper was taken away"))
    );
    // Nobody is told back, and nothing is owed.
    let boxed = rig.studio.channels.inbox.list(&agent("helper", &["inbox"]), true).await.unwrap();
    assert!(boxed.is_empty(), "{boxed:?}");
    assert_eq!(asks(&rig).withdraw_all_of("agent:helper", "again").await.unwrap(), 0);
    let (_, kept) = rig.studio.collections.record(&row).await.unwrap();
    assert_eq!(kept.data["permission"], json!("edit"));
}
