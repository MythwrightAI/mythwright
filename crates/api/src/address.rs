//! Where a node listens: taking its address as it starts, waiting briefly
//! where the one in the way was most likely this factory's own daemon a
//! moment ago, and, as it stops, waiting until nothing answers there any
//! more, so a daemon started next on the same data finds it free.

use std::net::SocketAddr;
use std::time::Duration;

use folders::{LOCK_PATIENCE, folder};
use kernel::{Error, Result};
use tokio::net::TcpListener;

/// Listens at `listen`. An address that is in use is waited for briefly
/// where whoever has it was most likely this factory's own daemon a
/// moment ago: at an address the factory is coming back to
/// (`remembered`), while something has its folder, which a daemon on
/// its way out still does, and where a daemon of it was served when it
/// ended without stopping (killed, it left where it was served). An
/// address outlives its holder for an instant, as the lock on its data
/// does, in a program the holder was starting (`let_go_of`), which a
/// daemon that stops waits for and one that was killed cannot. Anywhere
/// else a taken address is refused at once.
///
/// One that is still in use is refused for good, saying whose it is: a
/// daemon of this factory's, by what its folder says, or something
/// else's. Asked again it would be refused the same way, so what keeps a
/// daemon running does not ask again ([`folder`]).
pub(crate) async fn listen_at(
    listen: SocketAddr,
    remembered: bool,
    data: &std::path::Path,
) -> Result<TcpListener> {
    let mut waited = Duration::ZERO;
    loop {
        let err = match TcpListener::bind(listen).await {
            Ok(listener) => return Ok(listener),
            Err(err) => err,
        };
        if err.kind() != std::io::ErrorKind::AddrInUse {
            return Err(Error::unavailable(format!("cannot listen on {listen}: {err}")));
        }
        let ours = folder::is_held(data);
        let ended = !ours && folder::served_when_it_ended(data) == Some(listen);
        if (remembered || ours || ended) && waited < LOCK_PATIENCE {
            tokio::time::sleep(Duration::from_millis(25)).await;
            waited += Duration::from_millis(25);
            continue;
        }
        return Err(match ours {
            true => folder::in_the_way(data, listen),
            // Said to have been waited for only where it was.
            false => folder::taken(data, listen, &err, remembered, ended && waited >= LOCK_PATIENCE),
        });
    }
}

/// Waits, for [`LOCK_PATIENCE`] at most, until nothing answers at the
/// address this node listened at, telling `told` once where it has to.
///
/// What the node listened with is closed by then, but a program it was
/// starting has a copy of everything the node had open until it has
/// become itself: the system gives the copies back as it starts the
/// program, one at a time and the last opened first, so that the
/// folder's lock, opened just after the listener, is let go of before
/// it, and on a crowded machine the program may wait its turn between
/// the two for longer than a daemon takes to start. A daemon that had
/// ended in that moment was started again at its address and refused it
/// at once, with the folder free, as though something that is not the
/// factory had it (once in some fifty crowded playings of a package's
/// cases, each of which stops and starts its daemon several times).
///
/// Asked by connecting, as whether something listens always is: a
/// listener with nobody to take what comes is answered by the system.
/// Unix's alone: there a program is started from a copy of the daemon,
/// and on Windows what a program is handed is said, so nothing of this
/// is in it, and a connection nobody answers takes a second or two to be
/// refused there.
#[cfg(unix)]
pub(crate) async fn let_go_of(addr: SocketAddr, told: &impl Fn(&str)) {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    let mut at = addr;
    if at.ip().is_unspecified() {
        at.set_ip(match at.ip() {
            IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::LOCALHOST),
        });
    }
    // Answered at once where nothing listens. One that takes this long
    // is there, with nobody to take what comes.
    let answers = || async {
        match tokio::time::timeout(Duration::from_millis(250), tokio::net::TcpStream::connect(at)).await {
            Ok(Ok(_)) | Err(_) => true,
            Ok(Err(_)) => false,
        }
    };
    let mut waited = Duration::ZERO;
    while answers().await {
        if waited.is_zero() {
            told(&format!(
                "waiting for {at} to be let go of: a program this daemon was starting as it stopped still \
                 has it"
            ));
        }
        if waited >= LOCK_PATIENCE {
            told(&format!(
                "{at} still answers, and the daemon has stopped all the same: a start there is refused \
                 until whatever has it lets it go"
            ));
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
        waited += Duration::from_millis(25);
    }
}

#[cfg(not(unix))]
pub(crate) async fn let_go_of(_: SocketAddr, _: &impl Fn(&str)) {}
