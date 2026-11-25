//! One test binary for the crate; add modules here.
//!
//! A sandbox made as the daemon makes one, with nothing of a daemon:
//! the blueprints that ship as they are compiled in, a few made-up
//! actions that "reach outside" described and never bound, and a folder
//! of the test's own. Nothing waits on the clock on the wall: an act is
//! answered when what came of it is at rest. Every wait for the sandbox
//! is bounded all the same ([`in_time`]), and one that runs out says what
//! the sandbox's thread and each of its databases were at.

use std::time::Duration;

mod ends;
mod held;
mod kinds;
mod limits;
mod shapes;

use std::path::PathBuf;
use std::sync::Arc;

use blueprint::{ActionSpec, ValueType as T};
use sandbox::{Act, Answers, Limits, Made, Sandbox, Seen};
use serde_json::{Value, json};

/// The actions this suite's device "has" beside the ones a sandbox has
/// as they are: each described, as the daemon's are, and bound to
/// nothing. A sandbox stands in for them.
pub fn described() -> Vec<ActionSpec> {
    vec![
        ActionSpec::new("outside.run", "Run a program on the device.")
            .arg("cmd", T::List, "the program and its arguments")
            .runs("cmd")
            .output("code", "what it ended with")
            .stands(json!({ "code": 0, "stdout": "" })),
        ActionSpec::new("outside.ask", "Ask somebody outside the factory.")
            .required("prompt", T::String, "what to ask")
            .output("reply", "what they said"),
    ]
}

/// A folder of this test's own, which nothing is in yet.
pub fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("sandbox-{}", &kernel::id::secret()[..12]))
}

/// Held by every test that has a sandbox, for as long as it has one,
/// and wholly by the one that counts what sandboxes leave.
///
/// A test holds it once however many sandboxes it has ([`beside`]). It
/// is fair: once a test waits to hold it wholly, nobody is let in beside
/// those already in. A test that took it again for its second sandbox,
/// while holding it for its first, waited behind the one that waits to
/// be alone, which waited for the first sandbox to be over: neither went
/// on, and the suite stood still for an hour.
pub static ALONE: std::sync::LazyLock<Arc<tokio::sync::RwLock<()>>> = std::sync::LazyLock::new(Arc::default);

/// How long anything a test asks of a sandbox may take before it is
/// taken to wait for what will never come: far more than any act here
/// takes on a crowded machine.
pub const ANSWERED: Duration = Duration::from_secs(120);

/// `work`, which waits on a sandbox, or a failure that says what `what`
/// was and what the sandbox was at when it ran out of time.
pub async fn in_time<T>(what: &str, seen: &Seen, work: impl Future<Output = T>) -> T {
    match tokio::time::timeout(ANSWERED, work).await {
        Ok(done) => done,
        Err(_) => panic!("timed out waiting for {what}: {}", seen.said()),
    }
}

/// The lock every test with a sandbox holds, taken in time or failing
/// with what it waited for.
pub async fn held(alone: bool) -> Held {
    let lock = ALONE.clone();
    let taken = async move {
        match alone {
            true => Held::Alone(lock.write_owned().await),
            false => Held::Beside(lock.read_owned().await),
        }
    };
    match tokio::time::timeout(ANSWERED, taken).await {
        Ok(held) => held,
        Err(_) => panic!(
            "timed out waiting for the other tests' sandboxes to be over; these tests had sandboxes then: {:?}",
            HOLDERS.lock().expect("holders lock")
        ),
    }
}

/// Which tests hold [`ALONE`] beside the others, by their threads' names:
/// said by whoever waits for it and gives up.
static HOLDERS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// [`ALONE`] held beside the others by one test, which is named among
/// [`HOLDERS`] for as long as it holds it.
pub struct Beside {
    _held: Held,
    test: String,
}

impl Drop for Beside {
    fn drop(&mut self) {
        let mut holders = HOLDERS.lock().expect("holders lock");
        if let Some(at) = holders.iter().position(|test| *test == self.test) {
            holders.remove(at);
        }
    }
}

pub enum Held {
    Beside(tokio::sync::OwnedRwLockReadGuard<()>),
    Alone(tokio::sync::OwnedRwLockWriteGuard<()>),
}

thread_local! {
    /// What this test holds of [`ALONE`] beside the others, for as long
    /// as any sandbox of its own is there: each test is a thread of its
    /// own, and its sandboxes are made on it.
    static BESIDE: std::cell::RefCell<std::rc::Weak<Beside>> = const { std::cell::RefCell::new(std::rc::Weak::new()) };
}

/// [`ALONE`] held beside the other tests, by this test: taken where this
/// test holds it for no sandbox yet, and the hold it has where it does.
pub async fn beside() -> std::rc::Rc<Beside> {
    if let Some(had) = BESIDE.with(|beside| beside.borrow().upgrade()) {
        return had;
    }
    let held = held(false).await;
    let test = std::thread::current().name().unwrap_or("a test").to_string();
    HOLDERS.lock().expect("holders lock").push(test.clone());
    let had = std::rc::Rc::new(Beside { _held: held, test });
    BESIDE.with(|beside| *beside.borrow_mut() = std::rc::Rc::downgrade(&had));
    had
}

pub struct Rig {
    pub sandbox: Sandbox,
    pub seen: Seen,
    pub folder: PathBuf,
    _held: Option<std::rc::Rc<Beside>>,
}

/// What a sandbox of this suite is made from.
pub fn made(source: &str, folder: PathBuf) -> Made {
    let name = blueprint::check(source, &blueprint::NoCatalog)
        .blueprint
        .map(|read| read.name)
        .unwrap_or_else(|| "tried".to_string());
    Made {
        id: kernel::id::new("sandbox", 0),
        name,
        source: source.to_string(),
        own: Vec::new(),
        ships: Arc::new(conductor::standard()),
        actions: described(),
        events: Vec::new(),
        knows_events: false,
        folder,
        now_ms: 1_800_000_000_000,
        answers: Answers::Asked,
        told: Default::default(),
        limits: Limits::default(),
    }
}

impl Rig {
    pub async fn new(source: &str) -> Rig {
        Rig::with(source, Answers::Asked, Limits::default(), &[]).await
    }

    pub async fn with(source: &str, answers: Answers, limits: Limits, own: &[(&str, &str)]) -> Rig {
        let held = beside().await;
        let mut rig = Rig::made(source, answers, limits, own).await;
        rig._held = Some(held);
        rig
    }

    /// For the test that holds every other out itself.
    pub async fn alone(source: &str) -> Rig {
        Rig::made(source, Answers::Asked, Limits::default(), &[]).await
    }

    async fn made(source: &str, answers: Answers, limits: Limits, own: &[(&str, &str)]) -> Rig {
        let folder = folder();
        let own = own.iter().map(|(name, text)| (name.to_string(), text.to_string())).collect();
        let made = Made { own, answers, limits, ..made(source, folder.clone()) };
        let seen = Seen::default();
        let sandbox = in_time("a sandbox to be made", &seen, Sandbox::make_seen(made, seen.clone())).await;
        Rig { sandbox: sandbox.expect("a sandbox is made"), seen, folder, _held: None }
    }

    pub async fn act(&self, act: Value) -> Value {
        let act: Act = serde_json::from_value(act).expect("an act");
        let what = format!("the act {act:?} to be answered");
        in_time(&what, &self.seen, self.sandbox.act(act)).await.expect("the act is done")
    }

    pub async fn refused(&self, act: Value) -> String {
        let act: Act = serde_json::from_value(act).expect("an act");
        let what = format!("the act {act:?} to be refused");
        in_time(&what, &self.seen, self.sandbox.act(act)).await.expect_err("the act is refused").message
    }

    pub async fn apply(&self) -> Value {
        self.act(json!({ "apply": {} })).await
    }

    pub async fn read(&self) -> Value {
        in_time("the sandbox to be read", &self.seen, self.sandbox.read()).await.expect("the sandbox is read")
    }

    /// Ends the sandbox, in time.
    pub async fn end(&self) {
        in_time("the sandbox to end", &self.seen, self.sandbox.end()).await
    }
}

/// The lines of a timeline that are steps, each as `step: status`.
pub fn steps(shown: &Value) -> Vec<String> {
    let lines = shown["timeline"].as_array().cloned().unwrap_or_default();
    lines
        .iter()
        .filter(|line| line["what"] == "step")
        .map(|line| format!("{}: {}", line["step"].as_str().unwrap(), line["status"].as_str().unwrap()))
        .collect()
}

/// The topics said, in the order they were.
pub fn topics(shown: &Value) -> Vec<String> {
    let said = shown["happened"].as_array().cloned().unwrap_or_default();
    said.iter().map(|event| event["topic"].as_str().unwrap().to_string()).collect()
}
