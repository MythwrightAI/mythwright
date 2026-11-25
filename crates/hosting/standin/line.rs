//! A line that has a shell start the stand-in (`main.rs`, beside this):
//! for what a user writes for their system's shell, which a test hands
//! over as text (`run:`, a health check's `cmd:`). Shared by the suites
//! that do, each of which names this file (`#[path]`) and says where its
//! own crate's copy of the program is (`crate::STANDIN`).

/// The line that has either system's shell start the stand-in to do
/// `words`. The program's file, and a word with a space in it, is
/// between double quotation marks, which both shells take off; nothing
/// else is done to a word, so they are words with nothing in them that a
/// shell reads (no `$`, `%`, `&`, `;`, `>` or quotation mark).
pub fn line(words: &[&str]) -> String {
    let said = |word: &&str| if word.contains(' ') { format!("\"{word}\"") } else { word.to_string() };
    let words: Vec<String> = words.iter().map(said).collect();
    format!("\"{}\" {}", crate::STANDIN, words.join(" "))
}
