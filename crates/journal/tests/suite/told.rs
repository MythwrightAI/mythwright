//! Who an event may be handed to, by what it says hearing it takes
//! (`journal::told`): a table of cases, since everything that hands an
//! event to somebody asks this one question.

use std::sync::{Arc, Mutex};

use journal::Journal;
use kernel::{Event, ManualClock, Principal, PrincipalKind, Source};
use serde_json::json;

/// What was asked of whatever answers who is a member: of what, by whom
/// as an actor, holding what.
type Asked = (String, String, Vec<String>);

fn who(id: &str, scopes: &[&str]) -> Principal {
    Principal {
        kind: PrincipalKind::User,
        id: id.into(),
        name: id.into(),
        scopes: scopes.iter().map(|scope| scope.to_string()).collect(),
        key: None,
    }
}

fn taking(takes: Option<&str>) -> Event {
    let source = Source::actor("user:ada").taking(takes);
    Event { seq: 7, ts: 0, topic: "record.updated".into(), source, data: json!({ "id": "life-moods-1" }) }
}

async fn memory() -> Journal {
    Journal::memory(Arc::new(ManualClock::new())).await.unwrap()
}

#[tokio::test]
async fn an_event_is_handed_to_whoever_holds_what_it_takes_and_one_that_takes_nothing_to_anybody() {
    let journal = memory().await;
    // What the event takes, what somebody holds, and whether they are told.
    let cases: &[(Option<&str>, &[&str], bool)] = &[
        (None, &[], true),
        (None, &["blueprints:read"], true),
        (Some("private:journal"), &["private:journal"], true),
        (Some("private:journal"), &["private"], true),
        (Some("private:journal"), &["*"], true),
        (Some("private:journal"), &["private:letters"], false),
        (Some("private:journal"), &["projects", "records", "blueprints", "admin"], false),
        (Some("private:journal"), &[], false),
        (Some("private"), &["private:journal"], false),
        (Some("private"), &["private"], true),
    ];
    for (takes, holds, told) in cases {
        let (event, asks) = (taking(*takes), who("grace", holds));
        assert_eq!(journal.may_tell(&asks, &event), *told, "{takes:?} to whoever holds {holds:?}");
        // And the same of whoever is known only by what they held.
        assert_eq!(
            journal.may_tell_as("user:grace", &asks.scopes, &event),
            *told,
            "{takes:?} to whoever held {holds:?}"
        );
    }
}

#[tokio::test]
async fn what_is_kept_to_its_members_is_handed_to_nobody_until_something_answers_who_they_are() {
    let journal = memory().await;
    let event = taking(Some("member:ch_1"));
    // Nothing answers: nobody is told, whatever they hold.
    for holds in [&["*"][..], &["member"], &["member:ch_1"], &["private", "admin", "channels"], &[]] {
        assert!(!journal.may_tell(&who("grace", holds), &event), "told to whoever holds {holds:?}");
        assert!(!journal.may_tell(&Principal::system(), &event), "told to the system");
    }

    // Something answers: it is asked with what the thing is, who asks and
    // what they hold, each time, and its answer is the answer.
    let asked: Arc<Mutex<Vec<Asked>>> = Arc::default();
    journal.members_with(Arc::new({
        let asked = asked.clone();
        move |of, actor, held| {
            asked.lock().unwrap().push((of.to_string(), actor.to_string(), held.to_vec()));
            of == "ch_1" && actor == "user:grace"
        }
    }));
    assert!(journal.may_tell(&who("grace", &["channels:read"]), &event));
    assert!(!journal.may_tell(&who("ada", &["*"]), &event), "holding everything is not being a member");
    assert!(!journal.may_tell(&who("grace", &["channels:read"]), &taking(Some("member:ch_2"))));
    assert!(journal.may_tell_as("user:grace", &[], &event));
    assert_eq!(
        asked.lock().unwrap()[..2],
        [
            ("ch_1".to_string(), "user:grace".to_string(), vec!["channels:read".to_string()]),
            ("ch_1".to_string(), "user:ada".to_string(), vec!["*".to_string()]),
        ]
    );
    // It is not asked of what takes a scope, or nothing.
    let before = asked.lock().unwrap().len();
    assert!(journal.may_tell(&who("ada", &["private"]), &taking(Some("private:journal"))));
    assert!(journal.may_tell(&who("ada", &[]), &taking(None)));
    assert_eq!(asked.lock().unwrap().len(), before);

    // Said once: what is said a second time is not taken.
    journal.members_with(Arc::new(|_, _, _| true));
    assert!(!journal.may_tell(&who("ada", &["*"]), &event));
}

#[tokio::test]
async fn every_journal_of_a_factory_asks_the_one_thing_that_answers_who_is_a_member() {
    let factory = memory().await;
    factory.keeps_projects_in_memory().await.unwrap();
    // A project's journal made before anything answers, and one made after.
    let shop = factory.own("shop").await.unwrap();
    factory.members_with(Arc::new(|of, actor, _| of == "ch_1" && actor == "user:grace"));
    let life = factory.own("life").await.unwrap();
    // Each is a journal of its own, but where the suite is played with one
    // journal (`journal::apart`): there `own` gives the factory's, and what
    // is held below is that the one journal there is answers.
    if journal::apart() != Some(false) {
        assert!(!shop.db().is(factory.db()) && !life.db().is(factory.db()));
    }
    let event = taking(Some("member:ch_1"));
    for journal in [&factory, &shop, &life] {
        assert!(journal.may_tell(&who("grace", &[]), &event));
        assert!(!journal.may_tell(&who("ada", &["*"]), &event));
    }
}

#[tokio::test]
async fn whoever_answers_from_what_it_keeps_is_handed_the_one_memory_that_is_asked_however_often_it_says_so()
{
    type Kept = Mutex<Vec<&'static str>>;
    let answer = |kept: Arc<Kept>| -> journal::Members {
        Arc::new(move |_, actor, _| kept.lock().unwrap().contains(&actor))
    };
    let journal = memory().await;
    let event = taking(Some("member:ch_1"));
    let first = journal.members_from(Arc::new(Kept::new(vec!["user:grace"])), answer).unwrap();
    assert!(journal.may_tell(&who("grace", &[]), &event) && !journal.may_tell(&who("ada", &["*"]), &event));
    // Said again, with a memory of its own: what stands is handed back,
    // and what is changed there is what is asked.
    let again = journal.members_from(Arc::new(Kept::new(vec!["user:ada"])), answer).unwrap();
    assert!(Arc::ptr_eq(&first, &again));
    assert!(!journal.may_tell(&who("ada", &["*"]), &event), "the second memory answers nothing");
    again.lock().unwrap().push("user:ada");
    assert!(journal.may_tell(&who("ada", &[]), &event));
    // Nor does a plain saying take its place afterwards.
    journal.members_with(Arc::new(|_, _, _| false));
    assert!(journal.may_tell(&who("grace", &[]), &event));

    // Where something already answers from nothing, or from another kind
    // of thing, whoever asks is told so by being handed nothing.
    let plain = memory().await;
    plain.members_with(Arc::new(|_, _, _| true));
    assert!(plain.members_from(Arc::new(Kept::default()), answer).is_none());
    let other = memory().await;
    other.members_from(Arc::new(7usize), |_| Arc::new(|_, _, _| true)).unwrap();
    assert!(other.members_from(Arc::new(Kept::default()), answer).is_none());
    assert!(other.may_tell(&who("ada", &[]), &event), "what stood still answers");
}

#[tokio::test]
async fn what_is_one_actors_alone_is_handed_to_them_and_nobody_else_with_nothing_installed() {
    let journal = memory().await;
    let event = taking(Some(&kernel::principal::own("user:grace")));
    assert!(journal.may_tell(&who("grace", &[]), &event));
    assert!(journal.may_tell_as("user:grace", &[], &event));
    for (id, holds) in [("ada", &["*"][..]), ("ada", &["member:own:user:grace"]), ("grace-2", &["*"])] {
        assert!(!journal.may_tell(&who(id, holds), &event), "told to {id} holding {holds:?}");
    }
    assert!(!journal.may_tell(&Principal::system(), &event), "told to the system");
    // Whatever answers who is a member is not asked of it, and does not
    // answer for it.
    journal.members_with(Arc::new(|_, _, _| true));
    assert!(!journal.may_tell(&who("ada", &["*"]), &event));
    assert!(journal.may_tell(&who("grace", &[]), &event));
}
