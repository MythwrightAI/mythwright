//! The ports the factory gives out (`hosting::ports`).
//!
//! A service was given a port by asking the system for a free one and
//! letting go of it, and the system gave the same number to something
//! else before the service's program had listened there: the program
//! ended at once and was started again, and what answered at its
//! address meanwhile was the other program. The order is one the system
//! makes and no test can ask it for (which number it gives next is
//! thrown for), so what is held here is what keeps it from coming about:
//! a port that is given is none of those the system gives by itself, and
//! is spoken for from before its program is there until its service is
//! looked after no longer, its program having listened or not.
//!
//! A service here never listens: the moment between being given a port
//! and taking it, for as long as the test goes on.

use std::net::TcpListener;

use hosting::ports;
use serde_json::json;

use crate::standin::cmd;
use crate::{Rig, step};

/// A service of that name that does `words` and is given a port.
fn service(name: &str, words: &[&str], more: &str) -> String {
    let does = cmd(words);
    format!(
        "\
blueprint: {name}
permissions: [fleet]
steps:
  start:
    do: service.start
    with: {{ name: {name}, cmd: {does}{more} }}
"
    )
}

/// The ports this system gives by itself to whoever asks for any, as it
/// says them, where it says.
#[cfg(target_os = "linux")]
fn given_by_the_system() -> (u16, u16) {
    let said = std::fs::read_to_string("/proc/sys/net/ipv4/ip_local_port_range").unwrap();
    let mut ends = said.split_whitespace().map(|end| end.parse().unwrap());
    (ends.next().unwrap(), ends.next().unwrap())
}

#[test]
fn a_port_that_is_spoken_for_is_given_to_nobody_else() {
    let one = ports::free().unwrap();
    assert!(ports::GIVEN.contains(&one.port()), "{one:?}");
    // Not to whoever asks for it by its number, and not to whoever asks
    // for any where it is the only one there is.
    assert!(ports::speak_for(one.port()).is_none());
    let refused = ports::free_in(one.port()..=one.port()).unwrap_err();
    assert_eq!(
        refused.to_string(),
        format!("every port from {0} to {0} is spoken for or listened on", one.port())
    );
    // And those asked for while it is held are each another.
    let more: Vec<ports::Spoken> = (0..64).map(|_| ports::free().unwrap()).collect();
    let mut all: Vec<u16> = more.iter().map(ports::Spoken::port).chain([one.port()]).collect();
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), 65, "a port was given twice");
    // Nothing is spoken for that is not one of those that are given: a
    // port its owner named is the owner's to keep free.
    assert!(ports::speak_for(ports::GIVEN.start() - 1).is_none());
    assert!(ports::speak_for(ports::GIVEN.end() + 1).is_none());
}

#[test]
fn a_port_something_listens_on_is_given_to_no_service() {
    let spoken = ports::free().unwrap();
    let port = spoken.port();
    // A program that was told this number, and listens there with nobody
    // speaking for it.
    let _listens = TcpListener::bind(("127.0.0.1", port)).unwrap();
    drop(spoken);
    assert!(ports::free_in(port..=port).is_err(), "a port something listens on was given");
}

/// Linux's alone: the others do not say which ports they give.
#[cfg(target_os = "linux")]
#[test]
fn no_port_that_is_given_is_one_this_system_gives_by_itself() {
    let (first, last) = given_by_the_system();
    // Where the system was told to give from other ports than it does
    // unasked, and those reach into the factory's, nothing here holds.
    if first <= *ports::GIVEN.end() && last >= *ports::GIVEN.start() {
        eprintln!(
            "not played: this system gives out the ports from {first} to {last}, which reach into those a service is given"
        );
        return;
    }
    // Asked many times, as anybody asks, it gives none of them.
    let asked: Vec<TcpListener> = (0..64).map(|_| TcpListener::bind(("127.0.0.1", 0)).unwrap()).collect();
    for given in &asked {
        let port = given.local_addr().unwrap().port();
        assert!(!ports::GIVEN.contains(&port), "the system gave {port} by itself");
    }
}

#[tokio::test]
async fn a_port_a_service_is_given_is_nobody_elses_from_before_its_program_listens_until_the_service_is_gone()
{
    let rig = Rig::new().await;
    let applied = rig.apply(&service("web", &["idle"], ", restart: never"), json!({})).await;
    let run = rig.finished(&applied).await;
    let port = step(&run, "start").output["port"].as_u64().unwrap() as u16;

    // None of those the system gives by itself: nothing that asks it for
    // a free port, and no connection's near end, is given this one.
    assert!(ports::GIVEN.contains(&port), "it was given {port}, which the system gives to whoever asks next");
    // And no factory is given it, this one or another, though nothing
    // listens there yet.
    assert_eq!(rig.hosting.spoken_for(), [port]);
    assert!(ports::speak_for(port).is_none(), "its port was there for another to be given");
    assert!(ports::free_in(port..=port).is_err());

    // Gone, it keeps the port from nobody.
    rig.engine.stop(&applied.instance).await.unwrap();
    rig.event("service.stopped", |_| true).await;
    assert!(rig.hosting.spoken_for().is_empty(), "a service that is gone still has its port spoken for");
}

#[tokio::test]
async fn a_service_whose_program_has_ended_keeps_its_port_until_it_is_started_again() {
    let rig = Rig::new().await;
    // It ends badly at once, and is to be started again in a while:
    // nothing has the port meanwhile.
    let applied = rig.apply(&service("frail", &["exit", "1"], ", restart_backoff: 30s"), json!({})).await;
    let port = step(&rig.finished(&applied).await, "start").output["port"].as_u64().unwrap() as u16;
    rig.event("service.restarting", |event| event.data["service"] == "frail").await;
    assert_eq!(rig.hosting.spoken_for(), [port]);
    assert!(ports::speak_for(port).is_none(), "its port was there for another to be given between its lives");

    // Stopped while it waits, it will not be started again.
    rig.engine.stop(&applied.instance).await.unwrap();
    rig.event("service.stopped", |_| true).await;
    assert!(rig.hosting.spoken_for().is_empty(), "a service that is gone still has its port spoken for");
}

#[tokio::test]
async fn a_service_that_named_its_port_is_spoken_for_by_nobody() {
    let rig = Rig::new().await;
    // A port of its owner's choosing, which nothing here keeps free: one
    // found free for the test, and held until the service has it.
    let named = ports::free().unwrap();
    let port = named.port();
    let applied =
        rig.apply(&service("named", &["idle"], &format!(", port: {port}, restart: never")), json!({})).await;
    assert_eq!(step(&rig.finished(&applied).await, "start").output["port"], port);
    assert!(rig.hosting.spoken_for().is_empty(), "a port its owner named was spoken for");
    rig.engine.stop(&applied.instance).await.unwrap();
    rig.event("service.stopped", |_| true).await;
}

#[tokio::test]
async fn a_conductor_that_has_stopped_speaks_for_no_port_and_its_services_go_on() {
    let mut rig = Rig::new().await;
    let applied = rig.apply(&service("steady", &["idle"], ", restart: never"), json!({})).await;
    let port = step(&rig.finished(&applied).await, "start").output["port"].as_u64().unwrap() as u16;
    rig.event("service.healthy", |_| true).await;
    assert_eq!(rig.hosting.spoken_for(), [port]);

    // Stopped, it starts no program again and keeps no port: whatever
    // finds its services speaks for theirs.
    rig.engine.halt();
    rig.hosting.halt();
    assert!(rig.hosting.spoken_for().is_empty(), "a conductor that has stopped keeps a port from the next");

    // The one that comes back finds the service where it was, on the
    // port it had.
    rig.restart().await;
    let found = rig.hosting.services().remove(0);
    assert_eq!((found.name.as_str(), found.port), ("steady", port));
    rig.engine.stop(&applied.instance).await.unwrap();
    rig.event("service.stopped", |_| true).await;
    assert!(rig.hosting.spoken_for().is_empty());
}
