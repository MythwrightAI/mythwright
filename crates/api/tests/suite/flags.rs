//! A project's feature flags, through the routes: a flag set in
//! one environment is that environment's variable and nobody else's, in
//! the write that says so, with the change kept in its row's history; a
//! turning on where the environment's rule names who approves waits for
//! one of them, and a turning off never does; a key that holds less is
//! refused, and asks as itself; a flag past its day is told to whose it
//! is, once; and a percentage flag is started as a comparison of two ways
//! given its part and the rest.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::held::said;
use crate::kept::{key, person};

/// Shop, with its environments (`dev`, `prod`) and its flags.
async fn shop() -> Rig {
    let rig = Rig::new().await;
    let made =
        rig.send("/api/projects", json!({ "name": "Shop", "set_up": ["environments", "flags"] })).await;
    assert!(made["set_up"]["flags"].is_string(), "{made}");
    rig
}

/// A row of Shop's flags.
async fn flag(rig: &Rig, fields: Value) -> String {
    let row = rig.send("/api/collections/shop/flags/records", json!({ "fields": fields })).await;
    row["id"].as_str().unwrap().to_string()
}

async fn set(rig: &Rig, token: &str, flag: &str, environment: &str, value: Value) -> (StatusCode, Value) {
    let path = format!("/api/projects/shop/flags/{flag}/{environment}");
    rig.call(Method::PUT, &path, Some(token), Some(json!({ "value": value }))).await
}

/// What `environment` says of its variable `variable`, as its own route
/// shows it.
async fn variable(rig: &Rig, environment: &str, variable: &str) -> Value {
    let shown = rig.read(&format!("/api/projects/shop/environments/{environment}")).await;
    shown["vars"][variable]["value"].clone()
}

/// What the flags' own route says `flag` says in `environment`.
async fn says(rig: &Rig, flag: &str, environment: &str) -> Value {
    let listed = rig.read("/api/projects/shop/flags").await;
    let one =
        listed["flags"].as_array().unwrap().iter().find(|one| one["flag"] == json!(flag)).unwrap().clone();
    one["says"][environment].clone()
}

#[tokio::test]
async fn a_flag_set_in_one_environment_is_its_variable_there_alone_and_its_history_keeps_the_change() {
    let rig = shop().await;
    let row = flag(&rig, json!({ "name": "New checkout", "about": "The one-page checkout" })).await;

    let (status, came) = set(&rig, &rig.token, "new-checkout", "dev", json!("on")).await;
    assert_eq!((status, &came["came"]), (StatusCode::OK, &json!("set")), "{came}");
    // Said in the write that keeps it, naming the flag's row.
    let changed = rig.event("project.flag.changed", |event| event.data["environment"] == "dev").await;
    assert_eq!(
        (&changed.data["record"], &changed.data["variable"]),
        (&json!(row), &json!("FLAG_NEW_CHECKOUT"))
    );
    assert_eq!((&changed.data["value"], &changed.data["was"]), (&json!("on"), &Value::Null));
    // dev's variable, and nothing of prod's.
    assert_eq!(variable(&rig, "dev", "FLAG_NEW_CHECKOUT").await, json!("on"));
    assert_eq!(variable(&rig, "prod", "FLAG_NEW_CHECKOUT").await, Value::Null);
    assert_eq!(says(&rig, "new-checkout", "dev").await, json!({ "value": "on", "shown": "on" }));
    assert_eq!(says(&rig, "new-checkout", "prod").await, json!({ "value": null, "shown": "off" }));

    // What it says already is nothing more; what is not a switch's value is refused.
    let (status, came) = set(&rig, &rig.token, "new-checkout", "dev", json!(true)).await;
    assert_eq!((status, &came["came"]), (StatusCode::OK, &json!("same")), "{came}");
    let (status, refused) = set(&rig, &rig.token, "new-checkout", "dev", json!("maybe")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("`on` or `off`"), "{refused}");
    let (status, refused) = set(&rig, &rig.token, "old-checkout", "dev", json!("on")).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");

    // A flag's variable is the flag's: the environment's own route leaves it to it.
    let path = "/api/projects/shop/environments/dev/vars/FLAG_NEW_CHECKOUT";
    let (status, refused) =
        rig.call(Method::PUT, path, Some(&rig.token), Some(json!({ "value": "off" }))).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(said(&refused).contains("flag set"), "{refused}");

    // Each change is a line of the row's history.
    let lines = rig.event("collection.history", |event| event.data["topic"] == "project.flag.changed").await;
    assert_eq!(lines.data["record"], json!(row));
    let history = rig.read(&format!("/api/records/{row}/history")).await;
    let words: Vec<&str> =
        history["history"].as_array().unwrap().iter().filter_map(|line| line["said"].as_str()).collect();
    assert!(words.iter().any(|line| line.ends_with("set it to on in dev")), "{words:?}");

    // Taken away there, it says off again, as nothing said.
    let (status, gone) =
        rig.call(Method::DELETE, "/api/projects/shop/flags/new-checkout/dev", Some(&rig.token), None).await;
    assert_eq!((status, &gone["taken_away"]), (StatusCode::OK, &json!(true)), "{gone}");
    assert_eq!(variable(&rig, "dev", "FLAG_NEW_CHECKOUT").await, Value::Null);
}

#[tokio::test]
async fn turning_a_flag_on_under_a_rule_that_names_others_waits_for_one_of_them_and_turning_it_off_never_waits()
 {
    let rig = shop().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let grace = person(&rig, "Grace").await;
    let row = flag(&rig, json!({ "name": "new-checkout" })).await;
    let rules = "/api/projects/shop/environments/prod/rules";
    let (status, set_rule) = rig
        .call(Method::PUT, rules, Some(&rig.token), Some(json!({ "approve": ["grace@example.com"] })))
        .await;
    assert_eq!(status, StatusCode::OK, "{set_rule}");

    // Ada is not one the rule names: turning it on in prod waits.
    let (status, came) = set(&rig, &rig.token, "new-checkout", "prod", json!("on")).await;
    assert_eq!((status, &came["came"]), (StatusCode::ACCEPTED, &json!("asks")), "{came}");
    let asked = rig.event("project.approval.asked", |event| event.data["environment"] == "prod").await;
    assert_eq!(asked.data["to"], json!([grace.actor]), "{asked:?}");
    assert_eq!(
        variable(&rig, "prod", "FLAG_NEW_CHECKOUT").await,
        Value::Null,
        "nothing is set while it waits"
    );
    // Dev names nobody, and is set at once.
    let (status, came) = set(&rig, &rig.token, "new-checkout", "dev", json!("on")).await;
    assert_eq!((status, &came["came"]), (StatusCode::OK, &json!("set")), "{came}");

    // Grace lets it go on, and it is written saying who asked and who let it.
    let ask = asked.data["ask"].as_str().unwrap().to_string();
    let (status, approved) = rig
        .call(Method::POST, &format!("/api/asked/{ask}/approve"), Some(&grace.token), Some(json!({})))
        .await;
    assert_eq!(status, StatusCode::OK, "{approved}");
    let changed = rig.event("project.flag.changed", |event| event.data["environment"] == "prod").await;
    assert_eq!(
        (&changed.data["asked_by_name"], &changed.data["approved_by"]),
        (&json!("Ada"), &json!(grace.actor))
    );
    assert_eq!(changed.data["record"], json!(row));
    assert_eq!(variable(&rig, "prod", "FLAG_NEW_CHECKOUT").await, json!("on"));

    // Turning it off is never kept waiting; Grace, whom the rule names, turns it on at once.
    let (status, came) = set(&rig, &rig.token, "new-checkout", "prod", json!("off")).await;
    assert_eq!((status, &came["came"]), (StatusCode::OK, &json!("set")), "{came}");
    assert_eq!(variable(&rig, "prod", "FLAG_NEW_CHECKOUT").await, json!("off"));
    let (status, came) = set(&rig, &grace.token, "new-checkout", "prod", json!("on")).await;
    assert_eq!((status, &came["came"]), (StatusCode::OK, &json!("set")), "{came}");
    assert_eq!(variable(&rig, "prod", "FLAG_NEW_CHECKOUT").await, json!("on"));
}

#[tokio::test]
async fn a_key_that_holds_less_is_refused_and_one_of_somebody_the_rule_names_asks_as_itself() {
    let rig = shop().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let grace = person(&rig, "Grace").await;
    flag(&rig, json!({ "name": "new-checkout" })).await;
    let rules = "/api/projects/shop/environments/prod/rules";
    rig.call(Method::PUT, rules, Some(&rig.token), Some(json!({ "approve": ["grace@example.com"] }))).await;

    let reads = key(&rig, &rig.token, &["projects:read", "records:read"]).await;
    let (status, refused) = set(&rig, &reads, "new-checkout", "dev", json!("on")).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("projects:write"), "{refused}");
    assert_eq!(variable(&rig, "dev", "FLAG_NEW_CHECKOUT").await, Value::Null);

    // A key of Grace's is the key, which the rule does not name: it asks.
    let hers = key(&rig, &grace.token, &["projects", "records:read"]).await;
    let (status, came) = set(&rig, &hers, "new-checkout", "prod", json!("on")).await;
    assert_eq!((status, &came["came"]), (StatusCode::ACCEPTED, &json!("asks")), "{came}");
    let asked = rig.event("project.flag.asked", |event| event.data["environment"] == "prod").await;
    assert!(asked.data["by"].as_str().unwrap().starts_with("key:"), "{asked:?}");
}

#[tokio::test]
async fn a_flag_past_its_day_is_told_to_whose_it_is_once() {
    let rig = shop().await;
    let ada = rig.read("/api/me").await["actor"].as_str().unwrap().to_string();
    let row = flag(&rig, json!({ "name": "old-banner", "owner": ada, "until": "2020-01-31" })).await;
    let told = rig
        .event("inbox.told", |event| event.data["about"] == json!(row) && event.data["owner"] == json!(ada))
        .await;
    assert_eq!(told.data["kind"], "told", "{told:?}");
    let (_, inbox) = rig.call(Method::GET, "/api/inbox", Some(&rig.token), None).await;
    let line = inbox["lines"].as_array().unwrap().iter().find(|line| line["about"] == json!(row)).cloned();
    let line = line.expect("Ada is told");
    assert_eq!(line["why"], "due", "{line}");
    assert!(line["says"].as_str().unwrap().contains("FLAG_OLD_BANNER"), "{line}");
    // Written again, it is told no second time.
    let mark = rig.mark();
    let (status, _) = rig
        .call(
            Method::PATCH,
            &format!("/api/records/{row}"),
            Some(&rig.token),
            Some(json!({ "fields": { "about": "the winter one" } })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    rig.event("instance.finished", |event| event.seq > mark && event.data["key"] == "due").await;
    let (_, inbox) = rig.call(Method::GET, "/api/inbox", Some(&rig.token), None).await;
    let lines = inbox["lines"].as_array().unwrap().iter().filter(|line| line["about"] == json!(row)).count();
    assert_eq!(lines, 1, "{inbox}");
    assert_eq!(rig.count_of("inbox.told").await, 1);
}

/// A text of Shop's own that takes which thing it is for and the word a flag hands it.
const CHECKOUT: &str = "\
blueprint: checkout
params:
  order: { type: string }
  flag: { type: string, default: \"off\" }
steps:
  done:
    do: emit
    with: { topic: shop.checked_out, order: \"${params.order}\", flag: \"${params.flag}\" }
";

#[tokio::test]
async fn a_percentage_flag_is_started_as_two_ways_given_its_part_and_the_rest_and_counted_never_scored() {
    let rig = shop().await;
    flag(&rig, json!({ "name": "fast-path", "kind": "percentage" })).await;
    let (status, came) = set(&rig, &rig.token, "fast-path", "dev", json!("30%")).await;
    assert_eq!((status, &came["shown"]), (StatusCode::OK, &json!("30%")), "{came}");
    let (status, _) = rig
        .call(
            Method::PUT,
            "/api/projects/shop/file",
            Some(&rig.token),
            Some(json!({ "path": "@blueprints/checkout.bp", "content": CHECKOUT })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let compare = "/api/projects/shop/flags/fast-path/compare";
    let asked = json!({ "environment": "dev", "blueprint": "checkout", "per": "order", "param": "flag" });
    let (status, started) = rig.call(Method::POST, compare, Some(&rig.token), Some(asked.clone())).await;
    assert_eq!(status, StatusCode::OK, "{started}");
    let ways: Vec<(Value, Value)> = started["ways"]
        .as_array()
        .unwrap()
        .iter()
        .map(|way| (way["name"].clone(), way["share"].clone()))
        .collect();
    assert_eq!(ways, [(json!("on"), json!(30)), (json!("off"), json!(70))], "{started}");
    assert_eq!(started["of"], "checkout");
    // A switch is no percentage, and is not started so.
    flag(&rig, json!({ "name": "dark" })).await;
    let (status, refused) =
        rig.call(Method::POST, "/api/projects/shop/flags/dark/compare", Some(&rig.token), Some(asked)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("no percentage"), "{refused}");
}

/// Shop with its prod's rule naming Grace as approving, and her session.
async fn approved_by_grace(rig: &Rig) -> crate::kept::Person {
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let grace = person(rig, "Grace").await;
    let rules = "/api/projects/shop/environments/prod/rules";
    let (status, set_rule) = rig
        .call(Method::PUT, rules, Some(&rig.token), Some(json!({ "approve": ["grace@example.com"] })))
        .await;
    assert_eq!(status, StatusCode::OK, "{set_rule}");
    grace
}

/// The go-ahead asked for setting a flag to `shown` in prod, once it is
/// told: each asked so far is looked at in the order it was said, by what
/// it asks to be let go on.
async fn go_ask_of(rig: &Rig, shown: &str) -> String {
    let mut seen: Vec<String> = Vec::new();
    loop {
        let told = rig
            .event("project.approval.asked", |event| {
                event.data["ask"].as_str().is_some_and(|ask| !seen.iter().any(|one| one == ask))
            })
            .await;
        let ask = told.data["ask"].as_str().unwrap().to_string();
        let shown_as = rig.read(&format!("/api/asked/{ask}")).await;
        if shown_as["says"].as_str().unwrap_or_default().contains(&format!(" to {shown} in prod")) {
            return ask;
        }
        seen.push(ask);
    }
}

#[tokio::test]
async fn a_flag_turned_off_while_its_turning_on_waits_stays_off_once_that_is_approved() {
    let rig = shop().await;
    let grace = approved_by_grace(&rig).await;
    let row = flag(&rig, json!({ "name": "new-checkout" })).await;

    // Asked to be on in prod, it waits for Grace.
    let (status, came) = set(&rig, &rig.token, "new-checkout", "prod", json!("on")).await;
    assert_eq!((status, &came["came"]), (StatusCode::ACCEPTED, &json!("asks")), "{came}");
    assert!(came["asked"].as_str().is_some_and(|asked| !asked.is_empty()), "{came}");
    let ask = go_ask_of(&rig, "on").await;
    // Turned off meanwhile, which it already was: said all the same, and it takes the place of what waits.
    let (status, off) = set(&rig, &rig.token, "new-checkout", "prod", json!("off")).await;
    assert_eq!((status, &off["came"]), (StatusCode::OK, &json!("set")), "{off}");
    assert_eq!(variable(&rig, "prod", "FLAG_NEW_CHECKOUT").await, json!("off"));

    // Approved after, it writes nothing, and says so to whoever asked and on the flag's history.
    let (status, approved) = rig
        .call(Method::POST, &format!("/api/asked/{ask}/approve"), Some(&grace.token), Some(json!({})))
        .await;
    assert_eq!(status, StatusCode::OK, "{approved}");
    let unchanged = rig.event("project.flag.unchanged", |event| event.data["environment"] == "prod").await;
    assert_eq!((&unchanged.data["record"], &unchanged.data["overtaken"]), (&json!(row), &json!("changed")));
    assert_eq!(variable(&rig, "prod", "FLAG_NEW_CHECKOUT").await, json!("off"), "the approval undid nothing");
    rig.event("collection.history", |event| event.data["topic"] == "project.flag.unchanged").await;
    let history = rig.read(&format!("/api/records/{row}/history")).await;
    let words: Vec<&str> =
        history["history"].as_array().unwrap().iter().filter_map(|line| line["said"].as_str()).collect();
    assert!(
        words.iter().any(|line| line.starts_with("approved after the flag was changed: nothing written")),
        "{words:?}"
    );
    let told = rig
        .event("inbox.told", |event| event.data["about"] == json!(row) && event.data["kind"] == "told")
        .await;
    let (_, inbox) = rig.call(Method::GET, "/api/inbox", Some(&rig.token), None).await;
    let line =
        inbox["lines"].as_array().unwrap().iter().find(|line| line["id"] == told.data["line"]).cloned();
    assert!(line.unwrap()["says"].as_str().unwrap().contains("so nothing was written"), "{inbox}");
    assert_eq!(rig.count_of("project.flag.changed").await, 1, "only the turning off was written");
}

#[tokio::test]
async fn a_later_ask_of_a_flag_takes_the_place_of_one_still_waiting_and_only_it_lands() {
    let rig = shop().await;
    let grace = approved_by_grace(&rig).await;
    flag(&rig, json!({ "name": "fast-path", "kind": "percentage" })).await;

    let (_, first) = set(&rig, &rig.token, "fast-path", "prod", json!(30)).await;
    let (_, second) = set(&rig, &rig.token, "fast-path", "prod", json!(50)).await;
    assert_eq!((&first["came"], &second["came"]), (&json!("asks"), &json!("asks")));
    let earlier = go_ask_of(&rig, "30%").await;
    let later = go_ask_of(&rig, "50%").await;
    assert_ne!(earlier, later);

    // The later approved first lands; the earlier approved after it writes nothing.
    let approve = |ask: String| {
        let path = format!("/api/asked/{ask}/approve");
        let token = grace.token.clone();
        let rig = &rig;
        async move { rig.call(Method::POST, &path, Some(&token), Some(json!({}))).await }
    };
    assert_eq!(approve(later).await.0, StatusCode::OK);
    let landed = rig.event("project.flag.changed", |event| event.data["environment"] == "prod").await;
    assert_eq!(landed.data["value"], json!(50));
    assert_eq!(approve(earlier).await.0, StatusCode::OK);
    let unchanged = rig.event("project.flag.unchanged", |event| event.data["environment"] == "prod").await;
    assert_eq!((&unchanged.data["value"], &unchanged.data["now"]), (&json!(30), &json!(50)));
    assert_eq!(variable(&rig, "prod", "FLAG_FAST_PATH").await, json!(50));

    // Asked twice again and the earlier approved first: it was overtaken by the later ask, and writes nothing.
    let (_, third) = set(&rig, &rig.token, "fast-path", "prod", json!(70)).await;
    let (_, fourth) = set(&rig, &rig.token, "fast-path", "prod", json!(90)).await;
    assert_eq!((&third["came"], &fourth["came"]), (&json!("asks"), &json!("asks")));
    let (third, fourth) = (go_ask_of(&rig, "70%").await, go_ask_of(&rig, "90%").await);
    let mark = rig.mark();
    assert_eq!(approve(third).await.0, StatusCode::OK);
    let overtaken = rig.event("project.flag.unchanged", |event| event.seq > mark).await;
    assert_eq!(overtaken.data["overtaken"], json!("asked again"), "{overtaken:?}");
    assert_eq!(variable(&rig, "prod", "FLAG_FAST_PATH").await, json!(50));
    assert_eq!(approve(fourth).await.0, StatusCode::OK);
    rig.event("project.flag.changed", |event| event.data["value"] == json!(90)).await;
    assert_eq!(variable(&rig, "prod", "FLAG_FAST_PATH").await, json!(90));
}
