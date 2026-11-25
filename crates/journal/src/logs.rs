//! Text logs of runs and services.
//!
//! Output is written to plain files, one per stream, with each line stamped
//! with its time and where it came from. Files rather than rows because output
//! is large, append-only and read from the end, and because a file is
//! something a person can `tail` when everything else is broken.
//!
//! A run's steps write theirs under the data folder, or in a folder the
//! factory's owner named ([`crate::letgo`]). A folder named later is
//! where they are written from then on; one written before stays where
//! it is, and is read there: a step's file is called by its run and its
//! name and by nothing of where it is, so reading looks in the folder
//! now said, then in each said before it, the latest first, then in the
//! data folder, and takes the first that holds it.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use serde::Serialize;

/// A stream is rotated when it passes this size; one previous file is kept.
const ROTATE_BYTES: u64 = 8 * 1024 * 1024;
/// Open file handles kept ready for appending.
const OPEN_FILES: usize = 64;
/// Lines longer than this are cut; a build that prints a megabyte on one line
/// should not be able to fill the disk.
const MAX_LINE: usize = 16 * 1024;
/// The kind of log a run's steps write, which alone may be written in a
/// folder of the owner's.
pub const RUNS: &str = "runs";

#[derive(Clone)]
pub struct Logs {
    root: PathBuf,
    open: Arc<Mutex<HashMap<PathBuf, File>>>,
    /// Whether a step's log begins with what the step was handed.
    handed: Arc<AtomicBool>,
    /// How many bytes were written through these logs since they were made.
    written: Arc<AtomicU64>,
    /// Where a run's steps' output is written, where the owner named a
    /// folder, and the folders named before it, the latest first.
    runs: Arc<RwLock<(Option<PathBuf>, Vec<PathBuf>)>>,
    /// How far what runs' steps printed was last let go of, for what says
    /// so of a run without asking the journal ([`crate::letgo`]).
    went: Arc<RwLock<Option<crate::letgo::Went>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LogLine {
    /// Position of the line in its file; pass the last one back to read on.
    pub line: u64,
    pub ts: i64,
    /// `o` for standard output, `e` for standard error, `s` for the system.
    pub stream: char,
    pub text: String,
}

impl Logs {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let (open, handed, written, runs) = (Arc::default(), Arc::default(), Arc::default(), Arc::default());
        Self { root: root.into(), open, handed, written, runs, went: Arc::default() }
    }

    /// How far what runs' steps printed was last let go of.
    pub fn went(&self) -> Option<crate::letgo::Went> {
        *self.went.read().expect("logs lock")
    }

    /// Has it said that it went so far.
    pub fn has_gone(&self, went: Option<crate::letgo::Went>) {
        *self.went.write().expect("logs lock") = went;
    }

    /// Has a run's steps' output written in `now` from here on (in the
    /// data folder where it is nothing), with the folders it was written
    /// in `before`, the latest first, still read.
    pub fn write_runs_in(&self, now: Option<PathBuf>, before: Vec<PathBuf>) {
        *self.runs.write().expect("logs lock") = (now, before);
        // What was open was opened where the files were written until now.
        self.open.lock().expect("logs lock").clear();
    }

    /// Every folder the files of `kind` may be in: where they are written
    /// now first, then each written in before, the latest first, then the
    /// data folder's.
    pub fn folders(&self, kind: &str) -> Vec<PathBuf> {
        let usual = self.root.join(kind);
        if kind != RUNS {
            return vec![usual];
        }
        let runs = self.runs.read().expect("logs lock");
        let mut all: Vec<PathBuf> = runs.0.iter().chain(&runs.1).map(|folder| folder.join(kind)).collect();
        if !all.contains(&usual) {
            all.push(usual);
        }
        all
    }

    /// Where the log for `id` of `kind` is: the first of its folders that
    /// holds it, or where it is written now where none does.
    fn found(&self, kind: &str, id: &str) -> PathBuf {
        let file = file_of(id);
        let folders = self.folders(kind);
        let held = folders.iter().map(|folder| folder.join(&file)).find(|path| path.exists());
        held.unwrap_or_else(|| self.path(kind, id))
    }

    /// Has every step's log begin with what the step was handed, or stop
    /// doing so: for a factory started to be looked at, since it is a line
    /// and often a file for every step there is.
    pub fn say_what_is_handed(&self, says: bool) {
        self.handed.store(says, Ordering::Relaxed);
    }

    /// Whether a step's log is to begin with what the step was handed.
    pub fn says_what_is_handed(&self) -> bool {
        self.handed.load(Ordering::Relaxed)
    }

    /// How many bytes were written through these logs since they were
    /// made, whatever became of the files: for whoever holds what is
    /// written here to so much, without looking at the disk.
    pub fn written(&self) -> u64 {
        self.written.load(Ordering::Relaxed)
    }

    /// Where the log for `id` of `kind` is written from now.
    pub fn path(&self, kind: &str, id: &str) -> PathBuf {
        let root = match kind == RUNS {
            true => self.runs.read().expect("logs lock").0.clone().unwrap_or_else(|| self.root.clone()),
            false => self.root.clone(),
        };
        root.join(kind).join(file_of(id))
    }

    /// Appends one line. Failures are swallowed: losing a log line must never
    /// fail the work that produced it.
    pub fn append(&self, kind: &str, id: &str, ts: i64, stream: char, text: &str) {
        let path = self.path(kind, id);
        let mut open = self.open.lock().expect("logs lock");
        if !open.contains_key(&path) {
            if open.len() >= OPEN_FILES {
                open.clear();
            }
            let Some(file) = open_for_append(&path) else { return };
            open.insert(path.clone(), file);
        }
        let Some(file) = open.get_mut(&path) else { return };
        let text = text.trim_end_matches(['\n', '\r']);
        let cut = if text.len() > MAX_LINE {
            let mut end = MAX_LINE;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            &text[..end]
        } else {
            text
        };
        // One write per line so concurrent appenders never interleave.
        let line = format!("{ts} {stream} {}\n", cut.replace('\n', "\\n"));
        if file.write_all(line.as_bytes()).is_err() {
            open.remove(&path);
            return;
        }
        self.written.fetch_add(line.len() as u64, Ordering::Relaxed);
        if file.metadata().is_ok_and(|meta| meta.len() > ROTATE_BYTES) {
            open.remove(&path);
            let _ = fs::rename(&path, path.with_extension("log.1"));
        }
    }

    /// Up to `limit` lines after line number `after`.
    pub fn read(&self, kind: &str, id: &str, after: u64, limit: usize) -> Vec<LogLine> {
        let Ok(file) = File::open(self.found(kind, id)) else { return Vec::new() };
        BufReader::new(file)
            .lines()
            .map_while(Result::ok)
            .enumerate()
            .skip(after as usize)
            .filter_map(|(index, raw)| parse_line(index as u64 + 1, &raw))
            .take(limit)
            .collect()
    }

    /// The last `limit` lines.
    pub fn tail(&self, kind: &str, id: &str, limit: usize) -> Vec<LogLine> {
        let path = self.found(kind, id);
        let Ok(mut file) = File::open(&path) else { return Vec::new() };
        let length = file.metadata().map_or(0, |meta| meta.len());
        // Read a generous window from the end rather than the whole file.
        let window = (limit as u64 * 256).clamp(64 * 1024, 4 * 1024 * 1024);
        let start = length.saturating_sub(window);
        let total_before = if start == 0 { 0 } else { count_lines(&path, start) };
        if file.seek(SeekFrom::Start(start)).is_err() {
            return Vec::new();
        }
        let mut lines: Vec<LogLine> = BufReader::new(file)
            .lines()
            .map_while(Result::ok)
            .enumerate()
            // The first line of a window that starts mid-file is partial.
            .skip(usize::from(start > 0))
            .filter_map(|(index, raw)| parse_line(total_before + index as u64 + 1, &raw))
            .collect();
        let extra = lines.len().saturating_sub(limit);
        lines.drain(..extra);
        lines
    }

    /// Lets go of every file kept open to be added to. A line said
    /// afterwards opens its file again. For whoever is stopping: what is
    /// kept open here is otherwise held for as long as anything holds
    /// these logs, which in a process that goes on may be for ever.
    pub fn close(&self) {
        self.open.lock().expect("logs lock").clear();
    }

    /// Removes the log for `id`, if any, wherever it is.
    pub fn remove(&self, kind: &str, id: &str) {
        for folder in self.folders(kind) {
            let path = folder.join(file_of(id));
            self.open.lock().expect("logs lock").remove(&path);
            let _ = fs::remove_file(&path);
            let _ = fs::remove_file(path.with_extension("log.1"));
        }
    }

    /// Takes away the files of a run's steps last written before
    /// `before` (milliseconds since 1970), in every folder they may be
    /// in, but for those of the runs in `keep`, which are under way: a
    /// step that waits writes nothing for as long as it waits. Gives back
    /// how many files went and how many bytes they held. On the thread it
    /// is asked from, which may wait for the disk.
    pub fn let_go_before(&self, before: i64, keep: &std::collections::HashSet<String>) -> (u64, u64) {
        let (mut files, mut bytes) = (0, 0);
        for folder in self.folders(RUNS) {
            let Ok(entries) = fs::read_dir(&folder) else { continue };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                // A step's file is `<run>.<step>.log`, or that and `.1`
                // where it was rotated; nothing else here is this one's.
                let ours = name.ends_with(".log") || name.ends_with(".log.1");
                let run = name.split('.').next().unwrap_or_default();
                if !ours || keep.contains(run) {
                    continue;
                }
                let Ok(meta) = entry.metadata() else { continue };
                let written =
                    meta.modified().ok().and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok());
                if !meta.is_file() || written.is_none_or(|at| at.as_millis() as i64 >= before) {
                    continue;
                }
                let path = entry.path();
                self.open.lock().expect("logs lock").remove(&path);
                if fs::remove_file(&path).is_ok() {
                    (files, bytes) = (files + 1, bytes + meta.len());
                }
            }
        }
        (files, bytes)
    }
}

/// What the file of `id` is called, in whichever folder: what cannot be
/// in a file's name made plain.
fn file_of(id: &str) -> String {
    let safe: String =
        id.chars().map(|c| if c.is_ascii_alphanumeric() || "-_.".contains(c) { c } else { '_' }).collect();
    format!("{safe}.log")
}

fn open_for_append(path: &Path) -> Option<File> {
    fs::create_dir_all(path.parent()?).ok()?;
    OpenOptions::new().create(true).append(true).open(path).ok()
}

/// Newlines in the first `bytes` of the file.
fn count_lines(path: &Path, bytes: u64) -> u64 {
    let Ok(file) = File::open(path) else { return 0 };
    let mut reader = BufReader::new(file);
    let mut seen = 0u64;
    let mut lines = 0u64;
    while let Ok(buffer) = reader.fill_buf() {
        if buffer.is_empty() || seen >= bytes {
            break;
        }
        let take = buffer.len().min((bytes - seen) as usize);
        lines += buffer[..take].iter().filter(|byte| **byte == b'\n').count() as u64;
        seen += take as u64;
        reader.consume(take);
    }
    lines
}

fn parse_line(line: u64, raw: &str) -> Option<LogLine> {
    let (ts, rest) = raw.split_once(' ')?;
    let mut chars = rest.chars();
    let stream = chars.next()?;
    let text = chars.as_str().strip_prefix(' ').unwrap_or("");
    Some(LogLine { line, ts: ts.parse().ok()?, stream, text: text.to_string() })
}
