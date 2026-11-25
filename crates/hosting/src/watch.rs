//! Being told when a file changes, instead of looking.
//!
//! The daemon follows files that other programs write: the output of
//! everything it runs (which goes to files so that those programs outlive
//! it), and the client's folder. Looking at each of them on a timer costs
//! nothing for one and is a steady load for a hundred, whether or not any of
//! them is doing anything. So the operating system is asked to say when one
//! changes (inotify on Linux, FSEvents on macOS, ReadDirectoryChangesW on
//! Windows), and what is quiet costs nothing at all.
//!
//! Where the system cannot do that (its limit on watches is used up, or the
//! folder is on a filesystem that reports nothing), whoever cannot do
//! without the file's contents may ask to fall back to looking, slowly; see
//! [`Changes::or_looking`]. [`Watch::problem`] says when that is happening.
//!
//! On Windows there is one more thing the system does not say in time:
//! that a file was written to by a program that keeps it open, which is
//! what the output of everything the daemon runs is. Whoever asked to
//! fall back to looking looks there always.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use notify::event::{AccessKind, AccessMode, EventKind};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::Notify;
use tokio::time::Instant;

/// The least time between two wake-ups for the same files. A program that
/// writes without pause is read twenty times a second, not once a line.
const SETTLE: Duration = Duration::from_millis(50);
/// How often files are looked at when the system will not say that they
/// changed and whoever follows them asked for that.
const LOOK_EVERY: Duration = Duration::from_millis(250);

/// Whether the system says that a file was written to while whoever
/// writes it keeps it open. Windows says so of the first writing, and
/// then when the file is let go of or the system has put what it kept
/// on the disk: a program that goes on printing into the file it was
/// started with was heard of when it ended, and a step's second line
/// was not read for as long as its program went on (seen there, on
/// 2026-11-19). So there, whoever cannot do without what is in the
/// files looks as well as being told ([`Changes::or_looking`]).
const TOLD_OF_A_FILE_KEPT_OPEN: bool = !cfg!(windows);

/// Who is waiting on which file.
type Waiting = Mutex<HashMap<PathBuf, Vec<Weak<Notify>>>>;

/// The system's file notifications, for any number of folders.
#[derive(Clone)]
pub struct Watch {
    inner: Arc<Inner>,
}

struct Inner {
    waiting: Arc<Waiting>,
    /// Who is waiting on anything at all in which folder.
    within: Arc<Waiting>,
    /// `None` when the system would not give one.
    watcher: Mutex<Option<RecommendedWatcher>>,
    /// The folders asked about, by the name they were given under: the name
    /// each is watched under, or `None` for one that could not be watched.
    folders: Mutex<HashMap<PathBuf, Option<PathBuf>>>,
    problem: Mutex<Option<String>>,
}

/// Passes what the system reports on to whoever waits for those files, or
/// for anything in the folders they are in.
fn tell(waiting: &Waiting, within: &Waiting, event: notify::Result<notify::Event>) {
    let (Ok(waiting), Ok(within)) = (waiting.lock(), within.lock()) else { return };
    let wake = |waiters: &Vec<Weak<Notify>>| {
        waiters.iter().filter_map(Weak::upgrade).for_each(|signal| signal.notify_one());
    };
    match event {
        Ok(event) if !event.need_rescan() => {
            // Opening or reading a file does not change it, and reports of
            // the daemon's own reads would wake it to read again, for ever.
            // The last writer letting go is different: it is how a process
            // that died without a word is noticed.
            let only_looked = matches!(event.kind, EventKind::Access(_))
                && !matches!(event.kind, EventKind::Access(AccessKind::Close(AccessMode::Write)));
            if !only_looked {
                event.paths.iter().filter_map(|path| waiting.get(path)).for_each(wake);
                // Something in a folder, or the folder itself going.
                let folders = event.paths.iter().flat_map(|path| [path.parent(), Some(path.as_path())]);
                folders.flatten().filter_map(|folder| within.get(folder)).for_each(wake);
            }
        }
        // Reports were lost, or the watch itself failed: everyone looks once.
        _ => waiting.values().chain(within.values()).for_each(wake),
    }
}

impl Default for Watch {
    fn default() -> Self {
        Self::new()
    }
}

impl Watch {
    pub fn new() -> Watch {
        let (waiting, within): (Arc<Waiting>, Arc<Waiting>) = Default::default();
        let (told, told_within) = (waiting.clone(), within.clone());
        let heard = move |event| tell(&told, &told_within, event);
        let (watcher, problem) = match notify::recommended_watcher(heard) {
            Ok(watcher) => (Some(watcher), None),
            Err(err) => (None, Some(format!("the system gave no way to watch files: {err}"))),
        };
        Watch {
            inner: Arc::new(Inner {
                waiting,
                within,
                watcher: Mutex::new(watcher),
                folders: Mutex::default(),
                problem: Mutex::new(problem),
            }),
        }
    }

    /// Gives the system's notifications back, for good. A daemon that has
    /// stopped follows nothing; kept, each one that ever ran in a process
    /// would go on holding what the system gives a user only so many of,
    /// and the next to ask would be told there were none left.
    pub fn close(&self) {
        *self.inner.watcher.lock().expect("watch lock") = None;
        self.inner.folders.lock().expect("watch lock").clear();
        *self.inner.problem.lock().expect("watch lock") =
            Some("nothing is watched any more: what this watch was for has stopped".to_string());
    }

    /// Why some files are not being watched, if any are not.
    pub fn problem(&self) -> Option<String> {
        self.inner.problem.lock().expect("watch lock").clone()
    }

    /// Starts watching a folder (not the folders inside it) and answers the
    /// name the system knows it by, or `None` if it cannot be watched.
    pub fn folder(&self, folder: &Path) -> Option<PathBuf> {
        let mut folders = self.inner.folders.lock().expect("watch lock");
        if let Some(known) = folders.get(folder) {
            return known.clone();
        }
        // The system reports files by their real path, which is not always
        // the one given (a symbolic link on the way, a `..`). The folder is
        // watched under its real name so that the two can be matched.
        let real = kernel::whole::whole(folder).unwrap_or_else(|_| folder.to_path_buf());
        let watched = match self.inner.watcher.lock().expect("watch lock").as_mut() {
            Some(watcher) => match watcher.watch(&real, RecursiveMode::NonRecursive) {
                Ok(()) => Some(real),
                Err(err) => {
                    let problem = format!("{} cannot be watched: {err}", folder.display());
                    *self.inner.problem.lock().expect("watch lock") = Some(problem);
                    None
                }
            },
            None => None,
        };
        folders.insert(folder.to_path_buf(), watched.clone());
        watched
    }

    /// Watches a folder afresh. For one that may have been removed and made
    /// again: the system goes on watching the one that is gone.
    pub fn refresh(&self, folder: &Path) {
        let known = self.inner.folders.lock().expect("watch lock").remove(folder);
        if let Some(Some(real)) = known
            && let Some(watcher) = self.inner.watcher.lock().expect("watch lock").as_mut()
        {
            let _ = watcher.unwatch(&real);
        }
        self.folder(folder);
    }

    /// How many are told of what changes in `folder` ([`Watch::within`]),
    /// by the name it was asked under: for a test that holds a folder to
    /// be followed once, whatever it is followed for.
    pub fn followers(&self, folder: &Path) -> usize {
        let Some(Some(real)) = self.inner.folders.lock().expect("watch lock").get(folder).cloned() else {
            return 0;
        };
        let within = self.inner.within.lock().expect("watch lock");
        within.get(&real).map_or(0, |waiting| waiting.iter().filter(|one| one.strong_count() > 0).count())
    }

    /// Asks to be told when any of these files changes: is made, written to,
    /// let go of by whoever was writing it, moved into place, or removed. A
    /// file need not exist yet; its folder must.
    pub fn files(&self, paths: &[&Path]) -> Changes {
        let signal = Arc::new(Notify::new());
        let mut keys = Vec::new();
        let mut told = !paths.is_empty();
        for path in paths {
            match (path.parent().and_then(|folder| self.folder(folder)), path.file_name()) {
                (Some(folder), Some(name)) => keys.push(folder.join(name)),
                _ => told = false,
            }
        }
        let mut waiting = self.inner.waiting.lock().expect("watch lock");
        for key in &keys {
            waiting.entry(key.clone()).or_default().push(Arc::downgrade(&signal));
        }
        Changes {
            signal,
            told,
            look: false,
            not_before: Instant::now(),
            keys,
            waiting: self.inner.waiting.clone(),
        }
    }

    /// Asks to be told when anything in any of these folders changes: a
    /// file made there, written, moved in or out, or removed, or the
    /// folder itself going. Not what is in the folders inside them: each
    /// of those that matters is named too. A folder that is not there is
    /// not followed.
    pub fn within(&self, folders: &[&Path]) -> Changes {
        let signal = Arc::new(Notify::new());
        let mut keys = Vec::new();
        let mut told = !folders.is_empty();
        for folder in folders {
            match self.folder(folder) {
                Some(real) => keys.push(real),
                None => told = false,
            }
        }
        let mut waiting = self.inner.within.lock().expect("watch lock");
        for key in &keys {
            waiting.entry(key.clone()).or_default().push(Arc::downgrade(&signal));
        }
        Changes {
            signal,
            told,
            look: false,
            not_before: Instant::now(),
            keys,
            waiting: self.inner.within.clone(),
        }
    }
}

/// Changes to a set of files, as they happen.
pub struct Changes {
    signal: Arc<Notify>,
    told: bool,
    look: bool,
    not_before: Instant,
    keys: Vec<PathBuf>,
    waiting: Arc<Waiting>,
}

impl Changes {
    /// Whether the system is saying when these files change.
    pub fn told(&self) -> bool {
        self.told
    }

    /// For whoever cannot do without what is in the files: if the system
    /// will not say when they change, [`Changes::next`] also returns a few
    /// times a second, so that they are looked at instead. On Windows it
    /// always does: what the system says there of a file that is kept
    /// open comes when the file is let go of.
    pub fn or_looking(mut self) -> Self {
        self.look = true;
        self
    }

    /// Waits until one of the files has changed since this last returned.
    ///
    /// A change made while nobody was waiting is not lost, and neither is
    /// one made while a call to this was abandoned half-way. Anything that
    /// happened before the files were asked about is: read once after
    /// asking, then wait.
    ///
    /// This is a reason to look, not a promise that there is something to
    /// see. A file is reported as let go of when the *last* holder of it
    /// closes it, and a process being started holds, for a moment, a copy of
    /// everything its parent had open; so that report can arrive late and
    /// about nothing.
    pub async fn next(&mut self) {
        tokio::time::sleep_until(self.not_before).await;
        if !self.look || (self.told && TOLD_OF_A_FILE_KEPT_OPEN) {
            self.signal.notified().await;
        } else {
            tokio::select! {
                () = self.signal.notified() => {}
                () = tokio::time::sleep(LOOK_EVERY) => {}
            }
        }
        self.not_before = Instant::now() + SETTLE;
    }
}

impl Drop for Changes {
    fn drop(&mut self) {
        let Ok(mut waiting) = self.waiting.lock() else { return };
        for key in &self.keys {
            if let Some(waiters) = waiting.get_mut(key) {
                waiters
                    .retain(|other| other.upgrade().is_some_and(|other| !Arc::ptr_eq(&other, &self.signal)));
                if waiters.is_empty() {
                    waiting.remove(key);
                }
            }
        }
    }
}
