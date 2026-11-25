//! Which builds of the client an installed copy keeps.
//!
//! The same rule the client's build keeps `ui/dist` by
//! (`ui/scripts/generations.ts`, whose figures a test holds these to): a
//! page left open on an installed client fetches parts of it days later,
//! and the copy is installed again several times a day. So an install puts
//! the new build beside the ones the copy served before, and takes away
//! only those whose place was taken a week or more before, and those past
//! the twentieth.
//!
//! What the copy served is kept beside it and not in what is served
//! (`client.kept`), a build at a time: a line `build <id> <ms>` and then
//! a line for each of its files. A list an install from before wrote is
//! its files alone, a line each, and is read as one build made now.

/// A day, in milliseconds.
pub const DAY: u64 = 24 * 60 * 60 * 1000;
/// How long a build is kept once another has taken its place, in days.
pub const KEPT_FOR_DAYS: u64 = 7;
pub const KEPT_FOR: u64 = KEPT_FOR_DAYS * DAY;
/// How many builds before the current one are kept at most.
pub const KEPT_MOST: usize = 20;

/// One build: its id, when it was made (ms since the epoch), and its files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Build {
    pub id: String,
    /// `None` where a list from before said no time.
    pub at: Option<u64>,
    pub files: Vec<String>,
}

/// The builds to keep once `made` is in place at `made.at`, newest first:
/// `made`, then each build before it whose place was taken less than
/// `KEPT_FOR` before, `KEPT_MOST` of them at most.
pub fn generations(before: &[Build], made: &Build, now: u64) -> Vec<Build> {
    let made = Build { at: Some(now), ..made.clone() };
    let mut older: Vec<Build> = before
        .iter()
        .filter(|build| build.id != made.id)
        .map(|build| Build { at: Some(build.at.unwrap_or(now)), ..build.clone() })
        .collect();
    // Stable, so builds that say the same time stay in the order listed.
    older.sort_by_key(|build| std::cmp::Reverse(build.at));
    let mut kept = vec![made];
    for build in older {
        let replaced = kept.last().and_then(|last| last.at).unwrap_or(now);
        if kept.len() > KEPT_MOST || now.saturating_sub(replaced) >= KEPT_FOR {
            break;
        }
        kept.push(build);
    }
    kept
}

/// The files among `present` that no kept build is made of.
pub fn leftovers(kept: &[Build], present: &[String]) -> Vec<String> {
    let wanted: std::collections::BTreeSet<&str> =
        kept.iter().flat_map(|build| build.files.iter().map(String::as_str)).collect();
    present.iter().filter(|file| !wanted.contains(file.as_str())).cloned().collect()
}

/// What `client.kept` says, oldest last. A list from before is one build.
pub fn read(text: &str) -> Vec<Build> {
    let mut builds: Vec<Build> = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some(said) = line.strip_prefix("build ") {
            let mut words = said.split_whitespace();
            let id = words.next().unwrap_or_default().to_string();
            let at = words.next().and_then(|at| at.parse().ok());
            builds.push(Build { id, at, files: Vec::new() });
        } else {
            match builds.last_mut() {
                Some(build) => build.files.push(line.to_string()),
                None => builds.push(Build { id: "before".into(), at: None, files: vec![line.to_string()] }),
            }
        }
    }
    builds
}

/// `client.kept` for `builds`.
pub fn write(builds: &[Build]) -> String {
    let mut text = String::new();
    for build in builds {
        text.push_str(&format!("build {} {}\n", build.id, build.at.unwrap_or_default()));
        for file in &build.files {
            text.push_str(file);
            text.push('\n');
        }
    }
    text
}
