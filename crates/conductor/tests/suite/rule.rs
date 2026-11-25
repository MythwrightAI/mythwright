//! The one rule by which an instance that is there comes to a text, held
//! to from each of those who ask: a person (by name and key, and by the
//! instance itself), the look below a parent somebody applied, and a
//! `spawn` of a child that exists. And what a `call` asks instead.

use conductor::{Apply, InstanceFilter, InstanceInfo, InstanceStatus, RunInfo, RunStatus, Takes, Took};
use kernel::Source;
use serde_json::json;

use crate::handlers::{active, handled};
use crate::{Rig, user};

/// What somebody applies. Its handler starts a child for each card; none
/// of its own steps names one.
const PARENT: &str = "\
blueprint: parent
on:
  ready:
    event: board.item.moved
    steps:
      go: { spawn: child, as: \"kid-${ event.data.id }\", with: { id: ${ event.data.id } } }
";

/// The child, as it is when it is started.
const CHILD: &str = "\
blueprint: child
params: { id: 0 }
state: { seen: 0 }
steps:
  claim: { do: test.count, with: { key: claimed } }
on:
  poke:
    event: test.poke
    steps:
      note: { do: set, with: { seen: ${ state.seen + 1 } } }
      say: { do: test.count, with: { key: old } }
";

/// The texts a child may be asked to come to, by which case of the rule
/// each is.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Text {
    /// 1: the one it runs.
    Same,
    /// 3: one that says how a running instance takes it.
    Says,
    /// 4: one that begins as its own does.
    Alike,
    /// 5: one whose steps are others.
    Otherwise,
}

impl Text {
    fn source(self) -> String {
        let alike = CHILD.replace("key: old", "key: new");
        match self {
            Text::Same => CHILD.to_string(),
            Text::Alike => alike,
            // It begins otherwise as well: saying how is what decides.
            Text::Says => format!(
                "{}on_update:\n  - do: test.count\n    with: {{ key: carried }}\n",
                alike.replace("key: claimed", "key: claimed-otherwise")
            ),
            Text::Otherwise => alike.replace("key: claimed", "key: claimed-otherwise"),
        }
    }
}

/// Who asks.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Asks {
    /// A person, applying the instance again as it is.
    ByHand,
    /// A person, applying the blueprint under the parent and key it has.
    ByName,
    /// The look below its parent, which a person applied again.
    Below,
    /// The parent's handler, spawning the child it has.
    Spawn,
}

async fn moved(rig: &Rig, id: u64) {
    rig.journal.emit("board.item.moved", Source::default(), json!({ "id": id })).await.unwrap();
}

async fn poke(rig: &Rig) {
    rig.journal.emit("test.poke", Source::default(), json!({})).await.unwrap();
}

async fn keyed(rig: &Rig, key: &str) -> InstanceInfo {
    let live = rig.engine.instances(InstanceFilter { live_only: true, ..Default::default() }).await.unwrap();
    live.into_iter().find(|one| one.key == key).unwrap_or_else(|| panic!("no live `{key}`"))
}

/// The parent applied, and one child of it started by its handler,
/// settled and poked once: `(parent, child)`.
async fn started(rig: &Rig) -> (String, String) {
    rig.library.put("parent", PARENT);
    rig.library.put("child", CHILD);
    let applied = rig.apply(PARENT, json!({})).await;
    active(rig, &applied.instance).await;
    moved(rig, 1).await;
    handled(rig, "on.ready", 1).await;
    let kid = keyed(rig, "kid-1").await;
    active(rig, &kid.id).await;
    poke(rig).await;
    handled(rig, "on.poke", 1).await;
    (applied.instance, kid.id)
}

fn mains(runs: &[RunInfo]) -> Vec<&RunInfo> {
    runs.iter().filter(|run| run.flow == "main").collect()
}

/// Has `asks` ask for the child to be what is now there, and waits for
/// whatever comes of it to end. What the asker was told, where it is
/// told anything.
async fn ask(rig: &Rig, asks: Asks, parent: &str, kid: &str, text: Text) -> Option<Took> {
    let before = mains(&rig.engine.runs(kid, 20).await.unwrap()).len();
    let told = match asks {
        Asks::ByHand => rig.engine.apply_again(kid, user(), false).await.unwrap().took,
        Asks::ByName => {
            let request = Apply {
                key: Some("kid-1".into()),
                parent: Some(parent.to_string()),
                params: kernel::fields(json!({ "id": 1 })),
                ..Apply::new("child", user())
            };
            rig.engine.apply(request).await.unwrap().took
        }
        Asks::Below => {
            let over = rig.engine.apply(rig.request(PARENT, json!({}))).await.unwrap();
            assert!(!over.changed, "the parent's own text is what it was");
            None
        }
        Asks::Spawn => {
            moved(rig, 1).await;
            handled(rig, "on.ready", 2).await;
            None
        }
    };
    // What comes of it: a run of `on_update`, or of its steps, or neither.
    let runs_steps = text == Text::Otherwise && asks != Asks::Below;
    if text == Text::Says {
        handled(rig, "on_update", 1).await;
    } else if runs_steps {
        let runs = rig.engine.runs(kid, 20).await.unwrap();
        let all = mains(&runs);
        assert_eq!(all.len(), before + 1, "a run of its steps was started");
        rig.soon("its steps to finish", rig.engine.wait_run(&all[0].id)).await.unwrap();
    }
    told
}

/// What a child that came to `text` is found to be, as counts of what ran:
/// `[its first steps, the other text's steps, on_update, in place]`.
async fn came(rig: &Rig, kid: &str) -> [u64; 4] {
    let in_place = rig
        .events("instance.updated")
        .await
        .iter()
        .filter(|event| event.data["in_place"] == true && event.source.instance.as_deref() == Some(kid))
        .count() as u64;
    [
        rig.shared.count("claimed"),
        rig.shared.count("claimed-otherwise"),
        rig.shared.count("carried"),
        in_place,
    ]
}

#[tokio::test]
async fn whoever_asks_an_instance_comes_to_a_text_the_same_way() {
    for asks in [Asks::ByHand, Asks::ByName, Asks::Below, Asks::Spawn] {
        for text in [Text::Same, Text::Says, Text::Alike, Text::Otherwise] {
            let rig = Rig::new().await;
            let (parent, kid) = started(&rig).await;
            let source = text.source();
            rig.library.put("child", &source);
            let told = ask(&rig, asks, &parent, &kid, text).await;
            let case = format!("{asks:?} asking for {text:?}");

            // The look below asks for nothing to be run: where only its
            // steps would bring it to the text, it is left, and behind.
            let left = asks == Asks::Below && text == Text::Otherwise;
            let (ran, took) = match text {
                Text::Same => ([1, 0, 0, 0], None),
                Text::Says => ([1, 0, 1, 0], Some(Took::Carried)),
                Text::Alike => ([1, 0, 0, 1], Some(Took::InPlace)),
                Text::Otherwise if left => ([1, 0, 0, 0], None),
                Text::Otherwise => ([1, 1, 0, 0], Some(Took::Again)),
            };
            assert_eq!(came(&rig, &kid).await, ran, "{case}");
            if matches!(asks, Asks::ByHand | Asks::ByName) {
                assert_eq!(told, took, "{case}: what the one who asked is told");
            }
            let is = rig.instance(&kid).await;
            assert_eq!(is.status, InstanceStatus::Active, "{case}");
            let behind = rig.engine.shown_one(is.clone()).behind;
            if left {
                assert_eq!(is.revision, blueprint::revision(CHILD), "{case}");
                assert_eq!(behind.map(|behind| behind.takes), Some(Takes::Again), "{case}");
            } else {
                assert_eq!(is.revision, blueprint::revision(&source), "{case}");
                assert_eq!(behind, None, "{case}");
            }
            // What it holds is kept, whichever way it came.
            assert_eq!(is.state["seen"], 1, "{case}");
            assert_eq!(is.params, kernel::fields(json!({ "id": 1 })), "{case}");

            // And it listens as the text it now runs.
            poke(&rig).await;
            handled(&rig, "on.poke", 2).await;
            let new = if text == Text::Same || left { 0 } else { 1 };
            assert_eq!((rig.shared.count("old"), rig.shared.count("new")), (2 - new, new), "{case}");
        }
    }
}

#[tokio::test]
async fn asked_with_force_its_steps_run_again_whatever_the_two_texts_are() {
    for text in [Text::Same, Text::Says, Text::Alike, Text::Otherwise] {
        let rig = Rig::new().await;
        let (_, kid) = started(&rig).await;
        rig.library.put("child", &text.source());
        let forced = rig.engine.apply_again(&kid, user(), true).await.unwrap();
        assert_eq!(forced.took, Some(Took::Again), "{text:?}");
        assert_eq!(rig.finished(&forced).await.status, RunStatus::Ok);
        let steps = rig.shared.count("claimed") + rig.shared.count("claimed-otherwise");
        assert_eq!(steps, 2, "{text:?}: its steps ran a second time");
        assert_eq!(rig.shared.count("carried"), 0, "{text:?}: and nothing was done in their place");
    }
}

#[tokio::test]
async fn a_call_asks_for_the_steps_and_is_given_them_unless_the_child_runs_that_text_with_those_params() {
    let rig = Rig::new().await;
    // What is called lives on: it has a handler. It says what it counted.
    let called = format!("{CHILD}outputs:\n  claims: ${{ steps.claim.out.count }}\n");
    let caller = "\
blueprint: caller
params: { id: 1 }
steps:
  kid: { call: child, with: { id: ${ params.id } } }
on:
  poke: { event: test.poke, do: noop }
outputs:
  got: ${ steps.kid.out.claims }
";
    rig.library.put("child", &called);
    assert_eq!(rig.run(caller, json!({})).await.outputs, json!({ "got": 1 }));
    let again = |params: serde_json::Value| Apply { force: true, ..rig.request(caller, params) };
    let run = |request: Apply| async {
        let applied = rig.engine.apply(request).await.unwrap();
        rig.finished(&applied).await
    };

    // The text it runs, with the params it has: nothing is run, and the
    // caller is given what the steps gave when they last ran.
    assert_eq!(run(again(json!({}))).await.outputs, json!({ "got": 1 }));
    assert_eq!(rig.shared.count("claimed"), 1);

    // Other params: the steps.
    assert_eq!(run(again(json!({ "id": 2 }))).await.outputs, json!({ "got": 2 }));

    // A text that begins as its own does: the steps, where anyone else
    // asking would have had it take the text where it is.
    rig.library.put("child", &called.replace("key: old", "key: new"));
    assert_eq!(run(again(json!({ "id": 2 }))).await.outputs, json!({ "got": 3 }));
    let in_place = rig.events("instance.updated").await.iter().any(|event| event.data["in_place"] == true);
    assert!(!in_place, "a call is not answered by a text taken in place");

    // A text that says how it is taken: the steps all the same.
    let says = format!(
        "{}on_update:\n  - do: test.count\n    with: {{ key: carried }}\n",
        called.replace("key: old", "key: newer")
    );
    rig.library.put("child", &says);
    assert_eq!(run(again(json!({ "id": 2 }))).await.outputs, json!({ "got": 4 }));
    assert_eq!(rig.shared.count("carried"), 0, "nor by `on_update`");
}

/// A child whose text has no handler for a card that comes round again,
/// and the same with one: it takes the card up, once.
const WITHOUT: &str = "\
blueprint: child
params: { id: 0 }
steps:
  claim: { do: test.count, with: { key: claimed } }
on:
  poke: { event: test.poke, do: test.count, with: { key: poked } }
";

fn with_again() -> String {
    format!(
        "{WITHOUT}  again:\n    event: board.item.moved\n    for: {{ id: ${{ params.id }} }}\n    do: \
         test.count\n    with: {{ key: taken-up }}\n"
    )
}

#[tokio::test]
async fn a_text_taken_because_a_handlers_step_asked_hears_the_event_that_handler_was_handling() {
    let rig = Rig::new().await;
    rig.library.put("parent", PARENT);
    rig.library.put("child", WITHOUT);
    let applied = rig.apply(PARENT, json!({})).await;
    active(&rig, &applied.instance).await;
    moved(&rig, 1).await;
    handled(&rig, "on.ready", 1).await;
    let kid = keyed(&rig, "kid-1").await.id;
    active(&rig, &kid).await;
    // Another card's child, which is to hear nothing of this.
    moved(&rig, 2).await;
    handled(&rig, "on.ready", 2).await;
    let other = keyed(&rig, "kid-2").await.id;
    active(&rig, &other).await;

    // The card comes round again. The parent's handler spawns the child it
    // has; the text now there adds a handler for exactly this.
    rig.library.put("child", &with_again());
    moved(&rig, 1).await;
    handled(&rig, "on.ready", 3).await;
    handled(&rig, "on.again", 1).await;
    assert_eq!(rig.instance(&kid).await.revision, blueprint::revision(&with_again()));
    assert_eq!(rig.shared.count("claimed"), 2, "the steps of neither child ran again");
    assert_eq!(rig.shared.count("taken-up"), 1, "the move that brought the text was heard by what it adds");

    // Once: the next move is one more, and the other card's child, which
    // took nothing, heard neither.
    moved(&rig, 1).await;
    handled(&rig, "on.again", 2).await;
    assert_eq!(rig.shared.count("taken-up"), 2);
    assert_eq!(rig.instance(&other).await.revision, blueprint::revision(WITHOUT));
    let runs = rig.engine.runs(&other, 20).await.unwrap();
    assert!(runs.iter().all(|run| run.flow != "on.again"));

    // A person asking is not an event: what the text adds listens from then.
    let rig = Rig::new().await;
    rig.library.put("parent", PARENT);
    rig.library.put("child", WITHOUT);
    let applied = rig.apply(PARENT, json!({})).await;
    active(&rig, &applied.instance).await;
    moved(&rig, 1).await;
    handled(&rig, "on.ready", 1).await;
    let kid = keyed(&rig, "kid-1").await.id;
    active(&rig, &kid).await;
    rig.library.put("child", &with_again());
    assert_eq!(rig.engine.apply_again(&kid, user(), false).await.unwrap().took, Some(Took::InPlace));
    poke(&rig).await;
    handled(&rig, "on.poke", 1).await;
    assert_eq!(rig.shared.count("taken-up"), 0, "the move from before it took the text is not handed to it");
}

#[tokio::test]
async fn a_child_carried_over_because_a_handlers_step_asked_hears_that_event_too_even_across_a_restart() {
    let mut rig = Rig::new().await;
    rig.library.put("parent", PARENT);
    rig.library.put("child", WITHOUT);
    let applied = rig.apply(PARENT, json!({})).await;
    active(&rig, &applied.instance).await;
    moved(&rig, 1).await;
    handled(&rig, "on.ready", 1).await;
    let kid = keyed(&rig, "kid-1").await.id;
    active(&rig, &kid).await;

    // The text now there says how it is taken, and what it does holds
    // until it is let go: the conductor is stopped while it is held.
    let says = format!(
        "{}on_update:\n  - do: test.gate\n    with: {{ name: carrying }}\n",
        with_again().replace("key: claimed", "key: claimed-otherwise")
    );
    rig.library.put("child", &says);
    moved(&rig, 1).await;
    rig.reached("carrying").await;
    rig.restart().await;
    rig.reached("carrying").await;
    rig.open("carrying");
    handled(&rig, "on_update", 1).await;
    handled(&rig, "on.again", 1).await;
    assert_eq!(rig.instance(&kid).await.revision, blueprint::revision(&says));
    assert_eq!(rig.shared.count("taken-up"), 1, "the move that brought the text was heard, once");
    assert_eq!(rig.shared.count("claimed-otherwise"), 0, "and its steps were not run");
}

#[tokio::test]
async fn a_text_that_fails_to_carry_an_instance_over_leaves_no_place_for_the_handlers_only_it_has() {
    let rig = Rig::new().await;
    rig.library.put("parent", PARENT);
    rig.library.put("child", WITHOUT);
    let applied = rig.apply(PARENT, json!({})).await;
    active(&rig, &applied.instance).await;
    moved(&rig, 1).await;
    handled(&rig, "on.ready", 1).await;
    let kid = keyed(&rig, "kid-1").await.id;
    active(&rig, &kid).await;

    // What it says to do in place of the steps fails: the child is back on
    // the text it had, which has no such handler.
    let fails =
        format!("{}on_update:\n  - do: test.flaky\n    with: {{ message: not today }}\n", with_again());
    rig.library.put("child", &fails);
    moved(&rig, 1).await;
    handled(&rig, "on_update", 1).await;
    assert_eq!(rig.instance(&kid).await.revision, blueprint::revision(WITHOUT));

    // Much later a text with that handler is taken, by a person: it
    // listens from then, and is not handed the moves there were between.
    moved(&rig, 1).await;
    handled(&rig, "on_update", 2).await;
    rig.library.put("child", &with_again());
    assert_eq!(rig.engine.apply_again(&kid, user(), false).await.unwrap().took, Some(Took::InPlace));
    poke(&rig).await;
    handled(&rig, "on.poke", 1).await;
    assert_eq!(rig.shared.count("taken-up"), 0);
    moved(&rig, 1).await;
    handled(&rig, "on.again", 1).await;
    assert_eq!(rig.shared.count("taken-up"), 1);
}

#[tokio::test]
async fn a_handler_a_text_drops_and_a_later_text_brings_back_listens_from_when_it_came_back() {
    // However the instance came to each text: where it is, or by its steps.
    for by_steps in [false, true] {
        let rig = Rig::new().await;
        let with =
            |text: String| if by_steps { text.replace("key: claimed", "key: claimed-again") } else { text };
        rig.library.put("parent", PARENT);
        rig.library.put("child", &with_again());
        let applied = rig.apply(PARENT, json!({})).await;
        active(&rig, &applied.instance).await;
        moved(&rig, 1).await;
        handled(&rig, "on.ready", 1).await;
        let kid = keyed(&rig, "kid-1").await.id;
        active(&rig, &kid).await;
        moved(&rig, 1).await;
        handled(&rig, "on.again", 1).await;
        // The parent's step for that move may still be asking for the
        // child as the text becomes another: what it applies is the text
        // as it reads it in the child's turn ([`crate::in_turn`]).

        // The text loses the handler; moves go by unheard.
        rig.library.put("child", &with(WITHOUT.to_string()));
        let dropped = rig.engine.apply_again(&kid, user(), false).await.unwrap();
        if by_steps {
            rig.finished(&dropped).await;
        }
        moved(&rig, 1).await;
        moved(&rig, 1).await;
        handled(&rig, "on.ready", 4).await;

        // It is brought back, by a person: it hears what happens from then.
        let back = if by_steps { with_again() } else { with_again().replace("key: poked", "key: prodded") };
        rig.library.put("child", &back);
        let again = rig.engine.apply_again(&kid, user(), false).await.unwrap();
        if by_steps {
            rig.finished(&again).await;
        }
        assert_eq!(rig.instance(&kid).await.revision, blueprint::revision(&back));
        poke(&rig).await;
        handled(&rig, "on.poke", 1).await;
        assert_eq!(rig.shared.count("taken-up"), 1, "by steps: {by_steps}: not the moves there were between");
        moved(&rig, 1).await;
        handled(&rig, "on.again", 2).await;
        assert_eq!(rig.shared.count("taken-up"), 2, "by steps: {by_steps}");
    }
}

/// A handler whose run waits to be let go, so that a test can have its
/// text taken away and brought back while it has an event in hand.
fn held(key: &str) -> String {
    format!(
        "blueprint: held\non:\n  poke: {{ event: test.poke, do: test.count, with: {{ key: {key} }} }}\n  again:\n    \
         event: board.item.moved\n    steps:\n      wait: {{ do: test.gate, with: {{ name: hold }} }}\n      say: {{ \
         do: test.count, with: {{ key: \"taken-${{ event.data.id }}\" }} }}\n"
    )
}

#[tokio::test]
async fn a_handler_brought_back_before_the_one_that_was_dropped_has_ended_listens_from_when_it_came_back() {
    let rig = Rig::new().await;
    let without =
        "blueprint: held\non:\n  poke: { event: test.poke, do: test.count, with: { key: without } }\n";
    let applied = rig.apply(&held("first"), json!({})).await;
    active(&rig, &applied.instance).await;
    moved(&rig, 1).await;
    rig.reached("hold").await;

    // The text loses the handler while it has a move in hand: it is not
    // waited for, and its place in the journal is still there.
    rig.library.put("held", without);
    let dropped = rig.engine.apply_again(&applied.instance, user(), false).await.unwrap();
    assert_eq!(dropped.took, Some(Took::InPlace));
    moved(&rig, 2).await;
    moved(&rig, 2).await;
    poke(&rig).await;
    handled(&rig, "on.poke", 1).await;

    // It is brought back before the one that was dropped has ended, and
    // so before its place was taken away.
    rig.library.put("held", &held("second"));
    let back = rig.engine.apply_again(&applied.instance, user(), false).await.unwrap();
    assert_eq!(back.took, Some(Took::InPlace));
    moved(&rig, 3).await;
    for _ in 0..4 {
        rig.open("hold");
    }

    // The move it had in hand, and then the one made since it came back:
    // its runs are one after another, so a move from between would have
    // been the second.
    handled(&rig, "on.again", 2).await;
    assert_eq!(rig.shared.count("taken-1"), 1);
    assert_eq!(rig.shared.count("taken-2"), 0, "the moves there were between");
    assert_eq!(rig.shared.count("taken-3"), 1);
    poke(&rig).await;
    handled(&rig, "on.poke", 2).await;
    assert_eq!(rig.shared.count("taken-2"), 0, "the moves there were between");
}
