//! What a scenario's simulation found: how each play that was stopped
//! somewhere went, what is said of it where somebody is looking, and what
//! the test is held to.

use std::time::Duration;

use crate::simulation::{BEGAN, Place};

/// Which of a scenario's places this program plays, where the places are
/// played in shares (`SIMULATE_SHARE=2/4`: the second of four): every
/// place there is, counted from where it is in the first play, is in one
/// share and no other; what a test asks for by itself is played in every
/// share beside that ([`Grain::Shared`]), so that what it holds of those
/// places it holds whichever share is played. A run of every place is as
/// long as its longest
/// scenario, whose plays are a few at a time in one process; in shares,
/// each a process of its own, it is that much shorter, and a share that
/// was cut off is the only one played again. Unsaid, all of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Share {
    one: usize,
    of: usize,
}

impl Share {
    /// What this program was told, if anything.
    pub fn said() -> Option<Share> {
        std::env::var("SIMULATE_SHARE").ok().map(|said| Share::read(&said))
    }

    /// `2/4` is the second of four. Anything else is a mistake in what
    /// was asked, and is said so: played as everything, it would pass for
    /// a share having been played.
    pub fn read(said: &str) -> Share {
        let parts =
            said.split_once('/').map(|(one, of)| (one.trim().parse::<usize>(), of.trim().parse::<usize>()));
        match parts {
            Some((Ok(one), Ok(of))) if (1..=of).contains(&one) => Share { one: one - 1, of },
            _ => panic!("`SIMULATE_SHARE={said}` is not a share: say which of how many, as `2/4`"),
        }
    }

    /// Whether the place that is `nth` in the first play, from 0, is in it.
    pub fn takes(self, nth: usize) -> bool {
        nth % self.of == self.one
    }
}

#[test]
fn every_place_is_in_one_share_of_those_it_is_played_in_and_in_no_other() {
    for of in 1..=7 {
        for nth in 0..50 {
            let in_it = (1..=of).filter(|one| Share::read(&format!("{one}/{of}")).takes(nth)).count();
            assert_eq!(in_it, 1, "place {nth} of shares of {of}");
        }
    }
    assert!((0..50).all(|nth| Share::read("1/1").takes(nth)), "one share of one is every place");
    for wrong in ["0/4", "5/4", "4", "a/b", ""] {
        assert!(std::panic::catch_unwind(|| Share::read(wrong)).is_err(), "`{wrong}` was taken for a share");
    }
}

/// Which places a scenario is stopped at.
#[derive(Clone, Copy, Debug)]
pub enum Grain {
    /// Wherever this says: what a test asks for itself.
    Where(fn(&Place) -> bool),
    /// The first time each kind of run begins, each time an agent is given
    /// a turn, each time its program is there, and each time something
    /// begins to wait: one stop inside everything that takes time or is
    /// taken up again by a rule of its own. Places a scenario says are
    /// not held yet are left out.
    Kinds,
    /// After everything the daemon says. Slow, and for running by hand.
    Everything,
    /// A share of everything, and with it the places the test asks for
    /// itself: what a test holds of those it holds in every share.
    Shared(fn(&Place) -> bool, Share),
}

impl Grain {
    /// What the environment asks for, or `gate`, which is what the test
    /// asks for when nobody says: `SIMULATE=everything` for every place
    /// (with `SIMULATE_SHARE`, a share of them), `SIMULATE=kinds` for one
    /// of each kind, `SIMULATE=places` to play none and have them listed.
    pub fn asked_or(gate: Grain) -> Grain {
        match (std::env::var("SIMULATE").as_deref(), Share::said(), gate) {
            (Ok("everything"), None, _) => Grain::Everything,
            (Ok("everything"), Some(share), Grain::Where(own)) => Grain::Shared(own, share),
            (Ok("everything"), Some(share), _) => Grain::Shared(|_| false, share),
            (Ok("kinds"), ..) => Grain::Kinds,
            (Ok("places"), ..) => Grain::Where(|_| false),
            _ => gate,
        }
    }

    /// Whether the place that is `nth` in the first play, from 0, is one.
    pub fn takes(self, nth: usize, place: &Place) -> bool {
        match self {
            Grain::Everything => true,
            Grain::Where(such) => such(place),
            Grain::Shared(own, share) => own(place) || share.takes(nth),
            Grain::Kinds => match place.topic.as_str() {
                "run.started" | "step.waiting" => place.nth == 1,
                "agent.turn.started" | BEGAN => true,
                _ => false,
            },
        }
    }
}

/// How a play that was stopped somewhere went.
#[derive(Clone, Debug)]
pub enum Went {
    /// It came to what was meant.
    AsMeant,
    /// The daemon never said the thing it was to be stopped after: things
    /// were said in another order this time. What was said instead is
    /// written down where a play that went wrong is.
    NeverThere,
    /// It came to something else, or to nothing.
    Fault(Vec<String>),
    /// It did not come to rest: something was still under way when the
    /// play was given up on. What was under way, and what it had come to.
    Unrested(Vec<String>),
    /// It was not played: the scenario says the place is not held yet, and why.
    NotYet(&'static str),
}

/// How many times a place is played at most. A play that comes to rest
/// without the daemon having said what it was to be stopped after tried
/// nothing there, and is played again: which of two things came first in
/// one play is the machine's to say, and it mostly says the other the
/// next time. A place no play of that many came to is said to be never
/// reached.
pub const PLAYS: usize = 3;

/// Plays a place until a play has come to it, [`PLAYS`] times at most: how
/// the last of them went, and how many there were.
pub async fn until_there(mut play: impl AsyncFnMut() -> Went) -> (Went, usize) {
    let mut plays = 1;
    loop {
        let went = play().await;
        if !matches!(went, Went::NeverThere) || plays == PLAYS {
            return (went, plays);
        }
        plays += 1;
    }
}

/// One place a scenario was stopped at, and what came of it.
#[derive(Clone, Debug)]
pub struct Tried {
    pub place: Place,
    pub went: Went,
    /// Why it is not held yet, where the scenario says it is not.
    pub not_yet: Option<&'static str>,
    /// Why a play may go by it, where the scenario says it goes two ways
    /// there by design.
    pub either: Option<&'static str>,
    /// How many times it was played: more than once only where a play
    /// never came to it.
    pub plays: usize,
}

/// What a scenario's simulation found.
pub struct Found {
    pub name: &'static str,
    /// Every place such a play passes, in order.
    pub passed: Vec<Place>,
    /// Each place tried and how it went.
    pub tried: Vec<Tried>,
    pub took: Duration,
}

impl Found {
    /// Fails the test with every place the play did not come to what was
    /// meant, each with what it came to in its place; and for every place
    /// no play came to at all, unless the scenario says a play may go by
    /// it. A place the scenario says is not held yet is not held to
    /// anything here.
    pub fn holds(&self) {
        self.tell();
        let wrong = self.wrong();
        assert!(wrong.is_empty(), "{}", wrong.join("\n\n"));
    }

    /// Everything the simulation found that the scenario is held to, a
    /// failure of its own each, in its own words: places that came to
    /// something else, places that did not come to rest, and places no
    /// play came to.
    pub fn wrong(&self) -> Vec<String> {
        let played = self.tried.iter().filter(|tried| !matches!(tried.went, Went::NotYet(_))).count();
        let after =
            |tried: &Tried, why: &[String]| format!("  after {}:\n    {}", tried.place, why.join("\n    "));
        let mut wrong = Vec::new();
        let faults: Vec<String> = self
            .tried
            .iter()
            .filter_map(|tried| match (&tried.went, tried.not_yet) {
                (Went::Fault(why), None) => Some(after(tried, why)),
                _ => None,
            })
            .collect();
        if !faults.is_empty() {
            wrong.push(format!(
                "{}: stopped and started again at {played} of {} places, the daemon did not come to what was \
                 meant at {}:\n{}",
                self.name,
                self.passed.len(),
                faults.len(),
                faults.join("\n")
            ));
        }
        let unrested: Vec<String> = self
            .tried
            .iter()
            .filter_map(|tried| match &tried.went {
                Went::Unrested(why) => Some(after(tried, why)),
                _ => None,
            })
            .collect();
        if !unrested.is_empty() {
            wrong.push(format!(
                "{}: stopped and started again, the daemon did not come to rest at {} places: something was \
                 still under way when the play was given up on. That is not the application coming to the \
                 wrong thing, and is worth a look all the same:\n{}",
                self.name,
                unrested.len(),
                unrested.join("\n")
            ));
        }
        // A place no play came to was not tried, and saying so beside
        // those that were is not holding it. A play begins and ends in a
        // factory at rest, so what one play says another says too, unless
        // the scenario itself goes two ways: which it says, with why.
        let never: Vec<String> = self
            .tried
            .iter()
            .filter(|tried| matches!(tried.went, Went::NeverThere) && tried.either.is_none())
            .filter(|tried| tried.not_yet.is_none())
            .map(|tried| format!("  after {}", tried.place))
            .collect();
        if !never.is_empty() {
            wrong.push(format!(
                "{}: {} of the {played} places tried were never reached, in {PLAYS} plays each: every one of \
                 them came to rest without the daemon having said what it was to be stopped after, so nothing \
                 was tried there. The play with nothing in its way said it, so two things in the scenario \
                 come in either order. Where that is by chance (something done before the factory is at rest, \
                 somebody let go before what they were to wait behind), the scenario is to be written so that \
                 it is not; where it is by design, the scenario says so (`Scenario::either`, with the sentence \
                 why), and the place is then said and not failed. What the daemon said in the last of those \
                 plays is in target/tmp/simulation/, under the place's name:\n{}",
                self.name,
                never.len(),
                never.join("\n")
            ));
        }
        wrong
    }

    /// Says how each place went, where somebody is looking
    /// (`SIMULATE=...`, with `--nocapture`).
    fn tell(&self) {
        // The play with nothing in its way, and each play of a place.
        crate::played(self.passed.len(), 1 + self.tried.iter().map(|tried| tried.plays).sum::<usize>());
        if std::env::var("SIMULATE").is_err() && std::env::var("SIMULATE_AT").is_err() {
            return;
        }
        let count = |such: fn(&Went) -> bool| self.tried.iter().filter(|tried| such(&tried.went)).count();
        let never = |tried: &&Tried| matches!(tried.went, Went::NeverThere);
        eprintln!(
            "{}: {} places passed; of {} looked at, {} as meant, {} faults, {} never reached ({} of them where \
             the scenario goes two ways), {} not played, {} did not come to rest; {} played more than once; \
             {:.1} s",
            self.name,
            self.passed.len(),
            self.tried.len(),
            count(|went| matches!(went, Went::AsMeant)),
            count(|went| matches!(went, Went::Fault(_))),
            self.tried.iter().filter(never).count(),
            self.tried.iter().filter(never).filter(|tried| tried.either.is_some()).count(),
            count(|went| matches!(went, Went::NotYet(_))),
            count(|went| matches!(went, Went::Unrested(_))),
            self.tried.iter().filter(|tried| tried.plays > 1).count(),
            self.took.as_secs_f64()
        );
        // `SIMULATE=places` says every place there is, and plays none.
        if std::env::var("SIMULATE").as_deref() == Ok("places") {
            for place in &self.passed {
                eprintln!("  {place}");
            }
        }
        for tried in &self.tried {
            eprintln!("  after {}: {}", tried.place, tried.said());
        }
    }
}

impl Tried {
    /// How it went, in words for whoever is looking. A place never reached
    /// is said so at the end of its line, whatever is said of it under that.
    fn said(&self) -> String {
        let again = match self.plays {
            0 | 1 => String::new(),
            plays => format!("\n      at play {plays}: no play before it came to the place"),
        };
        let how = match (&self.went, self.either) {
            (Went::AsMeant, _) => format!("as meant{again}"),
            (Went::NeverThere, Some(why)) => format!(
                "never reached\n      in {} plays, which the scenario says may be, going two ways there: {why}",
                self.plays
            ),
            (Went::NeverThere, None) => format!(
                "never reached\n      in {} plays, and the scenario does not say that a play may go by it",
                self.plays
            ),
            (Went::NotYet(why), _) => format!("not played, not held yet: {why}"),
            (Went::Fault(why), _) => format!("FAULT{again}\n      {}", why.join("\n      ")),
            (Went::Unrested(why), _) => {
                format!("DID NOT COME TO REST{again}\n      {}", why.join("\n      "))
            }
        };
        match (&self.went, self.not_yet) {
            (Went::NotYet(_), _) | (_, None) => how,
            (_, Some(why)) => format!("{how}\n      (said not to be held yet: {why})"),
        }
    }
}

/// A place for the tests of what is found: something a stage said.
fn somewhere(step: &str) -> Place {
    Place {
        topic: "step.started".to_string(),
        blueprint: "std/working".to_string(),
        flow: "on.first".to_string(),
        step: step.to_string(),
        who: String::new(),
        nth: 1,
    }
}

fn found_in(tried: Vec<Tried>) -> Found {
    let passed = tried.iter().map(|tried| tried.place.clone()).collect();
    Found { name: "a scenario", passed, tried, took: Duration::ZERO }
}

fn tried(step: &str, went: Went, plays: usize) -> Tried {
    Tried { place: somewhere(step), went, not_yet: None, either: None, plays }
}

#[tokio::test]
async fn a_place_a_play_never_came_to_is_played_again_three_times_at_most_and_no_more_once_one_came_to_it() {
    // Never: three plays, and it is said to be never reached.
    let mut played = 0;
    let (went, plays) = until_there(async || {
        played += 1;
        Went::NeverThere
    })
    .await;
    assert!(matches!(went, Went::NeverThere));
    assert_eq!((plays, played), (PLAYS, 3));

    // The second play came to it: how that went is how it went, and there is no third.
    let mut played = 0;
    let (went, plays) = until_there(async || {
        played += 1;
        if played < 2 { Went::NeverThere } else { Went::Fault(vec!["it came to something else".to_string()]) }
    })
    .await;
    assert!(matches!(went, Went::Fault(_)));
    assert_eq!((plays, played), (2, 2));

    // A play that came to its place is not played again, however it went.
    for first in [Went::AsMeant, Went::Unrested(Vec::new()), Went::Fault(Vec::new())] {
        let mut played = 0;
        let (_, plays) = until_there(async || {
            played += 1;
            first.clone()
        })
        .await;
        assert_eq!((plays, played), (1, 1), "{first:?}");
    }
}

#[test]
fn a_place_never_reached_is_a_failure_of_its_own_however_few_of_them_there_are() {
    // One of a hundred: under the rule there was, which failed only where
    // a quarter of what was tried was missed, this passed.
    let mut all: Vec<Tried> = (0..99).map(|nth| tried(&format!("step-{nth}"), Went::AsMeant, 1)).collect();
    all.push(tried("each", Went::NeverThere, PLAYS));
    let found = found_in(all);

    let wrong = found.wrong();
    assert_eq!(wrong.len(), 1, "{wrong:#?}");
    assert!(
        wrong[0].starts_with("a scenario: 1 of the 100 places tried were never reached, in 3 plays each"),
        "{}",
        wrong[0]
    );
    assert!(
        wrong[0].ends_with("\n  after `step.started` of std/working (on.first), step `each`"),
        "{}",
        wrong[0]
    );
    assert!(wrong[0].contains("`Scenario::either`"), "{}", wrong[0]);
    assert!(std::panic::catch_unwind(|| found.holds()).is_err(), "it was held all the same");
}

#[test]
fn a_place_never_reached_where_the_scenario_says_it_goes_two_ways_is_said_and_not_failed() {
    let why = "a stage looks at its column when it is active or when the fleet is arranged, whichever it hears first";
    let either = Tried { either: Some(why), ..tried("each", Went::NeverThere, PLAYS) };
    let found = found_in(vec![tried("waiting", Went::AsMeant, 2), either.clone()]);

    assert_eq!(found.wrong(), Vec::<String>::new());
    found.holds();
    // It is said, with the sentence, and its line still ends as a place
    // never reached does: whatever counts those lines counts it.
    let said = either.said();
    assert!(said.starts_with("never reached\n"), "{said}");
    assert!(said.contains("in 3 plays") && said.contains(why), "{said}");
    // One that a later play came to says which play.
    assert!(found.tried[0].said().starts_with("as meant\n      at play 2"), "{}", found.tried[0].said());

    // Saying a place goes two ways excuses nothing but its never being
    // reached: a play that came to it and to the wrong thing is a fault.
    let fault =
        Tried { either: Some(why), ..tried("each", Went::Fault(vec!["cards: nothing".to_string()]), 1) };
    let wrong = found_in(vec![fault]).wrong();
    assert_eq!(wrong.len(), 1);
    assert!(wrong[0].contains("the daemon did not come to what was meant at 1"), "{}", wrong[0]);
}

#[test]
fn each_kind_of_thing_found_is_said_apart_and_what_is_not_held_yet_is_held_to_nothing() {
    let not_yet = |went: Went| Tried { not_yet: Some("it is known not to hold"), ..tried("known", went, 1) };
    let found = found_in(vec![
        tried("a", Went::Fault(vec!["cards: it came to nothing".to_string()]), 1),
        tried("b", Went::Unrested(vec!["still under way: a run".to_string()]), 1),
        tried("c", Went::NeverThere, PLAYS),
        not_yet(Went::Fault(vec!["as was known".to_string()])),
        not_yet(Went::NeverThere),
        not_yet(Went::NotYet("it is known not to hold")),
    ]);
    let wrong = found.wrong();
    assert_eq!(wrong.len(), 3, "{wrong:#?}");
    assert!(wrong[0].contains("did not come to what was meant at 1:") && wrong[0].contains("step `a`"));
    assert!(wrong[1].contains("did not come to rest at 1 places") && wrong[1].contains("step `b`"));
    assert!(wrong[2].contains("1 of the 5 places tried were never reached") && wrong[2].contains("step `c`"));
    assert!(!wrong.join("\n").contains("step `known`"), "{wrong:#?}");
}
