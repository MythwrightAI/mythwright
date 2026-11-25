//! How somebody arranged the side panel, kept with their account
//! (`people::arranged`): what an arrangement is held to, case for case
//! with the table the client reads, whose it is, and who is told what is
//! said of it.

use journal::EventQuery;
use kernel::principal::MEMBER_SCOPES;
use kernel::{ErrorKind, Principal, PrincipalKind};
use people::arranged::{LONGEST_NAME, checked};
use serde_json::{Value, json};

use crate::Rig;

/// The table the client reads too: arrangements and the sentence each
/// comes to, or nothing for one that is kept.
fn table() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/arrangedcases.json");
    let kept = std::fs::read_to_string(path).expect("the table of arrangements is in the client's sources");
    serde_json::from_str(&kept).expect("the table is a list of cases")
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

fn mine() -> Value {
    json!({ "groups": [{ "id": "g1", "name": "Shop work", "channels": ["ch_a", "ch_b"] }], "pinned": ["ch_c"],
            "things": ["/board/shop"] })
}

#[test]
fn an_arrangement_is_kept_or_refused_in_its_sentence_case_for_case() {
    let table = table();
    let cases = table["cases"].as_array().unwrap();
    assert!(cases.len() > 20, "the table is there");
    for case in cases {
        let came = checked(&case[0]).err();
        assert_eq!(came.as_deref(), case[1].as_str(), "{}", case[0]);
    }
    assert_eq!(table["longest"], json!(LONGEST_NAME), "the client holds a name to what is kept");
    assert!(cases.iter().filter(|case| case[1].is_null()).count() >= 3, "arrangements that are kept");
}

#[test]
fn what_is_kept_is_each_name_trimmed_and_nothing_said_is_nothing() {
    let kept = checked(&json!({ "groups": [{ "id": "g1", "name": "  Work " }] })).unwrap();
    assert_eq!(kept.groups[0].name, "Work");
    assert!(kept.groups[0].channels.is_empty() && kept.pinned.is_empty() && kept.things.is_empty());
    // An id is text and nothing more: one that names nothing is kept.
    let gone = checked(&json!({ "pinned": ["ch_in_the_trash"], "things": ["/project/gone"] })).unwrap();
    assert_eq!((gone.pinned.len(), gone.things.len()), (1, 1));
}

#[tokio::test]
async fn an_arrangement_is_its_owners_alone_and_said_to_them_alone() {
    let rig = Rig::new().await;
    let arranged = &rig.arranged;
    let ada = person("ada", &["*"]);
    let grace = person("grace", MEMBER_SCOPES);
    let none = arranged.of(&ada).await.unwrap();
    assert_eq!((none.arrangement, none.nth), (None, 0), "nothing kept until something is");

    let kept = arranged.keep(&grace, &mine(), false).await.unwrap();
    assert_eq!((kept.nth, kept.arrangement.as_ref().unwrap().groups[0].name.as_str()), (1, "Shop work"));
    // The same again is no write.
    assert_eq!(arranged.keep(&grace, &mine(), false).await.unwrap().nth, 1);
    // Somebody else's is not there for anybody else, whatever they hold.
    assert_eq!(arranged.of(&ada).await.unwrap().arrangement, None);
    // Handed over first, it is kept only where nothing is.
    let other = json!({ "pinned": ["ch_z"] });
    let first = arranged.keep(&grace, &other, true).await.unwrap();
    assert_eq!(first.arrangement, kept.arrangement, "what was kept stands");
    let ada_first = arranged.keep(&ada, &other, true).await.unwrap();
    assert_eq!(ada_first.arrangement.unwrap().pinned, ["ch_z"], "and is kept where nothing was");
    let changed = arranged.keep(&grace, &json!({}), false).await.unwrap();
    assert_eq!((changed.nth, changed.arrangement.unwrap().groups.len()), (2, 0), "kept whole");

    let said =
        rig.journal.read(EventQuery { topics: "arranged.*".into(), ..Default::default() }).await.unwrap();
    assert_eq!(said.len(), 3, "each change said once, and nothing that came to what was kept");
    assert_eq!(said[0].data["arrangement"]["groups"][0]["channels"], json!(["ch_a", "ch_b"]), "said whole");
    let told = |who: &Principal| said.iter().filter(|event| rig.journal.may_tell(who, event)).count();
    assert_eq!((told(&grace), told(&ada)), (2, 1));
    assert_eq!(told(&person("root", &["*"])), 0, "everything held is nobody");
}

#[tokio::test]
async fn reading_and_changing_each_take_their_scope_and_an_agent_keeps_none() {
    let rig = Rig::new().await;
    let arranged = &rig.arranged;
    assert_eq!(arranged.of(&person("grace", &["themes"])).await.unwrap_err().kind, ErrorKind::Denied);
    let reads = person("grace", &["arranged:read"]);
    assert!(arranged.of(&reads).await.is_ok());
    assert_eq!(arranged.keep(&reads, &mine(), false).await.unwrap_err().kind, ErrorKind::Denied);
    let writes = person("grace", &["arranged:write"]);
    assert!(arranged.keep(&writes, &mine(), false).await.is_ok());
    let refused = arranged.keep(&writes, &json!({ "folds": [] }), false).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid);
    assert!(refused.message.contains("nothing an arrangement says"), "{refused:?}");
    let agent = Principal { kind: PrincipalKind::Agent, ..person("worker", &["*"]) };
    assert!(arranged.of(&agent).await.unwrap_err().message.contains("an agent or a device has none"));
}
