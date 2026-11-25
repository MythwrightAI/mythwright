//! Two daemons, one fleet: a hub and a member joined over HTTP, with
//! something on the wire between them that keeps every byte that crosses.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::{Rig, soon};

/// What stands on the wire between a device and its hub in these tests.
/// It hands on every byte either way and keeps what it handed on; asked
/// to, it changes one letter of the next thing the hub sends down a link.
struct Recorder {
    url: String,
    to_hub: Arc<Mutex<Vec<u8>>>,
    to_device: Arc<Mutex<Vec<u8>>>,
    spoil: Arc<AtomicBool>,
}

async fn hand_on(
    mut from: tokio::net::tcp::OwnedReadHalf,
    mut to: tokio::net::tcp::OwnedWriteHalf,
    kept: Arc<Mutex<Vec<u8>>>,
    spoil: Option<Arc<AtomicBool>>,
) {
    let mut chunk = vec![0u8; 64 * 1024];
    while let Ok(read) = from.read(&mut chunk).await {
        if read == 0 {
            break;
        }
        let piece = &mut chunk[..read];
        let line = piece.windows(6).position(|window| window == b"data: ");
        if let (Some(at), Some(spoil)) = (line, &spoil)
            && at + 30 < read
            && spoil.swap(false, Ordering::SeqCst)
        {
            // One letter of a sealed frame, changed for another letter.
            piece[at + 26] = if piece[at + 26] == b'A' { b'B' } else { b'A' };
        }
        kept.lock().unwrap().extend_from_slice(piece);
        if to.write_all(piece).await.is_err() {
            break;
        }
    }
}

impl Recorder {
    /// Stands in front of the daemon at `hub`: what is sent to it goes there.
    async fn before(hub: SocketAddr) -> Recorder {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (to_hub, to_device) = (Arc::<Mutex<Vec<u8>>>::default(), Arc::<Mutex<Vec<u8>>>::default());
        let spoil = Arc::new(AtomicBool::new(false));
        tokio::spawn({
            let (to_hub, to_device, spoil) = (to_hub.clone(), to_device.clone(), spoil.clone());
            async move {
                while let Ok((device, _)) = listener.accept().await {
                    let Ok(hub) = TcpStream::connect(hub).await else { continue };
                    let ((from_device, back), (from_hub, on)) = (device.into_split(), hub.into_split());
                    tokio::spawn(hand_on(from_device, on, to_hub.clone(), None));
                    tokio::spawn(hand_on(from_hub, back, to_device.clone(), Some(spoil.clone())));
                }
            }
        });
        Recorder { url, to_hub, to_device, spoil }
    }

    /// Everything that crossed, either way.
    fn crossed(&self) -> Vec<u8> {
        [self.to_hub.lock().unwrap().clone(), self.to_device.lock().unwrap().clone()].concat()
    }

    fn saw(&self, words: &str) -> bool {
        self.crossed().windows(words.len()).any(|window| window == words.as_bytes())
    }

    /// The body of the first request the device made to `path`, as it crossed.
    fn sent_to(&self, path: &str) -> Vec<u8> {
        let crossed = self.to_hub.lock().unwrap().clone();
        let line = format!("POST {path} ");
        let at = crossed.windows(line.len()).position(|window| window == line.as_bytes()).expect("a request");
        let head = crossed[at..].windows(4).position(|window| window == b"\r\n\r\n").unwrap() + at + 4;
        let headers = String::from_utf8_lossy(&crossed[at..head]).to_lowercase();
        let length: usize = headers
            .lines()
            .find_map(|header| header.strip_prefix("content-length:"))
            .and_then(|length| length.trim().parse().ok())
            .expect("its length");
        crossed[head..head + length].to_vec()
    }
}

struct Fleet {
    hub: Rig,
    member: Rig,
    wire: Recorder,
    invitation: String,
}

/// A hub and a member that has joined it as `builder-1`, by an invitation
/// that says to reach the hub through the recorder.
async fn fleet() -> Fleet {
    let hub = Rig::new().await;
    let member = Rig::device("member-box").await;
    let wire = Recorder::before(hub.node().addr).await;
    let invited = hub.send("/api/devices", json!({ "name": "builder-1", "at": wire.url })).await;
    let invitation = invited["invitation"].as_str().unwrap().to_string();
    assert!(invitation.starts_with("inv_"), "{invited}");
    assert_eq!(
        invited["join"],
        format!("{} device join {invitation}", brand::CLI),
        "the answer is the command to run on the new device"
    );
    assert_eq!(invited["note"], json!(null), "it was told where it is reached: {invited}");
    // Invited, it is a device that has yet to come.
    let waiting = hub.read("/api/devices").await["devices"][0].clone();
    assert_eq!((&waiting["name"], &waiting["connected"]), (&json!("builder-1"), &json!(false)));
    assert!(waiting["invited"].is_i64() && waiting["id"].is_null(), "{waiting}");

    let joined = member.send("/api/gateway/join", json!({ "invitation": invitation })).await;
    assert_eq!(joined["hub"]["url"], wire.url);
    hub.event("device.connected", |event| event.data["device"] == "builder-1").await;
    Fleet { hub, member, wire, invitation }
}

fn said(answer: &Value) -> &str {
    answer["error"]["message"].as_str().unwrap_or_default()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_member_joins_a_hub_by_an_invitation_and_nothing_between_them_reads_what_they_say() {
    let Fleet { hub, member, wire, invitation } = fleet().await;

    // Each end knows the other, and by the key it proved.
    let seen = soon("the member to introduce itself", async {
        loop {
            let devices = hub.read("/api/devices").await;
            if !devices["devices"][0]["info"].is_null() {
                break devices;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!((&seen["this"], &seen["hub"]), (&json!("test-device"), &json!(null)));
    let device = &seen["devices"][0];
    assert_eq!((&device["name"], &device["connected"]), (&json!("builder-1"), &json!(true)));
    assert!(device["info"]["cpus"].as_u64().unwrap() > 0 && device["invited"].is_null(), "{device}");
    let link = soon("the member to be told its name", async {
        loop {
            let link = member.read("/api/devices").await["hub"].clone();
            if link["device"] == "builder-1" {
                break link;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!((&link["connected"], &link["hub"]), (&json!(true), &json!("test-device")));
    let theirs = member.read("/api/devices").await;
    assert_eq!(device["id"], theirs["id"], "the hub sees the key the member proves");
    assert_eq!(link["id"], seen["id"], "and the member the hub's");
    assert_ne!(seen["id"], theirs["id"]);
    assert_eq!(
        hub.read("/api/fleet").await["devices"][0]["name"],
        "builder-1",
        "the fleet's reading has it too"
    );

    // A step for the member runs there; its output and its log come back.
    let marker = member.dir.join("made-here");
    let does = crate::standin::cmd(&["say", "over here", "write", &marker.to_string_lossy(), "made"]);
    let source = format!(
        "blueprint: afar\npermissions: [shell]\nsteps:\n  there:\n    do: shell\n    device: builder-1\n    with: {{ cmd: {does} }}\n"
    );
    let applied = hub.send("/api/apply", json!({ "source": source, "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert!(marker.exists(), "the command ran on the member's side");
    let run = applied["run"].as_str().unwrap();
    let log = hub.read(&format!("/api/runs/{run}/steps/there/log")).await;
    assert_eq!(log["lines"][0]["text"], "over here");

    // All of it crossed the wire, and none of it could be read there: not
    // what was asked, not what came back, not who the device is, not what
    // let it in.
    assert!(wire.saw("POST /api/gateway/hello") && wire.saw("POST /api/gateway/report"));
    let secret = gateway::Invitation::read(&invitation).unwrap().secret;
    for hidden in ["over here", "made-here", "builder-1", "member-box", "test-device", "shell", &secret] {
        assert!(!wire.saw(hidden), "`{hidden}` crossed where it could be read");
    }
    // Nor is the invitation, or the installation's secret, in anything
    // either daemon said happened.
    for (rig, name) in [(&hub, "the hub"), (&member, "the member")] {
        let events = rig.read("/api/events/recent?limit=1000").await.to_string();
        let own = std::fs::read_to_string(rig.dir.join("data/installation.secret")).unwrap();
        assert!(!events.contains(&secret) && !events.contains(own.trim()), "{name} said too much");
    }

    // Removed from the fleet, its session is ended and its key opens nothing.
    let (status, _) = hub.call(Method::DELETE, "/api/devices/builder-1", Some(&hub.token), None).await;
    assert_eq!(status, StatusCode::OK);
    hub.event("device.removed", |event| event.data["device"] == "builder-1").await;
    assert_eq!(hub.read("/api/devices").await["devices"], json!([]));
    member.send("/api/gateway/leave", json!({})).await;
    assert_eq!(member.read("/api/devices").await["hub"], json!(null));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_step_for_another_device_is_handed_the_secret_that_device_keeps_and_the_hub_never_has_it() {
    let Fleet { hub, member, .. } = fleet().await;
    // Each keeps one under the same name, and they are not the same.
    let (theirs, ours) = ("kept-on-the-member-and-nowhere-else", "kept-on-the-hub");
    for (rig, value) in [(&member, theirs), (&hub, ours)] {
        let set = json!({ "value": value });
        let (status, answer) =
            rig.call(Method::PUT, "/api/secrets/deploy", Some(&rig.token), Some(set)).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
    }
    // What the program should find, written where only the member reads
    // it: the blueprint says the name and never the value.
    let expected = member.dir.join("expected");
    std::fs::write(&expected, theirs).unwrap();
    // It says so only where what it was handed is what was expected.
    let has = crate::standin::cmd(&["holds", &expected.to_string_lossy(), "%KEY%", "say", "handed"]);
    let source = |secret: &str| {
        format!(
            "blueprint: afar\npermissions: [shell, secrets]\nsteps:\n  there:\n    do: shell\n    device: builder-1\n    env: {{ KEY: {{ secret: {secret} }} }}\n    with: {{ cmd: {has} }}\n"
        )
    };
    let applied = hub.send("/api/apply", json!({ "source": source("deploy"), "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let run = applied["run"].as_str().unwrap();
    let log = hub.read(&format!("/api/runs/{run}/steps/there/log")).await;
    assert_eq!(log["lines"][0]["text"], "handed", "the member's own, not the hub's: {log}");
    // The hub was never sent it: nothing it answered or said has it.
    let said = hub.read("/api/events/recent?limit=1000").await.to_string();
    assert!(!said.contains(theirs) && !applied.to_string().contains(theirs));
    assert!(!hub.read(&format!("/api/runs/{run}")).await.to_string().contains(theirs));

    // One the member does not keep fails the step there, saying which.
    let applied = hub.send("/api/apply", json!({ "source": source("never-set"), "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "failed", "{applied}");
    assert!(applied.to_string().contains("there is no secret called `never-set`"), "{applied}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_step_for_another_device_that_prints_what_it_was_handed_gives_its_hub_the_secrets_name_and_never_its_value()
 {
    let Fleet { hub, member, .. } = fleet().await;
    // Kept on the member and nowhere else, and made here and written
    // nowhere, so that it is in no file but by a factory's doing.
    let theirs = &format!("kept-on-the-member-{}", &kernel::id::secret()[..16]);
    let set = json!({ "value": theirs });
    let (status, answer) =
        member.call(Method::PUT, "/api/secrets/deploy", Some(&member.token), Some(set)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    // The program prints its key, and complains with it: what it says
    // goes back to the hub, which is where the step's log is kept.
    let says = crate::standin::cmd(&["say", "handed %KEY%", "err", "warned of %KEY%"]);
    let source = format!(
        "blueprint: afar-said\npermissions: [shell, secrets]\nsteps:\n  there:\n    do: shell\n    device: builder-1\n    env: {{ KEY: {{ secret: deploy }} }}\n    with: {{ cmd: {says} }}\noutputs:\n  said: ${{ steps.there.out.stdout }}\n"
    );
    let applied = hub.send("/api/apply", json!({ "source": source, "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let mark = "[the secret `deploy`]";
    assert_eq!(applied["finished"]["outputs"]["said"], format!("handed {mark}"), "{applied}");
    let run = applied["run"].as_str().unwrap();
    let log = hub.read(&format!("/api/runs/{run}/steps/there/log")).await.to_string();
    assert!(log.contains(&format!("handed {mark}")) && log.contains(&format!("warned of {mark}")), "{log}");
    // It was taken out on the device that ran the program, before
    // anything was sent: nothing the hub answered, said or keeps has it,
    // and the member keeps it only where it is sealed.
    let said = hub.read("/api/events/recent?limit=1000").await.to_string();
    assert!(
        !said.contains(theirs.as_str())
            && !applied.to_string().contains(theirs.as_str())
            && !log.contains(theirs.as_str())
    );
    assert!(!member.read("/api/events/recent?limit=1000").await.to_string().contains(theirs.as_str()));
    for rig in [&hub, &member] {
        assert_eq!(crate::secrets::holding(&rig.dir.join("data"), theirs), Vec::<String>::new());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn what_a_step_for_another_device_prints_is_said_there_as_taking_what_set_its_run_off() {
    use crate::told::{RESTED, following, key, told_until_rested};
    let Fleet { hub, member, .. } = fleet().await;
    hub.send("/api/projects", json!({ "name": "Life" })).await;
    // A handler on the hub hears a record of a table kept private change,
    // and has the member print what the record is called.
    let says = crate::standin::cmd(&["say", "it went badly for %TITLE%"]);
    let source = format!(
        "blueprint: afar-heard\npermissions: [projects, records, shell]\nsteps:\n  moods:\n    do: collection.ensure\n    with:\n      name: moods\n      reads: private:journal\n      fields: {{ title: {{ type: text, required: true }} }}\non:\n  changed:\n    event: record.updated\n    for: {{ project: life }}\n    steps:\n      read: {{ do: record.get, with: {{ id: \"${{event.data.id}}\" }} }}\n      there:\n        do: shell\n        device: builder-1\n        env: {{ TITLE: \"${{steps.read.out.data.title}}\" }}\n        with: {{ cmd: {says} }}\n"
    );
    let applied = hub.send("/api/apply", json!({ "source": source, "project": "life", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    hub.send("/api/collections/life/moods/records", json!({ "fields": { "title": "Saturday" } })).await;
    // On the member, its owner and somebody who was not given the
    // journal there each follow what is said.
    let other = key(&member, kernel::principal::MEMBER_SCOPES).await;
    let (to_the_other, to_the_owner) =
        (following(&member, &other).await, following(&member, &member.token).await);

    let mut runs = hub.follow("run.finished");
    let change = json!({ "fields": { "title": "quince" } });
    let (status, changed) =
        hub.call(Method::PATCH, "/api/records/life-moods-1", Some(&hub.token), Some(change)).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    let ended = soon("the handler's run to end", async {
        loop {
            let event = runs.next().await.unwrap();
            if event.data["flow"] == "on.changed" {
                return event;
            }
        }
    })
    .await;
    assert_eq!(ended.data["status"], "ok", "{ended:?}");
    // The line was said on the member before its hub had the step's
    // answer; what is said there now is after it.
    let rested = member.node().app.journal.emit(RESTED, kernel::Source::actor("system"), json!({})).await;
    rested.unwrap();

    let line = |event: &&Value| event["topic"] == "log.line" && event.to_string().contains("quince");
    let owner = told_until_rested(to_the_owner, 1).await;
    let said =
        owner.iter().find(line).unwrap_or_else(|| panic!("the member's owner is told the line: {owner:?}"));
    assert_eq!(said["data"]["text"], "it went badly for quince", "{said}");
    assert_eq!(said["source"]["takes"], "private:journal", "{said}");
    let told = told_until_rested(to_the_other, 1).await;
    assert!(!told.iter().any(|event| event.to_string().contains("quince")), "{told:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn joining_is_refused_with_a_reason_when_it_cannot_work() {
    let Fleet { hub, member: _member, wire, invitation } = fleet().await;
    let other = Rig::device("other-box").await;
    let join = async |body: Value| other.post("/api/gateway/join", body).await;

    // An invitation lets one device in, once.
    let (status, used) = join(json!({ "invitation": invitation })).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(said(&used).contains("that invitation has been used"), "{used}");

    // The way it was done before is said to be gone, with what to do.
    let (status, before) = join(json!({ "url": hub.url, "token": "dev_0123456789" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(said(&before).contains("no longer joins by an address and a credential"), "{before}");
    assert!(said(&before).contains("device invite <name>"), "{before}");

    let (status, nonsense) = join(json!({ "invitation": hub.token })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a person's session is no invitation");
    assert!(said(&nonsense).contains("that is not an invitation"), "{nonsense}");

    // One that names a place where nothing is: said, and nothing is kept.
    let nowhere = hub.send("/api/devices", json!({ "name": "builder-2", "at": "http://127.0.0.1:9" })).await;
    let (status, unreached) = join(json!({ "invitation": nowhere["invitation"] })).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(said(&unreached), "could not reach the hub at http://127.0.0.1:9: nothing is answering there");
    assert_eq!(
        other.read("/api/devices").await["hub"],
        json!(null),
        "nothing was kept from the failed tries"
    );

    // Somewhere that answers and is not the hub that made the invitation:
    // the other daemon, which has a key of its own.
    let astray = hub.send("/api/devices", json!({ "name": "builder-3", "at": other.url })).await;
    let third = Rig::device("third-box").await;
    let (status, unproven) =
        third.post("/api/gateway/join", json!({ "invitation": astray["invitation"] })).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(said(&unproven).contains("did not prove it is the hub"), "{unproven}");

    // An invitation says where the hub is over plain HTTP, and for how long at most.
    for (wrong, why) in [
        (json!({ "name": "builder-4", "at": "https://hub.example.com" }), "looks like `http://host:port`"),
        (json!({ "name": "builder-4", "expires": "15d" }), "no more than fourteen"),
        (json!({ "name": "builder-4", "expires": "soon" }), "is not how long an invitation works"),
        (json!({ "name": "Builder 4" }), "lowercase letters, digits and dashes"),
    ] {
        let (status, refused) = hub.post("/api/devices", wrong.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{wrong}");
        assert!(said(&refused).contains(why), "{refused}");
    }
    // Left to tell for itself, it says where it listens, and that nothing
    // else can reach it there.
    let guessed = hub.send("/api/devices", json!({ "name": "builder-4" })).await;
    assert_eq!(guessed["at"][0], hub.url, "{guessed}");
    assert!(guessed["note"].as_str().unwrap().contains("only listens on 127.0.0.1"), "{guessed}");

    // Only an admin invites.
    let narrow = hub.send("/api/credentials", json!({ "name": "looker", "scopes": ["fleet:read"] })).await;
    let key = narrow["token"].as_str().unwrap();
    let (status, _) =
        hub.call(Method::POST, "/api/devices", Some(key), Some(json!({ "name": "builder-5" }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // And the link's own doors open to nothing but what is sealed for them:
    // no session of a person's, no words, no bytes of somebody's choosing.
    let raw = async |path: &str, body: Vec<u8>, token: Option<&str>| {
        let mut request = reqwest::Client::new().post(format!("{}{path}", hub.url)).body(body);
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        let response = request.send().await.unwrap();
        (response.status(), response.json::<Value>().await.unwrap_or_default())
    };
    for token in [None, Some(hub.token.as_str())] {
        let (status, link) = raw(gateway::wire::LINK_PATH, vec![7u8; 64], token).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{link}");
        assert!(said(&link).contains("holds no such session"), "{link}");
        let (status, report) = raw(gateway::wire::REPORT_PATH, Vec::new(), token).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{report}");
        let (status, hello) = raw(gateway::wire::HELLO_PATH, vec![7u8; 96], token).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{hello}");
        assert!(said(&hello).contains("was not made for this installation's key"), "{hello}");
    }
    let (status, _) = hub.call(Method::GET, gateway::wire::LINK_PATH, Some(&hub.token), None).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "there is no asking for a link in the clear");
    drop(wire);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_invitation_of_a_hub_that_listens_on_every_address_never_says_every_address() {
    let hub = Rig::owned(
        Rig::with("api-everywhere", |config, _| config.listen = SocketAddr::from(([0, 0, 0, 0], 0))).await,
    )
    .await;
    let port = hub.node().addr.port();
    let local = format!("http://127.0.0.1:{port}");
    // Asked from this machine with nothing else known: where it was asked,
    // and that another device cannot reach it there.
    let guessed = hub.send("/api/devices", json!({ "name": "builder-1" })).await;
    assert_eq!(guessed["at"], json!([local]), "{guessed}");
    assert!(guessed["note"].as_str().unwrap().contains("listens on every address"), "{guessed}");
    // A name it was told is its own is offered beside it, and then nothing
    // need be said.
    hub.send("/api/system/names", json!({ "name": "box.example.net" })).await;
    let named = hub.send("/api/devices", json!({ "name": "builder-2" })).await;
    assert_eq!(named["at"], json!([local, format!("http://box.example.net:{port}")]), "{named}");
    assert!(named["note"].is_null(), "{named}");
    assert!(!named.to_string().contains("0.0.0.0"), "{named}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_report_that_crossed_once_is_not_taken_again() {
    let Fleet { hub, member: _member, wire, .. } = fleet().await;
    soon("the member's first report to cross", async {
        while !wire.saw("POST /api/gateway/report")
            || hub.read("/api/devices").await["devices"][0]["info"].is_null()
        {
            tokio::task::yield_now().await;
        }
    })
    .await;
    // Whoever is on the wire has the report as it crossed, and sends it again.
    let report = wire.sent_to(gateway::wire::REPORT_PATH);
    let again = reqwest::Client::new()
        .post(format!("{}{}", hub.url, gateway::wire::REPORT_PATH))
        .body(report.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(again.status(), StatusCode::UNAUTHORIZED);
    let refused: Value = again.json().await.unwrap();
    assert!(said(&refused).contains("said before"), "{refused}");
    // Changed by a byte, it is not the device's at all.
    let mut changed = report;
    let last = changed.len() - 1;
    changed[last] ^= 1;
    let changed = reqwest::Client::new()
        .post(format!("{}{}", hub.url, gateway::wire::REPORT_PATH))
        .body(changed)
        .send()
        .await;
    assert_eq!(changed.unwrap().status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_command_changed_on_the_way_is_not_done_and_the_link_is_made_again() {
    let Fleet { hub, member, wire, .. } = fleet().await;
    let marker = member.dir.join("made-once");
    // The next thing the hub sends down the link has one letter changed.
    wire.spoil.store(true, Ordering::SeqCst);
    let does = crate::standin::cmd(&["append", &marker.to_string_lossy(), "ran"]);
    let source = format!(
        "blueprint: afar\npermissions: [shell]\nsteps:\n  there:\n    do: shell\n    device: builder-1\n    with: {{ cmd: {does} }}\n"
    );
    let applied = hub.send("/api/apply", json!({ "source": source, "wait": true })).await;
    // The member would not take it, shook hands again, was sent what was
    // still waiting for it, and did it: once.
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert!(!wire.spoil.load(Ordering::SeqCst), "something was changed on the way");
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "ran\n");
    assert!(hub.count_of("device.connected").await >= 2, "the link was made again");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_member_remembers_its_hub_across_a_restart_and_keeps_no_secret_of_the_hubs() {
    let Fleet { hub, mut member, invitation, .. } = fleet().await;
    let file = member.dir.join("data/gateway/hub.json");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let secret = member.dir.join("data/installation.secret");
        for private in [&file, &secret] {
            let mode = std::fs::metadata(private).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "{} is its owner's alone", private.display());
        }
    }
    // What it keeps of its hub is where it is and the key it must prove:
    // nothing that would let anyone else in.
    let kept = std::fs::read_to_string(&file).unwrap();
    let invited = gateway::Invitation::read(&invitation).unwrap();
    assert!(kept.contains(&invited.key) && !kept.contains(&invited.secret), "{kept}");

    member.restart().await;
    // The new daemon links by itself, with a handshake of its own: nobody
    // has to join again.
    let seq = hub.mark();
    let mut follow = hub.follow_after(seq.saturating_sub(50), "device.connected");
    let mut links = 0;
    soon("the member to link again", async {
        while links < 2 {
            follow.next().await.unwrap();
            links += 1;
        }
    })
    .await;
    let applied = hub
        .send(
            "/api/apply",
            json!({ "source": "blueprint: again\npermissions: [fleet]\nsteps:\n  look: { do: system.check, device: builder-1 }\n", "wait": true }),
        )
        .await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");

    // A device joined the way it was done before is not joined: it says so
    // once, with what to do, and puts what it had aside.
    member.stop().await;
    std::fs::write(&file, json!({ "url": hub.url, "token": "dev_0123456789", "allow": ["*"] }).to_string())
        .unwrap();
    member.start().await;
    let told = member.event("device.unjoined", |_| true).await;
    assert!(told.data["why"].as_str().unwrap().contains("device invite <name>"), "{:?}", told.data);
    assert_eq!(member.read("/api/devices").await["hub"], json!(null));
    assert!(!file.exists(), "it is not found again at the next start");
}
