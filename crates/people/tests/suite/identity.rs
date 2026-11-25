use std::time::Duration;

use kernel::{ErrorKind, PrincipalKind};
use people::{CredentialKind, Identity, Tier};

use crate::Rig;

#[tokio::test]
async fn the_first_account_owns_the_installation_and_can_sign_in() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    assert!(identity.needs_setup().await.unwrap());

    let registered =
        identity.register(" Ada@Example.com ", "Ada", "correct horse", None, true).await.unwrap();
    assert_eq!(
        (registered.account.email.as_str(), registered.account.role.as_str()),
        ("ada@example.com", "owner")
    );
    assert_eq!(registered.account.tier, Tier::Free);
    assert!(!identity.needs_setup().await.unwrap());

    let who = identity.authenticate(&registered.token).await.unwrap();
    assert_eq!((who.kind, who.name.as_str()), (PrincipalKind::User, "Ada"));
    assert!(who.can("admin") && who.can("projects:write") && who.can("shell"));

    let again = identity.sign_in("ada@example.com", "correct horse").await.unwrap();
    assert_ne!(again.token, registered.token, "each sign-in is its own session");
    identity.sign_out(&registered.token).await.unwrap();
    assert_eq!(identity.authenticate(&registered.token).await.unwrap_err().kind, ErrorKind::Unauthenticated);
    assert!(identity.authenticate(&again.token).await.is_ok());
}

#[tokio::test]
async fn sign_in_gives_nothing_away_and_slows_guessing() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    identity.register("ada@example.com", "Ada", "correct horse", None, true).await.unwrap();

    let wrong_password = identity.sign_in("ada@example.com", "wrong horse").await.unwrap_err();
    let unknown_email = identity.sign_in("nobody@example.com", "correct horse").await.unwrap_err();
    assert_eq!(wrong_password, unknown_email, "the two cases must be indistinguishable");
    assert_eq!(wrong_password.kind, ErrorKind::Unauthenticated);

    for _ in 0..4 {
        identity.sign_in("ada@example.com", "guess").await.unwrap_err();
    }
    let locked = identity.sign_in("ada@example.com", "correct horse").await.unwrap_err();
    assert_eq!(
        locked.kind,
        ErrorKind::Denied,
        "even the right password waits once there have been too many wrong ones"
    );

    rig.clock.advance(Duration::from_secs(16 * 60));
    assert!(identity.sign_in("ada@example.com", "correct horse").await.is_ok());
}

#[tokio::test]
async fn sessions_expire() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let token = identity.register("ada@example.com", "Ada", "correct horse", None, true).await.unwrap().token;
    rig.clock.advance(Duration::from_secs(29 * 24 * 3600));
    assert!(identity.authenticate(&token).await.is_ok());
    rig.clock.advance(Duration::from_secs(2 * 24 * 3600));
    assert_eq!(identity.authenticate(&token).await.unwrap_err().kind, ErrorKind::Unauthenticated);
}

#[tokio::test]
async fn registration_checks_what_it_is_given() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    for (email, name, password, expected) in [
        ("not-an-email", "Ada", "correct horse", "email"),
        ("ada@example.com", "  ", "correct horse", "name"),
        ("ada@example.com", "Ada", "short", "at least 8"),
    ] {
        let err = identity.register(email, name, password, None, true).await.unwrap_err();
        assert!(err.kind == ErrorKind::Invalid && err.message.contains(expected), "{}", err.message);
    }
    assert!(identity.needs_setup().await.unwrap(), "nothing was created");
}

#[tokio::test]
async fn bringing_other_people_in_needs_a_paid_tier_and_an_invitation() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let owner = identity
        .register("ada@example.com", "Ada", "correct horse", None, true)
        .await
        .unwrap()
        .account
        .principal();

    // After the first account, registration is by invitation only.
    let err = identity.register("grace@example.com", "Grace", "correct horse", None, true).await.unwrap_err();
    assert_eq!(err.kind, ErrorKind::Denied);

    // The free tier cannot invite.
    let err = identity.invite(&owner, None).await.unwrap_err();
    assert!(err.message.contains("premium or enterprise"), "{}", err.message);

    identity.set_tier(&owner, Tier::Premium).await.unwrap();
    let code = identity.invite(&owner, Some("grace@example.com")).await.unwrap();

    // An invitation is for the person it names, once.
    let err = identity
        .register("mallory@example.com", "Mallory", "correct horse", Some(&code), true)
        .await
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Denied);
    let grace =
        identity.register("grace@example.com", "Grace", "correct horse", Some(&code), true).await.unwrap();
    assert_eq!(grace.account.role, "member");
    let err = identity
        .register("grace2@example.com", "Grace", "correct horse", Some(&code), true)
        .await
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Denied, "already used");

    let member = identity.authenticate(&grace.token).await.unwrap();
    assert!(member.can("projects:write") && !member.can("admin"));
    assert_eq!(identity.invite(&member, None).await.unwrap_err().kind, ErrorKind::Denied);
    assert_eq!(identity.accounts().await.unwrap().len(), 2);

    let err =
        identity.register("ada@example.com", "Ada again", "correct horse", None, true).await.unwrap_err();
    assert_eq!(err.kind, ErrorKind::Denied);
}

#[tokio::test]
async fn an_agent_credential_is_scoped_to_its_slot_and_replaced_when_reissued() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let scopes = vec!["channels".to_string(), "board:read".to_string()];
    let first = identity.issue(CredentialKind::Agent, "builder", "builder", Some(scopes)).await.unwrap();
    assert!(first.starts_with("agt_") && first.len() > 50);

    let who = identity.authenticate(&first).await.unwrap();
    assert_eq!(
        (who.kind, who.id.as_str(), who.actor().as_str()),
        (PrincipalKind::Agent, "builder", "agent:builder")
    );
    assert!(who.can("channels:write") && who.can("board:read"));
    assert!(!who.can("board:write") && !who.can("shell") && !who.can("admin"));

    // Issuing again replaces: a slot only ever has one credential that works.
    let second = identity.issue(CredentialKind::Agent, "builder", "builder", None).await.unwrap();
    assert_eq!(identity.authenticate(&first).await.unwrap_err().kind, ErrorKind::Unauthenticated);
    let who = identity.authenticate(&second).await.unwrap();
    assert!(
        who.can("projects:write") && !who.can("shell"),
        "the defaults for an agent do not include a shell"
    );

    let listed = identity.credentials().await.unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed.iter().filter(|credential| credential.revoked.is_none()).count(), 1);
    assert!(!serde_json::to_string(&listed).unwrap().contains(&second[4..]), "tokens are never listed");

    identity.revoke_subject(CredentialKind::Agent, "builder").await.unwrap();
    assert!(identity.authenticate(&second).await.is_err());
    assert!(identity.authenticate("agt_made-up").await.is_err());
}

#[tokio::test]
async fn someone_vouched_for_by_the_identity_service_gets_an_account_here() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let first = identity
        .sign_in_external("provider", "user_01", "ada@example.com", "Ada", Tier::Premium)
        .await
        .unwrap();
    assert_eq!((first.account.role.as_str(), first.account.tier), ("owner", Tier::Premium));
    assert_eq!(identity.tier().await.unwrap(), Tier::Premium);

    // Signing in again is the same account, with whatever tier they have now.
    let again = identity
        .sign_in_external("provider", "user_01", "ada@example.com", "Ada", Tier::Enterprise)
        .await
        .unwrap();
    assert_eq!(again.account.id, first.account.id);
    assert_eq!(identity.accounts().await.unwrap().len(), 1);
    assert!(identity.authenticate(&again.token).await.unwrap().can("admin"));
    // They have no password here, so cannot be signed in to with one.
    assert!(identity.sign_in("ada@example.com", "anything at all").await.is_err());
}

#[tokio::test]
async fn the_first_account_is_made_from_the_machine_itself_and_one_that_was_invited_from_anywhere() {
    let rig = Rig::new().await;
    let identity = &rig.identity;

    // Until there is an owner, whoever registers becomes one: so that is
    // done where the installation runs, and being able to reach it is not
    // enough.
    let err =
        identity.register("mallory@example.com", "Mallory", "correct horse", None, false).await.unwrap_err();
    assert_eq!(err.kind, ErrorKind::Denied);
    assert!(err.message.contains("its first account is made on the machine it runs on"), "{}", err.message);
    assert!(identity.needs_setup().await.unwrap(), "nobody became the owner");

    let owner = identity
        .register("ada@example.com", "Ada", "correct horse", None, true)
        .await
        .unwrap()
        .account
        .principal();
    // Someone the owner invited registers from wherever they are.
    identity.set_tier(&owner, Tier::Premium).await.unwrap();
    let code = identity.invite(&owner, Some("grace@example.com")).await.unwrap();
    let grace =
        identity.register("grace@example.com", "Grace", "correct horse", Some(&code), false).await.unwrap();
    assert_eq!(grace.account.role, "member");
}

#[tokio::test]
async fn an_invitation_to_register_works_for_a_while_and_no_longer() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let owner = identity
        .register("ada@example.com", "Ada", "correct horse", None, true)
        .await
        .unwrap()
        .account
        .principal();
    identity.set_tier(&owner, Tier::Premium).await.unwrap();
    let day = Duration::from_secs(24 * 3600);

    // A week unless whoever makes it says; as long as they say, within thirty days.
    let (usual, until) = identity.invite_for(&owner, None, None).await.unwrap();
    assert_eq!(until - rig.journal.now_ms(), 7 * day.as_millis() as i64);
    let (short, _) = identity.invite_for(&owner, None, Some(day)).await.unwrap();
    let (long, _) = identity.invite_for(&owner, None, Some(day * 30)).await.unwrap();
    for wrong in [Duration::ZERO, day * 31] {
        let refused = identity.invite_for(&owner, None, Some(wrong)).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Invalid);
        assert!(refused.message.contains("no more than thirty days"), "{}", refused.message);
    }

    // It is read from the clock when it is used: a moment before its end
    // it lets somebody in, and from its end it lets nobody in.
    let register = async |email: &str, code: &str| {
        identity
            .register(email, "Grace", "correct horse", Some(code), false)
            .await
            .map(|signed| signed.account.role)
    };
    rig.clock.advance(day * 2);
    let over = register("grace@example.com", &short).await.unwrap_err();
    assert_eq!(over.kind, ErrorKind::Denied);
    assert!(over.message.contains("has stopped working"), "{}", over.message);
    rig.clock.advance(day * 5 - Duration::from_millis(1));
    assert_eq!(register("grace@example.com", &usual).await.unwrap(), "member");
    rig.clock.advance(day * 22);
    assert_eq!(register("hedy@example.com", &long).await.unwrap(), "member");
    let (late, _) = identity.invite_for(&owner, None, None).await.unwrap();
    rig.clock.advance(day * 7);
    assert_eq!(register("joan@example.com", &late).await.unwrap_err().kind, ErrorKind::Denied);
    assert_eq!(identity.accounts().await.unwrap().len(), 3, "nobody was let in by one that had run out");
}

#[tokio::test]
async fn that_something_was_withdrawn_is_said_when_something_was_and_heard_by_whoever_listened_before() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let ada = identity.register("ada@example.com", "Ada", "correct horse", None, true).await.unwrap();
    let heard = identity.withdrawn();

    // Signing out is asked with any token at all. One that is nobody's
    // ends nothing, and nobody who goes on being answered is asked again
    // who they are for it.
    identity.sign_out("ses_nobodys").await.unwrap();
    identity.revoke("cred_nobodys").await.unwrap();
    assert!(!heard.has_changed().unwrap(), "nothing was withdrawn");

    // One that is somebody's: whoever was listening before it was ended
    // hears that it was, however long after they look.
    identity.sign_out(&ada.token).await.unwrap();
    assert!(heard.has_changed().unwrap());
    // And ended again, it ends nothing more.
    let again = identity.withdrawn();
    identity.sign_out(&ada.token).await.unwrap();
    assert!(!again.has_changed().unwrap());
}

/// A wrong password tried for each of `emails` emails nobody has.
async fn guessed_at(identity: &Identity, emails: usize) {
    for n in 0..emails {
        let err = identity.sign_in(&format!("nobody{n}@example.com"), "a guess").await.unwrap_err();
        assert_eq!(err.kind, ErrorKind::Unauthenticated);
    }
}

#[tokio::test]
async fn wrong_passwords_are_remembered_for_no_more_emails_than_there_is_room_for() {
    let rig = Rig::new().await;
    // Room for a handful, so that using it up is not a thousand passwords
    // looked at: what happens then is the same whatever the room.
    let identity = &rig.identity.clone().remembering(16);
    identity.register("ada@example.com", "Ada", "correct horse", None, true).await.unwrap();
    // Anyone who can reach the daemon can try any email they like.
    guessed_at(identity, 48).await;
    assert!(
        identity.remembered_failures() <= 16,
        "{} emails are remembered, and whoever is guessing decides how many more",
        identity.remembered_failures()
    );
    // What it is for still holds for an account that is being guessed at.
    for _ in 0..5 {
        identity.sign_in("ada@example.com", "a guess").await.unwrap_err();
    }
    let locked = identity.sign_in("ada@example.com", "correct horse").await.unwrap_err();
    assert_eq!(locked.kind, ErrorKind::Denied, "{}", locked.message);
}

#[tokio::test]
async fn a_credential_made_for_a_device_is_nobodys() {
    // A device is known to its hub by the key it proves, and by nothing it
    // can send. One of the credentials devices used to hold, however it
    // came to be there, opens nothing.
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let token = identity.issue(CredentialKind::Device, "builder-1", "builder-1", None).await.unwrap();
    assert!(identity.authenticate(&token).await.is_err());
}

#[tokio::test]
async fn somebody_guessing_from_elsewhere_keeps_nobody_at_the_machine_out_and_cannot_guess_past_the_count() {
    let rig = Rig::new().await;
    let identity = &rig.identity.clone().remembering(16);
    identity.register("ada@example.com", "Ada", "correct horse", None, true).await.unwrap();
    // Guesses made side by side each find the ones before them: none gets
    // past the count by arriving together.
    let guesses: Vec<_> = (0..20)
        .map(|_| {
            let identity = identity.clone();
            tokio::spawn(
                async move { identity.sign_in("ada@example.com", "a guess").await.unwrap_err().kind },
            )
        })
        .collect();
    let mut looked_at = 0;
    for guess in guesses {
        looked_at += usize::from(guess.await.unwrap() == ErrorKind::Unauthenticated);
    }
    assert_eq!(looked_at, 5, "five passwords were looked at, and the rest turned away unread");
    // And whoever guesses at other emails, more of them than there is
    // room for, does not push that count out.
    guessed_at(identity, 48).await;
    let still = identity.sign_in("ada@example.com", "correct horse").await.unwrap_err();
    assert_eq!(still.kind, ErrorKind::Denied, "{}", still.message);
    // Whoever sits at the machine is counted apart, and signs in.
    identity.sign_in_from("ada@example.com", "correct horse", true).await.unwrap();
}
