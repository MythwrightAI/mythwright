//! What the detail of runs' steps is held back by as it is let go of
//! (`Engine::let_logs_go`), where something is being made as it is asked.
//!
//! An instance is written down, and said to have started, in one write,
//! and is in hand only once whoever made it has its answer and goes on:
//! in between, its row is alive and its text is known nowhere, so what it
//! could be owed is not known and nothing said of steps goes. A fleet's
//! stage has its column looked at (`std/due`) in a handler's run just
//! after the fleet's apply has answered, and a letting go asked in that
//! moment held everything back: the API suite's test of a fleet that is
//! up asked it before the fleet was at rest, and failed so once on the
//! hosted machine with the journal gathering commits.
//!
//! The order is made here: the asking is looked at only by the test, one
//! answer of the database at a time, and the letting go is asked between
//! each two of them ([`Stepped`]), so one of them is the moment after the
//! instance was written and before it was in hand, whatever the machine.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use conductor::Applied;

use crate::stepped::Stepped;
use crate::{Rig, user};

/// Something that hears nothing and keeps nothing, as `std/due`.
const LOOK: &str = "\
blueprint: look
durable: false
steps:
  look: { do: test.count, with: { key: looked } }
";

/// What letting go comes to now: why everything was held back, if it was.
async fn held(stepped: &mut Stepped, rig: &Rig) -> Option<String> {
    let went = stepped.through(false, rig.engine.let_logs_go(&BTreeMap::new())).await;
    let went = went.expect("the journal is open").expect("letting go is answered");
    went.expect("the factory keeps a day").held
}

#[tokio::test]
async fn something_written_down_and_not_yet_in_hand_holds_back_everything_and_nothing_once_it_is() {
    let rig = Rig::new().await;
    rig.engine.keep_logs_for(Some(1), &user()).await.unwrap();
    let mut stepped = Stepped::holding(&rig.journal);
    let mut asking: Pin<Box<dyn Future<Output = kernel::Result<Applied>> + Send + '_>> =
        Box::pin(rig.engine.apply(rig.request(LOOK, serde_json::json!({}))));
    let mut cx = Context::from_waker(Waker::noop());
    let mut between = Vec::new();
    let applied = loop {
        if let Poll::Ready(applied) = asking.as_mut().poll(&mut cx) {
            break applied.expect("it is applied");
        }
        between.extend(held(&mut stepped, &rig).await);
    };
    assert!(
        between.iter().any(|why| why.contains("`look` is alive and was not in hand")),
        "no letting go was asked while it was written and not in hand: {between:?}"
    );
    // In hand, it hears nothing of steps and waits for none.
    assert!(!applied.instance.is_empty());
    assert_eq!(held(&mut stepped, &rig).await, None);
}
