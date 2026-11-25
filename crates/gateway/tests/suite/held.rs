//! What the door and the link are held to, beyond letting the right device
//! in: a member is answered wherever it is; a first message said before
//! gets nobody anywhere; a device taken out is left with nothing; what a
//! hub sends is done as no more than its member allows; and a member reads
//! no more than a hub would send.

use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use conductor::StepStatus;
use gateway::door::{Hello, Welcome};
use gateway::own::{Keys, Own};
use gateway::sealed::{self, Kind, Session};
use gateway::wire::{HELLO_PATH, LINK_PATH};
use gateway::{Carrier, Gateway, HttpCarrier, Invitation, Shut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{Rig, ada, soon, step};

/// A device shaking hands by hand: its keys, its hub's, and numbers that
/// go up from above anything the device itself has said in these tests.
struct ByHand {
    keys: Keys,
    hub: [u8; 32],
    next: AtomicU64,
}

impl ByHand {
    /// The rig's member, as whoever held its keys could speak for it.
    fn member(rig: &Rig) -> ByHand {
        ByHand::at(&rig.member.dir, rig)
    }

    /// The installation whose data is in `dir`, which the hub may never
    /// have heard of.
    fn at(dir: &Path, rig: &Rig) -> ByHand {
        let keys = Own::open(dir).unwrap().link().unwrap();
        let hub = Invitation::read(&rig.invitation).unwrap().hub_key();
        // Above anything the device itself says while these tests run (its
        // numbers are the time or a little over), and nowhere a clock
        // would not reach: a number beyond that is not one a hub may tell
        // a device to go on from.
        let above = rig.member.journal.now_ms() as u64 + 400 * 24 * 3600 * 1000;
        ByHand { keys, hub, next: AtomicU64::new(above) }
    }

    /// One handshake from `from`: the session's name, this end of the
    /// session, and what the hub said in its sealed answer.
    async fn shake(
        &self,
        hub: &Gateway,
        invitation: Option<String>,
        from: IpAddr,
    ) -> Result<(Vec<u8>, Session, Welcome), Shut> {
        let number = self.next.fetch_add(1, Ordering::SeqCst);
        let hello = serde_json::to_vec(&Hello { invitation, number }).unwrap();
        let offer = sealed::offer(&self.keys, &self.hub, &hello).unwrap();
        let answer = hub.hello(&offer.message, from).await?;
        let (session, said) = offer.accept(&answer[16..]).expect("the hub's own answer");
        Ok((answer[..16].to_vec(), session, serde_json::from_slice(&said).unwrap()))
    }
}

/// What a device sends to open its link in `session`.
fn opening(name: &[u8], session: &Session) -> Vec<u8> {
    let (_, frame) = session.seal(Kind::Open, &[]).unwrap();
    [name, &frame].concat()
}

fn secret_of(invitation: &str) -> String {
    Invitation::read(invitation).unwrap().secret
}

const JUNK: &[u8] = b"not a handshake";

#[tokio::test]
async fn a_member_is_answered_wherever_it_is_whatever_else_came_from_there() {
    let rig = Rig::linked().await;
    let member = ByHand::member(&rig);
    let shared: IpAddr = [203, 0, 113, 20].into();
    // Many devices can be behind one address. From it comes what cannot be
    // opened, until the hub stops answering such things from there.
    for _ in 0..5 {
        assert!(matches!(rig.hub.gateway.hello(JUNK, shared).await.unwrap_err(), Shut::Unread(_)));
    }
    assert!(matches!(rig.hub.gateway.hello(JUNK, shared).await.unwrap_err(), Shut::Later { .. }));

    // The member behind that address is let in all the same: it proved a
    // key that is a member's, and that is all that was asked of it.
    let (name, session, welcome) =
        member.shake(&rig.hub.gateway, None, shared).await.expect("a member is not put off");
    assert!(welcome.ok, "{:?}", welcome.error);
    assert!(rig.hub.gateway.sealed_link(&opening(&name, &session)).await.is_ok());
}

#[tokio::test]
async fn an_invitation_the_hub_holds_lets_its_device_in_whatever_else_came_from_there() {
    let rig = Rig::linked().await;
    let shared: IpAddr = [203, 0, 113, 22].into();
    for _ in 0..5 {
        assert!(matches!(rig.hub.gateway.hello(JUNK, shared).await.unwrap_err(), Shut::Unread(_)));
    }
    let newcomer = ByHand::at(&rig.hub.dir.join("newcomer"), &rig);
    // A guess at an invitation is not looked at from there for a while,
    // and neither is a key the hub does not know.
    let guessed = newcomer.shake(&rig.hub.gateway, Some("a-guess".into()), shared).await;
    assert!(matches!(guessed, Err(Shut::Later { .. })), "a guess is not tried from there");
    assert!(matches!(newcomer.shake(&rig.hub.gateway, None, shared).await, Err(Shut::Later { .. })));

    // The device the hub invited may be behind that same address (a proxy
    // in front of the hub, a tunnel, one router). What it brings cannot be
    // guessed, and it is let in.
    let invited = rig.hub.gateway.invite("builder-2", None, vec!["http://hub".into()], &ada()).await.unwrap();
    let (name, session, welcome) = newcomer
        .shake(&rig.hub.gateway, Some(secret_of(&invited.invitation)), shared)
        .await
        .expect("an invitation the hub holds is not put off");
    assert!(welcome.ok, "{:?}", welcome.error);
    assert_eq!(welcome.device.as_deref(), Some("builder-2"));
    assert!(rig.hub.gateway.sealed_link(&opening(&name, &session)).await.is_ok());
    // Used, it is an invitation the hub holds no longer: anyone else who
    // has it is put off from there like any other guess.
    let other = ByHand::at(&rig.hub.dir.join("other"), &rig);
    let late = other.shake(&rig.hub.gateway, Some(secret_of(&invited.invitation)), shared).await;
    assert!(matches!(late, Err(Shut::Later { .. })));
}

#[tokio::test]
async fn a_key_the_hub_does_not_know_is_told_so_each_time_and_it_is_written_down_once() {
    let rig = Rig::linked().await;
    let stranger = ByHand::at(&rig.hub.dir.join("stranger"), &rig);
    let from: IpAddr = [203, 0, 113, 30].into();
    for _ in 0..5 {
        let (_, _, welcome) = stranger.shake(&rig.hub.gateway, None, from).await.expect("a sealed answer");
        assert!(!welcome.ok);
        assert!(welcome.error.unwrap().contains("does not know the key"));
    }
    // Each cost the hub a handshake to answer: counted against where they
    // came from like anything else refused, and then not answered.
    for _ in 0..7 {
        assert!(matches!(stranger.shake(&rig.hub.gateway, None, from).await, Err(Shut::Later { .. })));
    }
    assert_eq!(rig.hub.topics("device.refused").await.len(), 1, "said once for a place, not twelve times");
}

#[tokio::test]
async fn refusals_from_more_places_than_are_remembered_are_still_said_now_and_then() {
    let rig = Rig::linked().await;
    let stranger = ByHand::at(&rig.hub.dir.join("stranger"), &rig);
    // One from each of more places than the hub remembers having said so of.
    for place in 0..1100u16 {
        let from: IpAddr = [198, 18, (place >> 8) as u8, place as u8].into();
        let (_, _, welcome) = stranger.shake(&rig.hub.gateway, None, from).await.expect("a sealed answer");
        assert!(!welcome.ok);
    }
    let refusals = || async {
        let query =
            journal::EventQuery { topics: "device.refused".into(), limit: 5000, ..Default::default() };
        rig.hub.journal.read(query).await.unwrap().len()
    };
    let said = refusals().await;
    assert!(
        (1024..=1026).contains(&said),
        "said of each place while there was room, and of few after: {said}"
    );
    // No room left to remember another place is not a reason to say nothing
    // of it: a while later, one more is said.
    rig.clock.advance(std::time::Duration::from_secs(61));
    let (_, _, welcome) = stranger.shake(&rig.hub.gateway, None, [198, 19, 0, 1].into()).await.unwrap();
    assert!(!welcome.ok);
    assert_eq!(refusals().await, said + 1);
}

#[tokio::test]
async fn a_first_message_said_before_is_turned_away_and_ends_nothing_of_the_devices() {
    let rig = Rig::linked().await;
    let member = ByHand::member(&rig);
    let from: IpAddr = [203, 0, 113, 40].into();
    // Whoever watched the device shake hands has its first message.
    let seen = rig.wire.hellos.lock().unwrap().last().unwrap().clone();

    // The device shakes hands anew, and before it has said anything in the
    // session it was given, the one that watched sends what it saw, again
    // and again.
    let (name, session, welcome) = member.shake(&rig.hub.gateway, None, from).await.unwrap();
    assert!(welcome.ok);
    let again: IpAddr = [203, 0, 113, 41].into();
    for _ in 0..5 {
        let answer = rig.hub.gateway.hello(&seen, again).await.unwrap();
        assert_eq!(&answer[..16], &[0u8; 16], "said before: no session is made for it");
    }
    // And counted against where it comes from, as anything refused is:
    // after a few it is not answered at all.
    for _ in 0..7 {
        assert!(matches!(rig.hub.gateway.hello(&seen, again).await, Err(Shut::Later { .. })));
    }
    // The session the device was given is still its own.
    assert!(
        rig.hub.gateway.sealed_link(&opening(&name, &session)).await.is_ok(),
        "what was sent again took nothing from the device"
    );
    assert!(rig.hub.topics("device.refused").await.is_empty(), "and nothing was written down for it");
}

#[tokio::test]
async fn a_device_that_proved_a_number_the_hub_has_passed_is_told_where_to_go_on_from() {
    let rig = Rig::linked().await;
    // Somebody speaks for the member with a number far above its own: the
    // member's own next handshake is below what the hub has taken.
    let member = ByHand::member(&rig);
    let (name, session, _) = member.shake(&rig.hub.gateway, None, [203, 0, 113, 50].into()).await.unwrap();
    rig.hub.gateway.sealed_link(&opening(&name, &session)).await.unwrap();
    // Its link was taken from it by that. It shakes hands again, is told
    // (sealed, by its hub) what the hub has taken, and goes on from there.
    rig.until_linked().await;
    let run = rig
        .hub
        .run("blueprint: afar\npermissions: [fleet]\nsteps:\n  look: { do: system.check, device: builder-1 }\n")
        .await;
    assert_eq!(run.status, conductor::RunStatus::Ok, "{run:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_device_taken_out_as_it_shakes_hands_is_left_with_no_session() {
    let rig = Rig::linked().await;
    let member = Arc::new(ByHand::member(&rig));
    let hub = rig.hub.gateway.clone();
    for round in 0..12u8 {
        // Many handshakes of the device's own, and its removal, at once.
        let mut shaking: Vec<_> = (0..16u8)
            .map(|one| {
                let (member, hub) = (member.clone(), hub.clone());
                tokio::spawn(async move { member.shake(&hub, None, [198, 51, round, one].into()).await.ok() })
            })
            .collect();
        // One of them is through, so the rest are on their way in as the
        // device is taken out.
        let first = shaking.remove(0).await.unwrap();
        rig.hub.gateway.remove("builder-1", &ada()).await.unwrap();
        let mut answered = vec![first];
        for shaken in shaking {
            answered.push(shaken.await.unwrap());
        }
        for shaken in answered {
            let Some((name, session, _)) = shaken else { continue };
            // Whatever it was answered while it was being taken out, there
            // is nothing it can now open.
            assert!(
                hub.sealed_link(&opening(&name, &session)).await.is_err(),
                "a session outlived the device it was for"
            );
        }
        // Let in again, for the next round.
        let invited = hub.invite("builder-1", None, vec!["http://hub".into()], &ada()).await.unwrap();
        let again = member.shake(&hub, Some(secret_of(&invited.invitation)), [198, 51, round, 200].into());
        assert!(again.await.unwrap().2.ok);
    }
}

#[tokio::test]
async fn an_invitation_is_taken_again_from_the_device_it_let_in_and_from_no_other() {
    let rig = Rig::apart().await;
    let member = ByHand::member(&rig);
    let secret = secret_of(&rig.invitation);
    let from: IpAddr = [203, 0, 113, 60].into();
    // The device is let in; the answer never reaches it; it tries again.
    assert!(member.shake(&rig.hub.gateway, Some(secret.clone()), from).await.unwrap().2.ok);
    let (name, session, welcome) = member.shake(&rig.hub.gateway, Some(secret.clone()), from).await.unwrap();
    assert!(welcome.ok, "{:?}", welcome.error);
    assert_eq!(welcome.device.as_deref(), Some("builder-1"));
    assert!(rig.hub.gateway.sealed_link(&opening(&name, &session)).await.is_ok());
    assert_eq!(rig.hub.gateway.devices().await.unwrap().len(), 1);
    // Anyone else who has it gets nowhere with it.
    let other = ByHand::at(&rig.hub.dir.join("other"), &rig);
    let (_, _, welcome) =
        other.shake(&rig.hub.gateway, Some(secret), [203, 0, 113, 61].into()).await.unwrap();
    assert!(welcome.error.unwrap().contains("that invitation has been used"));
}

#[tokio::test]
async fn a_device_let_in_under_a_new_name_has_left_the_old_one() {
    let rig = Rig::linked().await;
    assert!(rig.hub.gateway.hub().linked_since("builder-1").is_some());
    let invited = rig.hub.gateway.invite("builder-9", None, vec!["http://hub".into()], &ada()).await.unwrap();
    let member = ByHand::member(&rig);
    let from: IpAddr = [203, 0, 113, 70].into();
    let (_, _, welcome) =
        member.shake(&rig.hub.gateway, Some(secret_of(&invited.invitation)), from).await.unwrap();
    assert_eq!(welcome.device.as_deref(), Some("builder-9"));
    assert!(rig.hub.gateway.hub().linked_since("builder-1").is_none(), "its link under the old name is over");
    let names: Vec<String> =
        rig.hub.gateway.devices().await.unwrap().into_iter().map(|device| device.name).collect();
    assert_eq!(names, ["builder-9"]);
}

#[tokio::test]
async fn what_a_hub_sends_is_done_as_no_more_than_its_member_allows_all_the_way_down() {
    let rig = Rig::apart().await;
    // The member lets its hub apply blueprints on it, and run no commands.
    rig.link(vec!["blueprints:run".into()]).await;
    let marker = rig.member.dir.join("was-here");
    // The hub applies one there as someone who may do anything, and the
    // blueprint's own step is a command.
    let touches = crate::standin::cmd(&["write", &marker.to_string_lossy(), "here"]);
    let source = format!(
        "blueprint: outer\npermissions: [blueprints, shell]\nsteps:\n  there:\n    do: blueprint.apply\n    device: builder-1\n    allow_failure: true\n    with:\n      source: |\n        blueprint: inner\n        steps:\n          touch: {{ do: shell, with: {{ cmd: {touches} }} }}\n"
    );
    let run = rig.hub.run(&source).await;
    let there = step(&run, "there");
    assert_eq!(there.status, StepStatus::Failed, "{run:?}");
    assert!(there.error.as_deref().unwrap_or_default().contains("shell"), "{:?}", there.error);
    assert!(!marker.exists(), "the command was not run on the member");
}

#[tokio::test]
async fn what_is_too_much_to_say_at_once_fails_its_step_and_is_not_sent_for_ever() {
    let rig = Rig::linked().await;
    // Too much to send to the device.
    let much = "a".repeat(2_000_000);
    let source = format!(
        "blueprint: loud\nsteps:\n  shout: {{ do: test.where, device: builder-1, allow_failure: true, with: {{ say: \"{much}\" }} }}\n  after: {{ do: test.where, device: builder-1, needs: [shout] }}\n"
    );
    let run = rig.hub.run(&source).await;
    let shout = step(&run, "shout");
    assert!(
        shout.error.as_deref().unwrap_or_default().contains("the most that is said at once over a link"),
        "{:?}",
        shout.error
    );
    assert_eq!(step(&run, "after").status, StepStatus::Ok, "the link went on: {run:?}");

    // Too much for the device to send back.
    let run = rig
        .hub
        .run("blueprint: wordy\nsteps:\n  tell: { do: test.big, device: builder-1, allow_failure: true, with: { bytes: 3000000 } }\n  after: { do: test.where, device: builder-1, needs: [tell] }\n")
        .await;
    let tell = step(&run, "tell");
    assert!(
        tell.error.as_deref().unwrap_or_default().contains("what this step gave out on its device is"),
        "{:?}",
        tell.error
    );
    assert_eq!(step(&run, "after").status, StepStatus::Ok, "and the link went on: {run:?}");
}

/// Something at an address that answers every request the same way: with
/// `head`, then `body` over and over until `most` bytes have gone, and then
/// holds the connection open or closes it.
async fn answering(head: &'static str, most: usize, hold: bool) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let at = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut asked = [0u8; 4096];
                let _ = socket.read(&mut asked).await;
                let _ = socket.write_all(head.as_bytes()).await;
                let piece = vec![b'a'; 64 * 1024];
                let mut sent = 0;
                while sent < most && socket.write_all(&piece).await.is_ok() {
                    sent += piece.len();
                }
                let _ = socket.flush().await;
                if hold {
                    // Until whoever asked hangs up.
                    let _ = socket.read(&mut asked).await;
                }
            });
        }
    });
    at
}

#[tokio::test]
async fn a_member_reads_no_more_than_a_hub_would_send_from_whatever_answers() {
    const OK: &str = "HTTP/1.1 200 OK\r\ncontent-type: application/octet-stream\r\nconnection: close\r\n\r\n";
    // An answer that does not end.
    let at = answering(OK, 16 * 1024 * 1024, false).await;
    let told = HttpCarrier::new(&at).exchange(HELLO_PATH, Vec::new()).await.unwrap_err();
    assert!(told.message.contains("sent more than a hub sends at once"), "{}", told.message);

    // A refusal whose words do not end: a line of them is kept.
    const NO: &str = "HTTP/1.1 403 Forbidden\r\ncontent-type: text/plain\r\nconnection: close\r\n\r\n";
    let at = answering(NO, 16 * 1024 * 1024, false).await;
    let told = HttpCarrier::new(&at).exchange(HELLO_PATH, Vec::new()).await.unwrap_err();
    assert!(told.said && told.message.len() <= 200, "{} bytes", told.message.len());
    assert!(
        told.message.starts_with("it said, unproven: "),
        "words that came unsealed say so: {}",
        told.message
    );

    // A link that sends and sends and never ends a frame: it is ended here,
    // not read for as long as it is sent.
    const STREAM: &str = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n";
    let at = answering(STREAM, 16 * 1024 * 1024, true).await;
    let mut pieces = HttpCarrier::new(&at).stream(LINK_PATH, Vec::new()).await.unwrap();
    let ended = soon("the link that never ends a frame to be ended", pieces.recv()).await;
    assert!(ended.is_none());
}

#[tokio::test]
async fn a_member_goes_nowhere_it_is_sent_on_to_and_waits_no_longer_than_a_minute_for_words() {
    // Sent on elsewhere: it does not go.
    const ON: &str = "HTTP/1.1 307 Temporary Redirect\r\nlocation: http://127.0.0.1:1/elsewhere\r\ncontent-length: 0\r\nconnection: close\r\n\r\n";
    let at = answering(ON, 0, false).await;
    let told = HttpCarrier::new(&at).exchange(HELLO_PATH, Vec::new()).await.unwrap_err();
    assert!(told.message.contains("307"), "{}", told.message);

    // Told in the clear to stay away for an hour: a minute is all it is worth.
    const LATER: &str = "HTTP/1.1 429 Too Many Requests\r\nretry-after: 3600\r\ncontent-length: 0\r\nconnection: close\r\n\r\n";
    let at = answering(LATER, 0, false).await;
    let told = HttpCarrier::new(&at).exchange(HELLO_PATH, Vec::new()).await.unwrap_err();
    assert_eq!(told.retry_after, Some(std::time::Duration::from_secs(60)));
}

#[tokio::test]
async fn an_address_that_is_not_plain_http_is_not_somewhere_a_link_goes() {
    let rig = Rig::apart().await;
    let mut elsewhere = Invitation::read(&rig.invitation).unwrap();
    elsewhere.at = vec!["file:///etc/passwd".into()];
    let said = rig.member.gateway.join(&elsewhere.written(), None, &ada()).await.unwrap_err().message;
    assert!(said.contains("is not somewhere a link goes"), "{said}");
    // And nothing that prints an invitation prints what lets a device in.
    let shown = format!("{:?}", Invitation::read(&rig.invitation).unwrap());
    assert!(!shown.contains(&secret_of(&rig.invitation)), "{shown}");
}

#[tokio::test]
async fn what_a_hub_sends_as_environment_reaches_a_program_only_where_the_hub_may_run_commands() {
    const SENT: &str = "blueprint: preloaded\nenv: { LD_PRELOAD: /tmp/theirs.so }\nsteps:\n  there: { do: test.env, device: builder-1 }\n";
    // A member that lets its hub run commands is given the environment the
    // hub sends with one: it decides what a program does as arguments do.
    let rig = Rig::linked().await;
    let run = rig.hub.run(SENT).await;
    assert_eq!(step(&run, "there").output["env"]["LD_PRELOAD"], "/tmp/theirs.so", "{run:?}");

    // One that does not is not: what it does run for its hub (a package's
    // program, an action of its own) runs in its own environment, and the
    // step's log on the hub says what was left out.
    let rig = Rig::apart().await;
    rig.link(vec!["fleet".into()]).await;
    let run = rig.hub.run(SENT).await;
    let there = step(&run, "there");
    assert_eq!(there.status, StepStatus::Ok, "{run:?}");
    assert_eq!(there.output["env"], serde_json::json!({}), "nothing of the hub's environment got there");
    let said: Vec<String> =
        rig.hub.engine.step_log(&run.id, "there", 0, 100).into_iter().map(|line| line.text).collect();
    assert!(
        said.iter().any(|line| line.contains("LD_PRELOAD") && line.contains("was left out")),
        "the hub's run shows it: {said:?}"
    );
}

#[tokio::test]
async fn a_step_sent_to_a_device_names_the_secret_it_is_to_be_handed_there_and_the_device_says_whether() {
    const SENT: &str = "blueprint: keyed\nsteps:\n  there:\n    do: test.secrets\n    device: builder-1\n    env: { DEPLOY_KEY: { secret: deploy } }\n";
    // What goes to the device is the name, by the variable it is to be in:
    // the hub has nothing under that name, and is not asked for anything.
    let rig = Rig::linked().await;
    let run = rig.hub.run(SENT).await;
    let there = step(&run, "there");
    assert_eq!(there.status, StepStatus::Ok, "{run:?}");
    assert_eq!(there.output["secrets"], serde_json::json!({ "DEPLOY_KEY": "deploy" }));

    // A device that does not let its hub use what it keeps secret, or run
    // commands on it, hands nothing over, and says which it is that it
    // does not allow. The step fails: a program started without what it
    // was to be handed would fail further on, saying less.
    for (allowed, lacks) in [(vec!["fleet", "shell"], "secrets:use"), (vec!["fleet", "secrets"], "shell")] {
        let rig = Rig::apart().await;
        rig.link(allowed.iter().map(|scope| scope.to_string()).collect()).await;
        let run = rig.hub.run(SENT).await;
        let there = step(&run, "there");
        assert_eq!(there.status, StepStatus::Failed, "{allowed:?}: {run:?}");
        let why = there.error.clone().unwrap_or_default();
        assert!(
            why.contains("does not let its hub hand a secret it keeps to a program")
                && why.contains(&format!("(`{lacks}`)")),
            "{allowed:?}: {why}"
        );
    }
}

#[tokio::test]
async fn a_step_under_an_environment_sent_to_a_device_is_refused_its_environments_secret_there_and_never_given_the_devices_own()
 {
    const SENT: &str = "blueprint: keyed\nsteps:\n  there:\n    do: test.secrets\n    device: builder-1\n    env: { DEPLOY_KEY: { secret: deploy } }\n";
    let rig = Rig::linked().await;
    let engine = &rig.hub.engine;
    let source = || kernel::Source::actor("user:ada");
    engine.make_environment("shop", "prod", &ada(), source()).await.unwrap();
    let keep =
        |environment: &mut conductor::environs::Environment| environment.keep_secret("deploy", &ada(), 1);
    engine.change_environment("shop", "prod", keep, &ada(), source()).await.unwrap();
    // The hub's runner hands on the name the environment keeps it under;
    // the device keeps none of a project's, and says so rather than hand
    // a secret of its own of the same name in its place.
    let applied = rig.hub.apply_under(SENT, Some("prod")).await;
    let run = rig.hub.finished(&applied).await;
    let there = step(&run, "there");
    assert_eq!(there.status, StepStatus::Failed, "{run:?}");
    let why = there.error.clone().unwrap_or_default();
    assert!(
        why.contains("keeps none of a project's environments' secrets: `deploy` of the `prod` environment"),
        "{why}"
    );
    // A secret the environment does not list is the factory's, the device's
    // own there, as it always was.
    let other = SENT.replace("{ secret: deploy }", "{ secret: other }").replace("keyed", "plain");
    let applied = rig.hub.apply_under(&other, Some("prod")).await;
    let run = rig.hub.finished(&applied).await;
    assert_eq!(
        step(&run, "there").output["secrets"],
        serde_json::json!({ "DEPLOY_KEY": "other" }),
        "{run:?}"
    );
}

#[tokio::test]
async fn answers_that_are_too_much_together_go_a_few_at_a_time_and_all_arrive() {
    let rig = Rig::linked().await;
    // Forty steps on the device, each giving out a fair amount, all held at
    // a gate there.
    let steps: String = (0..40)
        .map(|n| {
            format!("  s{n}: {{ do: test.big, device: builder-1, needs: [], with: {{ bytes: 60000, gate: pile }} }}\n")
        })
        .collect();
    let applied = rig.hub.apply(&format!("blueprint: pile\nmax_parallel: 40\nsteps:\n{steps}")).await;
    rig.shared.reached("pile", 40).await;
    // The link drops, and all forty finish while it is down: their answers
    // wait on the device, more of them together than one report carries,
    // though none is much by itself.
    rig.wire.cut();
    rig.until_unlinked().await;
    for _ in 0..40 {
        rig.shared.open("pile");
    }
    rig.wire.restore();
    rig.until_linked().await;
    // They go a few at a time, and every one arrives as it was.
    let run = rig.hub.finished(&applied).await;
    assert_eq!(run.status, conductor::RunStatus::Ok, "{:?}", run.error);
    for n in 0..40 {
        let said = step(&run, &format!("s{n}")).output["said"].as_str().map_or(0, str::len);
        assert_eq!(said, 60000, "step s{n} arrived whole");
    }
}

#[tokio::test]
async fn asking_for_the_link_is_given_up_when_nothing_answers_it_and_tried_again() {
    let rig = Rig::linked().await;
    // What is at the hub's address takes the asking for the link and never
    // answers it.
    rig.wire.mute.store(true, Ordering::SeqCst);
    rig.wire.cut();
    rig.wire.restore();
    let told = || rig.member.gateway.uplink().and_then(|link| link.status.error).unwrap_or_default();
    soon("the member to give up on what never answers", async {
        while !told().contains("did not answer in time") {
            // The silence a link is allowed, and a little: as often as it
            // takes for the member to be waiting when it passes.
            rig.clock.advance(std::time::Duration::from_secs(61));
            tokio::task::yield_now().await;
        }
    })
    .await;
    // It is not left waiting there: it tries again as after any failure,
    // and when the hub answers, it is linked.
    rig.wire.mute.store(false, Ordering::SeqCst);
    rig.until_linked().await;
    let run = rig
        .hub
        .run("blueprint: afar\npermissions: [fleet]\nsteps:\n  look: { do: system.check, device: builder-1 }\n")
        .await;
    assert_eq!(run.status, conductor::RunStatus::Ok, "{run:?}");
}

#[tokio::test]
async fn a_wait_asked_for_in_the_clear_is_never_shorter_than_the_members_own() {
    let rig = Rig::linked().await;
    // Whatever answers asks the member to come straight back, every time.
    *rig.wire.retry_after.lock().unwrap() = Some(std::time::Duration::ZERO);
    rig.wire.cut();
    rig.until_unlinked().await;
    let waits = soon("the member to schedule an attempt after a refusal", async {
        loop {
            let status = rig.member.gateway.uplink().unwrap().status;
            if let (Some(at), true) = (status.retry_at, status.failures >= 4) {
                break at - rig.member.journal.now_ms();
            }
            rig.clock.advance(std::time::Duration::from_secs(1));
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(waits >= 4_000, "it waits as long as it would have anyway, and longer each time: {waits} ms");
}

#[tokio::test]
async fn a_number_no_clock_will_reach_is_not_taken_from_a_hub_and_each_hub_has_a_count_of_its_own() {
    let rig = Rig::linked().await;
    let hub_id = rig.hub.gateway.id();
    let count = rig.member.dir.join("gateway").join(format!("hub.{hub_id}.said"));
    let counted = || std::fs::read_to_string(&count).ok().and_then(|kept| kept.trim().parse::<u64>().ok());
    let before = counted().expect("what this device has said to this hub is counted under the hub's key");
    assert!(rig.member.dir.join("gateway/hub.json").is_file(), "beside what it keeps of the hub, each whole");

    // Somebody speaks for the device with the highest number there is, so
    // that the hub, quite truthfully, tells the device to go on from there.
    let mut far = ByHand::member(&rig);
    far.next = AtomicU64::new(u64::MAX);
    let (name, session, welcome) = far.shake(&rig.hub.gateway, None, [203, 0, 113, 80].into()).await.unwrap();
    assert!(welcome.ok);
    rig.hub.gateway.sealed_link(&opening(&name, &session)).await.unwrap();
    // The device does not: taken, it would have no number left to say, to
    // this hub or any other. It says what it was told and stays where it is.
    let told = || rig.member.gateway.uplink().and_then(|link| link.status.error).unwrap_or_default();
    rig.until("the member to refuse a number no clock will reach", || told().contains("no clock will reach"))
        .await;
    let after = counted().unwrap();
    assert!(after >= before && after < u64::MAX / 2, "its count went on as it was: {after}");

    // Leaving the hub, it keeps nothing of what it said to it.
    rig.member.gateway.leave(&ada()).await.unwrap();
    assert!(!count.exists(), "the count went with the hub it was for");
}

#[tokio::test]
async fn a_session_nobody_spoke_in_is_over_after_a_minute_and_when_the_device_speaks_in_a_later_one() {
    let rig = Rig::linked().await;
    let member = ByHand::member(&rig);
    let from: IpAddr = [203, 0, 113, 90].into();
    // An answer that is held back on its way, and the one after it that
    // the device uses.
    let (held_name, held_session, _) = member.shake(&rig.hub.gateway, None, from).await.unwrap();
    let (name, session, _) = member.shake(&rig.hub.gateway, None, from).await.unwrap();
    rig.hub.gateway.sealed_link(&opening(&name, &session)).await.unwrap();
    // Delivered late, from wherever it was held, the earlier one opens
    // nothing and takes nothing from the device.
    assert!(
        rig.hub.gateway.sealed_link(&opening(&held_name, &held_session)).await.is_err(),
        "an answer from before the one the device spoke in is over"
    );

    // And one that nothing is said in at all is over in a minute.
    let (late_name, late_session, _) = member.shake(&rig.hub.gateway, None, from).await.unwrap();
    rig.clock.advance(std::time::Duration::from_secs(61));
    assert!(
        rig.hub.gateway.sealed_link(&opening(&late_name, &late_session)).await.is_err(),
        "a device speaks at once; an answer unused for a minute was not its to use"
    );
}

#[tokio::test]
async fn what_a_hub_sends_is_done_on_its_member_as_nobodys_key() {
    let rig = Rig::linked().await;
    // Applied on the hub with a key somebody made there. Which key that
    // is means something among the hub's keys and nothing among the
    // member's, where a key is withdrawn by its id alone.
    let source =
        "blueprint: keyed\nsteps:\n  here: { do: test.who }\n  there: { do: test.who, device: builder-1 }\n";
    let applied = rig
        .hub
        .engine
        .apply(conductor::Apply {
            source: Some(source.to_string()),
            ..conductor::Apply::new(
                "keyed",
                kernel::Principal { key: Some("cred_made_on_the_hub".into()), ..crate::ada() },
            )
        })
        .await
        .unwrap();
    let run = rig.hub.finished(&applied).await;
    assert_eq!(step(&run, "here").output["key"], "cred_made_on_the_hub");
    assert_eq!(step(&run, "there").output["device"], "member-box");
    assert_eq!(step(&run, "there").output["key"], serde_json::Value::Null);
}
