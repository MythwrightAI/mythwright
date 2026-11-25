//! Beginning with no registration (`people::opened`, `api::opened`): a
//! factory that makes its owner as it first starts, and the link that
//! signs somebody in once. Nothing here lets anybody in without proof: a
//! request from this machine with nothing to show is given nothing.

use std::sync::Arc;
use std::time::Duration;

use axum::http::{Method, StatusCode};
use kernel::ManualClock;
use serde_json::{Value, json};

use crate::Rig;

/// A node that makes its owner as it first starts, as one started from the
/// command line does, on a clock the test moves; with the session the start
/// made for whoever started it.
async fn made() -> (Rig, ManualClock, String) {
    let clock = ManualClock::new();
    let moved = clock.clone();
    let rig = Rig::with("api-opened", move |config, _| {
        config.makes_owner = true;
        config.clock = Some(Arc::new(moved));
    })
    .await;
    let token = rig.node().made.as_ref().expect("the start made the owner").token.clone();
    (rig, clock, token)
}

/// Each workspace there is, by its id and what it is called.
async fn workspaces(rig: &Rig, token: &str) -> Vec<(String, String)> {
    let spaces = rig.call(Method::GET, "/api/workspaces", Some(token), None).await.1;
    let each = spaces["workspaces"].as_array().unwrap().iter();
    each.map(|space| (space["id"].as_str().unwrap().to_string(), space["name"].as_str().unwrap().to_string()))
        .collect()
}

async fn link_as(rig: &Rig, token: &str) -> (StatusCode, Value) {
    rig.call(Method::POST, "/api/auth/once", Some(token), Some(json!({}))).await
}

async fn spend(rig: &Rig, once: &str) -> (StatusCode, Value) {
    rig.call(Method::POST, "/api/auth/link", None, Some(json!({ "once": once }))).await
}

#[tokio::test(flavor = "multi_thread")]
async fn a_factory_that_makes_its_owner_has_one_before_it_answers_anybody() {
    let (rig, _, token) = made().await;
    let (_, meta) = rig.call(Method::GET, "/api/meta", None, None).await;
    assert_eq!(meta["needs_setup"], false, "{meta}");
    // Nobody makes themselves the owner by registering from this machine.
    let ada = json!({ "email": "ada@example.com", "name": "Ada", "password": "correct horse" });
    let (status, body) = rig.call(Method::POST, "/api/auth/register", None, Some(ada)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    // The owner has no email, no password and no name of their own yet, and
    // nothing of the machine or of whoever runs it is taken for one.
    let (status, me) = rig.call(Method::GET, "/api/me", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK, "{me}");
    assert_eq!(me["account"]["role"], "owner");
    assert_eq!(me["account"]["email"], "");
    assert_eq!(me["account"]["name"], "Unnamed");
    assert_eq!(me["password"], false);
    assert_eq!(
        workspaces(&rig, &token).await,
        [("unnamed-workspace".to_string(), "Unnamed workspace".to_string())]
    );
    // And started again, it makes nobody more.
    let mut rig = rig;
    rig.restart().await;
    assert!(rig.node().made.is_none(), "a second start made another owner");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_request_from_this_machine_with_nothing_to_show_is_given_nothing() {
    let (rig, _, _) = made().await;
    let (status, body) = rig.call(Method::POST, "/api/auth/once", None, Some(json!({}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "a link for nobody: {body}");
    let (status, _) = rig.call(Method::POST, "/api/auth/once", Some("ses_made-up"), Some(json!({}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "a link for a made-up session");
    let (status, body) = spend(&rig, "once_made-up").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "a made-up link spent: {body}");
    // Something to prove is given to anybody, and answered wrongly it gives
    // nothing, and is spent: the right answer after it is refused too.
    let asked = rig.call(Method::POST, "/api/auth/once/asked", None, Some(json!({}))).await.1;
    let nonce = asked["nonce"].as_str().unwrap();
    let wrong = json!({ "nonce": nonce, "proof": "0".repeat(64) });
    let (status, body) = rig.call(Method::POST, "/api/auth/once/proved", None, Some(wrong)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    let right = api::opened::proves(&rig.dir.join("data"), nonce).unwrap();
    let again = json!({ "nonce": nonce, "proof": right });
    let (status, body) = rig.call(Method::POST, "/api/auth/once/proved", None, Some(again)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "a thing to prove answered twice: {body}");
    // And no password is set for an account by somebody who is nobody.
    let set = json!({ "email": "ada@example.com", "password": "correct horse" });
    let (status, _) = rig.call(Method::PUT, "/api/me/password", None, Some(set)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_link_signs_its_owner_in_once_and_not_after_its_minute() {
    let (rig, clock, token) = made().await;
    let (status, made) = link_as(&rig, &token).await;
    assert_eq!(status, StatusCode::OK, "{made}");
    let once = made["once"].as_str().unwrap().to_string();
    // Traded as a page trades it: a cookie, and the page's key.
    let body = json!({ "once": once, "page": true });
    let (status, signed) = rig.call(Method::POST, "/api/auth/link", None, Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{signed}");
    assert_eq!(signed["account"]["role"], "owner");
    assert!(signed["page"].is_string(), "{signed}");
    let (status, body) = spend(&rig, &once).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "spent twice: {body}");
    assert!(body["error"]["message"].as_str().unwrap().contains("used already"), "{body}");
    // One left past its minute is nobody's.
    let late = link_as(&rig, &token).await.1["once"].as_str().unwrap().to_string();
    clock.advance(Duration::from_secs(61));
    let (status, _) = spend(&rig, &late).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "a link spent past its minute");
    // One spent within it is a session as signing in gives.
    let soon = link_as(&rig, &token).await.1["once"].as_str().unwrap().to_string();
    clock.advance(Duration::from_secs(59));
    let (status, signed) = spend(&rig, &soon).await;
    assert_eq!(status, StatusCode::OK, "{signed}");
    let session = signed["token"].as_str().unwrap();
    assert_eq!(rig.call(Method::GET, "/api/me", Some(session), None).await.0, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_link_is_asked_with_all_its_account_holds_and_a_key_that_holds_less_is_refused() {
    let (rig, _, token) = made().await;
    let narrow = json!({ "name": "reads the board", "scopes": ["board:read"] });
    let (status, key) = rig.call(Method::POST, "/api/credentials", Some(&token), Some(narrow)).await;
    assert_eq!(status, StatusCode::OK, "{key}");
    let (status, body) = link_as(&rig, key["token"].as_str().unwrap()).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "a key that holds less made a session: {body}");
    // Nor does such a key give its account a password to sign in with.
    let set = json!({ "email": "ada@example.com", "password": "correct horse" });
    let (status, _) =
        rig.call(Method::PUT, "/api/me/password", Some(key["token"].as_str().unwrap()), Some(set)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // A key of the owner's that holds everything they do, `admin` among it, may.
    let whole = json!({ "name": "all of it" });
    let (status, key) = rig.call(Method::POST, "/api/credentials", Some(&token), Some(whole)).await;
    assert_eq!(status, StatusCode::OK, "{key}");
    assert!(key["scopes"].as_array().is_none_or(|held| held.iter().any(|scope| scope == "admin")), "{key}");
    let (status, made) = link_as(&rig, key["token"].as_str().unwrap()).await;
    assert_eq!(status, StatusCode::OK, "{made}");
    let (status, _) = spend(&rig, made["once"].as_str().unwrap()).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread")]
async fn proving_the_data_folder_gives_the_owners_link() {
    let (rig, _, _) = made().await;
    let asked = rig.call(Method::POST, "/api/auth/once/asked", None, Some(json!({}))).await.1;
    let nonce = asked["nonce"].as_str().unwrap();
    let proof = api::opened::proves(&rig.dir.join("data"), nonce).unwrap();
    let body = json!({ "nonce": nonce, "proof": proof });
    let (status, made) = rig.call(Method::POST, "/api/auth/once/proved", None, Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{made}");
    let (status, signed) = spend(&rig, made["once"].as_str().unwrap()).await;
    assert_eq!(status, StatusCode::OK, "{signed}");
    assert_eq!(signed["account"]["role"], "owner");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_owner_with_no_password_sets_one_and_is_then_signed_in_by_it_too() {
    let (rig, _, token) = made().await;
    let (status, body) = rig
        .call(Method::PUT, "/api/me/password", Some(&token), Some(json!({ "password": "correct horse" })))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "no email to sign in by: {body}");
    let set = json!({ "email": "Ada@Example.com", "password": "correct horse" });
    let (status, body) = rig.call(Method::PUT, "/api/me/password", Some(&token), Some(set.clone())).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["account"]["email"], "ada@example.com");
    let (status, body) = rig.call(Method::PUT, "/api/me/password", Some(&token), Some(set)).await;
    assert_eq!(status, StatusCode::CONFLICT, "one that has a password keeps it: {body}");
    let (status, renamed) =
        rig.call(Method::PUT, "/api/me/name", Some(&token), Some(json!({ "name": "Ada" }))).await;
    assert_eq!(status, StatusCode::OK, "{renamed}");
    let login = json!({ "email": "ada@example.com", "password": "correct horse" });
    let (status, signed) = rig.call(Method::POST, "/api/auth/login", None, Some(login)).await;
    assert_eq!(status, StatusCode::OK, "{signed}");
    assert_eq!(signed["account"]["name"], "Ada");
    // And a link signs them in as before.
    let (status, made) = link_as(&rig, &token).await;
    assert_eq!(status, StatusCode::OK, "{made}");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_workspace_made_with_the_owner_is_called_after_them_once_they_are_named_and_keeps_its_id() {
    let (rig, _, token) = made().await;
    let name = |name: &str| json!({ "name": name });
    let (status, body) = rig.call(Method::PUT, "/api/me/name", Some(&token), Some(name("Ada"))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // Called after her, by the id it was made with: what was kept of it by its id is kept.
    assert_eq!(
        workspaces(&rig, &token).await,
        [("unnamed-workspace".to_string(), "Ada's workspace".to_string())]
    );
    // Named once: called something else later, she takes the workspace with her no more.
    let (status, body) = rig.call(Method::PUT, "/api/me/name", Some(&token), Some(name("Ada L"))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(workspaces(&rig, &token).await[0].1, "Ada's workspace");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_workspace_somebody_called_something_else_is_not_called_after_the_owner() {
    let (rig, _, token) = made().await;
    let (status, body) = rig
        .call(
            Method::PATCH,
            "/api/workspaces/unnamed-workspace",
            Some(&token),
            Some(json!({ "name": "Print works" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) =
        rig.call(Method::PUT, "/api/me/name", Some(&token), Some(json!({ "name": "Ada" }))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(workspaces(&rig, &token).await[0].1, "Print works");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_folder_to_prove_is_said_to_a_program_on_this_machine() {
    let (rig, _, _) = made().await;
    let asked = rig.call(Method::POST, "/api/auth/once/asked", None, Some(json!({}))).await.1;
    let said = std::path::PathBuf::from(asked["data"].as_str().expect("the folder said"));
    assert_eq!(kernel::whole::whole(&said).unwrap(), kernel::whole::whole(&rig.dir.join("data")).unwrap());
}
