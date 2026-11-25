//! The themes somebody makes, kept with their account (`people::themes`):
//! what a theme's colours are held to, case for case with the client's
//! own rule, what else is checked where one is written, and who is told
//! what is said of one.

use journal::EventQuery;
use kernel::principal::MEMBER_SCOPES;
use kernel::{ErrorKind, Principal, PrincipalKind};
use people::themes::{COLOURS, LIGHT, LOOKS, MOST, SHIPPED, checked, unfit, unreadable, unreadable_with};
use serde_json::{Value, json};

use crate::Rig;

/// The table the client's `unreadable` is held to by its own test: a
/// theme's colours and the sentence they come to, or nothing.
fn table() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/themecases.json");
    let kept = std::fs::read_to_string(path).expect("the table of themes is in the client's sources");
    serde_json::from_str(&kept).expect("the table is what ships and a list of cases")
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

fn dusk() -> Value {
    json!({ "ground": "#191724", "text": "#E0DEF4", "accent": "#ebbcba", "ok": "#9ccfd8", "bad": "#eb6f92",
            "info": "#c4a7e7", "agent": "#31748f", "card": "#f6c177" })
}

#[test]
fn a_themes_colours_come_to_what_the_client_says_of_them_case_for_case() {
    let table = table();
    let cases = table["cases"].as_array().unwrap();
    assert!(cases.len() > 12, "the table is there");
    for case in cases {
        let said = case[1].as_str().map(str::to_string);
        assert_eq!(unreadable(&case[0]), said, "{}", case[0]);
    }
    // What the table is to have in it, so that neither rule is held to less.
    let says = |what: &str| cases.iter().any(|case| case[1].as_str().is_some_and(|said| said.contains(what)));
    assert!(says("too near") && says("is not a colour"), "both kinds of sentence");
    assert!(cases.iter().filter(|case| case[1].is_null()).count() >= 3, "themes that are read");
    for name in ["ground", "text", "card"] {
        assert!(says(&format!("{name} is not a colour")), "{name} wrong");
    }
    // And the themes that ship, which the client's list is held to there.
    let ships: Vec<&str> = table["ships"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
    assert_eq!(ships, SHIPPED);
}

#[test]
fn a_theme_and_its_light_version_come_to_what_the_client_says_of_them_case_for_case() {
    let table = table();
    let cases = table["light"].as_array().unwrap();
    assert!(cases.len() >= 6, "the table is there");
    for case in cases {
        let said = case[2].as_str().map(str::to_string);
        assert_eq!(unreadable_with(&case[0], Some(&case[1])), said, "{} {}", case[0], case[1]);
    }
    let says =
        |what: &str| cases.iter().any(|case| case[2].as_str().is_some_and(|said| said.starts_with(what)));
    assert!(
        says(LIGHT) && says("The text"),
        "the light version said to be what is wrong, and the dark one first"
    );
    assert!(cases.iter().any(|case| case[1].is_null() && case[2].is_null()), "a theme with no light version");
}

#[test]
fn a_light_version_is_eight_colours_kept_as_the_dark_ones_are() {
    let mut light = dusk();
    light["ground"] = json!("#FAF4ED");
    light["text"] = json!("#575279");
    let (_, colours, kept, _) = checked("Dusk", &dusk(), Some(&light), None).unwrap();
    let kept = kept.expect("a light version");
    assert_eq!((kept["ground"].as_str(), kept.len()), (Some("#faf4ed"), COLOURS.len()), "kept written small");
    assert_ne!(colours, kept);
    assert_eq!(checked("Dusk", &dusk(), Some(&Value::Null), None).unwrap().2, None, "nothing said is none");
    let mut more = light.clone();
    more["glow"] = json!("#ffffff");
    assert_eq!(
        checked("Dusk", &dusk(), Some(&more), None).unwrap_err(),
        format!("{LIGHT}`glow` is no colour of a theme: a theme's colours are {}.", COLOURS.join(", "))
    );
    light["text"] = light["ground"].clone();
    assert!(
        checked("Dusk", &dusk(), Some(&light), None)
            .unwrap_err()
            .starts_with(&format!("{LIGHT}The text is too near"))
    );
}

#[tokio::test]
async fn a_theme_with_a_light_version_is_another_theme_and_one_changed_without_it_has_none() {
    let rig = Rig::new().await;
    let ada = person("ada", &["themes"]);
    let mut light = dusk();
    light["ground"] = json!("#faf4ed");
    light["text"] = json!("#575279");
    let dark = rig.themes.make(&ada, "Dusk", &dusk(), None, None).await.unwrap();
    let both = rig.themes.make(&ada, "Dusk", &dusk(), Some(&light), None).await.unwrap();
    assert_ne!(dark.id, both.id, "the same name and colours with a light version is another theme");
    assert_eq!(
        rig.themes.make(&ada, "Dusk", &dusk(), Some(&light), None).await.unwrap().id,
        both.id,
        "and made again, the one there is"
    );
    assert_eq!(both.light.as_ref().map(|light| light["ground"].clone()), Some(json!("#faf4ed")));
    let read = rig.themes.of(&ada).await.unwrap();
    assert_eq!(read.own.iter().map(|one| one.light.is_some()).collect::<Vec<_>>(), [false, true]);
    let changed = rig.themes.change(&ada, &both.id, "Dusk", &dusk(), None, None).await.unwrap();
    assert_eq!(changed.light, None, "changed whole, with no light version said");
    let said =
        rig.journal.read(EventQuery { topics: "theme.kept".into(), ..Default::default() }).await.unwrap();
    assert_eq!(said.len(), 3, "each change said once");
    assert_eq!(said[1].data["theme"]["light"]["text"], "#575279", "what is said carries the light version");
}

#[test]
fn a_theme_is_a_name_and_its_eight_colours_and_nothing_else() {
    let (name, colours, light, look) = checked("  Dusk ", &dusk(), None, None).unwrap();
    assert_eq!(light, None, "one that says no light version has none");
    assert!(look.is_empty(), "and one that says no look says nothing of it");
    assert_eq!(name, "Dusk");
    assert_eq!(colours.keys().count(), COLOURS.len());
    assert_eq!(colours["text"], "#e0def4", "kept written small");
    let refused = |name: &str, colours: &Value| checked(name, colours, None, None).unwrap_err();
    let cases: [(&str, Value, &str); 7] = [
        ("", dusk(), "is called something"),
        ("   ", dusk(), "is called something"),
        (&"a".repeat(61), dusk(), "60 letters at most, and this one is 61"),
        ("Dusk\nand dawn", dusk(), "one line"),
        ("Dusk", json!("#191724"), "a mapping of eight"),
        ("Dusk", Value::Null, "a mapping of eight"),
        (
            "Dusk",
            {
                let mut more = dusk();
                more["glow"] = json!("#ffffff");
                more
            },
            "`glow` is no colour of a theme",
        ),
    ];
    for (name, colours, says) in cases {
        let said = refused(name, &colours);
        assert!(said.contains(says), "{name:?} {colours}: {said}");
    }
    assert!(checked(&"a".repeat(60), &dusk(), None, None).is_ok(), "as long as a name may be");
}

#[tokio::test]
async fn a_theme_is_its_makers_alone_until_it_is_offered_and_who_is_told_follows_it() {
    let rig = Rig::new().await;
    let themes = &rig.themes;
    let ada = person("ada", &["*"]);
    let grace = person("grace", MEMBER_SCOPES);
    let made = themes.make(&ada, "Dusk", &dusk(), None, None).await.unwrap();
    assert_eq!((made.owner.as_str(), made.name.as_str()), ("user:ada", "Dusk"));
    // Made again to the letter, it is the one there is.
    assert_eq!(themes.make(&ada, " Dusk", &dusk(), None, None).await.unwrap().id, made.id);
    assert_eq!(themes.of(&ada).await.unwrap().own.len(), 1);
    // Somebody else's is not there for anybody else, whatever they hold.
    assert!(themes.of(&grace).await.unwrap().offered.is_empty());
    for wrong in [
        themes.change(&grace, &made.id, "Mine", &dusk(), None, None).await.unwrap_err(),
        themes.take_away(&grace, &made.id).await.unwrap_err(),
        themes.choose(&grace, Some(&made.id)).await.unwrap_err(),
    ] {
        assert_eq!(wrong.kind, ErrorKind::NotFound, "{wrong:?}");
    }
    // Offering is for whoever may manage the factory.
    assert_eq!(themes.offer(&grace, &made.id).await.unwrap_err().kind, ErrorKind::Denied);
    let offered = themes.offer(&ada, &made.id).await.unwrap();
    assert_eq!(offered.offered.as_ref().map(|by| by.by.as_str()), Some("user:ada"));
    let seen = themes.of(&grace).await.unwrap();
    assert_eq!(seen.offered.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [made.id.as_str()]);
    assert_eq!(themes.choose(&grace, Some(&made.id)).await.unwrap().as_deref(), Some(made.id.as_str()));
    assert_eq!(themes.of(&grace).await.unwrap().chosen.as_deref(), Some(made.id.as_str()));
    // Withdrawn, it is chosen by nobody but its owner, and is theirs again.
    assert_eq!(themes.withdraw(&grace, &made.id).await.unwrap_err().kind, ErrorKind::Denied);
    themes.withdraw(&ada, &made.id).await.unwrap();
    assert_eq!(
        themes.of(&grace).await.unwrap(),
        people::themes::Kept { own: vec![], offered: vec![], chosen: None }
    );
    assert_eq!(themes.choose(&grace, Some("rose-pine")).await.unwrap().as_deref(), Some("rose-pine"));
    assert_eq!(themes.choose(&grace, Some("rose-pine")).await.unwrap().as_deref(), Some("rose-pine"));

    // What was said of it, and who was told each.
    let said = rig.journal.read(EventQuery { topics: "theme.*".into(), ..Default::default() }).await.unwrap();
    let topics: Vec<&str> = said.iter().map(|event| event.topic.as_str()).collect();
    assert_eq!(
        topics,
        ["theme.kept", "theme.offered", "theme.chosen", "theme.withdrawn", "theme.kept", "theme.chosen"],
        "nothing said of what came to what it was"
    );
    let told = |who: &Principal| -> Vec<&str> {
        said.iter()
            .filter(|event| rig.journal.may_tell(who, event))
            .map(|event| event.topic.as_str())
            .collect()
    };
    assert_eq!(told(&ada), ["theme.kept", "theme.offered", "theme.withdrawn", "theme.kept"]);
    assert_eq!(told(&grace), ["theme.offered", "theme.chosen", "theme.withdrawn", "theme.chosen"]);
    assert!(
        told(&person("root", &["*"])) == ["theme.offered", "theme.withdrawn"],
        "everything held is nobody"
    );
}

#[tokio::test]
async fn an_account_keeps_as_many_themes_as_it_is_let_and_an_agent_keeps_none() {
    let rig = Rig::new().await;
    let ada = person("ada", &["themes"]);
    for nth in 0..MOST {
        rig.themes.make(&ada, &format!("Dusk {nth}"), &dusk(), None, None).await.unwrap();
    }
    let full = rig.themes.make(&ada, "One more", &dusk(), None, None).await.unwrap_err();
    assert!(full.message.contains("take one away first"), "{full:?}");
    let agent = Principal { kind: PrincipalKind::Agent, ..person("worker", &["*"]) };
    assert!(rig.themes.of(&agent).await.unwrap_err().message.contains("an agent or a device has none"));
    // Reading takes reading, and changing takes changing.
    assert_eq!(rig.themes.of(&person("grace", &["inbox"])).await.unwrap_err().kind, ErrorKind::Denied);
    let reads = person("grace", &["themes:read"]);
    assert!(rig.themes.of(&reads).await.is_ok());
    assert_eq!(
        rig.themes.make(&reads, "Dusk", &dusk(), None, None).await.unwrap_err().kind,
        ErrorKind::Denied
    );
}

#[test]
fn a_themes_look_comes_to_what_the_client_says_of_it_case_for_case() {
    let table = table();
    let cases = table["look"].as_array().unwrap();
    for case in cases {
        let said = case[1].as_str().map(str::to_string);
        assert_eq!(unfit(&case[0]), said, "{}", case[0]);
    }
    // Each setting and each of its words is as the client says it, so that
    // neither offers a word the other refuses.
    let settings: Vec<(String, Vec<String>)> = table["looks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|setting| {
            let words = setting[1].as_array().unwrap().iter().filter_map(Value::as_str).map(str::to_string);
            (setting[0].as_str().unwrap().to_string(), words.collect())
        })
        .collect();
    let ours: Vec<(String, Vec<String>)> = LOOKS
        .iter()
        .map(|(setting, words)| (setting.to_string(), words.iter().map(|word| word.to_string()).collect()))
        .collect();
    assert_eq!(settings, ours);
    let says = |what: &str| cases.iter().any(|case| case[1].as_str().is_some_and(|said| said.contains(what)));
    assert!(
        says("is none of them") && says("nothing a theme's look says") && says("is a mapping"),
        "each sentence"
    );
    assert!(cases.iter().filter(|case| case[1].is_null()).count() >= 3, "looks that fit");
}

#[test]
fn a_look_is_kept_as_what_it_says_that_is_not_usual() {
    let look = json!({ "corners": "round", "size": "largest", "density": "usual" });
    let (_, _, _, kept) = checked("Dusk", &dusk(), None, Some(&look)).unwrap();
    assert_eq!(
        Value::Object(kept.clone()),
        json!({ "size": "largest", "corners": "round" }),
        "the usual left out"
    );
    assert!(checked("Dusk", &dusk(), None, Some(&Value::Null)).unwrap().3.is_empty(), "nothing said is none");
    let wrong = checked("Dusk", &dusk(), None, Some(&json!({ "size": "huge" }))).unwrap_err();
    assert_eq!(
        wrong,
        r#"A theme's size is one of smaller, usual, larger, largest, and "huge" is none of them."#
    );
    let wrong = checked("Dusk", &dusk(), None, Some(&json!({ "colour": "red" }))).unwrap_err();
    assert!(wrong.starts_with("`colour` is nothing a theme's look says"), "{wrong}");
}

#[tokio::test]
async fn a_theme_with_a_look_is_another_theme_and_one_changed_without_it_says_none() {
    let rig = Rig::new().await;
    let ada = person("ada", &["themes"]);
    let look = json!({ "letters": "serif", "size": "larger", "density": "compact", "corners": "square" });
    let plain = rig.themes.make(&ada, "Dusk", &dusk(), None, None).await.unwrap();
    let looked = rig.themes.make(&ada, "Dusk", &dusk(), None, Some(&look)).await.unwrap();
    assert_ne!(plain.id, looked.id, "the same name and colours with a look is another theme");
    assert_eq!(Value::Object(looked.look.clone()), look);
    assert_eq!(
        rig.themes.make(&ada, "Dusk", &dusk(), None, Some(&look)).await.unwrap().id,
        looked.id,
        "and made again, the one there is"
    );
    let usual = json!({ "size": "usual" });
    assert_eq!(
        rig.themes.make(&ada, "Dusk", &dusk(), None, Some(&usual)).await.unwrap().id,
        plain.id,
        "one that says only what is usual says nothing"
    );
    let read = rig.themes.of(&ada).await.unwrap();
    assert_eq!(read.own.iter().map(|one| one.look.len()).collect::<Vec<_>>(), [0, 4]);
    let changed = rig.themes.change(&ada, &looked.id, "Dusk", &dusk(), None, None).await.unwrap();
    assert!(changed.look.is_empty(), "changed whole, with no look said");
    let said =
        rig.journal.read(EventQuery { topics: "theme.kept".into(), ..Default::default() }).await.unwrap();
    assert_eq!(said.len(), 3, "each change said once");
    assert_eq!(said[1].data["theme"]["look"]["letters"], "serif", "what is said carries the look");
    assert_eq!(said[0].data["theme"]["look"], json!({}), "and says nothing of one that has none");
}
