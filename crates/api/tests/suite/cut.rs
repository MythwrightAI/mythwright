//! What was being done when the daemon stopped is done once.
//!
//! A step that was under way is done again when the daemon is back, where
//! its action is the same done twice. Each test here is of one such
//! action, asked the way a step done again asks it, on either side of
//! what it leaves behind.

use serde_json::json;
use studio::{Place, SaidBy};

use crate::Rig;

fn step(run: &str, at: Option<usize>) -> SaidBy {
    SaidBy {
        blueprint: "nightly".into(),
        instance: "in_night".into(),
        run: run.into(),
        step: "told".into(),
        at,
        doing: Some(doing(1, at, None)),
        detail: None,
        about_run: None,
    }
}

/// Which of the things done under a step's name something is, as the
/// conductor says it (`ActionCx::doing`): the try, the element, and the
/// entry of what is done around the step where it is one.
fn doing(attempt: u32, at: Option<usize>, entry: Option<&str>) -> String {
    let of = at.map_or(String::new(), |at| format!(" of {at}"));
    format!("try {attempt}{of}{}", entry.map_or(String::new(), |entry| format!(", {entry}")))
}

#[tokio::test]
async fn a_step_done_again_finds_what_it_said_and_it_is_said_once_whichever_side_of_the_saying_the_stop_fell()
{
    let rig = Rig::new().await;
    rig.send("/api/channels", json!({ "name": "floor" })).await;
    let channels = &rig.node().app.studio.channels;
    let journal = &rig.node().app.journal;
    let by = step("run_one", None);

    // Not said yet, there is nothing to find: the step says it.
    assert!(channels.said_already(&crate::own(), &by).await.unwrap().is_none());
    let place = Place { said_by: Some(&by), ..Place::default() };
    let said = channels.post_speaker(&crate::own(), "floor", "The night went well.", place).await.unwrap();
    assert_eq!(rig.count_of("channel.message").await, 1);

    // Stopped after it was said, and done again: the same message, and nothing said twice.
    let found = channels.said_already(&crate::own(), &by).await.unwrap().expect("what the step said");
    assert_eq!((found.id.as_str(), found.body.as_str()), (said.id.as_str(), "The night went well."));
    assert_eq!(rig.count_of("channel.message").await, 1);

    // Written down and never said, as a journal from before may hold it
    // where a stop fell between the two (a message and the saying of it
    // are one write now): it is said as it is found, once.
    assert_eq!(journal.forget_events("channel.message", "id", &said.id).await.unwrap(), 1);
    assert_eq!(rig.count_of("channel.message").await, 0);
    assert_eq!(
        channels.said_already(&crate::own(), &by).await.unwrap().map(|found| found.id),
        Some(said.id.clone())
    );
    assert_eq!(rig.count_of("channel.message").await, 1);
    let heard = rig.event("channel.message", |event| event.data["id"] == said.id.as_str()).await;
    assert_eq!(heard.data["body"], "The night went well.");

    // Another run's step of the same name, and another element of this one's list, said nothing yet.
    assert!(channels.said_already(&crate::own(), &step("run_two", None)).await.unwrap().is_none());
    assert!(channels.said_already(&crate::own(), &step("run_one", Some(1))).await.unwrap().is_none());
    // Nor what is done as that step ends: it is done under the step's
    // name, and what the step itself said is not what it said.
    let as_it_ends = |attempt: u32, entry: &str| SaidBy {
        doing: Some(doing(attempt, None, Some(entry))),
        ..step("run_one", None)
    };
    assert!(channels.said_already(&crate::own(), &as_it_ends(1, "on_success 0")).await.unwrap().is_none());
    let messages = rig.read("/api/channels/floor/messages").await;
    assert_eq!(messages["messages"].as_array().map(Vec::len), Some(1), "{messages}");
}

/// Several things are done under one step's name, and each says what it
/// has to say once: one of them done again finds its own message and
/// never another's. Each pair here was one and the same when a message
/// was found by its run, its step, its element and the number of its
/// entry: the second of the pair, done again, found the first's message
/// and said nothing.
#[tokio::test]
async fn what_is_done_around_a_step_finds_its_own_message_and_not_that_of_another_try_list_or_element() {
    let rig = Rig::new().await;
    rig.send("/api/channels", json!({ "name": "floor" })).await;
    let channels = &rig.node().app.studio.channels;
    let said = |doing: String, step: &str| SaidBy {
        step: step.into(),
        doing: Some(doing),
        ..self::step("run_one", None)
    };
    let pairs = [
        // What is done as a step starts, at its first try and at its second.
        (said(doing(1, None, Some("on_start 0")), "work"), said(doing(2, None, Some("on_start 0")), "work")),
        // The first thing done when it fails, and the first done when the next try goes well.
        (said(doing(1, None, Some("on_fail 0")), "work"), said(doing(2, None, Some("on_success 0")), "work")),
        // What is done as a step ends, inside steps done as one for the first of a list and for the next.
        (
            said(format!("all try 1 of 0 > {}", doing(1, None, Some("on_success 0"))), "all.one"),
            said(format!("all try 1 of 1 > {}", doing(1, None, Some("on_success 0"))), "all.one"),
        ),
        // And that step's own action, the first time those steps are tried and the second.
        (
            said(format!("all try 1 > {}", doing(1, None, None)), "all.one"),
            said(format!("all try 2 > {}", doing(1, None, None)), "all.one"),
        ),
    ];
    for (count, (first, second)) in pairs.iter().enumerate() {
        let place = Place { said_by: Some(first), ..Place::default() };
        let posted = channels.post_speaker(&crate::own(), "floor", "It is done.", place).await.unwrap();
        // Done again, the first finds what it said; the second, which has said nothing yet, finds nothing.
        let found = channels.said_already(&crate::own(), first).await.unwrap().map(|found| found.id);
        assert_eq!(found, Some(posted.id), "{first:?}");
        assert!(
            channels.said_already(&crate::own(), second).await.unwrap().is_none(),
            "{second:?} found what {first:?} said"
        );
        assert_eq!(rig.count_of("channel.message").await, count + 1);
    }
    // A message a program from before this kept says which element and no
    // more, and a step done again finds it by that, as it did.
    let before = SaidBy { doing: None, step: "old".into(), ..step("run_one", Some(2)) };
    let place = Place { said_by: Some(&before), ..Place::default() };
    let posted = channels.post_speaker(&crate::own(), "floor", "From before.", place).await.unwrap();
    let now = SaidBy { step: "old".into(), ..step("run_one", Some(2)) };
    assert_eq!(
        channels.said_already(&crate::own(), &now).await.unwrap().map(|found| found.id),
        Some(posted.id)
    );
    let other = SaidBy { step: "old".into(), ..step("run_one", Some(3)) };
    assert!(channels.said_already(&crate::own(), &other).await.unwrap().is_none());
}
