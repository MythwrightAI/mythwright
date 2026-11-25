//! Somebody a service outside vouches for (`people::outside`): the
//! account is the issuer's subject's and no other, whatever address the
//! statement carries.

use kernel::ErrorKind;
use people::{Account, Tier};

use crate::Rig;

/// The account called `email`, as the installation lists it.
async fn account(rig: &Rig, email: &str) -> Account {
    let all = rig.identity.accounts().await.unwrap();
    all.into_iter().find(|account| account.email == email).expect("an account with that address")
}

/// Whose each account is outside, as it is kept: nothing for an account
/// no service vouched for.
async fn kept_as(rig: &Rig) -> Vec<(String, Option<String>)> {
    rig.journal
        .db()
        .call(|connection| {
            let mut statement = connection.prepare("SELECT email, external FROM accounts ORDER BY email")?;
            let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
            rows.collect()
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn an_issuer_that_says_an_address_an_account_has_is_refused_and_nothing_of_that_account_is_changed() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let owner = identity.register("ada@example.com", "Ada", "correct horse", None, true).await.unwrap();
    assert_eq!((owner.account.role.as_str(), owner.account.tier), ("owner", Tier::Free));

    // A service says the owner's address, however it is written, and a
    // plan: it is handed nothing, and says what is in the way.
    let refused = identity
        .sign_in_external(
            "https://issuer.example",
            "user_01",
            " Ada@Example.com ",
            "Somebody",
            Tier::Enterprise,
        )
        .await
        .expect_err("nobody is signed in to an account by its address");
    assert_eq!(refused.kind, ErrorKind::Conflict, "{}", refused.message);
    assert!(refused.message.contains("already has the address ada@example.com"), "{}", refused.message);
    assert!(
        refused.message.contains("`https://issuer.example` vouches for as `user_01`"),
        "{}",
        refused.message
    );
    assert!(
        refused.message.contains("takes that account's own say"),
        "what to do is said: {}",
        refused.message
    );

    // The owner is who they were, on the plan they had, known outside as
    // nobody, and signed in the way they always were.
    assert_eq!(kept_as(&rig).await, [("ada@example.com".to_string(), None)]);
    let still = account(&rig, "ada@example.com").await;
    assert_eq!((still.id, still.name.as_str(), still.tier), (owner.account.id, "Ada", Tier::Free));
    assert_eq!(identity.tier().await.unwrap(), Tier::Free);
    assert!(identity.sign_in("ada@example.com", "correct horse").await.is_ok());

    // The same of an account that is one issuer's subject already: a
    // second issuer, and the first of another subject, are refused its
    // address, and it is still the first's.
    let grace = identity
        .sign_in_external("https://issuer.example", "user_02", "grace@example.com", "Grace", Tier::Premium)
        .await
        .unwrap();
    let hers = kept_as(&rig).await;
    for (issuer, subject) in [("https://another.example", "user_02"), ("https://issuer.example", "user_03")] {
        let refused = identity
            .sign_in_external(issuer, subject, "grace@example.com", "Grace", Tier::Enterprise)
            .await
            .expect_err("an account is one issuer's subject's");
        assert_eq!(refused.kind, ErrorKind::Conflict, "{issuer} {subject}: {}", refused.message);
    }
    assert_eq!(kept_as(&rig).await, hers);
    assert_eq!(account(&rig, "grace@example.com").await.tier, Tier::Premium);
    assert_eq!(identity.accounts().await.unwrap().len(), 2);
    assert!(identity.authenticate(&grace.token).await.is_ok());
}

#[tokio::test]
async fn the_same_issuer_and_subject_sign_in_as_one_account_whatever_address_the_statement_carries() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let other = identity.register("grace@example.com", "Grace", "correct horse", None, true).await.unwrap();
    let first = identity
        .sign_in_external("https://issuer.example", "user_01", "ada@example.com", "Ada", Tier::Free)
        .await
        .unwrap();
    assert_eq!(first.account.role, "member");

    // Again, with the plan they have now: the account they were.
    let again = identity
        .sign_in_external("https://issuer.example", "user_01", "ada@example.com", "Ada", Tier::Premium)
        .await
        .unwrap();
    assert_eq!((again.account.id.as_str(), again.account.tier), (first.account.id.as_str(), Tier::Premium));
    assert_ne!(again.token, first.token, "each sign-in is its own session");

    // And saying another address, an account's here among them: it is
    // still theirs by who vouches for them, the address finds nobody,
    // and the account whose address it is, is as it was.
    for email in ["ada@elsewhere.example", "grace@example.com"] {
        let moved = identity
            .sign_in_external("https://issuer.example", "user_01", email, "Ada", Tier::Enterprise)
            .await
            .unwrap();
        assert_eq!(moved.account.id, first.account.id, "{email}");
        assert_eq!(moved.account.email, "ada@example.com", "{email}");
    }
    assert_eq!(identity.accounts().await.unwrap().len(), 2);
    let grace = account(&rig, "grace@example.com").await;
    assert_eq!((grace.id, grace.tier), (other.account.id, Tier::Free), "the plan was written on one account");
    assert_eq!(account(&rig, "ada@example.com").await.tier, Tier::Enterprise);
}

#[tokio::test]
async fn two_issuers_that_say_the_same_subject_are_two_accounts_and_a_plan_is_written_on_the_one_it_is_said_of()
 {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let one = identity
        .sign_in_external("https://issuer.example", "user_01", "ada@example.com", "Ada", Tier::Free)
        .await
        .unwrap();
    let two = identity
        .sign_in_external("https://another.example", "user_01", "ada@another.example", "Ada", Tier::Free)
        .await
        .unwrap();
    assert_ne!(one.account.id, two.account.id);
    assert_eq!((one.account.role.as_str(), two.account.role.as_str()), ("owner", "member"));

    let again = identity
        .sign_in_external(
            "https://another.example",
            "user_01",
            "ada@another.example",
            "Ada",
            Tier::Enterprise,
        )
        .await
        .unwrap();
    assert_eq!(again.account.id, two.account.id);
    assert_eq!(account(&rig, "ada@example.com").await.tier, Tier::Free);
    assert_eq!(account(&rig, "ada@another.example").await.tier, Tier::Enterprise);

    // Who vouches and for whom are two things however they are written:
    // a mark that is in one of them does not make two pairs one.
    let split = identity.sign_in_external("a:b", "c", "one@example.com", "One", Tier::Free).await.unwrap();
    let other = identity.sign_in_external("a", "b:c", "two@example.com", "Two", Tier::Free).await.unwrap();
    assert_ne!(split.account.id, other.account.id);
    assert_eq!(identity.accounts().await.unwrap().len(), 4);

    // And a statement that leaves either out is nobody's.
    for (issuer, subject) in [("", "user_09"), ("https://issuer.example", " ")] {
        let refused = identity
            .sign_in_external(issuer, subject, "nobody@example.com", "Nobody", Tier::Free)
            .await
            .expect_err("a statement names who vouches and for whom");
        assert_eq!(refused.kind, ErrorKind::Invalid, "{}", refused.message);
    }
    assert_eq!(identity.accounts().await.unwrap().len(), 4);
}
