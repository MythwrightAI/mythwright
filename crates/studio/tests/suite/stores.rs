//! Declaring a table that a store service keeps. What such a table then
//! does, with a store answering, is tested through the API, where there is
//! one to answer.

use kernel::ErrorKind;
use serde_json::json;
use studio::Store;
use studio::collections::Spec;

use crate::{Rig, ada};

fn notes(store: &str) -> Spec {
    Spec::read(&json!({ "title": { "type": "text", "required": true } }), &json!(null), Some(store)).unwrap()
}

/// An address on this device at which nothing is listening, and what
/// keeps it so: the address is held, by something that never listens
/// there, for as long as that is kept. Taken and let go of, it was free
/// for whatever asked the system for an address next, and with other
/// suites starting daemons beside this one something else was now and
/// then listening there by the time the store was asked: it answered, or
/// hung up, and the refusal was said in other words than these tests hold.
fn nowhere() -> (tokio::net::TcpSocket, String) {
    let held = tokio::net::TcpSocket::new_v4().unwrap();
    held.bind("127.0.0.1:0".parse().unwrap()).unwrap();
    let port = held.local_addr().unwrap().port();
    (held, format!("http://127.0.0.1:{port}"))
}

#[tokio::test]
async fn a_table_kept_by_a_store_service_is_declared_with_the_address_the_service_answers_at() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let collections = &rig.studio.collections;

    let unsaid = collections.ensure("shop", "notes", notes("service"), None, &ada()).await.unwrap_err();
    assert_eq!(unsaid.kind, ErrorKind::Invalid);
    assert!(
        unsaid.message.starts_with("`notes` is to be kept by a store service and needs `at`"),
        "{}",
        unsaid.message
    );

    for written in ["cards.db", "ftp://127.0.0.1:21", "https://store.example", "127.0.0.1:7801"] {
        let spec = notes("service").kept_at(Some(written));
        let wrong = collections.ensure("shop", "notes", spec, None, &ada()).await.unwrap_err();
        assert_eq!(wrong.kind, ErrorKind::Invalid, "{written}");
        assert!(
            wrong.message.starts_with(&format!("`{written}` is not the address of a store service.")),
            "{}",
            wrong.message
        );
    }

    // An address means something only for a store service.
    let spec = notes("files").kept_at(Some("http://127.0.0.1:7801"));
    let stray = collections.ensure("shop", "notes", spec, None, &ada()).await.unwrap_err();
    assert_eq!(
        stray.message,
        "`at: http://127.0.0.1:7801` is the address of a store service, and `notes` is kept in `files`; say `store: service`, or leave `at` out"
    );
    // Written and left empty, as a blueprint's param that was not given is, it is not an address.
    let made = collections.ensure("shop", "notes", notes("local").kept_at(Some("  ")), None, &ada()).await;
    assert_eq!(made.unwrap().at, None);
}

#[tokio::test]
async fn a_table_is_not_declared_in_a_store_service_that_is_not_answering() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let (_held, at) = nowhere();
    let spec = notes("service").kept_at(Some(&format!("{at}/")));
    let silent = rig.studio.collections.ensure("shop", "notes", spec, None, &ada()).await.unwrap_err();
    assert_eq!(
        silent.kind,
        ErrorKind::Unavailable,
        "could not ask is not the same as no: {}",
        silent.message
    );
    assert_eq!(
        silent.message,
        format!(
            "`notes` of `shop` is kept by a store service at {at}, and nothing is answering there. Start the blueprint that hosts the store, or look at why it stopped: `{cli} services` and `{cli} instances` show what is running",
            cli = brand::CLI
        )
    );
    assert_eq!(rig.studio.collections.list(Some("shop")).await.unwrap(), []);
    assert!(rig.topics("collection.ensured").await.is_empty());

    // The same for a board, which is such a table.
    let columns = vec!["todo".to_string(), "done".to_string()];
    let unmade = rig.studio.board.make("shop", columns, Store::Service, None, &ada()).await.unwrap_err();
    assert!(unmade.message.contains("needs `at`"), "{}", unmade.message);
    assert!(!rig.studio.board.exists("shop").await);
}
