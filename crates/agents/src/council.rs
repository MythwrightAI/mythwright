//! Several agents asked about one thing at once, each for itself: a
//! council, which is `agent.task` told how many seats there are
//! (`seats`), how many have to agree (`needed`) and which of the words
//! an agent may end with is for the work and which against (`yes`,
//! `no`).
//!
//! Each seat is a turn of its own, in a conversation of its own
//! (`<thread>:<seat>`), of an agent other than those of the other seats
//! while there are enough of them, and asked in a word of its own
//! (`<asking>:<seat>`). They are chosen and queued as one thing, under
//! the turn at choosing, so that two seats do not find the same agent
//! with nothing in hand. As each answers, what the answers so far come
//! to is asked of the tally (`deciding::tally`); once that decides, the
//! seats still looking are called off and what was decided is the step's
//! answer.
//!
//! Nothing here is kept but by the step: the turns are written down as
//! they are queued (`seats` of what the step keeps), and what was
//! decided before anybody is called off (`decided`). So a step that the
//! daemon stopped while it waited goes on waiting for the seats it had,
//! and one stopped as it decided says what it decided and calls off what
//! was still looking. Done again from its start, each seat finds its
//! turn by its word as it is by now, and one that was called off for
//! the decision is no vote: what decided it is there again, and decides
//! it again. A seat that could not be asked, or whose agent could not
//! do it, has said neither, in the words of why.

use std::future::Future;
use std::pin::Pin;
use std::task::Poll;

use conductor::{ActionCx, ActionError, ActionResult};
use deciding::tally::{Counted, NO, Vote, YES, tally};
use kernel::Fields;
use serde_json::{Value, json};

use crate::Agents;
use crate::actions::{Asked, answered, ask, listed, text, turn};

/// What one seat is, as the step keeps it: the turn it was given, or the
/// words of why nobody was asked.
#[derive(Clone)]
enum Seat {
    Turn(String),
    Unasked(String),
}

impl Seat {
    fn kept(&self) -> Value {
        match self {
            Seat::Turn(id) => json!({ "turn": id }),
            Seat::Unasked(why) => json!({ "unasked": why }),
        }
    }

    fn read(kept: &Value) -> Seat {
        match kept["turn"].as_str() {
            Some(id) => Seat::Turn(id.to_string()),
            None => Seat::Unasked(kept["unasked"].as_str().unwrap_or("it could not be asked").to_string()),
        }
    }
}

/// How the votes are counted: said once, of the step.
struct Rules {
    seats: i64,
    needed: i64,
    yes: String,
    no: String,
}

/// A count the step was handed, as a whole number: a row of a table may
/// say one as `2.0`.
pub(crate) fn count(args: &Fields, key: &str) -> Option<i64> {
    let said = args.get(key)?;
    said.as_i64().or_else(|| said.as_f64().filter(|n| n.fract() == 0.0).map(|n| n as i64))
}

impl Rules {
    fn of(args: &Fields) -> Result<Rules, ActionError> {
        let rules = Rules {
            seats: count(args, "seats").unwrap_or(1),
            needed: count(args, "needed").unwrap_or(0),
            yes: text(args, "yes").unwrap_or_else(|| YES.to_string()),
            no: text(args, "no").unwrap_or_else(|| NO.to_string()),
        };
        // What makes no sense is said now, before anybody is asked.
        rules.count(&[]).map_err(ActionError::new)?;
        Ok(rules)
    }

    fn count(&self, votes: &[Vote]) -> Result<deciding::tally::Tallied, String> {
        tally(&Counted { votes, seats: self.seats, needed: self.needed, yes: &self.yes, no: &self.no })
    }
}

/// A vote that has come in: which seat, what it said, and how often the
/// daemon was stopped in the middle of its turn.
struct Cast {
    seat: usize,
    vote: Vote,
    again: u64,
}

pub(crate) async fn council(
    agents: Agents,
    cx: ActionCx,
    args: Fields,
    restarts: (bool, Option<u32>),
) -> ActionResult {
    let rules = Rules::of(&args)?;
    let thread = text(&args, "thread").unwrap_or_else(|| format!("instance:{}", cx.instance()));
    let apart = text(&args, "apart").unwrap_or_else(|| format!("{thread}:"));
    let asking = text(&args, "asking");
    let options = listed(&args, "options").unwrap_or_else(|| vec![rules.yes.clone(), rules.no.clone()]);
    // What each seat is asked with: the step's arguments, with its own
    // conversation and its own word for which asking it is.
    let seat_args = |seat: usize| {
        let mut own = args.clone();
        // Who an item is given to, or one agent named, is whoever works
        // it: a seat is chosen among those who look, apart from the rest.
        own.remove("slot");
        own.remove("given");
        // A council of one may be one agent named to look (`looker`): a
        // rung of a project's chain that reviews, where nobody of the role
        // is there to.
        if let Some(looker) = text(&args, "looker").filter(|_| rules.seats == 1) {
            own.insert("slot".into(), json!(looker));
        }
        own.insert("thread".into(), json!(format!("{thread}:{seat}")));
        own.insert("apart".into(), json!(apart));
        own.insert("options".into(), json!(options));
        if let Some(word) = &asking {
            own.insert("asking".into(), json!(format!("{word}:{seat}")));
        }
        if let Some(about) = own.get_mut("about").and_then(Value::as_object_mut) {
            about.insert("seat".into(), json!(seat));
        }
        own
    };
    for seat in 1..=rules.seats as usize {
        agents.may_go_on(&format!("{thread}:{seat}"), cx.principal()).await?;
    }
    let mut seats = Vec::new();
    {
        let _chosen = agents.inner.choosing.lock().await;
        for seat in 1..=rules.seats as usize {
            seats.push(match ask(&agents, &cx, &seat_args(seat), restarts).await {
                Ok(Asked::Turn(id)) => Seat::Turn(id),
                Ok(Asked::Nobody) => Seat::Unasked("nobody was there to look".to_string()),
                Err(why) => Seat::Unasked(why.message),
            });
        }
    }
    let kept: Vec<Value> = seats.iter().map(Seat::kept).collect();
    cx.checkpoint(json!({ "seats": kept })).await?;
    decide(&agents, &cx, &rules, &seats).await
}

/// The conductor was started again while the council waited: it goes on
/// waiting for the seats it had, or, where it had decided, says what it
/// decided and calls off whoever was still looking.
pub(crate) async fn resume(agents: Agents, cx: ActionCx, args: Fields, saved: Value) -> ActionResult {
    let rules = Rules::of(&args)?;
    let seats: Vec<Seat> = saved["seats"].as_array().into_iter().flatten().map(Seat::read).collect();
    for seat in &seats {
        if let Seat::Turn(id) = seat {
            // It is this step's, and nobody else's to be given.
            agents.claimed(id);
        }
    }
    cx.log('s', "went on waiting for the council after a restart");
    if !saved["decided"].is_null() {
        for seat in &seats {
            if let Seat::Turn(id) = seat
                && agents.turn(id).await.is_ok_and(|turn| !turn.is_finished())
            {
                agents.cancel(id).await;
            }
        }
        return Ok(saved["decided"].clone());
    }
    decide(&agents, &cx, &rules, &seats).await
}

/// A seat's vote, from how its turn ended: what its agent chose and said,
/// or, where it could not do it, neither, in the words of why.
async fn vote_of(
    agents: &Agents,
    cx: &ActionCx,
    seat: usize,
    ended: Result<crate::TurnInfo, ActionError>,
) -> Cast {
    let (by, again) = match &ended {
        Ok(info) => (info.slot.clone(), u64::from(info.restarts)),
        Err(_) => ("nobody".to_string(), 0),
    };
    let said = match ended {
        Ok(info) => answered(agents, cx, info).await,
        Err(why) => Err(why),
    };
    let vote = match said {
        Ok(out) => Vote {
            by,
            decision: out["decision"].as_str().unwrap_or_default().to_string(),
            words: out["reply"].as_str().unwrap_or_default().to_string(),
        },
        Err(why) => {
            return Cast { seat, vote: Vote { by, decision: String::new(), words: why.message }, again: 0 };
        }
    };
    Cast { seat, vote, again }
}

/// Waits for the seats, counting each as it comes in, until what they
/// have said decides it; then those still looking are called off, by
/// their turns being let go of here.
async fn decide(agents: &Agents, cx: &ActionCx, rules: &Rules, seats: &[Seat]) -> ActionResult {
    let mut cast: Vec<Cast> = Vec::new();
    let mut ended = Vec::new();
    let mut waiting = Vec::new();
    for (at, seat) in seats.iter().enumerate() {
        let seat_no = at + 1;
        match seat {
            Seat::Unasked(why) => cast.push(Cast {
                seat: seat_no,
                vote: Vote { by: "nobody".into(), decision: String::new(), words: why.clone() },
                again: 0,
            }),
            Seat::Turn(id) => match agents.turn(id).await {
                Ok(info) if info.is_finished() => ended.push((seat_no, info)),
                _ => waiting.push((seat_no, id.clone())),
            },
        }
    }
    // What had ended is counted whole: asked again after the council had
    // decided, a seat called off for the decision is one that chose
    // neither, and what decided it is there again and decides it again.
    for (seat, info) in ended {
        cast.push(vote_of(agents, cx, seat, Ok(info)).await);
    }
    type Waiting<'a> = Pin<Box<dyn Future<Output = Result<crate::TurnInfo, ActionError>> + Send + 'a>>;
    let mut looking: Vec<(usize, Waiting<'_>)> =
        waiting.into_iter().map(|(seat, id)| (seat, Box::pin(turn(agents, cx, id)) as Waiting<'_>)).collect();
    loop {
        if let Some(decided) = counted(rules, &cast)? {
            // Those still looking are let go of with `looking`, which
            // calls their turns off.
            return keep(cx, seats, decided).await;
        }
        if looking.is_empty() {
            return Err(ActionError::new("a council came to nothing with every seat counted"));
        }
        let (at, seat, ended) = std::future::poll_fn(|waker| {
            for (at, (seat, ending)) in looking.iter_mut().enumerate() {
                if let Poll::Ready(ended) = ending.as_mut().poll(waker) {
                    return Poll::Ready((at, *seat, ended));
                }
            }
            Poll::Pending
        })
        .await;
        // Kept in the order of their seats, so that of two that ended
        // together the one of the lower seat is counted first.
        drop(looking.remove(at));
        cast.push(vote_of(agents, cx, seat, ended).await);
    }
}

/// What the votes cast so far come to, where they decide it: the step's
/// answer, with the votes in the order of their seats, so that who
/// decided it is said the same way however they came in.
fn counted(rules: &Rules, cast: &[Cast]) -> Result<Option<Value>, ActionError> {
    let votes: Vec<Vote> = cast.iter().map(|one| one.vote.clone()).collect();
    if rules.count(&votes).map_err(ActionError::new)?.came.is_empty() {
        return Ok(None);
    }
    let mut ordered: Vec<&Cast> = cast.iter().collect();
    ordered.sort_by_key(|one| one.seat);
    let votes: Vec<Vote> = ordered.iter().map(|one| one.vote.clone()).collect();
    let came = rules.count(&votes).map_err(ActionError::new)?;
    let listed: Vec<Value> = ordered
        .iter()
        .map(|one| json!({ "seat": one.seat, "by": one.vote.by, "decision": one.vote.decision, "words": one.vote.words }))
        .collect();
    let went_on: Vec<Value> = ordered
        .iter()
        .filter(|one| one.again > 0)
        .map(|one| json!({ "agent": one.vote.by, "times": one.again }))
        .collect();
    Ok(Some(json!({
        "available": true,
        "came": came.came,
        "by": came.by,
        "words": came.words,
        "approved": came.approved,
        "against": came.against,
        "seats": rules.seats,
        "votes": listed,
        "went_on": went_on,
    })))
}

/// What was decided is kept before anybody still looking is called off,
/// so that a daemon stopped between the two says the same when it is
/// back, and calls them off then.
async fn keep(cx: &ActionCx, seats: &[Seat], decided: Value) -> ActionResult {
    let kept: Vec<Value> = seats.iter().map(Seat::kept).collect();
    cx.checkpoint(json!({ "seats": kept, "decided": decided })).await?;
    Ok(decided)
}
