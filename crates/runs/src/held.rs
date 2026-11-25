//! What the steps of a run have done that is not written down yet, and
//! which of a step's records are held so ([`holds`]).
//!
//! It came out of the runner, which offers it under the names it always
//! gave (`conductor::holds`, `Quiet`, `WRITTEN_BESIDE`, and `crate::held`
//! there): it is a rule over the rows a run writes, and holds no engine.

use crate::{Say, StepInfo, StepStatus};
use blueprint::ActionSpec;
use serde_json::Value;

/// What the steps of an instance's runs have done that is not written
/// down yet.
///
/// Where the instance keeps nothing as it goes (`durable: false`), that is
/// all of it. Such a run writes nothing down for each step: that is what
/// makes it cheap, and nothing would pick the rows up after a restart. But
/// what each step did is what anyone looking at the run afterwards wants
/// to know. So each step's record is held here while the run goes, shown
/// from here to whoever asks meanwhile (the runner's `Engine::run`), and
/// written once, with the run's ending and in the same piece of work
/// (`Store::end_run`). One write for a run, however many steps it had;
/// a run cut off by a restart has none, as it has nothing else.
///
/// Where the instance keeps what it does, it is the steps that were passed
/// over since the run last wrote (the runner's `RunCx::pass_over`): held
/// until the run's next write, and written in it. With each, what is said of it
/// where that is kept (`step.skipped`), said in that same write: a
/// conductor stopped between a record and what is said of it came back
/// to a step passed over that nobody was told of, or told of twice.
#[derive(Default)]
pub struct Held {
    /// Both under the one lock: whether a step that is passed over may be
    /// held is decided by what is at work, and the deciding and the
    /// holding are one thing, as a step's saying it is at work is another.
    runs: std::sync::Mutex<std::collections::HashMap<String, Of>>,
}

/// What is held of one run.
#[derive(Default)]
struct Of {
    /// Each with which of those held here it was the first time it was:
    /// they are shown, and written, in the order they began, which is
    /// the order they came here in and not the order of their names
    /// (when each began is a millisecond that several share).
    held: std::collections::BTreeMap<String, (u64, StepInfo)>,
    /// How many have been held here, and so what the next is given.
    came: u64,
    /// What is said of those passed over, where it is kept: said in the
    /// write that keeps their records, in the order they were passed.
    says: Vec<Say>,
    /// Its steps that are at work now, where the run is kept.
    going: std::collections::HashSet<String>,
    /// Its steps whose records are held until the run next writes, and
    /// which of them (the runner's `RunCx::hush`): as each begins and as it ends,
    /// where its action only decides; as it ends, where its action is the
    /// same done twice.
    quiet: std::collections::HashMap<String, Quiet>,
}

/// What each write of a step's record is measured as that would have
/// been held, and written with the next thing its run writes, but for
/// another step of the run at work in that moment ([`Held::quietly`],
/// [`Held::pass`]). Whether steps side by side overlap goes by how the
/// machine's work fell, so what one piece of work cost the factory is
/// read without them (`costed.rs` in the API suite).
pub const BESIDE: &str = "step.written_beside";

impl Of {
    /// Holds a step's record, in place of the one held of it: where it
    /// was, for one that is held already, and after everything that
    /// ever was for one that is not.
    fn hold(&mut self, info: StepInfo) {
        let place = self.held.get(&info.step).map(|(place, _)| *place).unwrap_or_else(|| {
            self.came += 1;
            self.came
        });
        self.held.insert(info.step.clone(), (place, info));
    }

    /// What is held, in the order it came.
    fn in_order(held: impl IntoIterator<Item = (u64, StepInfo)>) -> Vec<StepInfo> {
        let mut steps: Vec<(u64, StepInfo)> = held.into_iter().collect();
        steps.sort_by_key(|(place, _)| *place);
        steps.into_iter().map(|(_, info)| info).collect()
    }
}

impl Held {
    /// Has the records of `step` of `run` held as `quiet` says, from
    /// now until the run next writes (the runner's `RunCx::hush`).
    pub fn hush(&self, run: &str, step: &str, quiet: Quiet) {
        let mut runs = self.runs.lock().expect("held lock");
        runs.entry(run.to_string()).or_default().quiet.insert(step.to_string(), quiet);
    }

    pub fn put(&self, info: StepInfo) {
        let mut runs = self.runs.lock().expect("held lock");
        runs.entry(info.run.clone()).or_default().hold(info);
    }

    /// Holds a step that was passed over, where no step of its run is at
    /// work; hands it back to be written at once where one is.
    ///
    /// Held beside a step at work it would stay unwritten for as long as
    /// that step takes. A step says it is at work under this lock before
    /// its record is written ([`Held::notes`]), and its write takes what
    /// is held: so one that began before this looked has it written now,
    /// and one that begins after takes it with its own beginning.
    pub fn pass(&self, info: StepInfo, say: Option<Say>) -> Option<(StepInfo, Option<Say>)> {
        let mut runs = self.runs.lock().expect("held lock");
        let of = runs.entry(info.run.clone()).or_default();
        match of.going.is_empty() {
            true => {
                of.hold(StepInfo { saved: Value::Null, ..info });
                of.says.extend(say);
                None
            }
            false => Some((info, say)),
        }
    }

    /// Holds the record of a step whose action only decides, as it begins
    /// or as it has ended well, and of one whose action is the same done
    /// twice as it has ended well, with what is said of it, where no other
    /// step of its run is at work; hands it back to be written at once
    /// otherwise, and from then on, as a step's is. What it is held as
    /// at work is counted as at work, so that a step passed over beside
    /// it is written at once ([`Held::pass`]). The beginning of one whose
    /// ending alone is held is handed back to be written, and it stays
    /// one whose ending may be held.
    ///
    /// Handed back, it says whether it would have been held but for
    /// another step of its run at work ([`BESIDE`]).
    pub fn quietly(&self, info: StepInfo, says: Vec<Say>) -> Option<(StepInfo, Vec<Say>, bool)> {
        let mut runs = self.runs.lock().expect("held lock");
        let of = runs.entry(info.run.clone()).or_default();
        let quiet = of.quiet.get(&info.step).copied();
        let first = info.attempt == 1;
        if quiet == Some(Quiet::Ending) && info.status == StepStatus::Running && first {
            return Some((info, says, false));
        }
        let held = matches!(info.status, StepStatus::Running | StepStatus::Ok) && first;
        let beside = of.going.iter().any(|step| *step != info.step);
        if quiet.is_none() || !held || beside {
            of.quiet.remove(&info.step);
            let only_beside = quiet.is_some() && held && beside;
            return Some((info, says, only_beside));
        }
        if info.status == StepStatus::Running {
            of.going.insert(info.step.clone());
        } else {
            of.going.remove(&info.step);
            of.quiet.remove(&info.step);
        }
        of.hold(info);
        of.says.extend(says);
        None
    }

    /// Takes away the steps of `run` whose names begin with `prefix`.
    pub fn forget(&self, run: &str, prefix: &str) {
        if let Some(of) = self.runs.lock().expect("held lock").get_mut(run) {
            of.held.retain(|step, _| !step.starts_with(prefix));
        }
    }

    /// The steps of `run` as they stand, in the order they began.
    pub fn of(&self, run: &str) -> Vec<StepInfo> {
        let runs = self.runs.lock().expect("held lock");
        Of::in_order(runs.get(run).into_iter().flat_map(|of| of.held.values().cloned()))
    }

    /// Whether anything of `run` is held.
    pub fn any(&self, run: &str) -> bool {
        let runs = self.runs.lock().expect("held lock");
        runs.get(run).is_some_and(|of| !(of.held.is_empty() && of.says.is_empty()))
    }

    /// What is still to be said of the steps of `run` that were passed
    /// over, held no longer: whoever takes it says it with what it was
    /// about to write, the run's ending.
    pub fn says(&self, run: &str) -> Vec<Say> {
        let mut runs = self.runs.lock().expect("held lock");
        runs.get_mut(run).map(|of| std::mem::take(&mut of.says)).unwrap_or_default()
    }

    /// Holds the steps of `run` no longer: they are written down.
    pub fn written(&self, run: &str) {
        self.runs.lock().expect("held lock").remove(run);
    }

    /// Notes whether a step of a run that is kept is at work, from what
    /// is being written of it.
    pub fn notes(&self, info: &StepInfo) {
        let mut runs = self.runs.lock().expect("held lock");
        let going = &mut runs.entry(info.run.clone()).or_default().going;
        match info.status {
            StepStatus::Running => {
                going.insert(info.step.clone());
            }
            _ => {
                going.remove(&info.step);
            }
        }
    }

    /// The steps of `run` that are held, held no longer: whoever takes
    /// them writes them down with what it was about to write.
    ///
    /// Asked on the database's thread, by the piece of work that writes
    /// them, and not by whoever sends it. Taken before the sending, two
    /// steps of a run that go side by side could each send in the other
    /// order, and a step passed over once and run the next time round
    /// would be written as passed over after it was written as running.
    pub fn take(&self, run: &str) -> (Vec<StepInfo>, Vec<Say>) {
        let mut runs = self.runs.lock().expect("held lock");
        let Some(of) = runs.get_mut(run) else { return Default::default() };
        let (held, says) = (std::mem::take(&mut of.held), std::mem::take(&mut of.says));
        (Of::in_order(held.into_values()), says)
    }
}

/// Which of a step's records are held until its run next writes, where
/// nothing is done around the step (the runner's `RunCx::hush`), by what
/// its action says of itself ([`holds`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Quiet {
    /// Neither: it is written as it begins and as it ends.
    Not,
    /// Its beginning and its ending: its action only decides.
    Whole,
    /// Its ending alone: its action is the same done twice, and does more
    /// than decide.
    Ending,
}

/// Which records of a step of `action` are held until its run next
/// writes, where nothing is done around the step: both, where it only
/// decides; its ending, where it is the same done twice; neither
/// otherwise. Why the ending alone, for one that is the same done twice,
/// is the runner's `RunCx::hush` to say.
pub fn holds(action: &ActionSpec) -> Quiet {
    match (action.decides, action.idempotent) {
        (true, _) => Quiet::Whole,
        (false, true) => Quiet::Ending,
        (false, false) => Quiet::Not,
    }
}

/// What is held of a step of `name` where it is one of the language's
/// own actions ([`holds`]), from what the language says of each, read
/// once; nothing, for any other.
pub fn own(name: &str) -> Option<Quiet> {
    static OWN: std::sync::LazyLock<std::collections::HashMap<String, Quiet>> =
        std::sync::LazyLock::new(|| {
            blueprint::intrinsics().into_iter().map(|spec| (spec.name.clone(), holds(&spec))).collect()
        });
    OWN.get(name).copied()
}
