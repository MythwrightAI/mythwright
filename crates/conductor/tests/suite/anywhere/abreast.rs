//! The plays of a scenario, several at a time.
//!
//! A scenario is played once for each place it passed, and again for
//! each kind of second stop: some hundreds of plays for the longer ones,
//! which one test made one after another on its one thread while the
//! processors beside it stood idle. A test program is never over before
//! its longest test is, so that one thread was what a run of the whole
//! suite waited for.
//!
//! Here they are made several at a time, each on a thread, a runtime and
//! a conductor of its own, with its own journal answered a turn at a time
//! as before: a play goes forward by what its own database answers, and
//! knows nothing of the play beside it. Which plays are made is what it
//! was. A play is told where to be stopped a second time by the plays
//! before it (the first thing it keeps of a kind none of them was
//! stopped at), so they cannot simply be made together. What can be, is
//! found first: for each place and way, the play that is never stopped a
//! second time, which is told nothing by any other and says what its
//! conductor keeps by itself after its start. From those it is worked
//! out what each of the other plays will be told, and those are made
//! together too. Then the whole is gone through in the order it was
//! always made in, one play after another as [`super::anywhere`]
//! describes, and each play there asks for is taken from those already
//! made, where one was told that very thing: a play told one thing is as
//! good as another told the same. Where none was (what a conductor keeps
//! after a start is not held to come in one order from one play to the
//! next, and a play may have been stopped somewhere other than was
//! worked out), the play is made then, as it would have been. So nothing
//! is held by fewer plays than before or by other ones, on any machine,
//! and what was worked out ahead is only ever how soon.
//!
//! How many are made at a time goes by how many tests the program runs
//! at a time, for the whole program and not for each test: twice as many
//! plays as processors, whether they are of one test or of thirty. Twice,
//! since a play is two threads that take turns, its own and its
//! database's, and each waits while the other works: with as many plays
//! as processors a fifth of the processors stood idle.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};

use serde_json::Value;

use super::{Place, Play, Scenario, Tried, Way, Went, went};
use crate::stops::{Again, Kind};

/// One play to make: where it is stopped, how, and what it is told of a
/// second stop.
struct Asked<'a> {
    place: &'a Place,
    way: Way,
    second: Option<(Again, Way)>,
}

/// What a play came to, read on the thread it was made on.
struct Made {
    went: Went,
    /// What began each thing written down and not yet begun, and whether
    /// the play was where its scenario is there to be: of a play that
    /// went as meant, and nothing of any other.
    begun: Vec<(String, String, String)>,
    there: bool,
    /// Where it was stopped a second time, and what its conductor kept
    /// by itself after its start.
    again: Option<Place>,
    kinds: Vec<Kind>,
}

/// How many plays the program is making now, and what is told when one
/// is over.
static GOING: Mutex<usize> = Mutex::new(0);
static OVER: Condvar = Condvar::new();

/// How many plays the program makes at a time: twice as many as it
/// runs tests (`RUST_TEST_THREADS`, where whoever started it said), or
/// as it may use processors.
fn room() -> usize {
    let threads = std::env::var("RUST_TEST_THREADS")
        .ok()
        .and_then(|said| said.parse().ok())
        .filter(|threads| *threads > 0)
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from));
    2 * threads
}

/// A play's turn among all the plays of the program, given back when it
/// is over however it ended.
struct Turn;

impl Turn {
    fn wait() -> Turn {
        let mut going = GOING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        while *going >= room() {
            going = OVER.wait(going).unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        *going += 1;
        Turn
    }
}

impl Drop for Turn {
    fn drop(&mut self) {
        *GOING.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) -= 1;
        OVER.notify_one();
    }
}

/// What a play is made on: a runtime of its own, as a test has, so that
/// everything its conductor has running takes turns on one thread and
/// no other.
///
/// And takes them in one order on any machine. Of two things there to
/// be looked at, one woken on this thread and one woken by a database's
/// thread, a runtime looks at the first; but for every so manyth thing
/// it has looked at since it was made, when it looks at the other, so
/// that neither is kept waiting. How many it has looked at by then goes
/// by load: whoever sets a play up asks a database with nothing held,
/// and waits or does not, by whether the answer was there before it
/// looked. So which came first, of two things a conductor said in one
/// turn, was another from one run to the next, and with it which plays
/// were made. Here what this thread woke is always looked at first:
/// what a database woke is looked at when nothing else is left, which
/// is every turn, since a turn runs on until nothing is.
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .global_queue_interval(u32::MAX)
        .build()
        .expect("a runtime for a play")
}

/// The play with nothing in its way, on a thread and a runtime of its
/// own while this thread waits: what it came to, and each place it
/// passed. Made as every other play is, so that the order it passed its
/// places in, which is the order the others are made in, is one order.
pub(super) fn first(scenario: &Scenario) -> (Value, Vec<Place>) {
    let play = || {
        runtime().block_on(async {
            let mut play = Play::begin(scenario, None, Way::Shut, None).await;
            if let Err(why) = play.through(scenario).await {
                panic!("{}: with nothing in its way, {why}", scenario.name);
            }
            let usual = play.came().await;
            let mut places = play.passed.lock().unwrap().places.clone();
            places.dedup();
            (usual, places)
        })
    };
    match std::thread::scope(|scope| scope.spawn(play).join()) {
        Ok(came) => came,
        Err(why) => std::panic::resume_unwind(why),
    }
}

/// Makes one play, on the thread this is asked from.
fn make(scenario: &Scenario, usual: &Value, asked: Asked) -> Made {
    let _turn = Turn::wait();
    runtime().block_on(async {
        let mut play = Play::begin(scenario, Some(asked.place), asked.way, asked.second).await;
        let went = went(scenario, asked.place, usual, &mut play).await;
        // Read where the play ended as it was meant to: one that did
        // not may have a journal that answers nothing.
        let (begun, there) = match went {
            Went::AsMeant => (play.begun().await, (scenario.there)(&play.seen().await.0)),
            _ => (Vec::new(), false),
        };
        let passed = play.passed.lock().unwrap();
        Made { went, begun, there, again: passed.again.clone(), kinds: passed.kinds.clone() }
    })
}

/// Makes each of these plays, several at a time, and gives what each came
/// to in the order they were asked in. A play that panics (a scenario
/// that was refused outright) fails the test as it did, in its own words.
fn make_all(scenario: &Scenario, usual: &Value, asked: Vec<Asked>) -> Vec<Made> {
    let asked: Vec<Mutex<Option<Asked>>> = asked.into_iter().map(|one| Mutex::new(Some(one))).collect();
    let made: Vec<Mutex<Option<Made>>> = asked.iter().map(|_| Mutex::new(None)).collect();
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let threads: Vec<_> = (0..room().min(asked.len()))
            .map(|_| {
                scope.spawn(|| {
                    loop {
                        let at = next.fetch_add(1, Ordering::Relaxed);
                        let Some(one) = asked.get(at).and_then(|one| one.lock().unwrap().take()) else {
                            break;
                        };
                        *made[at].lock().unwrap() = Some(make(scenario, usual, one));
                    }
                })
            })
            .collect();
        for thread in threads {
            if let Err(why) = thread.join() {
                std::panic::resume_unwind(why);
            }
        }
    });
    made.into_iter()
        .map(|one| one.into_inner().unwrap().expect("every play that was asked for is made"))
        .collect()
}

/// What a play that is never stopped a second time is told: to be
/// stopped at a thing its conductor will not come to.
fn never() -> Option<(Again, Way)> {
    Some((Again::Each(usize::MAX), Way::Shut))
}

/// Whether a play told `at` would come to rest without being stopped a
/// second time, where its conductor keeps these things by itself.
fn rests(at: &Again, kinds: &[Kind]) -> bool {
    match at {
        Again::New(played) => kinds.iter().all(|kind| played.contains(kind)),
        Again::Each(nth) => kinds.len() < *nth,
    }
}

/// What every play of a scenario but the one with nothing in its way
/// came to: what was tried, in the order it always was, in how many
/// plays each thing that was written down and not yet begun was begun
/// by whom, and in how many plays that held the scenario was where it is
/// there to be.
pub(super) struct Played {
    pub tried: Vec<Tried>,
    pub begun: BTreeMap<(String, String, String), usize>,
    pub noted: usize,
}

/// Every play of a scenario but the one with nothing in its way.
pub(super) fn played(scenario: &Scenario, usual: &Value, places: &[Place], everything: bool) -> Played {
    gone_through(scenario, usual, places, everything, true)
}

/// The same, with the plays that can be worked out made `ahead` and
/// together, or with every play made as it is come to, one after
/// another, which is how they were always made and what the other is
/// held to ([`the_plays_made_several_at_a_time_are_those_made_one_after_another`]).
fn gone_through(
    scenario: &Scenario,
    usual: &Value,
    places: &[Place],
    everything: bool,
    ahead: bool,
) -> Played {
    let mut tried = Vec::new();
    let mut begun: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    let mut noted = 0;
    let stopped_at = places.iter().filter(|place| scenario.only.is_none_or(|only| only(place)));
    let both: Vec<(&Place, Way)> =
        stopped_at.flat_map(|place| [(place, Way::Shut), (place, Way::Unsaid)]).collect();
    if scenario.only.is_some() {
        // Stopped once at each, and no more: no play is told anything by
        // another.
        let asked = both.iter().map(|(place, way)| Asked { place, way: *way, second: None }).collect();
        for ((place, way), one) in both.iter().zip(make_all(scenario, usual, asked)) {
            for who in one.begun {
                *begun.entry(who).or_default() += 1;
            }
            tried.push(Tried {
                place: (*place).clone(),
                way: *way,
                again: None,
                rested: false,
                went: one.went,
            });
        }
        return Played { tried, begun, noted };
    }

    // For each place and way, the play that is never stopped a second
    // time: what its conductor keeps by itself after its start.
    let mut alone: Vec<Option<Made>> = both.iter().map(|_| None).collect();
    // And the others, each with what it was told.
    let mut made: Vec<(usize, Way, Again, Option<Made>)> = Vec::new();
    if ahead {
        let asked = both.iter().map(|(place, way)| Asked { place, way: *way, second: never() }).collect();
        alone = make_all(scenario, usual, asked).into_iter().map(Some).collect();
        // What each of the other plays will be told, if every play
        // keeps what that one kept, worked out as they are gone through
        // below; and those plays, made together.
        let mut told: Vec<(usize, Way, Again)> = Vec::new();
        let mut reached: BTreeMap<(Way, Way), BTreeSet<Kind>> = BTreeMap::new();
        for (index, (_, way)) in both.iter().enumerate() {
            let kinds = &alone[index].as_ref().expect("made above").kinds;
            for second in [Way::Shut, Way::Unsaid] {
                let done = reached.entry((*way, second)).or_default();
                for (nth, kind) in kinds.iter().enumerate() {
                    if everything {
                        told.push((index, second, Again::Each(nth + 1)));
                    } else if !done.contains(kind) {
                        told.push((index, second, Again::New(done.clone())));
                    }
                    done.insert(kind.clone());
                }
            }
        }
        let asked = told
            .iter()
            .map(|(index, second, at)| Asked {
                place: both[*index].0,
                way: both[*index].1,
                second: Some((at.clone(), *second)),
            })
            .collect();
        made = told
            .into_iter()
            .zip(make_all(scenario, usual, asked))
            .map(|((index, second, at), one)| (index, second, at, Some(one)))
            .collect();
    }

    // Gone through as it always was: each play it asks for taken from
    // those made, where one was told that very thing, and made now
    // where none was.
    let mut played: BTreeMap<(Way, Way), BTreeSet<Kind>> = BTreeMap::new();
    for (index, (place, way)) in both.iter().enumerate() {
        // What the play stopped once here kept by itself.
        let mut kept: Option<Vec<Kind>> = None;
        for second in [Way::Shut, Way::Unsaid] {
            let done = played.entry((*way, second)).or_default();
            let mut found = 0;
            loop {
                let more = kept.as_ref().is_none_or(|kept| match everything {
                    true => found < kept.len(),
                    false => kept.iter().any(|kind| !done.contains(kind)),
                });
                if !more {
                    break;
                }
                let at = if everything { Again::Each(found + 1) } else { Again::New(done.clone()) };
                let same = made.iter_mut().find(|(of, how, told, one)| {
                    (*of, *how) == (index, second) && *told == at && one.is_some()
                });
                let one = match same {
                    Some((.., one)) => one.take().expect("looked for one that is there"),
                    // The play never stopped a second time is what one
                    // told this comes to, where it would not be stopped
                    // at anything that one kept: the play stopped once.
                    None if kept.is_none()
                        && alone[index].as_ref().is_some_and(|alone| rests(&at, &alone.kinds)) =>
                    {
                        alone[index].take().expect("looked for one that is there")
                    }
                    None => {
                        let asked = Asked { place, way: *way, second: Some((at, second)) };
                        make_all(scenario, usual, vec![asked]).pop().expect("one play was asked for")
                    }
                };
                if matches!(one.went, Went::AsMeant) {
                    for who in one.begun {
                        *begun.entry(who).or_default() += 1;
                    }
                    noted += usize::from(one.there);
                }
                let (went, by_itself) = (one.went, one.kinds);
                let Some(at) = one.again else {
                    // It came to rest first: the play stopped once.
                    tried.push(Tried {
                        place: (*place).clone(),
                        way: *way,
                        again: None,
                        rested: kept.is_some(),
                        went,
                    });
                    kept = Some(by_itself);
                    break;
                };
                found += 1;
                done.insert(at.kind());
                let again = Some((by_itself.len(), second, at));
                tried.push(Tried { place: (*place).clone(), way: *way, again, rested: false, went });
            }
        }
    }
    Played { tried, begun, noted }
}

const TWO: &str = r#"blueprint: abreast
description: Two steps, the second after the first, and something said as it ends.
steps:
  one: { do: test.mark, with: { key: one } }
  two: { needs: [one], do: test.echo, with: { said: two } }
on_finish:
  - emit: { topic: abreast.over, how: "${ run.status }" }
"#;

/// What was tried, each play as a line: where and how it was stopped,
/// where and how a second time, and how it went.
fn lines(tried: &[Tried]) -> Vec<String> {
    let line = |one: &Tried| {
        let again = one
            .again
            .as_ref()
            .map_or(String::new(), |(after, way, at)| format!("; again after thing {after} ({at}), {way}"));
        let went = match &one.went {
            Went::AsMeant => "as meant".to_string(),
            Went::NeverThere => "never there".to_string(),
            Went::Fault(why) => format!("a fault: {}", why.join("; ")),
        };
        let rested = if one.rested { ", at rest before a second stop" } else { "" };
        format!("after {}, {}{again}{rested}: {went}", one.place, one.way)
    };
    tried.iter().map(line).collect()
}

#[tokio::test]
async fn the_plays_made_several_at_a_time_are_those_made_one_after_another() {
    let scenario = Scenario::of("abreast", TWO);
    let (usual, places) = first(&scenario);
    // One after another, each told by the plays before it; and with
    // what can be worked out made ahead and together.
    // At two of its places, which is enough for a play to be told
    // something by the play before it: this holds how the plays are
    // made, and each test that plays holds its own places.
    let places = &places[1..3];
    let in_turn = gone_through(&scenario, &usual, places, false, false);
    let abreast = gone_through(&scenario, &usual, places, false, true);
    eprintln!("{} places, {} plays each way", places.len(), in_turn.tried.len());
    assert!(in_turn.tried.iter().any(|one| one.again.is_some()), "some play is stopped a second time");
    assert!(in_turn.tried.iter().any(|one| one.again.is_none()), "and some play is stopped once");
    assert_eq!(lines(&abreast.tried), lines(&in_turn.tried));
    assert_eq!(abreast.begun, in_turn.begun);
    assert_eq!(abreast.noted, in_turn.noted);
    assert!(in_turn.tried.iter().all(|one| matches!(one.went, Went::AsMeant)), "{:?}", lines(&in_turn.tried));
}
