//! How a time is written for somebody, kept with their account
//! (`people::clocked`): what a setting is held to, case for case with the
//! table the client reads, whose it is, and who is told what is said of it.

use journal::EventQuery;
use kernel::principal::MEMBER_SCOPES;
use kernel::{ErrorKind, Principal, PrincipalKind};
use people::clocked::{LONGEST_ZONE, checked};
use serde_json::{Value, json};

use crate::Rig;

/// The table the client reads too: settings and the sentence each comes
/// to, or nothing for one that is kept.
fn table() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/clockcases.json");
    let kept = std::fs::read_to_string(path).expect("the table of settings is in the client's sources");
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

fn denver() -> Value {
    json!({ "zone": "America/Denver", "hours": "12h" })
}

#[test]
fn a_setting_is_kept_or_refused_in_its_sentence_case_for_case() {
    let table = table();
    let cases = table["cases"].as_array().unwrap();
    assert!(cases.len() > 15, "the table is there");
    for case in cases {
        let came = checked(&case[0]).err();
        assert_eq!(came.as_deref(), case[1].as_str(), "{}", case[0]);
    }
    assert_eq!(table["longest"], json!(LONGEST_ZONE), "the client holds a zone to what is kept");
    assert!(cases.iter().filter(|case| case[1].is_null()).count() >= 3, "settings that are kept");
}

#[test]
fn a_clock_of_twelve_hours_is_what_nothing_said_comes_to() {
    let kept = checked(&json!({ "zone": "Europe/Berlin" })).unwrap();
    assert_eq!((kept.zone.as_str(), kept.hours.as_str()), ("Europe/Berlin", "12h"));
    // A zone is text to the daemon: one that is no zone anybody knows,
    // written as a zone's name is, is kept, and the client holds it to
    // the zones its browser knows before it sends it.
    assert!(checked(&json!({ "zone": "Nowhere/Atall" })).is_ok());
}

#[tokio::test]
async fn a_setting_is_its_owners_alone_and_said_to_them_alone() {
    let rig = Rig::new().await;
    let clocked = &rig.clocked;
    let ada = person("ada", &["*"]);
    let grace = person("grace", MEMBER_SCOPES);
    let none = clocked.of(&ada).await.unwrap();
    assert_eq!((none.clock, none.nth), (None, 0), "nothing kept until something is");

    let kept = clocked.keep(&grace, &denver(), false).await.unwrap();
    assert_eq!((kept.nth, kept.clock.as_ref().unwrap().zone.as_str()), (1, "America/Denver"));
    // The same again is no write.
    assert_eq!(clocked.keep(&grace, &denver(), false).await.unwrap().nth, 1);
    // Somebody else's is not there for anybody else, whatever they hold.
    assert_eq!(clocked.of(&ada).await.unwrap().clock, None);
    // Handed over first, the zone a browser says is kept only where nothing is.
    let browser = json!({ "zone": "Asia/Tokyo" });
    assert_eq!(clocked.keep(&grace, &browser, true).await.unwrap().clock, kept.clock, "what was kept stands");
    let ada_first = clocked.keep(&ada, &browser, true).await.unwrap();
    assert_eq!(ada_first.clock.unwrap().zone, "Asia/Tokyo", "and is kept where nothing was");
    let changed =
        clocked.keep(&grace, &json!({ "zone": "America/Denver", "hours": "24h" }), false).await.unwrap();
    assert_eq!((changed.nth, changed.clock.unwrap().hours.as_str()), (2, "24h"), "kept whole");

    let said = rig.journal.read(EventQuery { topics: "clock.*".into(), ..Default::default() }).await.unwrap();
    assert_eq!(said.len(), 3, "each change said once, and nothing that came to what was kept");
    assert_eq!(said[0].data["clock"], denver(), "said whole");
    let told = |who: &Principal| said.iter().filter(|event| rig.journal.may_tell(who, event)).count();
    assert_eq!((told(&grace), told(&ada)), (2, 1));
    assert_eq!(told(&person("root", &["*"])), 0, "everything held is nobody");
}

#[tokio::test]
async fn reading_and_changing_each_take_their_scope_and_an_agent_keeps_none() {
    let rig = Rig::new().await;
    let clocked = &rig.clocked;
    assert_eq!(clocked.of(&person("grace", &["themes"])).await.unwrap_err().kind, ErrorKind::Denied);
    let reads = person("grace", &["clock:read"]);
    assert!(clocked.of(&reads).await.is_ok());
    assert_eq!(clocked.keep(&reads, &denver(), false).await.unwrap_err().kind, ErrorKind::Denied);
    let writes = person("grace", &["clock:write"]);
    assert!(clocked.keep(&writes, &denver(), false).await.is_ok());
    let refused = clocked.keep(&writes, &json!({ "zone": "America/New York" }), false).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid);
    assert!(refused.message.contains("is none of them"), "{refused:?}");
    let agent = Principal { kind: PrincipalKind::Agent, ..person("worker", &["*"]) };
    assert!(clocked.of(&agent).await.unwrap_err().message.contains("an agent or a device has none"));
}
