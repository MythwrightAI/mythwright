//! A text compiled once for a start of conductors.
//!
//! It came out of the runner (`crates/conductor/src/compiled.rs`), which
//! asks it under the name it always did (`crate::compiled` there): it is
//! a function of texts and of what the library answers, and keeps no
//! instance and runs nothing.
//!
//! A start compiles the text of everything alive, and of a fleet most
//! of that is the same few texts: its stages run one, each of its pools
//! and roles another, and a factory of three fleets runs every one of
//! them three times over. It compiled each twice besides, once to see
//! whether the instance keeps anything and once to take it up. A
//! compile is a check of the text against this device's actions and
//! events and against the blueprints the library has for the project
//! (whether a text it names is there; what the others say, for what it
//! hears), and reading what the others say was most of what it took.
//!
//! So what a start compiles itself is kept, for that start and no
//! longer: what each compile came to, by whether it was checked as a
//! text already an instance's or as one an instance is to be given, the
//! name it was compiled as and its text's revision; and what the library
//! says for each project, read once. A compile goes by nothing else of a
//! project than what it asks the library, so what it asked is kept with
//! it, and what it came to for one project is what it comes to for
//! another wherever the library answers the other the same: the same
//! texts that ship, run by fleets of several projects, are compiled once
//! for all of them, and a project with a text of its own under a name,
//! or that says something the others do not, has its own compile. A text
//! that does not compile is kept as not compiling for its project alone
//! (what it says names the file it was read from), and each instance
//! that runs it still fails by itself.
//!
//! Nothing is kept past the start: the actions and events that come and
//! go with packages, and the texts the library has, can change after it
//! (a package added, a project's file written), and nothing says when
//! they have. During it they are taken to be as they were when the start
//! first asked, which is what a start is. Only what the start's own task
//! compiles is kept: what it has taken up runs as tasks of its own, which
//! compile as anything does.

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use blueprint::{Blueprint, Said};
use kernel::Result;

tokio::task_local! {
    /// What a start has compiled, while it is under way.
    static FOR_A_START: Arc<Compiled>;
}

/// What the blueprints a project can see say to each other: each topic
/// written out, and what is said on it.
pub type Sayings = Arc<[(String, Said)]>;

/// A question a compile asked of the library for its project, with the
/// answer it was given.
#[derive(Clone, PartialEq)]
pub enum Asked {
    Blueprint(String, Option<bool>),
    Said(String, Option<Said>),
    Topics(Vec<String>),
}

/// What one compile came to, and what it asked for it.
#[derive(Clone)]
struct Made {
    project: Option<String>,
    asked: Arc<[Asked]>,
    made: Result<Arc<Blueprint>>,
}

/// Whether it is to be given, the name and the revision.
type Key = (bool, String, String);

/// What one start has compiled.
#[derive(Default)]
pub struct Compiled {
    kept: Mutex<HashMap<Key, Vec<Made>>>,
    said: Mutex<HashMap<Option<String>, Sayings>>,
    /// What the compile under way has asked so far.
    asking: Mutex<Option<Vec<Asked>>>,
    compiles: AtomicUsize,
}

impl Compiled {
    /// How many texts were compiled.
    pub fn count(&self) -> usize {
        self.compiles.load(Ordering::SeqCst)
    }
}

/// Does `start` with what it compiles kept for it, and gives what it
/// kept: a start inside another keeps with the one it is inside.
///
/// What a start does is a great deal, and is boxed here: kept in what
/// waits for it, it was several times as large, once for each way in,
/// and a start inside another's overflowed the stack it was asked on.
pub async fn for_a_start<T>(start: impl Future<Output = T>) -> (T, Arc<Compiled>) {
    let start = Box::pin(start);
    match FOR_A_START.try_with(Arc::clone) {
        Ok(kept) => (start.await, kept),
        Err(_) => {
            let kept = Arc::new(Compiled::default());
            (FOR_A_START.scope(kept.clone(), start).await, kept)
        }
    }
}

/// What the library says for `project`: read once for a start.
pub fn said_for(project: Option<&str>, read: impl FnOnce() -> Sayings) -> Sayings {
    let Ok(kept) = FOR_A_START.try_with(Arc::clone) else { return read() };
    let key = project.map(str::to_string);
    if let Some(said) = kept.said.lock().expect("compiled lock").get(&key) {
        return said.clone();
    }
    let said = read();
    kept.said.lock().expect("compiled lock").insert(key, said.clone());
    said
}

/// Notes what a compile under way asked of the library.
pub fn asked(question: impl FnOnce() -> Asked) {
    let _ = FOR_A_START.try_with(|kept| {
        if let Some(asking) = kept.asking.lock().expect("compiled lock").as_mut() {
            asking.push(question());
        }
    });
}

/// `compile` of `source` for `project`, or what it came to before in
/// this start for this project or for one the library answers the same
/// (`answers` asks it the questions again, for `project`).
pub fn once(
    new: bool,
    name: &str,
    source: &str,
    project: Option<&str>,
    compile: impl FnOnce() -> Result<Arc<Blueprint>>,
    answers: impl Fn(&[Asked]) -> bool,
) -> Result<Arc<Blueprint>> {
    let Ok(kept) = FOR_A_START.try_with(Arc::clone) else { return compile() };
    let key = (new, name.to_string(), blueprint::revision(source));
    let same: Vec<Made> = {
        let all = kept.kept.lock().expect("compiled lock");
        let made = all.get(&key).map(Vec::as_slice).unwrap_or_default();
        if let Some(found) = made.iter().find(|one| one.project.as_deref() == project) {
            return found.made.clone();
        }
        made.iter().filter(|one| one.made.is_ok()).cloned().collect()
    };
    // Asked again with the lock let go: answering reads the library.
    let found = same.into_iter().find(|one| answers(&one.asked));
    let (asked, made) = match found {
        Some(one) => (one.asked, one.made),
        None => {
            *kept.asking.lock().expect("compiled lock") = Some(Vec::new());
            let made = compile();
            let asked = kept.asking.lock().expect("compiled lock").take().unwrap_or_default();
            kept.compiles.fetch_add(1, Ordering::SeqCst);
            (asked.into(), made)
        }
    };
    let project = project.map(str::to_string);
    let mut all = kept.kept.lock().expect("compiled lock");
    all.entry(key).or_default().push(Made { project, asked, made: made.clone() });
    made
}
