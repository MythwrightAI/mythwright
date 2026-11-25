//! Secrets over the API: set by the owner, listed by name for whoever may
//! use them, handed to the program a blueprint runs, and given out by no
//! route.

use reqwest::{Method, StatusCode};
use serde_json::json;

use crate::Rig;

async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn a_secret_is_set_by_the_owner_listed_by_name_and_never_given_out() {
    let rig = Rig::new().await;
    let value = "sk_live_abc123";
    let (status, set) =
        rig.call(Method::PUT, "/api/secrets/stripe", Some(&rig.token), Some(json!({ "value": value }))).await;
    assert_eq!(status, StatusCode::OK, "{set}");
    // Listed by name, and that is all any route says of it.
    let listed = rig.read("/api/secrets").await;
    assert_eq!(listed["secrets"][0]["name"], "stripe");
    assert!(!listed.to_string().contains(value));
    let event = rig.event("secret.set", |_| true).await;
    assert_eq!(event.data["name"], "stripe");
    assert!(!serde_json::to_string(&*event).unwrap().contains(value));

    // Somebody who may use secrets sees which there are, and cannot set
    // one or take one away: that is the owner's.
    let user = key(&rig, &["secrets:use"]).await;
    let (status, _) = rig.call(Method::GET, "/api/secrets", Some(&user), None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, refused) =
        rig.call(Method::PUT, "/api/secrets/stripe", Some(&user), Some(json!({ "value": "mine" }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let (status, _) = rig.call(Method::DELETE, "/api/secrets/stripe", Some(&user), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // And somebody who may not, does not see even that.
    let other = key(&rig, &["blueprints"]).await;
    let (status, _) = rig.call(Method::GET, "/api/secrets", Some(&other), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // A name that could be a path is no name.
    let (status, refused) = rig
        .call(Method::PUT, "/api/secrets/..%2Fescape", Some(&rig.token), Some(json!({ "value": "x" })))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");

    // A blueprint is handed it by name, and what the blueprint gives out
    // does not have it unless its program printed it.
    // Its program says so only where the variable holds something.
    let has = crate::standin::cmd(&["given", "KEY", "say", "handed"]);
    let source = format!(
        "
blueprint: uses
permissions: [shell, secrets]
steps:
  have: {{ do: shell, env: {{ KEY: {{ secret: stripe }} }}, with: {{ cmd: {has} }} }}
outputs:
  said: ${{ steps.have.out.stdout }}
"
    );
    let applied = rig.send("/api/apply", json!({ "source": source, "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert_eq!(applied["finished"]["outputs"]["said"], "handed");
    assert!(!applied.to_string().contains(value));

    // Taken away, it is gone, and what names it says so.
    let (status, _) = rig.call(Method::DELETE, "/api/secrets/stripe", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(rig.read("/api/secrets").await["secrets"], json!([]));
    let again = rig.send("/api/apply", json!({ "source": source, "wait": true, "key": "second" })).await;
    assert_eq!(again["finished"]["status"], "failed", "{again}");
    assert!(
        again["finished"]["error"].as_str().unwrap().contains("there is no secret called `stripe`"),
        "{again}"
    );
}

/// The files under `dir` that hold `text`. A link is no file of the
/// folder and is not followed: what it names is looked at where it is,
/// where that is under the folder. The one link a factory makes that
/// leads out of its data is to its own program (`bin`), which is
/// nothing it keeps and in this suite is the test, 180 megabytes of
/// it: read through the link each time this was asked, it was two
/// seconds of a processor for each node looked through.
pub(crate) fn holding(dir: &std::path::Path, text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        let held =
            |path: &std::path::Path| crate::snapshots::holds(&std::fs::read(path).unwrap_or_default(), text);
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => found.extend(holding(&path, text)),
            Ok(kind) if kind.is_file() && held(&path) => found.push(path.display().to_string()),
            _ => {}
        }
    }
    found
}

/// Ends what a test's services run when the test is over, however it
/// ended. A test that starts a service stops the instance that owns it,
/// and then there is nothing left for this to do; one that fails before
/// it gets there would leave a program running that nobody ends, since a
/// factory leaves its services running on purpose. A service is ended by
/// its wrapper's group, and only while that process still says it is
/// that service's: a number alone may be another program's by then.
struct Ended<'a>(&'a Rig);

impl Drop for Ended<'_> {
    fn drop(&mut self) {
        for service in self.0.node().app.hosting.services() {
            let said = std::fs::read(format!("/proc/{}/environ", service.pid)).unwrap_or_default();
            let its_own = format!("svc.{}.", service.name);
            // Where a group is reached by its number (Unix), the number
            // has to be shown to be this service's still. Where it is a
            // hold this process has (Windows), nothing but what this
            // process started is ended by letting go of it.
            let still_its = said.windows(its_own.len()).any(|part| part == its_own.as_bytes());
            if service.pid > 1 && (still_its || cfg!(windows)) {
                // Said, so that what a failed test printed shows what was ended for it.
                eprintln!(
                    "the test was over with `{}` still running; its group {} was ended",
                    service.name, service.pid
                );
                let _ = hosting::group::stop(service.pid, true);
            }
        }
    }
}

/// Sets a secret as the owner.
async fn set(rig: &Rig, name: &str, value: &str) {
    let path = format!("/api/secrets/{name}");
    let (status, set) = rig.call(Method::PUT, &path, Some(&rig.token), Some(json!({ "value": value }))).await;
    assert_eq!(status, StatusCode::OK, "{set}");
}

#[tokio::test]
async fn the_owner_is_told_where_each_secret_is_named_and_what_is_named_and_not_kept_and_nobody_else_is() {
    let rig = Rig::new().await;
    let _ended = Ended(&rig);
    let value = "sk_live_abc123";
    set(&rig, "stripe", value).await;
    // An agent that is handed it, and one more that nobody has set.
    let handed = json!({ "name": "builder", "secrets": ["PAY_KEY=stripe", "MAIL=mail-password"] });
    rig.send("/api/agents", handed).await;
    // A service that is started with it, and a blueprint told which key by a param.
    let idles = crate::standin::cmd(&["idle"]);
    let serves = format!(
        "
blueprint: checkout
permissions: [fleet, shell, secrets]
steps:
  serve:
    do: service.start
    env: {{ STRIPE_KEY: {{ secret: stripe }} }}
    with: {{ name: checkout, restart: never, cmd: {idles} }}
"
    );
    let served = rig.send("/api/apply", json!({ "source": serves, "wait": true })).await;
    assert_eq!(served["finished"]["status"], "ok", "{served}");
    let told = "
blueprint: nightly
permissions: [fleet, shell, secrets]
params:
  key: { type: string }
steps:
  # secret: not-this-one, which is only said in a note
  send:
    do: service.start
    env: { KEY: { secret: \"${params.key}\" } }
    with: { name: mailer, restart: never, cmd: IDLES }
"
    .replace("IDLES", &idles);
    let asked = json!({ "source": told, "params": { "key": "stripe" }, "wait": true });
    let sending = rig.send("/api/apply", asked).await;
    assert_eq!(sending["finished"]["status"], "ok", "{sending}");
    // One that named it and is over names nothing any more: what is listed is what would fail.
    let once = "
blueprint: once
permissions: [shell, secrets]
steps:
  pay: { do: shell, env: { KEY: { secret: stripe } }, with: { run: 'exit 0' } }
";
    assert_eq!(
        rig.send("/api/apply", json!({ "source": once, "wait": true })).await["finished"]["status"],
        "ok"
    );

    let listed = rig.read("/api/secrets?named=true").await;
    // Somebody who may use secrets, to be asked the same below.
    let user = key(&rig, &["secrets:use", "agents", "blueprints"]).await;
    let (plain_status, names) = rig.call(Method::GET, "/api/secrets", Some(&user), None).await;
    let (named_status, refused) = rig.call(Method::GET, "/api/secrets?named=true", Some(&user), None).await;
    // The services are stopped before anything is held to: one left
    // running by a test that failed is a process nobody will end.
    for started in [&served, &sending] {
        rig.send(&format!("/api/instances/{}/stop", started["instance"].as_str().unwrap()), json!({})).await;
    }
    for service in ["checkout", "mailer"] {
        rig.event("service.stopped", |stopped| stopped.data["service"] == service).await;
    }

    assert!(!listed.to_string().contains(value), "{listed}");
    let all = listed["secrets"].as_array().unwrap();
    let one = |name: &str| {
        all.iter().find(|one| one["name"] == name).unwrap_or_else(|| panic!("{name} in {listed}"))
    };
    let stripe = one("stripe");
    assert_eq!((&stripe["kept"], &stripe["set_by"]), (&json!(true), &json!("Ada")), "{stripe}");
    assert!(stripe["set"].as_i64().is_some_and(|set| set > 0), "{stripe}");
    let agents = stripe["named"]["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 1, "{stripe}");
    assert_eq!((&agents[0]["name"], &agents[0]["variable"]), (&json!("builder"), &json!("PAY_KEY")));
    let mut services: Vec<(&str, &str)> = stripe["named"]["services"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| (one["name"].as_str().unwrap(), one["variable"].as_str().unwrap()))
        .collect();
    services.sort_unstable();
    assert_eq!(services, [("checkout", "STRIPE_KEY"), ("mailer", "KEY")], "{stripe}");
    let mut texts: Vec<&str> = stripe["named"]["instances"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| one["blueprint"].as_str().unwrap())
        .collect();
    texts.sort_unstable();
    assert_eq!(texts, ["checkout", "nightly"], "{stripe}");
    // Named by an agent's row and set by nobody: it is listed as that, so
    // that it is seen before the agent's turn fails for want of it.
    let missing = one("mail-password");
    assert_eq!(missing["kept"], false, "{missing}");
    assert_eq!(missing["named"]["agents"][0]["variable"], "MAIL");
    // What a note says is no naming, and nothing else is listed.
    assert_eq!(all.len(), 2, "{listed}");

    // Somebody who may use secrets sees which there are, and not who holds which.
    assert_eq!(plain_status, StatusCode::OK);
    assert!(names["secrets"][0]["named"].is_null(), "{names}");
    assert_eq!(named_status, StatusCode::FORBIDDEN, "{refused}");
    assert!(refused["error"]["message"].as_str().unwrap().contains("`admin`"), "{refused}");
}

#[tokio::test]
async fn what_a_program_was_handed_is_in_nothing_kept_of_what_it_printed_nor_in_a_copy_of_the_factory() {
    let shared = crate::snapshots::Shared::new();
    let prod = shared.factory("prod").await;
    // Made here and written nowhere, so that it is in no file but by
    // the factory's doing.
    let value = &format!("sk_live_{}", &kernel::id::secret()[..16]);
    let mark = "[the secret `stripe`]";
    set(&prod, "stripe", value).await;
    prod.send("/api/channels", json!({ "name": "general" })).await;
    // It prints its key, complains with it, gives it out as what it said,
    // and what it said is then posted in a channel.
    let source = "
blueprint: leaks
permissions: [shell, secrets, channels]
steps:
  pay:
    do: shell
    env: { STRIPE_KEY: { secret: stripe } }
    with: { cmd: SAYS }
  tell:
    needs: [pay]
    do: channel.post
    with: { channel: general, body: \"The shop says: ${steps.pay.out.stdout}\" }
outputs:
  said: ${ steps.pay.out.stdout }
"
    .replace("SAYS", &crate::standin::cmd(&["say", "paid with %STRIPE_KEY%", "err", "warned: %STRIPE_KEY%"]));
    let applied = prod.send("/api/apply", json!({ "source": source, "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert_eq!(applied["finished"]["outputs"]["said"], format!("paid with {mark}"));
    assert!(!applied.to_string().contains(value), "{applied}");
    // The step's log, as it is read back.
    let run = applied["run"].as_str().unwrap();
    let log = prod.read(&format!("/api/runs/{run}/steps/pay/log")).await.to_string();
    assert!(log.contains(&format!("warned: {mark}")) && !log.contains(value), "{log}");
    // What was said in the channel of it.
    let said = prod.read("/api/channels/general/messages").await.to_string();
    assert!(said.contains(&format!("The shop says: paid with {mark}")) && !said.contains(value), "{said}");
    // Every event the journal kept.
    let query = journal::EventQuery { topics: "**".into(), limit: 10_000, ..Default::default() };
    let events = prod.said(query).await;
    assert!(!serde_json::to_string(&events).unwrap().contains(value));

    // Nothing the factory keeps on the disk holds it as it is: where it is
    // kept it is sealed, and what the program printed never had it.
    let data = prod.dir.join("data");
    assert!(!holding(&data, "the secret `stripe`").is_empty(), "the looking would find it");
    assert_eq!(holding(&data, value), Vec::<String>::new());
    // And a copy of the factory has neither the secret nor what was said with it.
    let copy = shared.snapshot("copy");
    crate::snapshots::taken(&prod, &copy).await;
    assert_eq!(holding(&copy, value), Vec::<String>::new());
}
