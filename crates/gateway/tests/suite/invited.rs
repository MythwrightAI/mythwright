//! Who is let in: an invitation works once, for a while, for one name; a
//! device is known by the key it proved; and a member holds its hub to
//! the key its invitation named.

use std::time::Duration;

use gateway::{Invitation, Shut};

use crate::{Device, Rig, ada};

/// What a third device is told when it tries `invitation` over the rig's wire.
async fn turned_away(rig: &Rig, invitation: &str) -> String {
    let other = Device::new("other-box", &rig.clock, &rig.shared).await;
    other.gateway.join(invitation, None, &ada()).await.unwrap_err().message
}

#[tokio::test]
async fn an_invitation_lets_one_device_in_once() {
    let rig = Rig::linked().await;
    let devices = rig.hub.gateway.devices().await.unwrap();
    assert_eq!(devices.len(), 1);
    // The hub knows the member by the key it proved, which is the member's own.
    assert_eq!(devices[0].id.as_deref(), Some(rig.member.gateway.id().as_str()));
    assert_eq!(devices[0].invited, None);
    // And the member holds the hub to the hub's.
    assert_eq!(rig.member.gateway.uplink().unwrap().id, rig.hub.gateway.id());

    let said = turned_away(&rig, &rig.invitation).await;
    assert!(said.contains("that invitation has been used"), "{said}");
    assert_eq!(rig.hub.gateway.devices().await.unwrap().len(), 1, "nobody else came in by it");
    let refused = rig.hub.topics("device.refused").await;
    assert_eq!(refused.len(), 1);
    // Nothing that was said of any of it holds the invitation or its secret.
    let secret = Invitation::read(&rig.invitation).unwrap().secret;
    let query = journal::EventQuery { limit: 1000, ..Default::default() };
    for device in [&rig.hub, &rig.member] {
        let events = format!("{:?}", device.journal.read(query.clone()).await.unwrap());
        assert!(!events.contains(&secret) && !events.contains(&rig.invitation[..40]));
    }
}

#[tokio::test]
async fn an_invitation_stops_working_when_it_said_it_would() {
    let rig = Rig::apart().await;
    rig.clock.advance(Duration::from_secs(24 * 3600 + 1));
    let said = rig.member.gateway.join(&rig.invitation, None, &ada()).await.unwrap_err().message;
    assert!(said.contains("has stopped working"), "{said}");
    assert!(rig.member.gateway.uplink().is_none(), "it joined nothing");
    assert!(rig.hub.gateway.devices().await.unwrap().is_empty(), "and is no longer waited for");
}

#[tokio::test]
async fn an_invitation_the_hub_did_not_make_is_refused_and_nonsense_is_not_one() {
    let rig = Rig::apart().await;
    let mut forged = Invitation::read(&rig.invitation).unwrap();
    forged.secret = kernel::id::secret();
    let said = rig.member.gateway.join(&forged.written(), None, &ada()).await.unwrap_err().message;
    assert!(said.contains("is not one this hub made"), "{said}");
    for nonsense in ["", "dev_0123456789", "inv_not-letters-that-mean-anything", "http://hub"] {
        let said = rig.member.gateway.join(nonsense, None, &ada()).await.unwrap_err().message;
        assert!(said.contains("that is not an invitation"), "{nonsense}: {said}");
    }
    // A hub does not join itself.
    let own = rig.hub.gateway.join(&rig.invitation, None, &ada()).await.unwrap_err().message;
    assert!(own.contains("a hub does not join itself"), "{own}");
    // The real one still works: nothing above used it up.
    rig.link(vec!["*".into()]).await;
}

#[tokio::test]
async fn a_name_that_is_in_the_fleet_is_not_given_to_another_device() {
    let rig = Rig::apart().await;
    let at = vec!["http://hub".to_string()];
    let second = rig.hub.gateway.invite("builder-1", None, at.clone(), &ada()).await.unwrap();
    rig.link(vec!["*".into()]).await;
    let said = turned_away(&rig, &second.invitation).await;
    assert!(said.contains("is in this fleet already, and it is not this one"), "{said}");
    let again = rig.hub.gateway.invite("builder-1", None, at, &ada()).await.unwrap_err().message;
    assert!(again.contains("is in this fleet already"), "{again}");
    // The one that is in is still the one that is in.
    let devices = rig.hub.gateway.devices().await.unwrap();
    assert_eq!(devices[0].id.as_deref(), Some(rig.member.gateway.id().as_str()));
}

#[tokio::test]
async fn a_device_removed_at_the_hub_is_told_so_by_the_hub_forgets_it_and_is_not_let_back_in() {
    let rig = Rig::linked().await;
    rig.hub.gateway.remove("builder-1", &ada()).await.unwrap();
    assert!(rig.hub.gateway.devices().await.unwrap().is_empty());
    // Told by the hub itself when it next asks to be let in, the member
    // is joined to nothing, and keeps what it was told for whoever looks.
    rig.until("the member to be told it is out", || rig.member.gateway.uplink().is_none()).await;
    assert!(rig.hub.gateway.hub().linked_since("builder-1").is_none());
    crate::soon("what the member was told to be kept and said", async {
        while rig.member.gateway.unjoined().await.is_none()
            || rig.member.topics("device.unjoined").await.is_empty()
        {
            tokio::task::yield_now().await;
        }
    })
    .await;
    let kept = rig.member.gateway.unjoined().await.unwrap();
    assert!(kept["why"].as_str().unwrap().contains("does not know the key"), "{kept}");
    assert_eq!(rig.member.topics("device.unjoined").await.len(), 1);
    // Removed again, there is nothing of that name to remove.
    assert!(rig.hub.gateway.remove("builder-1", &ada()).await.is_err());
}

#[tokio::test]
async fn what_answers_without_the_hubs_key_is_not_this_devices_hub() {
    let rig = Rig::linked().await;
    // Another installation where the hub was: one made again, a copy, or
    // somebody on the way. It has a key of its own, as every one does.
    let impostor = Device::new("hub", &rig.clock, &rig.shared).await;
    assert_ne!(impostor.gateway.id(), rig.hub.gateway.id());
    rig.wire.rewire(&impostor.gateway);
    let told = || rig.member.gateway.uplink().and_then(|link| link.status.error).unwrap_or_default();
    rig.until("the member to find it is not its hub", || told().contains("did not prove it is the hub"))
        .await;
    assert!(told().contains("Nothing it asks is done"), "{}", told());
    assert!(!rig.member.gateway.uplink().unwrap().status.connected);
    assert!(impostor.gateway.hub().linked_since("builder-1").is_none(), "no link was opened to it");
    assert_eq!(rig.member.topics("device.unproven").await.len(), 1, "said once, not at every try");
    // Its own hub back where it was, the member is its member again.
    rig.wire.rewire(&rig.hub.gateway);
    rig.until_linked().await;
}

#[tokio::test]
async fn a_place_refused_too_often_is_not_answered_for_a_while() {
    let rig = Rig::apart().await;
    let from: std::net::IpAddr = [203, 0, 113, 7].into();
    for _ in 0..5 {
        let refused = rig.hub.gateway.hello(b"not a handshake", from).await.unwrap_err();
        assert!(matches!(refused, Shut::Unread(_)), "{refused:?}");
    }
    let put_off = rig.hub.gateway.hello(b"not a handshake", from).await.unwrap_err();
    let Shut::Later { wait, .. } = put_off else { panic!("{put_off:?}") };
    assert!(wait <= Duration::from_secs(15 * 60) && wait > Duration::from_secs(14 * 60), "{wait:?}");
    // Another place is answered meanwhile; and this one again, after the while.
    let elsewhere = rig.hub.gateway.hello(b"not a handshake", [203, 0, 113, 8].into()).await.unwrap_err();
    assert!(matches!(elsewhere, Shut::Unread(_)));
    rig.clock.advance(Duration::from_secs(15 * 60 + 1));
    let after = rig.hub.gateway.hello(b"not a handshake", from).await.unwrap_err();
    assert!(matches!(after, Shut::Unread(_)), "{after:?}");
}

#[tokio::test]
async fn a_first_message_sent_again_by_somebody_else_ends_nothing_and_gets_them_nowhere() {
    let rig = Rig::linked().await;
    // Whoever watched the device shake hands has its first messages, and
    // sends the last of them again, over and over. Each is answered, as the
    // device's own was: there is no telling them apart yet.
    let seen = rig.wire.hellos.lock().unwrap().last().unwrap().clone();
    // From a new place each time, as one who means to go on would: from one
    // place, a few would be answered and the rest not at all.
    for place in 0..40u8 {
        let from: std::net::IpAddr = [203, 0, 114, place].into();
        let answer = rig.hub.gateway.hello(&seen, from).await.unwrap();
        // The answer names a session; without the keys of the handshake
        // there is nothing to say in it that opens.
        let mut asking = answer[..16].to_vec();
        asking.extend([0u8; 40]);
        assert!(rig.hub.gateway.sealed_link(&asking).await.is_err());
        assert!(rig.hub.gateway.sealed_report(&asking).await.is_err());
    }
    // The device's own session was not ended by any of it: work for it is
    // done over the link it had, which never dropped.
    let run = rig
        .hub
        .run("blueprint: afar\npermissions: [fleet]\nsteps:\n  look: { do: system.check, device: builder-1 }\n")
        .await;
    assert_eq!(run.status, conductor::RunStatus::Ok, "{run:?}");
    let links = rig.hub.topics("device.connected").await.len();
    assert_eq!(links, 1, "the link was never made again");
}
