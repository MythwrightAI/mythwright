//! A node that is stopped in the instant its journal shuts.
//!
//! A journal that has shut at a place it was told answers nothing
//! (`Journal::shut_after`), and a node that stops writes down what was
//! measured first, where its journal has not shut. One that shuts between
//! the node's looking and its asking left the stop waiting for ever: a
//! test told of its place a moment early stopped its node there, and its
//! gate with it. What a stop asks of a journal that shuts is given up
//! when it shuts (`Journal::unless_shut`).

use crate::Rig;

#[tokio::test]
async fn a_node_stopped_as_its_journal_shuts_gives_up_what_it_was_writing_down_and_stops() {
    let mut rig = Rig::new().await;
    let (journal, engine) = {
        let app = &rig.node().app;
        (app.journal.clone(), app.engine.clone())
    };
    // Something measured that is worth a write: what a stop writes down.
    journal.measure("test.stopping", &[], 1.0, true);

    // The order is made, and not left to which of two threads is the
    // quicker. The database does its work one piece at a time, in the
    // order it was asked: it is kept at a first piece until the test lets
    // it go, and the piece behind that shuts the journal. So the node's
    // stop, begun while the database is kept there, finds a journal that
    // has not shut and asks it to write, and the journal shuts before
    // that is come to.
    let (go, kept) = std::sync::mpsc::channel::<()>();
    let waiting = journal.db().call(move |_| {
        let _ = kept.recv();
        Ok(())
    });
    let its = journal.clone();
    let shutting = journal.db().call(move |_| {
        its.shut_now();
        Ok(())
    });

    let node = rig.node.take().expect("the node is running");
    let let_go = async {
        // Halted, a node goes straight on, with nothing waited for in
        // between, to look whether its journal has shut and to ask for
        // what was measured to be written: once it is halting and has
        // given way, it has asked.
        while !engine.is_halting() {
            tokio::task::yield_now().await;
        }
        assert!(!journal.is_shut(), "the journal had not shut when the node looked");
        go.send(()).expect("the database is kept at its first piece of work");
    };
    crate::soon("a node to stop whose journal shut as it was stopping", async {
        tokio::join!(node.stop(), let_go);
    })
    .await;
    assert!(journal.is_shut(), "the journal shut behind what the node asked");
    drop((waiting, shutting));

    // And it stopped as a node does: it said so, the one thing kept after
    // the place, and one started on what it had goes on from there.
    rig.start().await;
    let query = journal::EventQuery {
        topics: "system.stopped,system.started".to_string(),
        limit: 100,
        ..Default::default()
    };
    let said: Vec<String> = rig.said(query).await.into_iter().map(|event| event.topic).collect();
    assert_eq!(said, ["system.started", "system.stopped", "system.started"]);
}
