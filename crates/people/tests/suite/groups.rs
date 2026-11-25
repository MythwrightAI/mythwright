//! Groups of people and agents: what a member holds by one, held at its
//! next request; nobody given by one what whoever put them there does not
//! hold; an agent never changing a group it is in; and what goes with a
//! group, or a place in one, as it is taken away.

use kernel::{ErrorKind, Principal};
use people::{CredentialKind, Identity, Tier};

use crate::Rig;

/// The owner and a member, Grace, with her session and her account's id.
async fn two(identity: &Identity) -> (Principal, String, String) {
    let owner = identity.register("ada@example.com", "Ada", "correct horse", None, true).await.unwrap();
    let owner = owner.account.principal();
    identity.set_tier(&owner, Tier::Premium).await.unwrap();
    let code = identity.invite(&owner, None).await.unwrap();
    let grace =
        identity.register("grace@example.com", "Grace", "correct horse", Some(&code), true).await.unwrap();
    (owner, grace.account.id, grace.token)
}

fn scopes(list: &[&str]) -> Vec<String> {
    list.iter().map(|scope| scope.to_string()).collect()
}

async fn said(rig: &Rig, topic: &str) -> Vec<kernel::Event> {
    rig.journal.read(journal::EventQuery { topics: topic.into(), ..Default::default() }).await.unwrap()
}

#[tokio::test]
async fn what_a_group_holds_is_held_by_its_people_and_agents_at_their_next_request() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let (owner, grace, session) = two(identity).await;
    let token = identity.issue(CredentialKind::Agent, "helper", "helper", None).await.unwrap();
    assert!(!identity.authenticate(&session).await.unwrap().can("environments:secrets"));
    assert!(!identity.authenticate(&token).await.unwrap().can("environments:secrets"));

    let group = identity.make_group("deploys", &scopes(&["environments:secrets"]), &owner).await.unwrap();
    assert_eq!(group.holds, ["environments:secrets"]);
    assert!(group.members.is_empty());
    identity.join_group("deploys", &format!("user:{grace}"), &owner).await.unwrap();
    identity.join_group(&group.id, "agent:helper", &owner).await.unwrap();

    // Worked out as each asks: no new sign-in, and the agent's credential as it was.
    let member = identity.authenticate(&session).await.unwrap();
    assert!(member.can("environments:secrets") && member.can("board:read") && !member.can("admin"));
    let agent = identity.authenticate(&token).await.unwrap();
    assert!(agent.can("environments:secrets") && !agent.can("shell"), "{:?}", agent.scopes);
    let held = identity.holder("agent:helper").await.unwrap().unwrap();
    assert!(held.can("environments:secrets"));
    let held = identity.holder(&format!("user:{grace}")).await.unwrap().unwrap();
    assert!(held.can("environments:secrets"));
    let account = identity.account(&grace).await.unwrap();
    assert!(identity.principal_of(&account).can("environments:secrets"));
    assert!(!account.principal().can("environments:secrets"), "the role and what was given, alone");
    // Read inside a write as a write finds it, as what is told goes by.
    let (actor, inside) = (format!("user:{grace}"), rig.journal.db());
    let read = inside.call(move |connection| people::held(connection, &actor)).await.unwrap();
    assert!(kernel::principal::covers(&read, "environments:secrets"));
    let read = inside.call(|connection| people::held(connection, "agent:helper")).await.unwrap();
    assert!(kernel::principal::covers(&read, "environments:secrets"));

    assert_eq!(identity.grouped().of("agent:helper").len(), 1);
    assert_eq!(identity.grouped().members(&group.id), ["agent:helper".to_string(), format!("user:{grace}")]);
    assert_eq!(said(&rig, "identity.group.joined").await.len(), 2);
}

#[tokio::test]
async fn a_group_changed_is_held_at_the_next_request_and_narrowed_takes_it_from_the_keys_its_people_made() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let (owner, grace, session) = two(identity).await;
    identity.make_group("deploys", &[], &owner).await.unwrap();
    identity.join_group("deploys", &format!("user:{grace}"), &owner).await.unwrap();
    assert!(!identity.authenticate(&session).await.unwrap().can("environments:secrets"));

    let widened =
        identity.hold_group("deploys", &scopes(&["environments:secrets", "admin"]), &owner).await.unwrap();
    assert!(widened.changed);
    let member = identity.authenticate(&session).await.unwrap();
    assert!(member.can("environments:secrets") && member.can("admin"));
    // A key made while she holds it holds it, as any key holds its maker's.
    let key = identity.issue(CredentialKind::Key, &grace, "ci", Some(member.scopes.clone())).await.unwrap();
    assert!(identity.authenticate(&key).await.unwrap().can("admin"));
    let before = identity.withdrawn();

    let narrowed = identity.hold_group("deploys", &scopes(&["environments:secrets"]), &owner).await.unwrap();
    assert_eq!(narrowed.keys, 1);
    assert!(!identity.authenticate(&session).await.unwrap().can("admin"));
    let key = identity.authenticate(&key).await.unwrap();
    assert!(
        !key.can("admin") && key.can("environments:secrets") && key.can("board:read"),
        "{:?}",
        key.scopes
    );
    assert!(before.has_changed().unwrap(), "whatever was told something because of it asks again");

    // Said once each, the same list again no change.
    let again = identity.hold_group("deploys", &scopes(&["environments:secrets"]), &owner).await.unwrap();
    assert!(!again.changed);
    let held = said(&rig, "identity.group.held").await;
    assert_eq!(held.len(), 3, "made, widened, narrowed");
    assert_eq!(held[2].data["keys"], 1);
}

#[tokio::test]
async fn leaving_a_group_or_its_being_taken_away_takes_what_it_held_from_the_person_and_their_keys() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let (owner, grace, session) = two(identity).await;
    let actor = format!("user:{grace}");
    identity.make_group("deploys", &scopes(&["environments:secrets"]), &owner).await.unwrap();
    identity.make_group("admins", &scopes(&["admin"]), &owner).await.unwrap();
    identity.join_group("deploys", &actor, &owner).await.unwrap();
    identity.join_group("admins", &actor, &owner).await.unwrap();
    let held = identity.authenticate(&session).await.unwrap().scopes;
    let key = identity.issue(CredentialKind::Key, &grace, "ci", Some(held)).await.unwrap();

    let left = identity.leave_group("deploys", &actor, &owner).await.unwrap();
    assert_eq!(left.keys, 1);
    assert!(!identity.authenticate(&session).await.unwrap().can("environments:secrets"));
    let still = identity.authenticate(&key).await.unwrap();
    assert!(!still.can("environments:secrets") && still.can("admin"));
    let not_in = identity.leave_group("deploys", &actor, &owner).await.unwrap_err();
    assert_eq!(not_in.kind, ErrorKind::Conflict);

    let gone = identity.remove_group("admins", &owner).await.unwrap();
    assert!(gone.group.is_none());
    assert_eq!(gone.keys, 1);
    assert!(!identity.authenticate(&session).await.unwrap().can("admin"));
    assert!(!identity.authenticate(&key).await.unwrap().can("admin"));
    assert!(identity.group("admins").is_err());
    let removed = said(&rig, "identity.group.removed").await;
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].data["members"][0], actor.as_str());
}

#[tokio::test]
async fn nobody_gives_by_a_group_what_they_do_not_hold_and_only_whoever_manages_the_factory_changes_one() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let (owner, grace, session) = two(identity).await;
    identity.make_group("deploys", &scopes(&["environments:secrets"]), &owner).await.unwrap();
    let member = identity.authenticate(&session).await.unwrap();
    let refused = identity.make_group("mine", &[], &member).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);

    // Given `admin` and not the environments' secrets.
    identity.give(&grace, "admin", &owner).await.unwrap();
    let manager = identity.authenticate(&session).await.unwrap();
    let refused =
        identity.make_group("secrets", &scopes(&["environments:secrets"]), &manager).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    assert!(refused.message.contains("environments:secrets"), "{}", refused.message);
    let made = identity.make_group("readers", &scopes(&["board:read"]), &manager).await.unwrap();
    let refused = identity
        .hold_group(&made.id, &scopes(&["board:read", "environments:secrets"]), &manager)
        .await
        .unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    // Putting somebody in a group gives what the group holds.
    let ada = owner.actor();
    let refused = identity.join_group("deploys", &ada, &manager).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    identity.join_group("readers", &ada, &manager).await.unwrap();
    // Narrowing gives nothing, and is hers to do.
    identity.hold_group(&made.id, &[], &manager).await.unwrap();

    // An area written whole is held where each thing done in it is.
    identity.make_group("whole", &scopes(&["environments"]), &owner).await.unwrap();
    let refused = identity.make_group("hers too", &scopes(&["environments"]), &manager).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);

    // What is not a scope, a name taken, a group in a group, an account that is not there.
    let wrong = identity.make_group("odd", &scopes(&["everything"]), &owner).await.unwrap_err();
    assert_eq!(wrong.kind, ErrorKind::Invalid);
    let taken = identity.make_group("Deploys", &[], &owner).await.unwrap_err();
    assert_eq!(taken.kind, ErrorKind::Conflict);
    let nothing = identity.make_group("  ", &[], &owner).await.unwrap_err();
    assert_eq!(nothing.kind, ErrorKind::Invalid);
    let nested = identity.join_group("deploys", &format!("group:{}", made.id), &owner).await.unwrap_err();
    assert_eq!(nested.kind, ErrorKind::Invalid);
    let nobody = identity.join_group("deploys", "user:acct_nobody", &owner).await.unwrap_err();
    assert_eq!(nobody.kind, ErrorKind::NotFound);
    assert!(identity.group("deploys").unwrap().members.is_empty());
}

#[tokio::test]
async fn an_agent_never_changes_a_group_it_is_in_nor_its_own_place_in_one() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let (owner, _, _) = two(identity).await;
    let everything = scopes(&["admin", "board", "environments:secrets"]);
    let token = identity.issue(CredentialKind::Agent, "boss", "boss", Some(everything)).await.unwrap();
    let agent = identity.authenticate(&token).await.unwrap();
    identity.make_group("crew", &scopes(&["board"]), &owner).await.unwrap();

    let own = identity.join_group("crew", "agent:boss", &agent).await.unwrap_err();
    assert_eq!(own.kind, ErrorKind::Denied);
    assert!(own.message.contains("never changes a group it is in"), "{}", own.message);
    // A group it is not in is changed as by anybody who manages the factory.
    identity.join_group("crew", "agent:other", &agent).await.unwrap();

    identity.join_group("crew", "agent:boss", &owner).await.unwrap();
    let refusals = [
        identity.rename_group("crew", "mine", &agent).await.unwrap_err(),
        identity.hold_group("crew", &scopes(&["board", "admin"]), &agent).await.unwrap_err(),
        identity.leave_group("crew", "agent:other", &agent).await.unwrap_err(),
        identity.leave_group("crew", "agent:boss", &agent).await.unwrap_err(),
        identity.remove_group("crew", &agent).await.unwrap_err(),
    ];
    for refused in refusals {
        assert_eq!(refused.kind, ErrorKind::Denied, "{}", refused.message);
    }
    assert_eq!(identity.group("crew").unwrap().members, ["agent:boss", "agent:other"]);
}

#[tokio::test]
async fn groups_read_again_from_their_rows_are_what_the_memory_said() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let (owner, grace, _) = two(identity).await;
    let made = identity.make_group("deploys", &scopes(&["environments:secrets"]), &owner).await.unwrap();
    identity.make_group("gone", &[], &owner).await.unwrap();
    identity.join_group("deploys", &format!("user:{grace}"), &owner).await.unwrap();
    identity.join_group("deploys", "agent:helper", &owner).await.unwrap();
    identity.rename_group(&made.id, "Deploys", &owner).await.unwrap();
    identity.remove_group("gone", &owner).await.unwrap();

    let again = Identity::open(rig.journal.clone()).await.unwrap();
    assert_eq!(again.groups(), identity.groups());
    assert_eq!(again.groups().len(), 1);
    assert_eq!(again.group("deploys").unwrap().name, "Deploys");
    assert_eq!(again.grouped().held_by("agent:helper"), ["environments:secrets"]);

    // An agent taken away leaves every group in the write that takes it away.
    let was = identity.grouped().letting_go("agent:helper");
    assert_eq!(was.len(), 1);
    let by = kernel::Source::actor(owner.actor());
    rig.journal
        .write(move |writing| people::groups::forget_member(writing, "agent:helper", by))
        .await
        .unwrap();
    let again = Identity::open(rig.journal.clone()).await.unwrap();
    assert_eq!(again.group("deploys").unwrap().members, [format!("user:{grace}")]);
    assert_eq!(identity.group("deploys").unwrap().members, [format!("user:{grace}")]);
}
