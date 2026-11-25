//! How what somebody reads looks for them, kept with their account
//! (`people::appearance`): what a setting is held to, case for case with
//! the table the client reads, whose it is, and who is told what is said
//! of it.

use journal::EventQuery;
use kernel::principal::MEMBER_SCOPES;
use kernel::{ErrorKind, Principal, PrincipalKind};
use people::appearance::{SETTINGS, checked};
use serde_json::{Value, json};

use crate::Rig;

/// The table the client reads too: the words of each setting, and
/// settings with the sentence each comes to, or nothing for one that is kept.
fn table() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/appearancecases.json");
    let kept = std::fs::read_to_string(path).expect("the table of settings is in the client's sources");
    serde_json::from_str(&kept).expect("the table is its words and a list of cases")
}

fn person(id: &str, scopes: &[&str]) -> Principal {
    Principal {
        kind: PrincipalKind::User,
        id: id.into(),
        name: id.into(),
        scopes: scopes.iter().map(|scope| scope.to_string()).collect(),
        key: None,
    }
}

#[test]
fn a_setting_is_kept_or_refused_in_its_sentence_case_for_case() {
    let table = table();
    for (name, words, usual) in SETTINGS {
        assert_eq!(table["words"][name], json!(words), "the client offers the words of `{name}` kept here");
        assert!(words.contains(&usual), "what `{name}` is where nothing is said is one of its words");
    }
    let cases = table["cases"].as_array().unwrap();
    assert!(cases.len() > 10, "the table is there");
    for case in cases {
        let came = checked(&case[0]).err();
        assert_eq!(came.as_deref(), case[1].as_str(), "{}", case[0]);
    }
}

#[test]
fn what_is_not_said_is_as_it_usually_is() {
    let kept = checked(&json!({ "width": "wide" })).unwrap();
    assert_eq!(
        (kept.text.as_str(), kept.width.as_str(), kept.colour.as_str()),
        ("usual", "wide", "automatic")
    );
    let none = checked(&json!({})).unwrap();
    assert_eq!(
        (none.text.as_str(), none.width.as_str(), none.colour.as_str()),
        ("usual", "standard", "automatic")
    );
}

#[test]
fn a_setting_kept_before_contents_were_a_word_of_it_shows_them() {
    let before = json!({ "text": "larger", "width": "wide", "colour": "dark" });
    let kept: people::appearance::Setting = serde_json::from_value(before).unwrap();
    assert_eq!((kept.width.as_str(), kept.contents.as_str()), ("wide", "shown"));
    let hidden = checked(&json!({ "contents": "hidden" })).unwrap();
    assert_eq!((hidden.text.as_str(), hidden.contents.as_str()), ("usual", "hidden"));
}

#[test]
fn a_setting_kept_before_the_typeface_was_a_word_of_it_reads_sans() {
    let before = json!({ "text": "larger", "width": "wide", "colour": "dark", "contents": "hidden" });
    let kept: people::appearance::Setting = serde_json::from_value(before).unwrap();
    assert_eq!((kept.contents.as_str(), kept.typeface.as_str()), ("hidden", "sans"));
    let serif = checked(&json!({ "typeface": "serif" })).unwrap();
    assert_eq!((serif.text.as_str(), serif.typeface.as_str()), ("usual", "serif"));
}

#[tokio::test]
async fn a_setting_is_its_owners_said_to_them_alone_and_kept_whole() {
    let rig = Rig::new().await;
    let appearance = &rig.appearance;
    let ada = person("ada", &["*"]);
    let grace = person("grace", MEMBER_SCOPES);
    let none = appearance.of(&ada).await.unwrap();
    assert_eq!((none.appearance, none.nth), (None, 0), "nothing kept until something is");

    let wide = json!({ "width": "wide" });
    let kept = appearance.keep(&grace, &wide).await.unwrap();
    assert_eq!((kept.nth, kept.appearance.as_ref().unwrap().width.as_str()), (1, "wide"));
    assert_eq!(appearance.keep(&grace, &wide).await.unwrap().nth, 1, "the same again is no write");
    assert_eq!(appearance.of(&ada).await.unwrap().appearance, None, "somebody else's is nobody else's");
    let changed = appearance.keep(&grace, &json!({ "text": "larger", "colour": "dark" })).await.unwrap();
    let whole = changed.appearance.unwrap();
    assert_eq!(
        (changed.nth, whole.text.as_str(), whole.width.as_str(), whole.colour.as_str()),
        (2, "larger", "standard", "dark"),
        "kept whole: what is not said is as usual"
    );

    let said =
        rig.journal.read(EventQuery { topics: "appearance.*".into(), ..Default::default() }).await.unwrap();
    assert_eq!(said.len(), 2, "each change said once, and nothing that came to what was kept");
    assert_eq!(
        said[0].data["appearance"],
        json!({ "text": "usual", "width": "wide", "colour": "automatic", "contents": "shown", "typeface": "sans" })
    );
    let told = |who: &Principal| said.iter().filter(|event| rig.journal.may_tell(who, event)).count();
    assert_eq!((told(&grace), told(&ada)), (2, 0));
}

#[tokio::test]
async fn reading_and_changing_take_what_themes_take_and_an_agent_keeps_none() {
    let rig = Rig::new().await;
    let appearance = &rig.appearance;
    assert_eq!(appearance.of(&person("grace", &["clock:read"])).await.unwrap_err().kind, ErrorKind::Denied);
    let reads = person("grace", &["themes:read"]);
    assert!(appearance.of(&reads).await.is_ok());
    assert_eq!(appearance.keep(&reads, &json!({})).await.unwrap_err().kind, ErrorKind::Denied);
    let writes = person("grace", &["themes:write"]);
    assert!(appearance.keep(&writes, &json!({ "colour": "light" })).await.is_ok());
    let refused = appearance.keep(&writes, &json!({ "width": "full" })).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid);
    assert!(refused.message.contains("is none of them"), "{refused:?}");
    let agent = Principal { kind: PrincipalKind::Agent, ..person("worker", &["*"]) };
    assert!(appearance.of(&agent).await.unwrap_err().message.contains("an agent or a device has none"));
}
