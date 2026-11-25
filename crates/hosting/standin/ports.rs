//! A port that is a test's own, for a program the test tells where to
//! listen before the program is there to take it.
//!
//! A rig found such a port by asking the system for a free one and
//! letting go of it (listen on port 0, read the number, drop the
//! listener), or had its daemon take any port and then started the
//! daemon again where it had been. Either way the number was nobody's
//! for a moment, and what the system is handed back it gives to whoever
//! asks next: on a machine where suites and gates start daemons side by
//! side something else was listening there by the time the daemon came
//! to, and it ended saying that it could not (`cannot listen on …:
//! Address already in use`).
//!
//! A program that can take its own port is told port 0 and says where it
//! is, and nothing is found for it. This is for the rest: what is
//! started again where it was, what keeps a daemon running and so starts
//! it again as it was started, a program whose first words the test does
//! not read. Such a port is one of numbers no system gives by itself
//! ([`TESTS`]), and is spoken for, for as long as what is handed out is
//! kept, by a socket the system holds for the test's process and takes
//! back when that ends, however it ends: so no other test has it, in
//! this process or in another suite's or another gate's, and nothing is
//! written or cleared. It is the way a factory gives its services theirs
//! (`hosting::ports`), from other numbers, so that what a test's factory
//! gives and what the test gives never meet.
//!
//! It is beside the stand-in because that is where what the suites share
//! is, a file for each thing (`#[path]`), and is the standard library
//! alone.

use std::hash::{BuildHasher, RandomState};
use std::net::{Ipv4Addr, UdpSocket};
use std::ops::RangeInclusive;

/// The ports a test is given one of. No system gives one of these by
/// itself (Linux gives from 32768 up unless it is told otherwise, the
/// others from 49152), and no factory gives one to a service
/// (`hosting::ports::GIVEN` is 21000 to 25535, spoken for from 61000
/// up).
pub const TESTS: RangeInclusive<u16> = 26_000..=28_999;

/// How far above a port the number is that says it is spoken for:
/// 29000 to 31999, which nothing gives by itself either.
const SPOKEN_AT: u16 = 3_000;

/// A port that is this test's for as long as this is kept.
///
/// What says so is the other kind of socket at another number, which is
/// the port's alone: never on the port, where it would be in the way of
/// the program that is to listen there.
#[derive(Debug)]
pub struct Port {
    number: u16,
    _held: UdpSocket,
}

impl Port {
    pub fn number(&self) -> u16 {
        self.number
    }
}

/// A port nobody speaks for and nothing listens on, this test's from
/// here on.
pub fn free() -> Port {
    free_in(TESTS).unwrap_or_else(|| {
        panic!(
            "every port from {} to {} is spoken for by a test or listened on: are that many daemons of \
             tests running here at once?",
            TESTS.start(),
            TESTS.end()
        )
    })
}

/// The same from among `ports`, which a test of this holds to a range of
/// one: the first, from a place in them that is thrown for, that nobody
/// speaks for and nothing listens on.
pub fn free_in(ports: RangeInclusive<u16>) -> Option<Port> {
    let (first, last) = (u64::from(*ports.start()), u64::from(*ports.end()));
    let count = (last + 1).saturating_sub(first);
    // Thrown for, so that suites that begin together do not ask for the
    // same numbers one after another.
    let from = RandomState::new().hash_one(std::process::id()) % count.max(1);
    (0..count).find_map(|nth| {
        let number = (first + (from + nth) % count) as u16;
        // Spoken for first and looked at second: looked at first, two
        // that ask together would both find it free. Where the number
        // that says so cannot be had for any reason, the port is taken
        // to be somebody's, which is the safe way to be wrong.
        let held = UdpSocket::bind((Ipv4Addr::LOCALHOST, number.checked_add(SPOKEN_AT)?)).ok()?;
        (!listened_on(number)).then_some(Port { number, _held: held })
    })
}

/// Whether something on this device is listening on `port` already: a
/// program that was told that number, since nothing is given it.
///
/// Asked by connecting, and not by listening there for a moment: a
/// program another test starts in that moment holds everything this
/// process has open until it becomes itself, the listener with the
/// rest, and the daemon that came to listen meanwhile would be refused
/// by what was only a look.
#[cfg(unix)]
fn listened_on(port: u16) -> bool {
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;
    // Answered at once where nothing listens. One that takes this long
    // to answer is there, and too busy to say so.
    let at = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    match TcpStream::connect_timeout(&at, Duration::from_millis(250)) {
        Ok(_) => true,
        Err(err) => err.kind() != std::io::ErrorKind::ConnectionRefused,
    }
}

/// The same where a connection nothing answers is not refused for a
/// second or two, and where a program that is started holds nothing of
/// this process's: asked by listening there for a moment.
#[cfg(not(unix))]
fn listened_on(port: u16) -> bool {
    std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_err()
}
