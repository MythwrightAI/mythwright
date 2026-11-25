//! The ports the factory gives out.
//!
//! A service that names no port is given one, and is told it before its
//! program is there to take it: the program listens a moment later, which
//! on a busy machine is seconds, and again each time it is started again.
//! So a port that is given has to be nobody else's from the moment it is
//! given until its service is gone.
//!
//! Asking the system for a free one does not make it so. What the system
//! gives (listen on port 0, read the number, let go) is the asker's only
//! while it is held, and from the moment it is let go the system gives
//! the same number to whoever asks next: another program's listener, the
//! near end of somebody's connection. A service was given its port that
//! way, something else on the machine was given the number before the
//! service's program listened, and the program ended at once, unable to
//! listen, and was started again on the same port for as long as the
//! other held it; and what answered meanwhile at the service's address,
//! to its health check and to whatever was sent there, was the other
//! program.
//!
//! So a port is given from numbers the system gives nobody by itself
//! ([`GIVEN`]), and is spoken for from before it is given until its
//! service is looked after no longer ([`Spoken`]), so that no other
//! factory on the device gives it either: not another daemon, and not
//! another service of this one started in the same moment.
//!
//! What is not held by this: a program that listens on a number because
//! it was told that number takes it whoever was given it, here as on any
//! port; and a system that has been told to give out ports from a range
//! that reaches into [`GIVEN`] gives them out as it did.

use std::io;
use std::net::{Ipv4Addr, UdpSocket};
use std::ops::RangeInclusive;

/// The ports a service is given one of. No system gives one of these by
/// itself to a program that asks for any port, or to the near end of a
/// connection: Linux gives from 32768 to 60999 unless it is told
/// otherwise, and the others from 49152 up.
pub const GIVEN: RangeInclusive<u16> = 21_000..=25_535;

/// How far above a port the number is that says it is spoken for: the
/// numbers from 61000 up, which are registered to nobody and which Linux
/// does not give by itself either.
const SPOKEN_AT: u16 = 40_000;

/// A port that is spoken for, for as long as this is kept.
///
/// What says so is a socket the system holds for this process and takes
/// back when the process ends, however it ends: nothing is written
/// anywhere, and nothing is left to be cleared after a daemon that was
/// killed. It carries nothing and is never read. It is not on the port
/// itself, where it would keep the service's own program from
/// listening, nor the other kind of socket at the same number, which
/// would keep out a program that serves by that kind as well: it is the
/// other kind at another number, which is the port's alone
/// ([`SPOKEN_AT`]).
#[derive(Debug)]
pub struct Spoken {
    port: u16,
    _held: UdpSocket,
}

impl Spoken {
    pub fn port(&self) -> u16 {
        self.port
    }
}

/// Speaks for `port`, where it is one of those that are given and nobody
/// on this device speaks for it: another factory, or this one.
///
/// Where a system gives the number that says so to something else for a
/// moment (the others than Linux give from all of those numbers), the
/// port is taken to be spoken for, which is the safe way to be wrong.
pub fn speak_for(port: u16) -> Option<Spoken> {
    if !GIVEN.contains(&port) {
        return None;
    }
    let held = UdpSocket::bind((Ipv4Addr::LOCALHOST, port + SPOKEN_AT)).ok()?;
    Some(Spoken { port, _held: held })
}

/// A port for a service: one nobody speaks for and nothing listens on,
/// spoken for from here on.
pub fn free() -> io::Result<Spoken> {
    free_in(GIVEN)
}

/// How many ports that nobody speaks for may be found listened on before
/// no more are tried. One is rare: nothing is given a port from among
/// these but by a factory, which speaks for it. So many one after
/// another is a device on which the looking itself is not answered, and
/// each look that is not answered waits ([`listened_on`]).
const LOOKED_AT_MOST: usize = 32;

/// The same from among `ports`, which a test holds to a range of one:
/// the first, from a place in them that is thrown for, that nobody
/// speaks for and nothing listens on.
pub fn free_in(ports: RangeInclusive<u16>) -> io::Result<Spoken> {
    let (first, last) = (u64::from(*ports.start()), u64::from(*ports.end()));
    let count = (last + 1).saturating_sub(first);
    // Thrown for, so that two factories that begin together do not ask
    // for the same numbers one after another.
    let from = kernel::id::random_below(count);
    let mut listened = 0;
    for nth in 0..count {
        let port = (first + (from + nth) % count) as u16;
        // Spoken for first and looked at second: looked at first, two
        // that ask together would both find it free.
        let Some(spoken) = speak_for(port) else { continue };
        if !listened_on(port) {
            return Ok(spoken);
        }
        listened += 1;
        if listened == LOOKED_AT_MOST {
            let why = format!(
                "{listened} ports from {first} to {last} that nobody speaks for are each listened on, or \
                 do not say that they are not; no more were tried"
            );
            return Err(io::Error::new(io::ErrorKind::AddrNotAvailable, why));
        }
    }
    let why = format!("every port from {first} to {last} is spoken for or listened on");
    Err(io::Error::new(io::ErrorKind::AddrNotAvailable, why))
}

/// Whether something on this device is listening on `port` already: a
/// program that was told that number, since nothing is given it.
///
/// Asked by connecting, and not by listening there for a moment. A
/// program started by another thread in that moment holds everything
/// this process has open until it becomes itself, the listener with the
/// rest, and a service's program that came to listen meanwhile would be
/// refused by what was only a look.
#[cfg(unix)]
fn listened_on(port: u16) -> bool {
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;
    // Answered at once where nothing listens. One that takes this long
    // to answer is there, and too busy to say so.
    let at = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    match TcpStream::connect_timeout(&at, Duration::from_millis(250)) {
        Ok(_) => true,
        Err(err) => err.kind() != io::ErrorKind::ConnectionRefused,
    }
}

/// The same where a connection nothing answers is not refused for a
/// second or two, and where a program that is started holds nothing of
/// this process's: asked by listening there for a moment.
#[cfg(not(unix))]
fn listened_on(port: u16) -> bool {
    std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_err()
}
