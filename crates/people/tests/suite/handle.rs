//! What somebody with an account is named by after an `@`
//! (`people::handle`), held to the table the client's own rule is held
//! to, and who a word names among the accounts there are.

use people::{CredentialKind, Tier, handle, held, named};

use crate::Rig;

/// The cases the client's `handleOf` is held to by its own test: a name
/// and the word it is named by.
fn cases() -> Vec<(String, String)> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/handles.json");
    let kept = std::fs::read_to_string(path).expect("the table of handles is in the client's sources");
    serde_json::from_str(&kept).expect("the table is a list of pairs: a name and its handle")
}

#[test]
fn a_person_is_named_by_the_word_the_client_offers_for_their_name_case_for_case() {
    let cases = cases();
    assert!(cases.len() > 20, "the table is there");
    for (name, named_by) in &cases {
        assert_eq!(&handle(name), named_by, "{name:?}");
    }
    // What the table is to have in it, so that neither rule is held to less.
    let has = |wanted: &dyn Fn(&str, &str) -> bool| cases.iter().any(|(name, by)| wanted(name, by));
    assert!(has(&|name, by| name.contains(' ') && by.contains('-')), "a space");
    assert!(has(&|name, _| name.chars().any(char::is_uppercase)), "a capital");
    assert!(has(&|name, by| name.contains('_') && by.contains('_')), "an underscore, which stays");
    assert!(has(&|name, by| name.contains('-') && by.contains('-')), "a dash");
    assert!(has(&|name, by| !name.is_ascii() && !by.is_empty()), "an accent");
    assert!(has(&|name, by| name.is_empty() && by.is_empty()), "an empty name");
    assert!(has(&|name, by| !name.is_empty() && by.is_empty()), "a name nothing of which can be written");
    let shared = cases.iter().filter(|(_, by)| by == "ada").count();
    assert!(shared >= 2, "two names with one handle");
    // A handle is what a mention is read as: nothing in one ends a mention.
    for (_, by) in &cases {
        assert!(
            by.chars().all(|letter| letter.is_ascii_lowercase()
                || letter.is_ascii_digit()
                || "-_".contains(letter))
        );
        assert!(!by.starts_with('-') && !by.ends_with('-'), "{by}");
    }
}

#[tokio::test]
async fn a_word_names_every_account_called_so_as_each_is_now_and_nobody_where_it_is_nobodys() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let owner =
        identity.register("ada@example.com", "Ada Lovelace", "correct horse", None, true).await.unwrap();
    let owner = owner.account.principal();
    identity.set_tier(&owner, Tier::Premium).await.unwrap();
    let join = async |email: &str, name: &str| {
        let code = identity.invite(&owner, Some(email)).await.unwrap();
        identity.register(email, name, "correct horse", Some(&code), false).await.unwrap().account.principal()
    };
    let other = join("ada@example.org", "ada  lovelace!").await;
    let grace = join("grace@example.com", "Grace").await;
    join("nobody@example.com", "!!!").await;
    let asked = |names: &[&str]| {
        let names: Vec<String> = names.iter().map(|name| name.to_string()).collect();
        rig.journal.db().call(move |connection| named(connection, &names))
    };
    // Two accounts called the same are both named, the earlier first,
    // each with what it holds: the owner more than a member.
    let both = asked(&["ada-lovelace"]).await.unwrap();
    assert_eq!(both, [owner.clone(), other]);
    assert!(both[0].can("admin") && !both[1].can("admin"));
    assert_eq!(asked(&["grace", "nobody", "ada"]).await.unwrap(), [grace]);
    // No word names an account nothing of whose name can be written.
    assert!(asked(&[""]).await.unwrap().is_empty());
    assert!(asked(&[]).await.unwrap().is_empty());
}

#[tokio::test]
async fn what_somebody_holds_now_is_read_where_it_is_asked_and_is_nothing_for_whoever_is_gone() {
    let rig = Rig::new().await;
    let identity = &rig.identity;
    let owner = identity.register("ada@example.com", "Ada", "correct horse", None, true).await.unwrap();
    let owner = owner.account.principal();
    let asked = |actor: &str| {
        let actor = actor.to_string();
        rig.journal.db().call(move |connection| held(connection, &actor))
    };
    // An account holds what it is given by what it is.
    assert_eq!(asked(&owner.actor()).await.unwrap(), owner.scopes);
    assert!(asked("user:acct_nobody").await.unwrap().is_empty());
    // An agent holds what its credential does, while it has one.
    let scopes = vec!["inbox".to_string(), "channels:read".to_string()];
    identity.issue(CredentialKind::Agent, "scout", "scout", Some(scopes.clone())).await.unwrap();
    assert_eq!(asked("agent:scout").await.unwrap(), scopes);
    identity.revoke_subject(CredentialKind::Agent, "scout").await.unwrap();
    assert!(asked("agent:scout").await.unwrap().is_empty());
    // A key acts as its maker and is asked as its maker; anybody else holds nothing.
    for nobody in ["bot:conductor", "system", "device:laptop", "agent:nobody", ""] {
        assert!(asked(nobody).await.unwrap().is_empty(), "{nobody}");
    }
}

fn somebody(id: &str, name: &str, email: &str) -> people::Account {
    people::Account {
        id: id.into(),
        email: email.into(),
        name: name.into(),
        role: "member".into(),
        tier: Tier::Free,
        created: 0,
        given: Vec::new(),
    }
}

#[test]
fn a_word_given_as_a_cards_assignee_is_an_agent_by_its_name_or_a_person_by_handle_or_email_or_refused() {
    use people::handle::assignee;
    let agents = vec!["shop-worker-1".to_string(), "ada".to_string()];
    let accounts = vec![
        somebody("u1", "Hedy Lamarr", "hedy@example.org"),
        somebody("u2", "Ada", "ada@example.org"),
        somebody("u3", "Grace", "grace@example.org"),
        somebody("u4", "Grace", "grace.h@example.org"),
    ];
    // What, the word, and whom it comes to.
    let found: Vec<(&str, &str, &str)> = vec![
        ("an actor as it is written", "user:u3", "user:u3"),
        ("an agent by its name", "shop-worker-1", "agent:shop-worker-1"),
        ("a person by their handle", "hedy-lamarr", "user:u1"),
        ("a person by their email, however it is cased", "HEDY@example.org", "user:u1"),
        ("one of two people called the same, by email", "grace.h@example.org", "user:u4"),
    ];
    for (case, word, whom) in found {
        assert_eq!(assignee(word, &agents, &accounts).as_deref(), Ok(whom), "{case}");
    }
    // What is refused, and what the sentence says.
    let refused: Vec<(&str, &str, &[&str])> = vec![
        (
            "nobody's: who it could be listed",
            "hedy",
            &[
                "`hedy` is no agent's name and no person's handle or email here",
                "shop-worker-1",
                "hedy-lamarr (hedy@example.org)",
            ],
        ),
        ("an agent's name and a person's handle", "ada", &["`agent:ada`", "`user:u2`"]),
        (
            "two people's handle",
            "grace",
            &["names 2 people", "grace (grace.h@example.org)", "give it by email"],
        ),
    ];
    for (case, word, says) in refused {
        let said = assignee(word, &agents, &accounts).unwrap_err();
        for part in says {
            assert!(said.contains(part), "{case}: {said}");
        }
    }
    let empty = assignee("hedy", &[], &[]).unwrap_err();
    assert!(empty.ends_with("there is nobody yet"), "{empty}");
}
