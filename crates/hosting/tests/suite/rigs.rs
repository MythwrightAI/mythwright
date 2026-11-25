//! What the rigs of the suites share that is not the stand-in: a port
//! that is a test's own (`crates/hosting/standin/ports.rs`), for a
//! program a test tells where to listen before the program is there, or
//! starts again where it was.
//!
//! It is held here as the ports a factory gives its services are held
//! (`ports.rs`): by the system refusing what is kept, which no copy of a
//! socket in a program some other test is just starting makes wrong.
//! Nothing says that a port can be had again once it was let go, for the
//! same reason.

use std::net::{TcpListener, UdpSocket};

#[path = "../../standin/ports.rs"]
mod own;

/// How far above a port the number is that says a test has it, as the
/// file itself says it.
const SPOKEN_AT: u16 = 3_000;

#[test]
fn a_port_that_is_a_tests_own_is_no_other_tests_while_it_is_kept_and_is_free_for_its_program() {
    let kept: Vec<own::Port> = (0..32).map(|_| own::free()).collect();
    let mut numbers: Vec<u16> = kept.iter().map(own::Port::number).collect();
    numbers.sort_unstable();
    numbers.dedup();
    assert_eq!(numbers.len(), kept.len(), "each is another's: {kept:?}");

    for port in &kept {
        let number = port.number();
        // Nobody is given it again while it is kept: not from among all
        // of them, and not where it is the only one there is.
        assert!(own::free_in(number..=number).is_none(), "{number} was given twice");
        // What says so is the system's, held for this process: anything
        // else on the device that goes by it is refused there.
        assert!(
            UdpSocket::bind(("127.0.0.1", number + SPOKEN_AT)).is_err(),
            "nothing keeps {number} from another process"
        );
    }
    // And the port itself is left to the program that is told it.
    let number = kept[0].number();
    let listens = TcpListener::bind(("127.0.0.1", number));
    assert!(listens.is_ok(), "the program told {number} could not listen there: {listens:?}");
}

#[test]
fn the_ports_a_test_is_given_are_none_a_factory_gives_and_none_that_says_a_port_is_spoken_for() {
    let (first, last) = (*own::TESTS.start(), *own::TESTS.end());
    // The numbers that say one of them is a test's.
    let (said_first, said_last) = (first + SPOKEN_AT, last + SPOKEN_AT);
    assert!(said_first > last, "a number that says a port is a test's is itself a port: {first} to {last}");
    // Clear of what a factory gives its services, and of the numbers
    // that say one of those is spoken for, which are 40000 above them.
    let given = hosting::ports::GIVEN;
    assert!(first > *given.end() && said_last < given.start() + 40_000, "{first} to {said_last}, {given:?}");
    // And below what any system gives by itself to whoever asks for any
    // port, or to the near end of a connection: Linux from 32768 unless
    // it is told otherwise, the others from 49152.
    assert!(first > 1_024 && said_last < 32_768, "{first} to {said_last}");

    let one = own::free();
    assert!(own::TESTS.contains(&one.number()), "{one:?}");
}

/// Linux's alone: the others do not say which ports they give.
#[cfg(target_os = "linux")]
#[test]
fn no_port_a_test_is_given_is_one_this_system_gives_by_itself() {
    let said = std::fs::read_to_string("/proc/sys/net/ipv4/ip_local_port_range").unwrap();
    let mut ends = said.split_whitespace().map(|end| end.parse::<u16>().unwrap());
    let (first, last) = (ends.next().unwrap(), ends.next().unwrap());
    let (ours_first, ours_last) = (*own::TESTS.start(), *own::TESTS.end() + SPOKEN_AT);
    // Where the system was told to give from other ports than it does
    // unasked, and those reach into the tests', nothing here holds.
    if first <= ours_last && last >= ours_first {
        eprintln!(
            "not played: this system gives out the ports from {first} to {last}, which reach into those a test is given"
        );
        return;
    }
    // Asked many times, as anybody asks, it gives none of them.
    let asked: Vec<TcpListener> = (0..64).map(|_| TcpListener::bind(("127.0.0.1", 0)).unwrap()).collect();
    for given in &asked {
        let port = given.local_addr().unwrap().port();
        assert!(!(ours_first..=ours_last).contains(&port), "the system gave {port} by itself");
    }
}

#[test]
fn a_port_something_listens_on_is_given_to_no_test() {
    // A program that was told the number, whoever had it: here the test
    // itself, on a port that is its own so that no other test is given
    // it meanwhile.
    let taken = own::free();
    let number = taken.number();
    let _listens = TcpListener::bind(("127.0.0.1", number)).unwrap();
    // Let go of, it is still nobody's to be given while something
    // listens there (and where a copy of what spoke for it is in a
    // program some other test is starting, it is nobody's for that).
    drop(taken);
    assert!(own::free_in(number..=number).is_none(), "{number} is listened on, and was given");
}
