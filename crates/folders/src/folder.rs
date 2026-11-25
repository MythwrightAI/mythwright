//! A factory's data folder, and who has it.
//!
//! A factory is one daemon on one data folder. Who has the folder is said
//! by three files in it that are no part of what the factory keeps:
//!
//! - `lock`, held by the daemon for as long as it runs, and by whatever
//!   works on the files of a factory that is stopped ([`crate::repack`]);
//! - `serving`, where the daemon that holds the lock listens: written
//!   when it has the folder, taken away as it stops listening. It means
//!   something only while the lock is held: a daemon that was killed
//!   leaves it behind, and a daemon of an earlier program never wrote it;
//! - `keeper`, held by what keeps a daemon running (`up`) for as long as
//!   it does.
//!
//! So a folder that is held and says where is being served there; one
//! that is held and says nothing has a daemon that is stopping, or a
//! command at work on its files; and one whose keeper is held has a
//! daemon that will be started again whatever becomes of the one there
//! is. A second daemon, or a second keeper, is told which of those it
//! met, in a sentence that says what to do: before this a keeper whose
//! factory was served by another went on starting a daemon that could
//! not listen, every half minute for as long as it was left, and took
//! the address when the other was stopped on purpose.

use std::fs::{File, OpenOptions, TryLockError};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use kernel::{Error, Result};

/// Held by whoever has the folder: a daemon, or a command at work on a
/// stopped factory's files.
pub const LOCK: &str = "lock";
/// Where the daemon that holds the folder listens.
pub const SERVING: &str = "serving";
/// Held by what keeps a daemon running.
pub const KEEPER: &str = "keeper";

/// Whether something has the folder just now: a daemon, or a command at
/// work on its files. A lock outlives its holder for an instant in any
/// child that was being started as the holder exited, so whoever decides
/// by this that a daemon really is running has waited first.
pub fn is_held(data: &Path) -> bool {
    // Never made here: a folder no daemon was started on has none.
    let Ok(lock) = OpenOptions::new().read(true).open(data.join(LOCK)) else { return false };
    matches!(lock.try_lock(), Err(TryLockError::WouldBlock))
}

/// Where the daemon that has the folder listens: `None` where nothing
/// has the folder, and where what has it says nothing (a daemon that is
/// stopping, a command at work on the files, a daemon of an earlier
/// program).
pub fn served(data: &Path) -> Option<SocketAddr> {
    if !is_held(data) {
        return None;
    }
    // One read in the moment it is written says nothing, as one that is
    // not there does: whoever asked asks again.
    std::fs::read_to_string(data.join(SERVING)).ok()?.trim().parse().ok()
}

/// The folder, had by a daemon for as long as this is kept.
pub struct Had {
    data: PathBuf,
    _lock: File,
}

impl Had {
    /// Says where the daemon that has the folder listens.
    pub fn serves(data: &Path, lock: File, addr: SocketAddr) -> Result<Had> {
        let file = data.join(SERVING);
        std::fs::write(&file, format!("{addr}\n")).map_err(|err| Error::from(err).context(file.display()))?;
        Ok(Had { data: data.to_path_buf(), _lock: lock })
    }

    /// Says nothing more of where: the daemon is about to stop listening,
    /// and has the folder until it has put its files away.
    pub fn serves_no_more(&self) {
        forget(&self.data);
    }
}

/// Where a daemon of this factory was served when it ended without
/// stopping, read where nothing has the folder: a daemon that stops takes
/// it away first, and one that was killed leaves it.
pub fn served_when_it_ended(data: &Path) -> Option<SocketAddr> {
    std::fs::read_to_string(data.join(SERVING)).ok()?.trim().parse().ok()
}

/// Takes away what a daemon said of where it listens. Asked by that
/// daemon as it stops, and by whatever else comes to have the folder: a
/// daemon that was killed left it, and held by another it would be read
/// as that one's.
pub(crate) fn forget(data: &Path) {
    let _ = std::fs::remove_file(data.join(SERVING));
}

/// Which factory a folder is, for a sentence: by its profile where
/// it has one, and always by where its data is.
fn called(data: &Path) -> String {
    match crate::profile::Stamp::read(data) {
        Ok(Some(stamp)) => format!("the `{}` factory at {}", stamp.profile, data.display()),
        _ => format!("the factory at {}", data.display()),
    }
}

/// How the daemon at `at` is stopped from a terminal. By its address
/// and never by nothing: a command told neither an address nor a name
/// goes to wherever a factory with no name usually listens, which may be
/// another.
fn stops(at: SocketAddr) -> String {
    format!("`{} --url {} down` stops it", brand::CLI, crate::own_url(at))
}

/// Why a second daemon is not started on a folder that is being served.
/// It is so however often it is asked, which is what its kind says.
pub fn running(data: &Path, at: SocketAddr) -> Error {
    Error::conflict(format!(
        "{} is already running, at {}: a factory has one daemon, and a second would drive the same fleet. \
         Nothing was started. {}",
        called(data),
        crate::own_url(at),
        stops(at)
    ))
}

/// Why a daemon is not started on a folder that something has and that
/// says nothing of being served. That passes, which is what its kind
/// says: whatever keeps a daemon running asks again.
pub fn busy(data: &Path) -> Error {
    let cli = brand::CLI;
    Error::unavailable(format!(
        "{} is held and is not being served: a daemon of it is still stopping, or a command that works on \
         a stopped factory's files is at work there. Nothing was started. Start it when that is over; \
         `{cli} up` goes on asking by itself",
        called(data)
    ))
}

/// Why a daemon is not started at an address that is in use, while
/// something has its folder: by what the folder says of where it is
/// served, and where it says nothing by the address itself.
pub fn in_the_way(data: &Path, listen: SocketAddr) -> Error {
    match served(data) {
        Some(at) => running(data, at),
        None => Error::conflict(format!(
            "{} is held by a daemon that does not say where it listens, and {listen} is in use, most \
             likely by it. Nothing was started. `{cli} --url {url} status` says what answers there, and \
             `{cli} --url {url} down` stops it if it is this factory",
            called(data),
            cli = brand::CLI,
            url = crate::own_url(listen),
        )),
    }
}

/// Why a daemon is not started at an address something else has, with
/// nothing of this factory running. `kept` is what is said besides where
/// the address is the one the factory listened at last, and `ended`
/// where a daemon of it was served there when it ended without stopping.
pub fn taken(data: &Path, listen: SocketAddr, why: &std::io::Error, kept: bool, ended: bool) -> Error {
    let cli = brand::CLI;
    let said = match (kept, ended) {
        (true, _) => "; that is where this factory listened last, and it listens there again so that its \
                      address stays the same. Free the address, or start it once with another (`--listen`)"
            .to_string(),
        (false, true) => format!(
            "; a daemon of {} was served there and ended without stopping, and the address was waited for \
             as long as a program that daemon was starting could have it: what has it now is something \
             else. Nothing was started. Free the address, or start it at another (`--listen \
             127.0.0.1:<port>`); `{cli} --url {} status` says whether that is another factory",
            called(data),
            crate::own_url(listen),
        ),
        (false, false) => format!(
            ". Something that is not this factory has the address: no daemon of {} is running. Nothing \
             was started. Start it at another address (`--listen 127.0.0.1:<port>`), or stop what has \
             this one; `{cli} --url {} status` says whether that is another factory",
            called(data),
            crate::own_url(listen),
        ),
    };
    Error::conflict(format!("cannot listen on {listen}: {why}{said}"))
}

/// What keeps a daemon running, known to the folder for as long as this
/// is kept.
pub struct Keeping {
    _lock: File,
}

/// What somebody who would keep a factory's daemon running is told of
/// its folder.
pub enum Keep {
    /// Nobody else keeps it: this one does, from here.
    Kept(Keeping),
    /// No daemon has had this folder yet, so there is nothing to keep
    /// and nobody to be in the way of. Nothing is written into a folder
    /// no daemon has held: it may be one nothing can be started on.
    Nowhere,
    /// Another keeps it, and its daemon is up: why this one is not
    /// needed, and how the factory is stopped.
    Running(String),
    /// Another keeps it, and no daemon of it is up just now: it is
    /// stopping, with what keeps it about to end, or it is about to be
    /// started again. Asked again, it is one of the others.
    Waiting(String),
}

/// Whether something keeps the daemon of the factory at `data` running
/// (`up`): one that is stopped is then started again. Looked at without
/// being taken: one asked in the very moment a keeper comes to take it is
/// told to wait a moment by what it met, and asks again.
pub fn kept(data: &Path) -> bool {
    // Never made here: a folder no keeper had has none.
    let Ok(lock) = OpenOptions::new().write(true).open(data.join(KEEPER)) else { return false };
    matches!(lock.try_lock(), Err(TryLockError::WouldBlock))
}

/// Asks to be what keeps the daemon of the factory at `data` running.
/// It is asked once here; whoever is told to wait asks again.
pub fn keep(data: &Path) -> Result<Keep> {
    if !data.join(LOCK).is_file() {
        return Ok(Keep::Nowhere);
    }
    let file = data.join(KEEPER);
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&file)
        .map_err(|err| Error::from(err).context(file.display()))?;
    match lock.try_lock() {
        Ok(()) => Ok(Keep::Kept(Keeping { _lock: lock })),
        Err(TryLockError::WouldBlock) => Ok(match served(data) {
            Some(at) => Keep::Running(format!(
                "{} is already running, at {}, and is kept running by another `{} up`: one is enough, \
                 and a second would start a daemon that cannot listen for as long as it was left. Nothing \
                 was started. {}, and what keeps it running ends with it",
                called(data),
                crate::own_url(at),
                brand::CLI,
                stops(at)
            )),
            None => Keep::Waiting(format!(
                "{} has another `{} up` keeping its daemon running, and no daemon of it is up just now: \
                 it is stopping, or about to be started again. Waiting to see which: this one goes on \
                 when the other has ended, and ends when the other's daemon is up",
                called(data),
                brand::CLI
            )),
        }),
        Err(TryLockError::Error(err)) => Err(Error::from(err).context(file.display())),
    }
}
