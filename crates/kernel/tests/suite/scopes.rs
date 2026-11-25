//! Which scopes there are, as one may be written down to be held.

use kernel::principal::{SCOPES, covers, named};

#[test]
fn a_scope_is_a_line_an_area_or_one_of_a_word_of_somebodys_own_and_never_everything() {
    for (scope, _) in SCOPES {
        assert!(named(scope), "{scope}");
    }
    for right in ["board", "environments", "private:hr", "emit:shop.paid", "extensions:reach"] {
        assert!(named(right), "{right}");
    }
    for wrong in ["*", "", "nothing", "board:dance", "admin:read", "private:", "emit:a b", "secrets:write"] {
        assert!(!named(wrong), "{wrong}");
    }
}

#[test]
fn the_area_of_a_factorys_secrets_does_not_hold_the_secrets_of_an_environment() {
    let held = |list: &[&str]| list.iter().map(|one| one.to_string()).collect::<Vec<_>>();
    assert!(!covers(&held(&["secrets", "secrets:use", "admin"]), "environments:secrets"));
    assert!(covers(&held(&["environments"]), "environments:secrets"));
}

#[test]
fn somebody_held_within_a_ceiling_holds_what_both_hold_and_never_more_than_they_did() {
    let held = |list: &[&str]| list.iter().map(|one| one.to_string()).collect::<Vec<_>>();
    let who = |scopes: &[&str]| kernel::Principal {
        kind: kernel::PrincipalKind::User,
        id: "ada".into(),
        name: "Ada".into(),
        scopes: held(scopes),
        key: Some("cred_1".into()),
    };
    let cases: &[(&[&str], &[&str], &[&str])] = &[
        // Everything, held to a few: those few.
        (&["*"], &["board:read", "agents"], &["agents", "board:read"]),
        // A whole area against a part of it, either way round: the part.
        (&["board"], &["board:read"], &["board:read"]),
        (&["board:read", "board:write"], &["board"], &["board:read", "board:write"]),
        // What only one of them holds: nothing.
        (&["board:read"], &["agents:read"], &[]),
        (&["board:read"], &[], &[]),
    ];
    for (holds, allow, comes) in cases {
        let within = who(holds).within(&held(allow));
        assert_eq!(within.scopes, held(comes), "{holds:?} within {allow:?}");
        assert_eq!(
            (within.name.as_str(), within.key.as_deref()),
            ("Ada", Some("cred_1")),
            "still whoever it was"
        );
    }
}
