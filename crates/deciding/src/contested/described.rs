//! What a person asked about a contest is told of its entries, in words
//! they can act on from the card alone: which entry a letter
//! is, whose work it is and how much it changed, why the others were not
//! kept, and what they may answer. A person asked "Take it (A)?" did not
//! know what A was, answered `yes`, and was asked again.

use super::outcome::Entry;

/// The words that take the one entry whose work was kept.
pub const YES: &[&str] = &["yes", "y", "take", "ok", "okay"];
/// The words that take no entry and leave the card for a person.
pub const NO: &[&str] = &["no", "n", "leave"];

/// An answer as a word of [`YES`] or [`NO`] reads it: lowercase, with
/// nothing after it that a sentence puts there.
pub fn word(answer: &str) -> String {
    answer.trim().trim_end_matches(['.', '!', ',']).trim().to_lowercase()
}

/// What `git diff --stat` says it changed, in a few words: how many files,
/// and the lines added and taken away (`11 files, +634 −26`). Nothing
/// where it says nothing that is read so.
pub fn changed(stat: &str) -> String {
    let Some(last) = stat.lines().rev().find(|line| line.contains("changed")) else {
        return String::new();
    };
    let mut files = None;
    let (mut added, mut taken) = (0u64, 0u64);
    for part in last.split(',') {
        let mut words = part.split_whitespace();
        let (Some(count), Some(what)) = (words.next(), words.next()) else { continue };
        let Ok(count) = count.parse::<u64>() else { continue };
        if what.starts_with("file") {
            files = Some(count);
        } else if what.starts_with("insertion") {
            added = count;
        } else if what.starts_with("deletion") {
            taken = count;
        }
    }
    match files {
        Some(1) => format!("1 file, +{added} \u{2212}{taken}"),
        Some(files) => format!("{files} files, +{added} \u{2212}{taken}"),
        None => String::new(),
    }
}

/// The most of what an agent said that is said again in a question.
pub const BRIEF: usize = 120;

/// What an agent said, briefly: its first line that says something, cut
/// at a word where it is longer than [`BRIEF`].
pub fn brief(said: &str) -> String {
    let Some(first) = said.lines().map(str::trim).find(|line| !line.is_empty()) else {
        return String::new();
    };
    let first = first.split_whitespace().collect::<Vec<_>>().join(" ");
    if first.chars().count() <= BRIEF {
        return first;
    }
    let cut: String = first.chars().take(BRIEF).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(kept, _)| kept).trim_end_matches([',', ';', ':']);
    format!("{cut}…")
}

/// An entry's work as somebody reading only the card knows it: whose
/// work it is, the agent that did it, its branch and how much it changed,
/// each where its row says it.
pub fn work(entry: &Entry) -> String {
    let about: Vec<String> = [
        entry.agent.clone(),
        if entry.branch.is_empty() { String::new() } else { format!("branch {}", entry.branch) },
        changed(&entry.changed),
    ]
    .into_iter()
    .filter(|one| !one.is_empty())
    .collect();
    match about.is_empty() {
        true => format!("{}'s work", entry.way),
        false => format!("{}'s work ({})", entry.way, about.join(", ")),
    }
}

/// An entry whose work was kept, by its letter: `B is otter's work (…)`.
pub fn lettered(entry: &Entry) -> String {
    format!("{} is {}", entry.letter, work(entry))
}

/// Why an entry's work was not kept: what its row says it came to, and
/// what its agent said, briefly.
pub fn not_kept(entry: &Entry) -> String {
    let came = match entry.state.as_str() {
        "failed" => "it failed".to_string(),
        "unkept" => "it ended with nothing kept".to_string(),
        "working" => "it did not finish".to_string(),
        "" => "it ended".to_string(),
        other => format!("it ended {other}"),
    };
    let said = brief(&entry.said);
    match said.is_empty() {
        true => format!("{}'s was not kept ({came})", entry.way),
        false => format!("{}'s was not kept ({came}; its agent said \u{201c}{said}\u{201d})", entry.way),
    }
}

/// Where what a question names is shown: each entry is marked as named
/// in the card's tab of its tests, which the question on the card leads to.
pub const MARKED: &str = "Each is marked in the card's A/B testing tab.";

/// What a person is asked where `kept` are the entries whose work was
/// kept and `lead` says what is asked: with one kept, whether to take
/// it, answered yes or no; with several, each on a line of its own by
/// its letter, answered with the letter. Those not kept are said too,
/// with why.
pub fn question(entries: &[Entry], kept: &[usize], lead: &str) -> String {
    let others: Vec<String> =
        (0..entries.len()).filter(|at| !kept.contains(at)).map(|at| not_kept(&entries[at])).collect();
    if let [only] = kept {
        let only = &entries[*only];
        let mut said = format!("Only {}'s work could be kept: {}", only.way, lettered(only));
        for other in &others {
            said.push_str(&format!("; {other}"));
        }
        return format!("{lead}{said}. Take {}'s work? {MARKED} Answer yes or no.", only.way);
    }
    let mut lines: Vec<String> = Vec::new();
    if !lead.trim().is_empty() {
        lines.push(lead.trim().to_string());
    }
    if !lead.contains('?') {
        lines.push("Which entry is taken?".into());
    }
    lines.extend(kept.iter().map(|at| format!("{}.", lettered(&entries[*at]))));
    lines.extend(others.iter().map(|other| format!("{other}.")));
    lines.push(MARKED.into());
    lines.push("Answer with the letter of the one to take, or no to take none.".into());
    lines.join("\n")
}
