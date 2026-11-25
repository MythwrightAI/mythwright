//! Being told that a file changed.
//!
//! These use the system's own notifications, as the daemon does. Where a
//! test must show that something was *not* reported, it does not wait to see:
//! it makes a second change, waits for that to be reported, and then asks
//! whether the first is waiting too. The system reports in order.

use std::path::{Path, PathBuf};
use std::time::Duration;

use hosting::{Changes, Watch};

use crate::soon;

fn folder(label: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("watch-{label}-{}", &kernel::id::secret()[..12]));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Makes a file without this process ever holding it open for writing.
///
/// These tests share a process with others that start programs, and a
/// program being started holds a copy of every file its parent has open until
/// it is under way. A file written from here could so be reported as "let go
/// of" some moments after it was closed, by which time a test is listening.
///
/// So it is written by another program: the stand-in, which writes the
/// line it is told into the file it is told.
fn written_elsewhere(path: &Path, line: &str) {
    let done = std::process::Command::new(crate::STANDIN).arg("write").arg(path).arg(line).status().unwrap();
    assert!(done.success());
}

fn append(path: &Path, text: &str) {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path).unwrap();
    file.write_all(text.as_bytes()).unwrap();
}

/// A watch, where the system gives this process one. It gives one user
/// only so many, and the suites run beside this one start many nodes at
/// once: where it gave none there is nothing of following files to hold,
/// and a test says so in a line and waits for nothing that will not be
/// said.
fn given() -> Option<Watch> {
    let watch = Watch::new();
    match watch.problem() {
        Some(problem) if problem.starts_with("the system gave no way to watch files") => {
            eprintln!("{problem}: nothing is held here of following files");
            None
        }
        _ => Some(watch),
    }
}

/// Whether a change is waiting to be heard right now.
async fn heard(changes: &mut Changes) -> bool {
    tokio::time::timeout(Duration::ZERO, changes.next()).await.is_ok()
}

#[tokio::test]
async fn a_file_being_written_to_is_heard_of() {
    let Some(watch) = given() else { return };
    let dir = folder("written");
    let mut changes = watch.files(&[&dir.join("out")]);
    assert!(changes.told(), "{:?}", watch.problem());

    append(&dir.join("out"), "one\n");
    soon("the first write", changes.next()).await;
    append(&dir.join("out"), "two\n");
    soon("the second write", changes.next()).await;
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn a_change_made_while_nobody_was_waiting_is_not_lost() {
    let Some(watch) = given() else { return };
    let dir = folder("unattended");
    let mut changes = watch.files(&[&dir.join("out")]);
    let mut marker = watch.files(&[&dir.join("marker")]);
    append(&dir.join("out"), "written before anyone asked\n");
    append(&dir.join("marker"), "x");
    soon("the marker", marker.next()).await;
    assert!(heard(&mut changes).await);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn reading_a_file_is_not_a_change_to_it() {
    let Some(watch) = given() else { return };
    // The daemon reads the files it follows. If its own reading counted, it
    // would wake itself to read again for ever.
    let dir = folder("read");
    written_elsewhere(&dir.join("out"), "something to read");
    let mut changes = watch.files(&[&dir.join("out")]);
    let mut marker = watch.files(&[&dir.join("marker")]);

    assert_eq!(std::fs::read_to_string(dir.join("out")).unwrap(), "something to read\n");
    let _ = std::fs::metadata(dir.join("out")).unwrap();
    append(&dir.join("marker"), "x");
    soon("the marker", marker.next()).await;
    assert!(!heard(&mut changes).await, "a read was taken for a change");
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn a_file_moved_into_place_is_heard_of_and_its_neighbours_are_not() {
    let Some(watch) = given() else { return };
    let dir = folder("moved");
    written_elsewhere(&dir.join("index.html"), "old");
    let mut page = watch.files(&[&dir.join("index.html")]);
    let mut marker = watch.files(&[&dir.join("marker")]);

    // The half-written copy is somebody else's file.
    written_elsewhere(&dir.join("index.html.new"), "new");
    append(&dir.join("marker"), "x");
    soon("the marker", marker.next()).await;
    assert!(!heard(&mut page).await, "a neighbouring file was taken for the page");

    std::fs::rename(dir.join("index.html.new"), dir.join("index.html")).unwrap();
    soon("the page being replaced", page.next()).await;
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn nothing_is_heard_once_whoever_asked_has_gone() {
    let Some(watch) = given() else { return };
    let dir = folder("gone");
    let first = watch.files(&[&dir.join("out")]);
    drop(first);
    // Asking again after that starts clean, and is heard as usual.
    let mut again = watch.files(&[&dir.join("out")]);
    append(&dir.join("out"), "x");
    soon("a write", again.next()).await;
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn files_that_cannot_be_watched_are_looked_at_instead_when_that_is_asked_for() {
    // Held where the system gave this process no watch at all, too: then
    // nothing can be watched, and that is the problem there is.
    let watch = Watch::new();
    let dir = folder("unwatchable");
    let missing = dir.join("not").join("there");
    let mut changes = watch.files(&[&missing.join("out")]).or_looking();
    assert!(!changes.told());
    let why = watch.problem().unwrap_or_default();
    assert!(
        why.contains("cannot be watched") || why.starts_with("the system gave no way to watch files"),
        "{why}"
    );
    // Nothing will ever be reported, and this still comes back.
    soon("a look", changes.next()).await;
    soon("another look", changes.next()).await;
    let _ = std::fs::remove_dir_all(dir);
}

/// Linux's alone: only its notifications say that whoever was writing a
/// file let go of it, having written nothing.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn a_writer_letting_go_of_a_file_is_heard_of() {
    let Some(watch) = given() else { return };
    // How a process that was killed outright is noticed: it wrote nothing,
    // but the system closed its output for it.
    let dir = folder("closed");
    let file = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("out")).unwrap();
    let mut changes = watch.files(&[&dir.join("out")]);
    drop(file);
    soon("the file being let go of", changes.next()).await;
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn anything_made_changed_or_removed_in_a_folder_that_is_followed_is_heard_of_and_nothing_under_it_is() {
    let Some(watch) = given() else { return };
    let dir = folder("within");
    std::fs::create_dir_all(dir.join("inner")).unwrap();
    let mut changes = watch.within(&[&dir]);
    assert!(changes.told(), "{:?}", watch.problem());

    // A file nobody named beforehand.
    append(&dir.join("new.md"), "one\n");
    soon("a file made there", changes.next()).await;
    append(&dir.join("new.md"), "two\n");
    soon("the same file written to", changes.next()).await;
    std::fs::remove_file(dir.join("new.md")).unwrap();
    soon("the file removed", changes.next()).await;

    // What is in a folder inside it is that folder's, and is heard of by
    // whoever follows that one.
    let mut inner = watch.within(&[&dir.join("inner")]);
    written_elsewhere(&dir.join("inner").join("deep.md"), "deep");
    soon("a file in the folder inside", inner.next()).await;
    assert!(!heard(&mut changes).await, "the folder above was told of a file in the one inside it");
    let _ = std::fs::remove_dir_all(dir);
}

/// How many of this process's ways of being told of changes are following
/// `dir`, as the system itself says (Linux lists what each one follows).
#[cfg(target_os = "linux")]
fn following(dir: &Path) -> usize {
    use std::os::unix::fs::MetadataExt;
    let folder = format!("ino:{:x} ", std::fs::metadata(dir).unwrap().ino());
    std::fs::read_dir("/proc/self/fdinfo")
        .unwrap()
        .flatten()
        .filter(|entry| {
            std::fs::read_to_string(entry.path()).is_ok_and(|said| {
                said.lines().any(|line| line.starts_with("inotify") && line.contains(&folder))
            })
        })
        .count()
}

/// Linux's alone: only there does the system list what each of a
/// process's ways of being told is following (`following`, above).
#[cfg(target_os = "linux")]
#[tokio::test]
async fn a_watch_that_is_closed_gives_the_systems_notifications_back_and_follows_nothing_more() {
    let Some(watch) = given() else { return };
    let dir = folder("closed");
    let changes = watch.files(&[&dir.join("out")]);
    assert!(changes.told(), "{:?}", watch.problem());
    assert_eq!(following(&dir), 1);

    watch.close();

    // Given back by the thread that held them, a moment after it is told.
    soon("the system's notifications to be given back", async {
        while following(&dir) > 0 {
            tokio::task::yield_now().await;
        }
    })
    .await;
    // And whoever asks it to follow something now is told that it will not.
    assert!(!watch.files(&[&dir.join("other")]).told());
    assert!(watch.problem().unwrap().contains("nothing is watched any more"));
    let _ = std::fs::remove_dir_all(dir);
}
