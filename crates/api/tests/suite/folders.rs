//! A factory's data folder says who has it (`api::folder`): the daemon
//! that serves it and where, so that a second is told that and not that
//! an address is in use; nobody, with an address that is something
//! else's; or something that serves nothing, which passes.

use api::{Config, Node};
use kernel::ErrorKind;

use crate::Rig;

/// Why a node asked to start on `data`, listening at `listen`, was not.
async fn refused(data: &std::path::Path, listen: &str) -> kernel::Error {
    let config = Config { listen: listen.parse().unwrap(), ..Config::new(data) };
    Node::start(config).await.err().expect("the node is refused")
}

#[tokio::test]
async fn a_second_node_on_the_same_data_is_told_where_the_first_is_whichever_address_it_asks_for() {
    let rig = Rig::new().await;
    let data = rig.dir.join("data");
    let at = rig.url.trim_start_matches("http://").to_string();
    assert_eq!(api::folder::served(&data).map(|at| at.to_string()), Some(at.clone()));

    // At an address of its own, and at the one the first has: the same
    // thing is so, and it is said the same way. It conflicts with how
    // things stand, which asking again does not change.
    for listen in ["127.0.0.1:0", &at] {
        let err = refused(&data, listen).await;
        assert_eq!(err.kind, ErrorKind::Conflict, "{listen}: {}", err.message);
        assert!(
            err.message.contains(&format!(
                "the factory at {} is already running, at {}",
                data.display(),
                rig.url
            )),
            "{listen}: {}",
            err.message
        );
        assert!(
            err.message.contains(&format!("`{} --url {} down` stops it", brand::CLI, rig.url)),
            "{listen}: {}",
            err.message
        );
        assert!(!err.message.contains("cannot listen"), "{listen}: {}", err.message);
    }
}

#[tokio::test]
async fn an_address_something_else_has_is_said_to_be_something_elses_and_nothing_is_made_for_the_factory() {
    let rig = Rig::new().await;
    // Something that is no factory, and a factory that is another: each
    // has an address a factory with nothing running is told to take.
    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let theirs = taken.local_addr().unwrap().to_string();
    let other = rig.dir.join("another");
    for listen in [theirs.as_str(), rig.url.trim_start_matches("http://")] {
        let err = refused(&other, listen).await;
        assert_eq!(err.kind, ErrorKind::Conflict, "asking again does not free an address: {}", err.message);
        assert!(err.message.starts_with(&format!("cannot listen on {listen}: ")), "{}", err.message);
        assert!(
            err.message.contains("Something that is not this factory has the address"),
            "{}",
            err.message
        );
        assert!(err.message.contains("`--listen 127.0.0.1:<port>`"), "what to do is said: {}", err.message);
        assert!(!err.message.contains("already running"), "{}", err.message);
    }
    assert!(!other.exists(), "nothing was made for a factory that was not started");
}

#[tokio::test]
async fn a_folder_that_is_held_and_says_nothing_of_being_served_is_waited_for_and_not_taken_for_a_daemon() {
    let mut rig = Rig::new().await;
    let data = rig.dir.join("data");
    let serving = data.join("serving");
    assert!(serving.is_file(), "a node that has the folder says where it serves it");

    // A node that stopped says nothing more of where: until its files
    // are put away it still has the folder, and whoever comes then is
    // not told that it is being served.
    rig.stop().await;
    assert!(!serving.exists(), "it says it serves no more as it stops");
    assert_eq!(api::folder::served(&data), None);

    // Held as a daemon that is stopping holds it, or a command at work
    // on a stopped factory's files: that passes, which is what its kind
    // says, and what keeps a daemon running asks again for.
    let lock = std::fs::File::open(data.join("lock")).unwrap();
    lock.lock().unwrap();
    assert!(api::folder::is_held(&data));
    let err = refused(&data, "127.0.0.1:0").await;
    assert_eq!(err.kind, ErrorKind::Unavailable, "{}", err.message);
    assert!(err.message.contains("is held and is not being served"), "{}", err.message);
    assert!(err.message.contains("still stopping"), "{}", err.message);

    // What a daemon that was killed left saying where it listened is
    // nothing while nobody has the folder, and is taken away by what
    // comes to have it without serving it: read while that holds the
    // folder, it would be taken for a daemon.
    lock.unlock().unwrap();
    std::fs::write(&serving, "127.0.0.1:9\n").unwrap();
    assert_eq!(api::folder::served(&data), None, "nobody has the folder");
    api::repack::look(&data).unwrap();
    assert!(!serving.exists(), "what worked on the stopped factory's files took it away");

    // And the factory starts as it always did.
    rig.start().await;
    assert_eq!(api::folder::served(&data).map(|at| format!("http://{at}")), Some(rig.url.clone()));
}
