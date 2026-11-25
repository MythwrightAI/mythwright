//! How deep a tree may go: what a step that starts a child is held to.
//! A function of plain values, which the runner asks where a step starts
//! a child and offers under the names it always gave
//! (`conductor::deepest`).
//!
//! A text may call itself where its step says `when`, and texts may call
//! each other in a ring; nothing else in the language says when either
//! ends. So where a step starts a child (the runner's `child::start`) the
//! instance's chain of parents is counted, and a child that would be
//! deeper than [`DEEPEST`] is not started: its step fails in a sentence
//! that names the text, how deep it would be, and which texts went round.
//!
//! The bound is well above anything that ships. The deepest that ships
//! goes is eleven, measured: four texts above a list of things that is
//! gone through a thing at a time by a text that calls itself
//! (`std/through`), a level for each thing after the first, six where
//! nothing says how many, then the thing the last level calls and what
//! that asks for. A list of n things goes n + 5 deep, so the bound leaves
//! room for a list of a hundred and twenty, and for a project's own texts
//! that nest a good deal more than anything here. A tree that goes round
//! without end makes as many instances as the bound before it is told,
//! each over once its step has failed.

/// How many instances deep a tree may be, the one at its top the first.
pub const DEEPEST: usize = 128;

/// Why a step may not start `name` under the instance whose chain of
/// blueprints up to the top of its tree is `chain` (its own first), or
/// nothing where the child would be no deeper than [`DEEPEST`]. `verb` is
/// what the step does: `call` or `spawn`.
pub fn too_deep(name: &str, verb: &str, chain: &[String]) -> Option<String> {
    let deep = chain.len() + 1;
    if deep <= DEEPEST {
        return None;
    }
    // What went round: the texts that are in the chain more than once, the
    // one there most often first.
    let mut round: Vec<(usize, &str)> = Vec::new();
    for one in chain.iter().map(String::as_str).chain([name]) {
        match round.iter_mut().find(|(_, known)| *known == one) {
            Some((times, _)) => *times += 1,
            None => round.push((1, one)),
        }
    }
    round.retain(|(times, _)| *times > 1);
    round.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
    let named: Vec<String> = round.iter().map(|(_, text)| format!("`{text}`")).collect();
    let why = match named.as_slice() {
        [] => "Nothing goes round in it: it is texts started under texts this far, which is to be cut into \
               fewer levels."
            .to_string(),
        [one] => format!(
            "{one} calls itself and the `when` of the step that does keeps holding: say one that stops holding."
        ),
        many => format!(
            "{} call each other in a ring that nothing ends: say a `when` that stops holding where one of them \
             starts the next.",
            many.join(", ")
        ),
    };
    let did = if verb == "spawn" { "begun" } else { "called" };
    Some(format!(
        "`{name}` was not {did}: it would be {deep} deep in its tree, and a tree goes {DEEPEST} deep at most. {why}"
    ))
}
