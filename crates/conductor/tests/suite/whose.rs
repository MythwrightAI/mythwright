//! What is heard with nothing said: an instance's own project's events
//! and what is nobody's, the factory's own; another project's only by
//! saying so (`scope: fleet`). Played, and stopped at every place.

use serde_json::json;

use crate::anywhere::{Does, Scenario, anywhere, once, then};

/// Hears a thing and says that it did, with which.
const HEARER: &str = r#"
blueprint: hearer
on:
  heard:
    event: w.thing
    steps:
      told: { do: emit, with: { topic: w.heard, n: "${event.data.n}" } }
"#;

/// The same, saying that it hears the whole factory.
const EVERYWHERE: &str = r#"
blueprint: everywhere
on:
  heard:
    event: w.thing
    scope: fleet
    steps:
      told: { do: emit, with: { topic: w.heard, n: "${event.data.n}" } }
"#;

/// Waits for a thing, with nothing said and then saying the whole
/// factory, and says what each wait was given.
const WAITER: &str = r#"
blueprint: waiter
steps:
  own:
    do: wait
    with: { event: w.thing, say: { topic: w.waiting, n: 1 } }
  first: { do: emit, with: { topic: w.given, to: own, n: "${steps.own.out.event.data.n}" } }
  any:
    do: wait
    with: { event: w.thing, scope: fleet, say: { topic: w.waiting, n: 2 } }
  second: { do: emit, with: { topic: w.given, to: any, n: "${steps.any.out.event.data.n}" } }
"#;

fn things(scenario: Scenario) -> Scenario {
    scenario
        .and(once(
            |seen| seen.said("instance.active") >= 1,
            Does::Say("w.thing", json!({ "act": 1, "n": 1, "project": "blog" })),
        ))
        .and(once(
            |seen| seen.said("w.thing") >= 1,
            Does::Say("w.thing", json!({ "act": 2, "n": 2, "project": "shop" })),
        ))
        .and(once(|seen| seen.said("w.thing") >= 2, Does::Say("w.thing", json!({ "act": 3, "n": 3 }))))
}

#[tokio::test]
async fn a_handler_that_says_no_scope_hears_its_own_projects_and_what_is_nobodys_wherever_the_conductor_is_stopped()
 {
    let mut hearer = Scenario::of("hearer", HEARER);
    hearer.acts = vec![then(Does::ApplyFor("shop", HEARER, json!({})))];
    let found = anywhere(things(hearer)).await;
    // Another project's thing was said and not heard: its own and
    // nobody's were.
    assert_eq!(found.usual["said"]["w.thing"], 3, "{}", found.usual);
    assert_eq!(found.usual["said"]["w.heard"], 2, "{}", found.usual);
    found.held();
}

#[tokio::test]
async fn a_handler_hears_another_projects_event_only_because_it_says_fleet_wherever_the_conductor_is_stopped()
{
    let mut everywhere = Scenario::of("everywhere", EVERYWHERE);
    everywhere.acts = vec![then(Does::ApplyFor("shop", EVERYWHERE, json!({})))];
    let found = anywhere(things(everywhere)).await;
    assert_eq!(found.usual["said"]["w.heard"], 3, "{}", found.usual);
    found.held();
}

#[tokio::test]
async fn something_in_no_project_hears_what_is_nobodys_and_no_projects_wherever_the_conductor_is_stopped() {
    let found = anywhere(things(Scenario::of("hearer", HEARER))).await;
    assert_eq!(found.usual["said"]["w.heard"], 1, "{}", found.usual);
    found.held();
}

#[tokio::test]
async fn a_wait_hears_as_a_handler_does_and_another_projects_only_saying_fleet_wherever_the_conductor_is_stopped()
 {
    let mut waiter = Scenario::of("waiter", WAITER);
    waiter.acts = vec![
        then(Does::ApplyFor("shop", WAITER, json!({}))),
        // Another project's first: the wait that says nothing lets it by
        // and is given its own project's.
        once(
            |seen| seen.said("w.waiting") >= 1,
            Does::Say("w.thing", json!({ "act": 1, "n": 1, "project": "blog" })),
        ),
        once(
            |seen| seen.said("w.thing") >= 1,
            Does::Say("w.thing", json!({ "act": 2, "n": 2, "project": "shop" })),
        ),
        // And the one that says the whole factory is given another's.
        once(
            |seen| seen.said("w.waiting") >= 2,
            Does::Say("w.thing", json!({ "act": 3, "n": 3, "project": "blog" })),
        ),
    ];
    let found = anywhere(waiter).await;
    assert_eq!(found.usual["said"]["w.given"], 2, "{}", found.usual);
    let run = found.usual["runs"].to_string();
    assert!(
        run.contains(r#""n":"2""#) || run.contains(r#""n":2"#),
        "the first wait was given its own project's: {}",
        found.usual
    );
    found.held();
}

/// What a blueprint says once it has heard two things after `after`, so
/// that a test can know a third was not heard in between.
async fn heard(rig: &crate::Rig, n: i64) -> bool {
    rig.events("w.heard")
        .await
        .iter()
        .any(|event| event.data["n"] == json!(n) || event.data["n"] == json!(n.to_string()))
}

#[tokio::test]
async fn a_text_from_before_the_word_hears_the_whole_factory_as_it_did_until_it_is_applied_again() {
    let mut rig = crate::Rig::on_a_file().await;
    let asked = conductor::Apply { project: Some("shop".into()), ..rig.request(HEARER, json!({})) };
    let applied = rig.engine.apply(asked).await.unwrap();
    rig.event("instance.active", |_| true).await;
    // Taken by this program, it says how it reads what it does not say.
    assert_eq!(rig.instance(&applied.instance).await.heard.as_deref(), Some("project"));

    // As a program from before left it: nothing kept of that in its row.
    let id = applied.instance.clone();
    rig.journal
        .db()
        .call(move |connection| connection.execute("UPDATE instances SET heard = NULL WHERE id = ?1", [&id]))
        .await
        .unwrap();
    rig.restart().await;
    assert_eq!(rig.instance(&applied.instance).await.heard, None);

    // Another project's thing is heard, as it was before the upgrade.
    rig.journal
        .emit("w.thing", kernel::Source::actor("user:ada"), json!({ "n": 1, "project": "blog" }))
        .await
        .unwrap();
    rig.event("w.heard", |event| event.data["n"] == json!(1) || event.data["n"] == json!("1")).await;

    // Applied again, it reads as a text taken now does: its own project's
    // and what is nobody's. Told by what it hears after: the one in
    // between, another project's, is not among it.
    let again =
        conductor::Apply { project: Some("shop".into()), force: true, ..rig.request(HEARER, json!({})) };
    let applied = rig.engine.apply(again).await.unwrap();
    rig.event("instance.active", |event| {
        event.seq > 0 && event.source.instance.as_deref() == Some(&applied.instance)
    })
    .await;
    assert_eq!(rig.instance(&applied.instance).await.heard.as_deref(), Some("project"));
    let after = rig.mark();
    rig.event("instance.active", move |event| event.seq <= after).await;
    rig.journal
        .emit("w.thing", kernel::Source::actor("user:ada"), json!({ "n": 2, "project": "blog" }))
        .await
        .unwrap();
    rig.journal
        .emit("w.thing", kernel::Source::actor("user:ada"), json!({ "n": 3, "project": "shop" }))
        .await
        .unwrap();
    rig.event("w.heard", |event| event.data["n"] == json!(3) || event.data["n"] == json!("3")).await;
    assert!(!heard(&rig, 2).await, "another project's was heard by a text applied now");
}

/// A text with one handler, written as each case says.
/// Applies `source` for `project`, under `parent` where there is one.
fn asked(rig: &crate::Rig, source: &str, project: &str, parent: Option<&str>) -> conductor::Apply {
    conductor::Apply {
        project: Some(project.to_string()),
        parent: parent.map(str::to_string),
        ..rig.request(source, json!({}))
    }
}

#[tokio::test]
async fn what_is_started_under_something_is_of_its_project_and_one_asked_for_as_anothers_is_refused() {
    let rig = crate::Rig::new().await;
    let parent = rig.engine.apply(asked(&rig, HEARER, "shop", None)).await.unwrap().instance;
    crate::handlers::active(&rig, &parent).await;

    // Everything kept for what runs is told apart by project through the
    // instance it is of, and a tree of instances is of one project: so
    // one of another project is not started under it, and nothing of it
    // is written or said.
    let refused = rig.engine.apply(asked(&rig, EVERYWHERE, "blog", Some(&parent))).await;
    let refused = refused.expect_err("something of another project under it");
    assert!(refused.message.contains("a tree of instances is never of two"), "{}", refused.message);
    assert!(refused.message.contains("`shop`") && refused.message.contains("`blog`"), "{}", refused.message);
    let started = rig.events("instance.started").await;
    assert_eq!(started.len(), 1, "{started:?}");

    // As its parent's, it is.
    let child = rig.engine.apply(asked(&rig, EVERYWHERE, "shop", Some(&parent))).await.unwrap();
    crate::handlers::active(&rig, &child.instance).await;
    assert_eq!(rig.instance(&child.instance).await.project.as_deref(), Some("shop"));
}

/// What the database keeps of where handlers have got to: the handler,
/// what the journal it is a place in is called, the place, and whose.
async fn places(rig: &crate::Rig) -> Vec<(String, String, u64, String)> {
    let kept = rig.journal.db().call(|connection| {
        let mut statement =
            connection.prepare("SELECT handler, journal, seq, project FROM places ORDER BY instance")?;
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, i64>(2)? as u64, row.get(3)?)))?
            .collect()
    });
    kept.await.unwrap()
}

#[tokio::test]
async fn a_handlers_place_is_kept_by_what_the_journal_it_follows_is_called_and_says_its_project() {
    let mut rig = crate::Rig::new().await;
    let here = rig.journal.name().to_string();
    assert_eq!(here, journal::FACTORY, "the one journal there is, is the factory's own");
    let hearer = rig.engine.apply(asked(&rig, HEARER, "shop", None)).await.unwrap().instance;
    crate::handlers::active(&rig, &hearer).await;
    // Listening, it has a place: in that journal, and of its project.
    let kept = places(&rig).await;
    let [(handler, journal, at, project)] = kept.as_slice() else { panic!("{kept:?}") };
    assert_eq!((handler.as_str(), journal, project.as_str()), ("heard", &here, "shop"));
    assert!(*at > 0 && *at <= rig.journal.last_seq(), "{kept:?}");

    // An event that starts a run moves it, in that journal and no other.
    let said = rig.journal.emit("w.thing", kernel::Source::default(), json!({ "n": 1 })).await.unwrap();
    rig.event("w.heard", |event| event.data["n"] == json!(1) || event.data["n"] == json!("1")).await;
    assert_eq!(places(&rig).await, [("heard".to_string(), here.clone(), said.seq, "shop".to_string())]);

    // And a conductor that starts goes on from it: what was said while
    // none was there is heard once, by the place kept under that name.
    rig.engine.halt();
    rig.engine.halted().await;
    rig.journal.emit("w.thing", kernel::Source::default(), json!({ "n": 2 })).await.unwrap();
    rig.restart().await;
    rig.event("w.heard", |event| event.data["n"] == json!(2) || event.data["n"] == json!("2")).await;
    assert_eq!(rig.events("w.heard").await.len(), 2);
}

fn hears(lines: &str) -> blueprint::Blueprint {
    let text = format!("blueprint: h\non:\n  it:\n{lines}    do: noop\n");
    blueprint::check(&text, &blueprint::NoCatalog).blueprint.unwrap_or_else(|| panic!("it reads: {text}"))
}

#[test]
fn a_handler_that_will_hear_nothing_is_said_to_whoever_applies_it_and_one_that_will_is_not() {
    use blueprint::{EventSpec, ValueType::String};
    let described = [
        EventSpec::new("board.item.moved", "A card moved.").field("project", String, "its project"),
        EventSpec::new("board.item.removed", "A card went.").field("project", String, "its project"),
        EventSpec::new("channel.message", "Something was said.").maybe("project", String, "its project"),
        EventSpec::new("system.started", "The daemon started."),
    ];
    // What is written, for which project it is applied, and what the
    // sentence says where there is one.
    let cases: &[(&str, Option<&str>, Option<&str>)] = &[
        // Always some project's, and this is in none.
        ("    event: board.item.moved\n", None, Some("is always some project's")),
        ("    event: board.item.*\n", None, Some("is always some project's")),
        // In its project it hears them.
        ("    event: board.item.moved\n", Some("shop"), None),
        // Saying the whole factory, it hears them from anywhere.
        ("    event: board.item.moved\n    scope: fleet\n", None, None),
        ("    event: board.item.moved\n    scope: own\n", None, None),
        // Sometimes nobody's, or nobody's, or a blueprint's own to say.
        ("    event: channel.message\n", None, None),
        ("    event: system.started\n", None, None),
        ("    event: w.thing\n", None, None),
        ("    event: \"board.item.moved,channel.message\"\n", None, None),
        // Another project named in `for:` is not heard by being named.
        ("    event: w.thing\n    for: { project: blog }\n", Some("shop"), Some("names the project `blog`")),
        ("    event: w.thing\n    for: { project: blog }\n", None, Some("applied for no project")),
        ("    event: w.thing\n    for: { project: blog }\n    scope: fleet\n", Some("shop"), None),
        // Its own named, or one worked out when it listens, says nothing here.
        ("    event: w.thing\n    for: { project: shop }\n", Some("shop"), None),
        ("    event: w.thing\n    for: { project: \"${params.project}\" }\n", Some("shop"), None),
    ];
    for (written, project, wanted) in cases {
        let text = hears(written);
        let found = conductor::whose::unheard(&text, *project, &described);
        match wanted {
            None => assert!(found.is_empty(), "{written:?} for {project:?} said {:?}", found[0].said),
            Some(wanted) => {
                assert_eq!(found.len(), 1, "{written:?} for {project:?} said nothing");
                let said = &found[0].said;
                assert!(said.contains(wanted), "{written:?} for {project:?}: {said}");
                assert!(said.contains("scope: fleet") && said.contains("--project"), "{said}");
                assert_eq!(found[0].name, "it");
            }
        }
    }
    // A step that waits is said the same way, wherever it is written:
    // among the steps, in a handler, inside steps done as one.
    let waits = |with: &str| {
        let text = format!(
            "blueprint: w\nsteps:\n  first:\n    do: wait\n    with: {{ {with} }}\n  both:\n    task:\n      \
             inside:\n        do: wait\n        with: {{ {with} }}\non:\n  it:\n    event: w.thing\n    \
             steps:\n      then:\n        do: wait\n        with: {{ {with} }}\n"
        );
        blueprint::check(&text, &blueprint::NoCatalog).blueprint.unwrap_or_else(|| panic!("it reads: {text}"))
    };
    let names = |with: &str, project: Option<&str>| -> Vec<std::string::String> {
        let found = conductor::whose::unheard(&waits(with), project, &described);
        assert!(
            found.iter().all(|one| one.said.contains("waits for") && one.said.contains("`scope: fleet`"))
        );
        found.into_iter().map(|one| one.name).collect()
    };
    assert_eq!(names("event: board.item.moved", None), ["first", "inside", "then"]);
    for heard in [
        names("event: board.item.moved", Some("shop")),
        names("event: board.item.moved, scope: fleet", None),
        names("event: channel.message", None),
        names("event: w.thing", None),
        names("for: 5m", None),
        names("event: \"${params.topic}\"", None),
    ] {
        assert_eq!(heard, Vec::<std::string::String>::new());
    }
}
