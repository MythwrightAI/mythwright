//! A person given a scope beyond what their role holds: by whom, what,
//! and what taking it away takes with it.

use kernel::ErrorKind;
use people::{CredentialKind, Tier};

use crate::Rig;

/// The owner, and a member signed in: each as their principal, with the
/// member's session.
async fn two(rig: &Rig) -> (kernel::Principal, String, String) {
    let identity = &rig.identity;
    let owner = identity.register("ada@example.com", "Ada", "correct horse", None, true).await.unwrap();
    let owner = owner.account.principal();
    identity.set_tier(&owner, Tier::Premium).await.unwrap();
    let code = identity.invite(&owner, None).await.unwrap();
    let grace =
        identity.register("grace@example.com", "Grace", "correct horse", Some(&code), true).await.unwrap();
    (owner, grace.account.id, grace.token)
}

#[tokio::test]
async fn a_scope_given_is_held_at_once_by_the_person_and_given_once() {
    let rig = Rig::new().await;
    let (owner, grace, session) = two(&rig).await;
    let identity = &rig.identity;
    assert!(!identity.authenticate(&session).await.unwrap().can("environments:secrets"));

    let given = identity.give(&grace, "environments:secrets", &owner).await.unwrap();
    assert!(given.changed);
    assert_eq!(given.account.given, ["environments:secrets"]);
    // Worked out from the account as the session is used: no new sign-in.
    let member = identity.authenticate(&session).await.unwrap();
    assert!(member.can("environments:secrets") && !member.can("admin"));
    let said =
        rig.journal.read(journal::EventQuery { topics: "identity.given".into(), ..Default::default() });
    let said = said.await.unwrap();
    assert_eq!(said.len(), 1);
    assert_eq!(said[0].data["scope"], "environments:secrets");
    assert_eq!(said[0].source.actor.as_deref(), Some(owner.actor().as_str()));

    // Given again, or given what the role holds, nothing changes and nothing is said.
    for held in ["environments:secrets", "board:read", "projects"] {
        let again = identity.give(&grace, held, &owner).await.unwrap();
        assert!(!again.changed, "{held}");
    }
    assert_eq!(identity.account(&grace).await.unwrap().given, ["environments:secrets"]);
    let said =
        rig.journal.read(journal::EventQuery { topics: "identity.given".into(), ..Default::default() });
    assert_eq!(said.await.unwrap().len(), 1);
}

#[tokio::test]
async fn nobody_gives_what_they_do_not_hold_or_without_admin_and_only_a_scope_there_is() {
    let rig = Rig::new().await;
    let (owner, grace, session) = two(&rig).await;
    let identity = &rig.identity;
    let member = identity.authenticate(&session).await.unwrap();
    let ada = identity.accounts().await.unwrap().into_iter().find(|one| one.role == "owner").unwrap();

    // A member does not manage the factory.
    let refused = identity.give(&grace, "board", &member).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied, "{}", refused.message);
    // Given `admin`, she gives what she holds and nothing else.
    identity.give(&grace, "admin", &owner).await.unwrap();
    let member = identity.authenticate(&session).await.unwrap();
    let refused = identity.give(&grace, "private", &member).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    assert!(refused.message.contains("you do not hold `private`"), "{}", refused.message);
    for wrong in ["*", "nothing", "board:dance", "private:", "admin:read"] {
        let refused = identity.give(&grace, wrong, &owner).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Invalid, "{wrong}: {}", refused.message);
    }
    assert!(identity.give(&grace, "private:hr", &owner).await.unwrap().changed);
    // Named rightly and held by her role already: nothing to give.
    for held in ["emit:shop.paid", "extensions:reach", "fleet:write"] {
        assert!(!identity.give(&grace, held, &owner).await.unwrap().changed, "{held}");
    }
    // The owner holds everything there is; an account that is not there is not.
    let refused = identity.give(&ada.id, "private:hr", &owner).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict, "{}", refused.message);
    let refused = identity.give("acct_nobody", "board", &owner).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::NotFound);
}

#[tokio::test]
async fn taken_away_it_goes_from_the_person_and_from_the_keys_they_made_with_it() {
    let rig = Rig::new().await;
    let (owner, grace, session) = two(&rig).await;
    let identity = &rig.identity;
    identity.give(&grace, "environments:secrets", &owner).await.unwrap();
    let scopes = |list: &[&str]| Some(list.iter().map(|one| one.to_string()).collect());
    let with = identity
        .issue(CredentialKind::Key, &grace, "deploys", scopes(&["environments:secrets", "board"]))
        .await;
    let with = with.unwrap();
    let without =
        identity.issue(CredentialKind::Key, &grace, "reads", scopes(&["board:read"])).await.unwrap();
    assert!(identity.authenticate(&with).await.unwrap().can("environments:secrets"));
    let mut told = identity.withdrawn();
    told.mark_unchanged();

    let taken = identity.take(&grace, "environments:secrets", &owner).await.unwrap();
    assert!(taken.changed && taken.account.given.is_empty());
    assert_eq!(taken.keys, 1, "the one key that held it");
    assert!(!identity.authenticate(&session).await.unwrap().can("environments:secrets"));
    let key = identity.authenticate(&with).await.unwrap();
    assert!(!key.can("environments:secrets") && key.can("board:write"), "{:?}", key.scopes);
    assert_eq!(identity.authenticate(&without).await.unwrap().scopes, ["board:read"]);
    assert!(told.has_changed().unwrap(), "whoever was told something because of it asks again");

    // What the role holds is not taken here, and what was not given is said to be so.
    let refused = identity.take(&grace, "board", &owner).await.unwrap_err();
    assert!(refused.message.contains("by being a member, not by being given it"), "{}", refused.message);
    let refused = identity.take(&grace, "admin", &owner).await.unwrap_err();
    assert!(
        refused.message.contains("was not given `admin`; what they were given is nothing"),
        "{}",
        refused.message
    );
    let member = identity.authenticate(&session).await.unwrap();
    assert_eq!(identity.take(&grace, "board", &member).await.unwrap_err().kind, ErrorKind::Denied);
}
