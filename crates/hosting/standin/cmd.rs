//! How a suite has a blueprint start the stand-in (`main.rs`, beside
//! this): shared by the suites that do, each of which names this file
//! (`#[path]`) and says where its own crate's copy of the program is
//! (`crate::STANDIN`, from `CARGO_BIN_EXE_<crate>-standin`).

/// A word as a blueprint takes it whatever is in it: between single
/// quotation marks, where a backslash is a backslash (a path on Windows
/// is full of them, and between double ones each begins an escape).
pub fn word(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// What `cmd:` is given for the stand-in to do `words`: a list, as a
/// blueprint takes one.
pub fn cmd(words: &[&str]) -> String {
    let all: Vec<String> = std::iter::once(crate::STANDIN).chain(words.iter().copied()).map(word).collect();
    format!("[{}]", all.join(", "))
}
