//! The stand-in (`main.rs`, beside this) as a program and its arguments,
//! for whatever takes those as a list (an agent's program in a rig's
//! configuration): shared by the suites that say so, each of which names
//! this file (`#[path]`) and says where its own crate's copy of the
//! program is (`crate::STANDIN`).

/// The stand-in doing `words`.
pub fn argv(words: &[&str]) -> Vec<String> {
    std::iter::once(crate::STANDIN).chain(words.iter().copied()).map(str::to_string).collect()
}
