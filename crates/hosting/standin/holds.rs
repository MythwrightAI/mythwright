//! Places a stand-in waits at that are files (`held AT WHO` in `main.rs`,
//! beside this): shared by the suites whose rigs hold stand-ins, each of
//! which names this file (`#[path]`), and beside it `let_go.rs` and
//! `listening.rs` where it uses what is in them.
//!
//! A rig's stand-ins are told a file to wait at before anybody knows
//! whether a test will hold them: where there is no such file a
//! stand-in goes on, and a test that is to hold it makes the file
//! ([`hold`]). What is in the file is where this process listens for it,
//! on this device, and a stand-in that waits is a connection there. That
//! is what a pipe with a name was, where there are such pipes, and is
//! the same on every system.
//!
//! A hold lets go one of two ways, as the pipes did. `let_go` (in the
//! file called that) waits until somebody is waiting there and lets go
//! everybody who is. And [`pass`] lets so many go, those waiting first and then the next to
//! come, so that a test can say "however often it is asked from now"
//! without waiting for anybody. Nothing waits on a clock, and however a
//! test ends, whoever was held finds its connection closed and goes.
//!
//! Only a stand-in that is still there is waiting. One whose program was
//! ended while it waited (a turn called off, a daemon's stop) has let go
//! of its connection, and is not counted: letting go waits for one that
//! is there, and a count is not spent on one that is gone, as a line
//! written down a pipe was read only by whoever was still reading.
//!
//! All of it is on the runtime of the test that makes the hold, and is
//! gone with it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;

/// How long anybody is waited for before the test has failed.
pub(super) const WITHIN: Duration = Duration::from_secs(30);

#[derive(Default)]
pub(super) struct Hold {
    pub(super) state: Mutex<State>,
    pub(super) came: Notify,
}

#[derive(Default)]
pub(super) struct State {
    /// How many of the next to come go on at once.
    passes: usize,
    /// Who waits: each a connection, let go by being dropped.
    waiting: Vec<TcpStream>,
}

impl State {
    /// Forgets whoever waited and is gone: having said who it is, a
    /// stand-in says nothing more, so a connection with anything to read
    /// is one that was closed from the other end.
    pub(super) fn there(&mut self) -> &mut Vec<TcpStream> {
        self.waiting.retain(|stream| {
            matches!(stream.try_read(&mut [0]), Err(err) if err.kind() == std::io::ErrorKind::WouldBlock)
        });
        &mut self.waiting
    }
}

/// Reads the line in which a stand-in says who it is: whether it said one.
async fn said_who(stream: &mut TcpStream) -> bool {
    let mut byte = [0];
    let read = async {
        loop {
            match stream.read(&mut byte).await {
                Ok(1) if byte[0] == b'\n' => return true,
                Ok(1) => {}
                _ => return false,
            }
        }
    };
    tokio::time::timeout(WITHIN, read).await.unwrap_or(false)
}

/// The holds this process has made, each by the file that says where it is.
static HOLDS: Mutex<BTreeMap<PathBuf, Arc<Hold>>> = Mutex::new(BTreeMap::new());

/// Somewhere on this device to listen, and the file at `path` saying
/// where: written beside and given its name, so that whoever reads it
/// reads the whole of it.
pub(super) fn listening_at(path: &Path) -> TcpListener {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let beside = path.with_extension("making");
    std::fs::write(&beside, format!("{}\n", listener.local_addr().unwrap())).unwrap();
    std::fs::rename(&beside, path).unwrap();
    TcpListener::from_std(listener).unwrap()
}

pub(super) fn made(path: &Path) -> Arc<Hold> {
    let made = HOLDS.lock().unwrap().get(path).cloned();
    made.unwrap_or_else(|| panic!("nothing made a hold at {}", path.display()))
}

/// Makes a hold at `path`: from now on a stand-in told to wait there
/// waits, until it is let go.
pub fn hold(path: &Path) {
    let listener = listening_at(path);
    let hold = Arc::new(Hold::default());
    HOLDS.lock().unwrap().insert(path.to_path_buf(), hold.clone());
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let hold = hold.clone();
            tokio::spawn(async move {
                if !said_who(&mut stream).await {
                    return;
                }
                let mut state = hold.state.lock().unwrap();
                match state.passes {
                    0 => state.waiting.push(stream),
                    // Let go as it comes: its connection is dropped here.
                    _ => state.passes -= 1,
                }
                drop(state);
                hold.came.notify_waiters();
            });
        }
    });
}

/// Lets so many go at the hold at `path`: those who are waiting, and
/// then as many of the next to come as are left of the count.
pub fn pass(path: &Path, how_many: usize) {
    let hold = made(path);
    let mut state = hold.state.lock().unwrap();
    let waiting = how_many.min(state.there().len());
    state.waiting.drain(..waiting);
    state.passes += how_many - waiting;
}
