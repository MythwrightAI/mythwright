//! The places a scenario goes two ways at, by design, each with why.
//!
//! A place a play never came to is a failure (`found.rs`), unless the
//! scenario says that two things in it come in either order there, and
//! one of the orders has no such place (`Scenario::either`). These are
//! what the scenarios say. Each is narrow on purpose: it names the
//! handlers that have a run in one order and none in the other, and
//! nothing beside them, so that a place of anything else that a play goes
//! by is still a failure. An order that is only by chance is not here:
//! the scenario is written so that it does not happen (two cards where
//! one is worked at a time: its act ends when the second has asked for
//! its place, and not before).
//!
//! What is said here is what was seen, counted over every scenario's
//! first play crowded on two processors and on four, and read from what
//! a play that never reached its place wrote down. Nothing here excuses
//! a play that came to its place and to the wrong thing.

use crate::simulation::Place;

/// Whether a place is something `blueprint` said while one of these of
/// its handlers ran.
fn in_flows(place: &Place, blueprint: &str, flows: &[&str]) -> bool {
    place.within(blueprint) && flows.contains(&place.flow.as_str())
}

/// Whether a place is something a stage said while one of these of its
/// handlers ran, whichever kind of thing is done in its column.
fn of_a_stage(place: &Place, flows: &[&str]) -> bool {
    ["std/working", "std/reviewing", "std/planning"].iter().any(|text| in_flows(place, text, flows))
}

/// A fleet that comes up: what a stage does on hearing one of two things,
/// whichever it hears first, or on hearing something that may have been
/// said before it was there to hear.
pub fn coming_up(place: &Place) -> Option<&'static str> {
    if of_a_stage(place, &["on.first", "on.staffed"]) {
        return Some(
            "a stage that begins with agents looks at its column when it hears that it is active itself or \
             that the fleet has been arranged, whichever it hears first, and the other of the two has no run",
        );
    }
    if place.within("std/due") {
        return Some(
            "a stage has its column looked at once for each thing that has it looked at (that it is active, \
             that the fleet is arranged, that its pool is listening), and which of those it hears is as said \
             of the handlers that hear them",
        );
    }
    of_a_stage(place, &["on.listening"]).then_some(
        "a stage and its pool are started side by side, and a pool that says it is listening before its \
         stage is there to hear it is not heard by it",
    )
}

/// A fleet that comes up over a card made ready before it. The card is
/// found, has its turn asked for and is worked and handed on while the
/// fleet is still coming up, by stages and pools that are started side by
/// side: each has a run for what it heard, and none for what was said
/// before it was listening. The card is said to be due once or twice,
/// its place is asked of a pool that hears or does not, a turn that came
/// twice is given back, and the stage that reviews hears the card come to
/// it or is not there yet. What comes of the card is held wherever the
/// daemon is stopped; that every play says each of those things is not.
pub fn coming_up_over_a_card(place: &Place) -> Option<&'static str> {
    let handler = |blueprint: &str| place.within(blueprint) && place.flow.starts_with("on.");
    let stage =
        |such: &dyn Fn(&str) -> bool| such("std/working") || such("std/reviewing") || such("std/planning");
    let state =
        place.topic == "instance.state" && (stage(&|text| place.within(text)) || place.within("std/pool"));
    coming_up(place).or((stage(&handler) || handler("std/pool") || state).then_some(
        "the card is found, asked for and handed on while the stages and their pools are still coming up \
         side by side, and each has a run for what it heard and none for what was said before it was \
         listening",
    ))
}

/// Two cards where one is worked, and reviewed, at a time. The second is
/// handed to review while the first is being reviewed or once that is
/// over, by which of the second's work and the first's review is done
/// sooner: whoever works and whoever reviews are both let go as soon as
/// they are there. A review asked for meanwhile waits for its place and
/// is told so, as the second card was when it waited to be worked, and
/// is given the place as the first review ends; one asked for afterwards
/// is given it as it asks. So what is said of something waiting is said
/// once in every play, for the work, and a second time in some.
pub fn a_second_review(place: &Place) -> Option<&'static str> {
    let again = place.nth > 1;
    let waited = (again && in_flows(place, "std/cards", &["on.waits", "on.taken"]))
        || (again && in_flows(place, "std/pool", &["on.asked"]) && place.step == "told")
        // A place given as it is asked for: the two cards' work and the
        // first review, and the second review where it did not wait.
        || (place.topic == "pool.granted" && in_flows(place, "std/pool", &["on.asked"]) && place.nth > 2)
        || (place.topic == "pool.granted" && in_flows(place, "std/pool", &["on.ended"]))
        || (in_flows(place, "std/review", &["on.begin"]) && place.step == "taken")
        // Eighteen lines of the cards' histories are written however the
        // reviews fall, and two more where the second waited.
        || (place.topic == "collection.history" && place.nth > 18);
    waited.then_some(
        "the second card is handed to review while the first is being reviewed, and waits for its place \
         there, or once that review is over, and does not",
    )
}

/// A review that says how it came out and then ends itself, and the
/// stage that began it hears both, each in its turn: what the review came
/// to takes the card off what the stage keeps of its reviews
/// (`decided`), and its ending takes it off where that is not done yet
/// (`ended`, whose guard asks whether the stage keeps anything). Where
/// what it came to is taken off first, the ending is passed over and has
/// no run; which of the two is first is the order the review's last two
/// things are heard in and the work of the first is done in, and both
/// come to the same card, merged, with nothing kept.
pub fn a_review_over(place: &Place) -> Option<&'static str> {
    in_flows(place, "std/reviewing", &["on.ended"]).then_some(
        "a review says how it came out and then ends itself, and where the stage has taken the card off \
         what it keeps for how it came out before the ending's turn comes, the ending is passed over and \
         has no run",
    )
}

/// A card that leaves the column it was planned in. Its proposal hears
/// the move and ends itself in the first step of what hears it
/// (`moved` in `std/proposal`); the steps after that one are never come
/// to, and the run is cancelled as its instance stops. The run says of a
/// step that it is cancelled only where it had come to that step before
/// it was cancelled itself, so which of them are said is the order the
/// stopping and the run come in: all of them in one play, the last two
/// in another. Nothing is done by any of them either way.
pub fn a_proposal_ended(place: &Place) -> Option<&'static str> {
    (place.topic == "step.cancelled" && in_flows(place, "std/proposal", &["on.moved"])).then_some(
        "a proposal ends itself in the first step of what hears its card moved, and which of the steps \
         after it the run says are cancelled, before it is cancelled itself, is the order the stopping and \
         the run come in",
    )
}

/// A place of the tests' own: something a blueprint said in a handler.
fn said(topic: &str, blueprint: &str, flow: &str, step: &str, nth: usize) -> Place {
    Place {
        topic: topic.to_string(),
        blueprint: blueprint.to_string(),
        flow: flow.to_string(),
        step: step.to_string(),
        who: String::new(),
        nth,
    }
}

#[test]
fn a_scenario_says_two_ways_only_of_what_has_a_run_in_one_order_and_none_in_the_other() {
    // A fleet coming up: the look at a column, by whichever handler, and
    // the pool heard or not. Nothing else of a stage, and nothing of a card.
    assert!(coming_up(&said("stage.due", "std/due", "main", "each", 2)).is_some());
    for flow in ["on.first", "on.staffed", "on.listening"] {
        assert!(coming_up(&said("run.started", "std/working", flow, "", 1)).is_some(), "{flow}");
        assert!(coming_up(&said("step.started", "std/working", flow, "look", 2)).is_some(), "{flow}");
    }
    for (blueprint, flow) in [("std/working", "on.go"), ("std/reviewing", "main"), ("std/pool", "on.asked")] {
        assert_eq!(coming_up(&said("run.started", blueprint, flow, "", 1)), None, "{blueprint} {flow}");
    }
    assert_eq!(coming_up(&said("run.started", "std/lane-round", "main", "", 1)), None);

    // Over a card: what the stages and their pools hear, and what each
    // keeps; never the card's round, what the cards themselves say, or
    // what the fleet is made of as it is started.
    for (topic, blueprint, flow) in [
        ("run.started", "std/reviewing", "on.came"),
        ("review.due", "std/reviewing", "on.came"),
        ("pool.granted", "std/pool", "on.released"),
        ("instance.state", "std/pool", ""),
    ] {
        assert!(coming_up_over_a_card(&said(topic, blueprint, flow, "", 1)).is_some(), "{topic} {flow}");
    }
    for (topic, blueprint, flow) in [
        ("run.started", "std/lane-round", "main"),
        ("run.started", "std/working", "main"),
        ("instance.started", "std/pool", ""),
        ("run.started", "std/cards", "on.moved"),
        ("board.item.moved", "", ""),
        ("collection.history", "", ""),
    ] {
        assert_eq!(coming_up_over_a_card(&said(topic, blueprint, flow, "", 1)), None, "{topic} {blueprint}");
    }

    // A second review: the first time something waits is the second
    // card's work, which the scenario makes sure of, and is every play's.
    // A proposal that ends itself: the steps it never came to, said to be
    // cancelled or not. Nothing else of a proposal, and no step that ran.
    assert!(a_proposal_ended(&said("step.cancelled", "std/proposal", "on.moved", "round", 1)).is_some());
    assert_eq!(a_proposal_ended(&said("step.finished", "std/proposal", "on.moved", "gone", 1)), None);
    assert_eq!(a_proposal_ended(&said("step.cancelled", "std/proposal", "on.said", "round", 1)), None);
    assert_eq!(a_proposal_ended(&said("step.cancelled", "std/planning", "on.moved", "round", 1)), None);

    assert_eq!(a_second_review(&said("card.waiting", "std/cards", "on.waits", "told", 1)), None);
    assert!(a_second_review(&said("card.waiting", "std/cards", "on.waits", "told", 2)).is_some());
    assert_eq!(a_second_review(&said("pool.waiting", "std/pool", "on.asked", "told", 1)), None);
    assert!(a_second_review(&said("pool.waiting", "std/pool", "on.asked", "told", 2)).is_some());
    assert_eq!(a_second_review(&said("pool.granted", "std/pool", "on.asked", "granted", 2)), None);
    assert!(a_second_review(&said("pool.granted", "std/pool", "on.asked", "granted", 3)).is_some());
    assert!(a_second_review(&said("pool.granted", "std/pool", "on.ended", "granted", 1)).is_some());
    assert_eq!(a_second_review(&said("pool.granted", "std/pool", "on.released", "granted", 1)), None);
    assert_eq!(a_second_review(&said("step.started", "std/pool", "on.asked", "granted", 4)), None);
    // A review over: what hears its ending, and nothing else of a stage.
    assert!(a_review_over(&said("run.started", "std/reviewing", "on.ended", "", 1)).is_some());
    assert!(a_review_over(&said("step.started", "std/reviewing", "on.ended", "", 1)).is_some());
    for (blueprint, flow) in
        [("std/reviewing", "on.decided"), ("std/reviewing", "on.came"), ("std/working", "on.ended")]
    {
        assert_eq!(a_review_over(&said("run.started", blueprint, flow, "", 1)), None, "{blueprint} {flow}");
    }

    assert_eq!(a_second_review(&said("collection.history", "", "", "", 18)), None);
    assert!(a_second_review(&said("collection.history", "", "", "", 19)).is_some());
    assert_eq!(a_second_review(&said("run.started", "std/review", "on.begin", "", 2)), None);
}
