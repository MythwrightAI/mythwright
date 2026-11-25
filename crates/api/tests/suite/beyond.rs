//! A person given a scope beyond what their role holds, through the
//! daemon: by whom, held at once, and taken away from them and their keys.

use reqwest::{Method, StatusCode};
use serde_json::json;

use crate::Rig;
use crate::held::said;
use crate::kept::person;

#[tokio::test]
async fn a_member_given_the_secrets_of_environments_sets_one_at_once_and_loses_it_with_her_keys_when_it_is_taken()
 {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    rig.send("/api/projects", json!({ "name": "shop" })).await;
    rig.send("/api/projects/shop/environments", json!({ "name": "prod" })).await;
    let grace = person(&rig, "Grace").await;
    let id = grace.actor.trim_start_matches("user:").to_string();
    let given = format!("/api/accounts/{id}/given/environments:secrets");
    let secret = "/api/projects/shop/environments/prod/secrets/stripe";
    let value = Some(json!({ "value": "sk_live_given" }));
    let (status, refused) = rig.call(Method::PUT, secret, Some(&grace.token), value.clone()).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");

    // A member does not give it, to herself or anybody.
    let (status, refused) = rig.call(Method::PUT, &given, Some(&grace.token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // The owner does; she holds it from her next request, signed in as she was.
    let (status, answer) = rig.call(Method::PUT, &given, Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(
        (&answer["changed"], &answer["account"]["given"]),
        (&json!(true), &json!(["environments:secrets"]))
    );
    let accounts = rig.read("/api/accounts").await;
    let listed =
        accounts["accounts"].as_array().unwrap().iter().find(|one| one["id"] == id.as_str()).unwrap();
    assert_eq!(listed["given"], json!(["environments:secrets"]), "{accounts}");
    let (status, set) = rig.call(Method::PUT, secret, Some(&grace.token), value.clone()).await;
    assert_eq!(status, StatusCode::OK, "{set}");
    let event = rig.event("identity.given", |event| event.data["account"] == id.as_str()).await;
    assert_eq!(event.data["scope"], "environments:secrets");

    // A key she makes may hold it; one she makes without it does not.
    let key = rig
        .call(
            Method::POST,
            "/api/credentials",
            Some(&grace.token),
            Some(json!({ "name": "deploys", "scopes": ["environments:secrets", "projects"] })),
        )
        .await;
    assert_eq!(key.0, StatusCode::OK, "{}", key.1);
    let key = key.1["token"].as_str().unwrap().to_string();
    let (status, _) = rig.call(Method::PUT, secret, Some(&key), value.clone()).await;
    assert_eq!(status, StatusCode::OK);

    // Taken away: from her, and from her key, which holds what is left.
    let (status, answer) = rig.call(Method::DELETE, &given, Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!((&answer["keys"], &answer["account"]["given"]), (&json!(1), &json!([])));
    for token in [&grace.token, &key] {
        let (status, refused) = rig.call(Method::PUT, secret, Some(token), value.clone()).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    }
    let (status, read) = rig.call(Method::GET, "/api/projects/shop", Some(&key), None).await;
    assert_eq!(status, StatusCode::OK, "what the key held besides it it holds still: {read}");
    let taken = rig.event("identity.taken", |event| event.data["account"] == id.as_str()).await;
    assert_eq!(taken.data["keys"], 1);

    // What there is not to give, and what was never given, are said so.
    let (status, refused) = rig
        .call(Method::PUT, &format!("/api/accounts/{id}/given/secrets:write"), Some(&rig.token), None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    let (status, refused) = rig.call(Method::DELETE, &given, Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(said(&refused).contains("was not given `environments:secrets`"), "{refused}");
}
