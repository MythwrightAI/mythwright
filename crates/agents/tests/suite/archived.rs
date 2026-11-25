//! A channel archived as whoever keeps agents holds it
//! (`Agents::archive_channel`): not while a turn of it waits or is in
//! hand, open or kept to its members, since its agent's answer would be
//! refused there with nobody told; and no turn queued there once it is.

use agents::TurnRequest;
use kernel::ErrorKind;

use crate::kept::walls;
use crate::{Rig, ada, own};

#[tokio::test]
async fn a_channel_is_not_archived_while_an_agent_is_asked_something_there_and_nothing_is_asked_there_once_it_is()
 {
    let rig = Rig::holding(walls()).await;
    rig.agent("builder", "fake").await;
    let channels = &rig.studio.channels;
    let shop = channels.get(&own(), "shop").await.unwrap();

    // Asked something there, and holding it: refused, naming the agent
    // and the ways out, with nothing archived.
    rig.say("@builder hold this [gate:held]").await;
    rig.fake.saw(1).await;
    let refused = rig.agents.archive_channel("shop", &ada()).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("`builder` is being asked something in #shop"), "{}", refused.message);
    assert!(refused.message.contains(&format!("`{} agent stop builder`", brand::CLI)), "{}", refused.message);
    assert_eq!(channels.get(&own(), "shop").await.unwrap().archived, None);
    rig.fake.open("held");
    rig.turns_finished(1).await;

    // Once it has answered, archived; and nothing is queued there after.
    let archived = rig.agents.archive_channel("shop", &ada()).await.unwrap();
    assert!(archived.archived.is_some());
    let asked = TurnRequest {
        slot: "builder".into(),
        kind: "task".into(),
        thread: "channel:shop".into(),
        prompt: "the quince".into(),
        channel: Some("shop".into()),
        ..TurnRequest::default()
    };
    let refused = rig.agents.enqueue(asked).await.unwrap_err();
    assert!(refused.message.contains("#shop is archived"), "{}", refused.message);
    let back = channels.may_write("shop", &ada()).await.unwrap();
    channels.unarchive(&back, "shop", &ada()).await.unwrap();

    // Kept to its members, the same of a turn of it in hand.
    rig.agents.keep_channel("shop", &ada()).await.unwrap();
    channels.post(&shop.id, &ada(), "@builder hold this too [gate:again]", None).await.unwrap();
    rig.fake.saw(2).await;
    let refused = rig.agents.archive_channel(&shop.id, &ada()).await.unwrap_err();
    assert!(refused.message.contains("`builder` is being asked something in `shop`"), "{}", refused.message);
    rig.fake.open("again");
    rig.turns_finished(2).await;
    assert!(rig.agents.archive_channel(&shop.id, &ada()).await.unwrap().archived.is_some());
}
