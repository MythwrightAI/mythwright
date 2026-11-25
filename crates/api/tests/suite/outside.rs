//! An event said from outside: by a script, by another program, by a
//! service somewhere that calls an address when something has happened.
//! Whoever says it holds a key that lets it say that topic, and a key that
//! is written into an address can do nothing else. A service that signs
//! what it sends is a caller the owner described, and is taken at its word
//! only when the signature is one its secret makes.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, KeyInit, Mac};
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};
use sha2::Sha256;

use crate::Rig;

/// Counts what it is told has been paid, and keeps the last order.
const SHOP: &str = "\
blueprint: shop
state: { paid: 0, last: none }
on:
  paid:
    event: shop.paid
    do: set
    with: { paid: ${ state.paid + 1 }, last: \"${ event.data.order }\" }
";

async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "outside", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

/// What a service that holds `secret` signs `message` as.
fn signature(secret: &[u8], message: &[u8]) -> Vec<u8> {
    let mut signing = Hmac::<Sha256>::new_from_slice(secret).unwrap();
    signing.update(message);
    signing.finalize().into_bytes().to_vec()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Seconds since 1970, as a service says when it signed.
fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

/// A call as a service makes it: no key, no session, these bytes and
/// these headers.
async fn called(rig: &Rig, path: &str, headers: &[(&str, &str)], body: &[u8]) -> (StatusCode, Value) {
    let mut request = reqwest::Client::new().post(format!("{}{path}", rig.url)).body(body.to_vec());
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = request.send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    (status, serde_json::from_str(&text).unwrap_or(Value::String(text)))
}

async fn secret(rig: &Rig, name: &str, value: &str) {
    let (status, set) = rig
        .call(Method::PUT, &format!("/api/secrets/{name}"), Some(&rig.token), Some(json!({ "value": value })))
        .await;
    assert_eq!(status, StatusCode::OK, "{set}");
}

async fn caller(rig: &Rig, name: &str, described: Value) -> (StatusCode, Value) {
    rig.call(Method::PUT, &format!("/api/callers/{name}"), Some(&rig.token), Some(described)).await
}

fn said(body: &Value) -> String {
    body["error"]["message"].as_str().unwrap_or_default().to_string()
}

async fn shop(rig: &Rig) -> String {
    let applied = rig.send("/api/apply", json!({ "source": SHOP, "wait": true })).await;
    let id = applied["instance"].as_str().unwrap().to_string();
    rig.event("instance.active", |event| event.source.instance.as_deref() == Some(id.as_str())).await;
    id
}

async fn state(rig: &Rig, id: &str) -> Value {
    let read = rig.read(&format!("/api/instances/{id}")).await;
    read["instance"]["state"].clone()
}

#[tokio::test]
async fn an_event_is_said_from_outside_by_whoever_holds_a_key_for_its_topic_and_a_handler_hears_it() {
    let rig = Rig::new().await;
    let id = shop(&rig).await;
    let paid = key(&rig, &["emit:shop.paid"]).await;

    let (status, answer) = rig
        .call(Method::POST, "/api/emit/shop.paid", Some(&paid), Some(json!({ "order": "o-17", "total": 40 })))
        .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["topic"], "shop.paid");
    rig.event("run.finished", |event| event.data["flow"] == "on.paid").await;
    assert_eq!(state(&rig, &id).await, json!({ "paid": 1, "last": "o-17" }));
    // What it carries is what was sent, and it says which key said it: by
    // the id the factory gave the key, which nobody chooses.
    let event = rig.event("shop.paid", |_| true).await;
    assert_eq!(event.data, json!({ "order": "o-17", "total": 40 }));
    assert_eq!(rig.number(event.seq), answer["seq"].as_u64().unwrap());
    let keys = rig.read("/api/credentials").await;
    let id = keys["credentials"][0]["id"].as_str().unwrap();
    assert_eq!(event.source.actor.as_deref(), Some(format!("key:{id}").as_str()), "{keys}");

    // The key says one topic, and not another.
    let (status, refused) =
        rig.call(Method::POST, "/api/emit/shop.refunded", Some(&paid), Some(json!({}))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("needs `emit:shop.refunded`"), "{refused}");
    // Nor can it do anything else at all.
    let (status, _) = rig.call(Method::GET, "/api/instances", Some(&paid), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // Nobody says anything without a key.
    let (status, _) = rig.call(Method::POST, "/api/emit/shop.paid", None, Some(json!({}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // A person who is signed in says what they like of a blueprint's own,
    // and is said to have.
    let (status, answer) = rig.post("/api/emit/shop.paid", json!({ "order": "o-18" })).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let event = rig.event("shop.paid", |event| event.data["order"] == "o-18").await;
    assert!(
        event.source.actor.as_deref().is_some_and(|actor| actor.starts_with("user:")),
        "{:?}",
        event.source
    );
}

#[tokio::test]
async fn what_the_system_says_of_itself_cannot_be_said_from_outside_and_what_is_said_is_a_mapping() {
    let rig = Rig::new().await;
    // Not even by one who may do everything.
    for topic in ["instance.failed", "board.item.moved", "run.finished", "service.started"] {
        let (status, refused) = rig.post(&format!("/api/emit/{topic}"), json!({})).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{topic}: {refused}");
        assert!(said(&refused).contains("only the system may publish under"), "{topic}: {refused}");
    }
    let (status, refused) = rig.post("/api/emit/Shop Paid", json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("is not a topic"), "{refused}");
    let (status, refused) = rig.post("/api/emit/shop.paid", json!([1, 2])).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("what an event carries is a mapping"), "{refused}");
    assert_eq!(rig.count_of("shop.paid").await, 0);
}

#[tokio::test]
async fn a_service_that_can_only_be_given_an_address_is_given_one_with_a_key_that_says_events_and_nothing_else()
 {
    let rig = Rig::new().await;
    let id = shop(&rig).await;
    let narrow = key(&rig, &["emit:shop.paid"]).await;

    // No header, no session: the address is all it has.
    let (status, answer) = rig
        .call(Method::POST, &format!("/in/{narrow}/shop.paid"), None, Some(json!({ "order": "o-21" })))
        .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    rig.event("run.finished", |event| event.data["flow"] == "on.paid").await;
    assert_eq!(state(&rig, &id).await["last"], "o-21");
    // Still one topic.
    let (status, _) =
        rig.call(Method::POST, &format!("/in/{narrow}/shop.refunded"), None, Some(json!({}))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // A key that can do more is not taken in an address: an address is
    // written down wherever it passes.
    let wide = key(&rig, &["emit", "blueprints"]).await;
    let (status, refused) = rig
        .call(Method::POST, &format!("/in/{wide}/shop.paid"), None, Some(json!({ "order": "o-22" })))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("holds more than saying events"), "{refused}");
    // And one that is no key at all is nobody.
    let (status, _) = rig.call(Method::POST, "/in/key_nothing/shop.paid", None, Some(json!({}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(rig.count_of("shop.paid").await, 1);
}

#[tokio::test]
async fn a_service_that_signs_what_it_sends_is_taken_only_when_the_signature_is_its_own() {
    let rig = Rig::new().await;
    let id = shop(&rig).await;
    let signing = "s3cr3t-of-the-payments-service";
    secret(&rig, "pay-signing", signing).await;
    let (status, made) = caller(
        &rig,
        "payments",
        json!({ "topic": "shop.paid", "secret": "pay-signing", "header": "X-Signature", "prefix": "sha256=" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{made}");
    assert_eq!(made["address"], "/in/payments");
    let described = rig.event("caller.set", |_| true).await;
    assert_eq!(described.data["name"], "payments");
    assert_eq!(described.data["topic"], "shop.paid");

    // Signed with the secret, over the bytes that are sent: taken, and said
    // to have been said by that caller.
    let body = br#"{"order":"o-31","total":40}"#;
    let right = format!("sha256={}", hex(&signature(signing.as_bytes(), body)));
    let (status, answer) = called(&rig, "/in/payments", &[("X-Signature", &right)], body).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["topic"], "shop.paid");
    rig.event("run.finished", |event| event.data["flow"] == "on.paid").await;
    assert_eq!(state(&rig, &id).await, json!({ "paid": 1, "last": "o-31" }));
    let event = rig.event("shop.paid", |_| true).await;
    assert_eq!(event.source.actor.as_deref(), Some("caller:payments"));
    assert_eq!(event.data, json!({ "order": "o-31", "total": 40 }));

    // Signed with another secret.
    let other = format!("sha256={}", hex(&signature(b"a guess", body)));
    let (status, refused) = called(&rig, "/in/payments", &[("X-Signature", &other)], body).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{refused}");
    assert!(said(&refused).contains("does not match what was sent"), "{refused}");
    // The right signature, over something else than arrived.
    let changed = br#"{"order":"o-31","total":4000}"#;
    let (status, refused) = called(&rig, "/in/payments", &[("X-Signature", &right)], changed).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{refused}");
    // No signature at all, and one that is not written as one.
    let (status, refused) = called(&rig, "/in/payments", &[], body).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{refused}");
    assert!(said(&refused).contains("has no `X-Signature` header"), "{refused}");
    let (status, refused) = called(&rig, "/in/payments", &[("X-Signature", "sha256=zz")], body).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{refused}");
    let (status, refused) = called(&rig, "/in/payments", &[("X-Signature", "md5=00")], body).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{refused}");
    assert!(said(&refused).contains("no signature after `sha256=`"), "{refused}");
    // A key does not open this address, and nobody is called `nobody`.
    let (status, _) = called(&rig, "/in/nobody", &[("X-Signature", &right)], body).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // Nothing that was refused was said, and nothing said gives the secret.
    assert_eq!(rig.count_of("shop.paid").await, 1);
    for told in [&refused, &made, &rig.read("/api/callers").await] {
        assert!(!told.to_string().contains(signing), "{told}");
    }
    assert!(!serde_json::to_string(&*described).unwrap().contains(signing));

    // Who is let in from outside is the owner's to say and to see.
    let member = key(&rig, &["blueprints", "emit", "secrets:use"]).await;
    let (status, _) = rig.call(Method::GET, "/api/callers", Some(&member), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let mine = json!({ "topic": "shop.paid", "secret": "pay-signing", "header": "X-Mine" });
    let (status, _) = rig.call(Method::PUT, "/api/callers/mine", Some(&member), Some(mine)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = rig.call(Method::DELETE, "/api/callers/payments", Some(&member), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Taken away, its address answers nobody, however well they sign.
    let (status, _) = rig.call(Method::DELETE, "/api/callers/payments", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    rig.event("caller.removed", |event| event.data["name"] == "payments").await;
    let (status, _) = called(&rig, "/in/payments", &[("X-Signature", &right)], body).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(rig.count_of("shop.paid").await, 1);
}

/// The second of the published examples of this way of signing (RFC 4231):
/// a key, a text and the signature they make, written down long before any
/// of this was. What signs in these tests and what checks in the daemon are
/// the same library, and this is what says the library's answer is the
/// world's.
#[tokio::test]
async fn a_signature_made_elsewhere_by_the_published_rule_is_one_this_checks() {
    let rig = Rig::new().await;
    secret(&rig, "published", "Jefe").await;
    let described = json!({ "topic": "shop.paid", "secret": "published", "header": "X-Signature" });
    assert_eq!(caller(&rig, "example", described).await.0, StatusCode::OK);
    let text = b"what do ya want for nothing?";
    let published = "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843";

    // It checks, and so the call gets as far as being read: what was signed
    // is no mapping, which is all that is wrong with it.
    let (status, refused) = called(&rig, "/in/example", &[("X-Signature", published)], text).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("what an event carries is a mapping"), "{refused}");
    // One digit of it changed, and it does not.
    let nearly = published.replace("5bdc", "5bdd");
    let (status, refused) = called(&rig, "/in/example", &[("X-Signature", &nearly)], text).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{refused}");
    assert_eq!(rig.count_of("shop.paid").await, 0);
}

#[tokio::test]
async fn a_call_that_says_when_it_was_signed_is_not_taken_late_and_is_said_once_however_often_it_is_sent() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let signing = "s3cr3t-with-a-time";
    secret(&rig, "pay-signing", signing).await;
    let described = json!({
        "topic": "shop.paid", "secret": "pay-signing", "header": "Pay-Signature",
        "prefix": "v1=", "time_prefix": "t=", "signed": "{time}.{body}",
    });
    assert_eq!(caller(&rig, "payments", described).await.0, StatusCode::OK);
    let body = br#"{"order":"o-40"}"#;
    let sign = |at: u64, secret: &str| {
        let message = [at.to_string().as_bytes(), b".", body.as_slice()].concat();
        hex(&signature(secret.as_bytes(), &message))
    };

    // Signed just now.
    let at = now();
    let header = format!("t={at},v1={}", sign(at, signing));
    let (status, first) = called(&rig, "/in/payments", &[("Pay-Signature", &header)], body).await;
    assert_eq!(status, StatusCode::OK, "{first}");
    // Sent again, by the service that did not hear the answer or by someone
    // who saw the call pass: answered as it was, and said no second time.
    let (status, again) = called(&rig, "/in/payments", &[("Pay-Signature", &header)], body).await;
    assert_eq!(status, StatusCode::OK, "{again}");
    assert_eq!(again["seq"], first["seq"]);
    assert_eq!(again["again"], true);
    assert_eq!(rig.count_of("shop.paid").await, 1);
    // Written in other letters it is the same signature, and the same call.
    let shouted = format!("t={at},v1={}", sign(at, signing).to_uppercase());
    let (status, again) = called(&rig, "/in/payments", &[("Pay-Signature", &shouted)], body).await;
    assert_eq!((status, &again["again"]), (StatusCode::OK, &json!(true)), "{again}");
    assert_eq!(rig.count_of("shop.paid").await, 1);

    // Signed an hour ago, rightly: too old to be taken.
    let old = at - 3600;
    let header = format!("t={old},v1={}", sign(old, signing));
    let (status, refused) = called(&rig, "/in/payments", &[("Pay-Signature", &header)], body).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{refused}");
    assert!(said(&refused).contains("more than 5m from now"), "{refused}");
    // That old signature under a time of now: the time was signed too.
    let header = format!("t={at},v1={}", sign(old, signing));
    let (status, refused) = called(&rig, "/in/payments", &[("Pay-Signature", &header)], body).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{refused}");
    assert!(said(&refused).contains("does not match"), "{refused}");
    // No time said.
    let header = format!("v1={}", sign(at, signing));
    let (status, refused) = called(&rig, "/in/payments", &[("Pay-Signature", &header)], body).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{refused}");
    assert!(said(&refused).contains("does not say when this was signed"), "{refused}");

    // A service that is changing its secret signs with both for a while:
    // either one is its own.
    let later = at + 1;
    let header = format!("t={later},v1={},v1={}", sign(later, "the secret before"), sign(later, signing));
    let (status, answer) = called(&rig, "/in/payments", &[("Pay-Signature", &header)], body).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(rig.count_of("shop.paid").await, 2);
}

#[tokio::test]
async fn a_service_that_signs_an_id_and_a_time_from_headers_with_a_key_in_base64_is_described_and_checked() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let key = b"\x00\x01 not text at all \xff\xfe";
    secret(&rig, "hooks", &STANDARD.encode(key)).await;
    let described = json!({
        "topic": "shop.paid", "secret": "hooks", "key": "base64",
        "header": "Msg-Signature", "prefix": "v1,", "encoding": "base64",
        "time_header": "Msg-Timestamp", "signed": "{header:msg-id}.{time}.{body}", "within": "2m",
    });
    assert_eq!(caller(&rig, "hooks", described).await.0, StatusCode::OK);
    let body = br#"{"order":"o-50"}"#;
    let at = now().to_string();
    let message = ["msg_1.".as_bytes(), at.as_bytes(), b".", body.as_slice()].concat();
    let header =
        format!("v0,{} v1,{}", STANDARD.encode(b"an older way"), STANDARD.encode(signature(key, &message)));
    let headers = [("Msg-Id", "msg_1"), ("Msg-Timestamp", at.as_str()), ("Msg-Signature", header.as_str())];
    let (status, answer) = called(&rig, "/in/hooks", &headers, body).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    // The id was signed: the same call under another is not the same call.
    let headers = [("Msg-Id", "msg_2"), ("Msg-Timestamp", at.as_str()), ("Msg-Signature", header.as_str())];
    let (status, refused) = called(&rig, "/in/hooks", &headers, body).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{refused}");
    assert_eq!(rig.count_of("shop.paid").await, 1);
}

#[tokio::test]
async fn a_description_of_a_caller_that_would_prove_nothing_is_refused_when_it_is_given() {
    let rig = Rig::new().await;
    secret(&rig, "pay-signing", "s3cr3t").await;
    let base = json!({ "topic": "shop.paid", "secret": "pay-signing", "header": "X-Signature" });
    let with = |more: Value| {
        let mut described = base.clone();
        for (key, value) in more.as_object().unwrap() {
            described[key] = value.clone();
        }
        described
    };
    for (described, why) in [
        // A signature that does not cover what was sent.
        (with(json!({ "signed": "{time}", "time_prefix": "t=" })), "has no `{body}` in it"),
        // A time nobody signed can be written by anyone.
        (with(json!({ "time_header": "X-Time" })), "a time that was not signed"),
        (with(json!({ "signed": "{time}.{body}" })), "nothing says where the time is"),
        (with(json!({ "signed": "{body}{nonce}" })), "what can stand there"),
        (with(json!({ "encoding": "text" })), "written as `hex` or as `base64`"),
        (with(json!({ "within": "soon", "time_prefix": "t=", "signed": "{time}{body}" })), "`within`"),
        // What the system says of itself is not a caller's to say either.
        (with(json!({ "topic": "instance.failed" })), "only the system may publish under"),
        (with(json!({ "secret": "never-set" })), "there is no secret called `never-set`"),
        (with(json!({ "signd": "{body}" })), "does not describe a caller"),
    ] {
        let (status, refused) = caller(&rig, "payments", described.clone()).await;
        assert!(status.is_client_error(), "{described}: {refused}");
        assert!(said(&refused).contains(why), "{described}: {refused}");
    }
    let (status, refused) = caller(&rig, "Pay Ments", base.clone()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("cannot be a caller's name"), "{refused}");
    assert_eq!(rig.read("/api/callers").await["callers"], json!([]));
}
