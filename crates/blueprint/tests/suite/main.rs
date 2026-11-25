//! One test binary for the crate. Add a module here rather than a new file
//! beside this directory: every extra test target is another link step.

mod address;
mod came;
mod carried;
mod check;
mod differs;
mod events;
mod forked;
mod format;
mod graph;
mod held;
mod inside;
mod labels;
mod names;
mod outset;
mod placed;
mod robust;
mod shown;
mod summary;
mod task;
mod used;
mod vocabulary;

use blueprint::{ActionList, ActionSpec, Checked, ValueType};

/// The actions the fixtures use, shaped like the real ones.
pub fn catalog() -> ActionList {
    use ValueType::*;
    ActionList::new(vec![
        ActionSpec::new("shell", "Run a command.")
            .required("run", String, "the command line")
            .arg("cwd", String, "working directory")
            .capability("shell")
            .output("code", "exit code")
            .output("stdout", "what it printed")
            .output("stderr", "what it printed of what went wrong")
            .output("duration_ms", "how long it took"),
        ActionSpec::new("system.check", "Check free resources.")
            .arg("min_free_mem_mb", Int, "memory floor")
            .capability("system")
            .output("ok", "whether the floor is met")
            .idempotent(),
        ActionSpec::new("agent.consult", "Ask an agent to choose.")
            .required("prompt", String, "the question")
            .arg("slot", String, "which agent")
            .arg("options", List, "allowed answers")
            .capability("agents")
            .output("decision", "the answer"),
    ])
}

pub fn check(source: &str) -> Checked {
    blueprint::check(source, &catalog())
}

/// The codes of every error, in order.
pub fn error_codes(source: &str) -> Vec<&'static str> {
    check(source).errors().map(|diagnostic| diagnostic.code).collect()
}

/// The first error's message, for asserting on wording that users see.
pub fn first_error(source: &str) -> String {
    let checked = check(source);
    checked.errors().next().map(|d| d.message.clone()).unwrap_or_else(|| {
        panic!("expected an error, got: {:?}", checked.diagnostics);
    })
}
