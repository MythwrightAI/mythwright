//! A forum's topics as plain values: titles told apart, what a change
//! comes to, and where a reply goes among the others.

use kernel::topics::{
    DEPTH, Line, Link, PARENT, Place, Tree, between, chain, change_said, followed_then, landing, link_of,
    linking_of, links_in, links_to, made_status, placed_under, revised, status_of, status_said,
    tells_followers, title_key, titled,
};
use serde_json::{Map, Value, json};

fn row(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

#[test]
fn two_titles_are_one_whatever_their_capitals_and_spaces() {
    assert_eq!(title_key("  Payment   provider OPTIONS "), "payment provider options");
    assert_eq!(title_key("Payment provider options"), title_key("payment Provider  options"));
    assert_ne!(title_key("Payment providers"), title_key("Payment provider"));
}

#[test]
fn a_title_is_some_words_on_one_line_and_not_too_many() {
    assert_eq!(titled("  Why the build\nis slow ").unwrap(), "Why the build is slow");
    assert!(titled("   ").unwrap_err().contains("is words"));
    assert!(titled(&"x".repeat(201)).unwrap_err().contains("200 characters"));
}

#[test]
fn a_change_of_what_a_topic_says_is_a_revision_and_of_anything_else_is_not() {
    let was = row(json!({ "title": "A", "body": "one", "status": "open" }));
    assert!(revised(None, &was), "a topic made is its first revision");
    assert!(revised(Some(&was), &row(json!({ "title": "A", "body": "two", "status": "open" }))));
    assert!(revised(Some(&was), &row(json!({ "title": "A", "body": "one", "summary": "known" }))));
    assert!(!revised(Some(&was), &row(json!({ "title": "A", "body": "one", "status": "closed" }))));
    // Nothing and empty words are the same.
    assert!(!revised(Some(&was), &row(json!({ "title": "A", "body": "one", "summary": "" }))));
}

#[test]
fn a_topic_whose_row_says_no_status_has_none() {
    let cases: &[(Value, Option<&str>)] = &[
        (json!({}), None),
        (json!({ "status": null }), None),
        (json!({ "status": "" }), None),
        (json!({ "status": "open" }), Some("open")),
        (json!({ "status": "standing" }), Some("standing")),
    ];
    for (data, status) in cases {
        assert_eq!(status_of(&row(data.clone())), *status, "{data}");
    }
}

#[test]
fn a_topic_is_made_open_standing_or_with_no_status_and_never_answered_or_closed() {
    assert_eq!(made_status(None), Ok(None));
    assert_eq!(made_status(Some("")), Ok(None));
    assert_eq!(made_status(Some(" open ")), Ok(Some("open")));
    assert_eq!(made_status(Some("standing")), Ok(Some("standing")));
    assert!(made_status(Some("answered")).unwrap_err().contains("marking one of its replies"));
    assert!(made_status(Some("closed")).unwrap_err().contains("`open` or `standing`"));
    assert!(made_status(Some("sometimes")).unwrap_err().contains("`open` or `standing`"));
}

#[test]
fn a_status_that_came_to_another_is_said_by_what_it_came_to() {
    let open = row(json!({ "status": "open" }));
    let cases: &[(Option<Value>, Value, Option<&str>)] = &[
        (None, json!({}), None),
        (None, json!({ "status": null }), None),
        (None, json!({ "status": "open" }), Some("topic.opened")),
        (None, json!({ "status": "standing" }), Some("topic.standing")),
        (Some(json!({ "status": "open" })), json!({ "status": "answered" }), Some("topic.answered")),
        (Some(json!({ "status": "answered" })), json!({ "status": "closed" }), Some("topic.closed")),
        (Some(json!({ "status": "closed" })), json!({ "status": "open" }), Some("topic.opened")),
        (Some(json!({})), json!({ "status": "open" }), Some("topic.opened")),
        (Some(json!({ "status": "closed" })), json!({}), Some("topic.untracked")),
        (Some(json!({ "status": "open" })), json!({ "status": "" }), Some("topic.untracked")),
        (Some(json!({})), json!({ "status": null, "title": "B" }), None),
        (Some(json!({ "status": "open" })), json!({ "status": "open", "title": "B" }), None),
    ];
    for (before, after, said) in cases {
        let before = before.as_ref().map(|before| row(before.clone()));
        assert_eq!(status_said(before.as_ref(), &row(after.clone())), *said, "{before:?} to {after}");
    }
    assert_eq!(status_said(Some(&open), &open), None);
}

#[test]
fn a_follower_is_told_of_a_reply_a_revision_a_status_and_a_move_and_of_nothing_else() {
    let cases: &[(&str, Value, bool, bool)] = &[
        ("record.commented", json!({ "body": "Linking." }), true, true),
        ("topic.revised", json!({ "nth": 2 }), false, true),
        ("topic.answered", json!({}), false, true),
        ("topic.closed", json!({}), true, true),
        ("topic.opened", json!({}), false, true),
        ("topic.standing", json!({}), false, true),
        ("topic.untracked", json!({}), false, true),
        // Under another page, or to the top, is a move; another order among the same pages is none.
        ("topic.moved", json!({ "parent": "@-topics-1", "was": { "parent": null } }), true, true),
        ("topic.moved", json!({ "parent": null, "was": { "parent": "@-topics-1" } }), false, true),
        ("topic.moved", json!({ "parent": "@-topics-1", "was": { "parent": "@-topics-2" } }), true, true),
        ("topic.moved", json!({ "parent": "@-topics-1", "was": { "parent": "@-topics-1" } }), true, false),
        ("topic.moved", json!({ "parent": null, "was": { "parent": "" } }), false, false),
        // A wiki put in another place is told of; a page carried along with it is not.
        ("topic.placed", json!({ "was": "workspace:w1" }), false, true),
        ("topic.placed", json!({ "was": "workspace:w1" }), true, false),
        ("topic.followed", json!({ "follows": true }), false, false),
        ("record.updated", json!({ "changed": ["body"] }), false, false),
        ("record.created", json!({}), false, false),
    ];
    for (topic, data, under, told) in cases {
        assert_eq!(tells_followers(topic, data, *under), *told, "{topic} {data} under: {under}");
    }
}

/// Who follows a page now, who began or stopped since, and who followed it then.
type Followed<'a> = (&'a [&'a str], &'a [(&'a str, bool)], &'a [&'a str]);

#[test]
fn who_followed_a_page_at_a_change_is_who_follows_it_now_but_whoever_began_since() {
    let of = |names: &[&str]| names.iter().map(|name| name.to_string()).collect::<Vec<_>>();
    let said = |pairs: &[(&str, bool)]| {
        pairs.iter().map(|(who, follows)| (who.to_string(), *follows)).collect::<Vec<_>>()
    };
    let cases: &[Followed] = &[
        (&["nell"], &[], &["nell"]),
        // She began following after it: she did not then.
        (&["nell", "mira"], &[("mira", true)], &["nell"]),
        // She stopped since: she follows it no longer, and is told nothing more.
        (&["nell"], &[("mira", false)], &["nell"]),
        // Stopped and began again since: she did then, and does now.
        (&["nell", "mira"], &[("mira", false), ("mira", true)], &["nell", "mira"]),
        // Began, stopped and began again since: she did not then.
        (&["mira"], &[("mira", true), ("mira", false), ("mira", true)], &[]),
        (&[], &[("nell", true), ("nell", false)], &[]),
    ];
    for (now, since, then) in cases {
        assert_eq!(followed_then(&of(now), &said(since)), of(then), "{now:?} with {since:?}");
    }
}

#[test]
fn what_a_change_a_follower_is_told_of_did_is_said_in_a_few_words_and_a_reply_by_its_own() {
    let cases: &[(&str, Option<&str>)] = &[
        ("record.commented", None),
        ("topic.revised", Some("Revised the page")),
        ("topic.answered", Some("Marked it answered")),
        ("topic.closed", Some("Closed it")),
        ("topic.opened", Some("Marked it open")),
        ("topic.standing", Some("Marked it standing")),
        ("topic.untracked", Some("Removed its status")),
        ("topic.moved", Some("Moved it")),
        ("topic.placed", Some("Moved it")),
        ("topic.followed", None),
    ];
    for (topic, said) in cases {
        assert_eq!(change_said(topic), *said, "{topic}");
    }
}

#[test]
fn a_reply_is_put_under_what_it_answers_and_beside_it_at_the_deepest() {
    assert_eq!(placed_under(&[]), None);
    assert_eq!(placed_under(&[10]), Some(10));
    let deep: Vec<u64> = (1..DEPTH as u64).collect();
    assert_eq!(placed_under(&deep), Some(DEPTH as u64 - 1), "the eighth is still under the seventh");
    let deepest: Vec<u64> = (1..=DEPTH as u64).collect();
    assert_eq!(placed_under(&deepest), Some(DEPTH as u64 - 1), "beside the eighth, at the eighth");
}

#[test]
fn a_chain_runs_from_its_top_down_to_the_reply() {
    let replies = [(1, None), (2, Some(1)), (3, Some(2)), (4, Some(1)), (5, Some(99))];
    assert_eq!(chain(&replies, 3), vec![1, 2, 3]);
    assert_eq!(chain(&replies, 4), vec![1, 4]);
    assert_eq!(chain(&replies, 1), vec![1]);
    assert_eq!(chain(&replies, 5), vec![5], "one under a reply that is not there is at the top");
    assert!(chain(&replies, 7).is_empty());
    // A ring is not followed for ever.
    assert_eq!(chain(&[(1, Some(2)), (2, Some(1))], 1).len(), 2);
}

#[test]
fn a_place_is_a_workspace_or_a_project_and_never_both_or_neither() {
    assert_eq!(Place::of(&row(json!({ "workspace": "ada" }))), Some(Place::Workspace("ada".into())));
    assert_eq!(
        Place::of(&row(json!({ "project": "shop", "workspace": "" }))),
        Some(Place::Project("shop".into()))
    );
    assert_eq!(Place::of(&row(json!({ "project": "shop", "workspace": "ada" }))), None);
    assert_eq!(Place::of(&row(json!({}))), None);
    assert_eq!(Place::Project("shop".into()).cells(), (Value::Null, json!("shop")));
}

/// The table of the tree's cases, which the client reads too.
fn wikis() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/wikicases.json");
    let kept = std::fs::read_to_string(path).expect("the table of wikis is in the client's sources");
    serde_json::from_str(&kept).expect("the table is a mapping of cases")
}

/// The pages a case says, or the table's own where it says none.
fn pages(table: &Value, said: &Value) -> Vec<Line> {
    let said = if said.is_null() { &table["pages"] } else { said };
    said.as_array()
        .unwrap()
        .iter()
        .map(|line| Line {
            id: line["id"].as_str().unwrap().to_string(),
            parent: line["parent"].as_str().map(str::to_string),
            position: line["position"].as_f64().unwrap(),
            space: line["space"].as_str().map(str::to_string),
        })
        .collect()
}

#[test]
fn a_page_is_read_under_what_its_row_says_and_one_under_nothing_there_or_in_a_ring_is_a_wiki() {
    let table = wikis();
    let cases = table["read"].as_array().unwrap();
    assert!(cases.len() >= 5, "the table is there");
    for case in cases {
        let lines = pages(&table, &case[0]);
        let tree = Tree::read(&lines);
        for (id, meant) in case[1].as_object().unwrap() {
            assert_eq!(json!(tree.path(id)), *meant, "{id} of {}", case[0]);
            assert_eq!(tree.wiki(id), meant[0].as_str(), "{id}");
        }
    }
}

#[test]
fn what_a_page_holds_is_every_page_under_it_each_before_its_own_in_the_order_they_stand() {
    let table = wikis();
    let lines = pages(&table, &Value::Null);
    let tree = Tree::read(&lines);
    for case in table["held"].as_array().unwrap() {
        let came: Vec<&str> = match case[0].as_str() {
            Some(page) => tree.subtree(page),
            None => tree.beside(None).iter().map(|line| line.id.as_str()).collect(),
        };
        assert_eq!(json!(came), case[1], "{}", case[0]);
    }
}

#[test]
fn a_page_moved_lands_where_its_case_says_or_is_refused_by_its_word() {
    let table = wikis();
    let cases = table["moved"].as_array().unwrap();
    assert!(cases.len() > 20, "the table is there");
    for case in cases {
        let lines = pages(&table, &case[0]);
        let tree = Tree::read(&lines);
        let asked = &case[1];
        let came = landing(
            &tree,
            asked["page"].as_str().unwrap(),
            asked["under"].as_str(),
            asked["before"].as_str(),
        );
        let meant = &case[2];
        match came {
            Err(refused) => assert_eq!(json!({ "refused": refused.word() }), *meant, "{case}"),
            Ok(landed) if landed.stays => assert_eq!(*meant, json!({ "stays": true }), "{case}"),
            Ok(landed) => {
                let said: Vec<(String, f64)> = meant["positions"]
                    .as_array()
                    .unwrap_or_else(|| panic!("{case} came to {landed:?}"))
                    .iter()
                    .map(|one| (one[0].as_str().unwrap().to_string(), one[1].as_f64().unwrap()))
                    .collect();
                assert_eq!(landed.positions, said, "{case}");
            }
        }
    }
}

#[test]
fn a_page_lands_between_two_and_where_there_is_no_room_the_level_is_numbered_again() {
    let table = wikis();
    for case in table["between"].as_array().unwrap() {
        assert_eq!(between(case[0].as_f64(), case[1].as_f64()), case[2].as_f64(), "{case}");
    }
    assert_eq!(PARENT, "parent");
}

/// The table of links' cases, which the client reads too.
fn link_cases() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/linkcases.json");
    let kept = std::fs::read_to_string(path).expect("the table of links is in the client's sources");
    serde_json::from_str(&kept).expect("the table is a mapping of cases")
}

#[test]
fn the_pages_a_text_links_to_are_read_case_for_case_as_the_client_reads_them() {
    let table = link_cases();
    for case in table["cases"].as_array().unwrap() {
        let text = case[0].as_str().unwrap();
        let wanted: Vec<Link> = case[1]
            .as_array()
            .unwrap()
            .iter()
            .map(|link| Link { topic: link[0].as_str().unwrap().to_string(), reply: link[1].as_u64() })
            .collect();
        assert_eq!(links_in(text), wanted, "{text}");
    }
}

#[test]
fn a_link_is_to_a_page_by_its_id_and_to_a_reply_by_its_number() {
    assert_eq!(link_of("topic:@-topics-49"), Some(Link { topic: "@-topics-49".into(), reply: None }));
    assert_eq!(
        link_of("topic:@-topics-49#817039"),
        Some(Link { topic: "@-topics-49".into(), reply: Some(817039) })
    );
    assert_eq!(link_of("topic:"), None);
    assert_eq!(link_of("topic:@-topics-49#"), None);
    assert_eq!(link_of("https://example.org"), None);
    assert!(links_to("[it](topic:@-topics-4)", "@-topics-4"));
    assert!(!links_to("[it](topic:@-topics-40)", "@-topics-4"), "an id is whole");
}

#[test]
fn the_links_in_an_articles_boxes_are_its_links_and_those_in_other_code_are_not() {
    let body = "```infobox\ntitle: Frog\nRange: see [Toads](topic:@-topics-2)\n```\n\n```js\n[no](topic:@-topics-3)\n```\n\n```note\nSee [Frogs in culture](topic:@-topics-4).\n```\n";
    let found: Vec<String> = links_in(body).into_iter().map(|link| link.topic).collect();
    assert_eq!(found, ["@-topics-2", "@-topics-4"]);
    assert!(links_to(body, "@-topics-2") && !links_to(body, "@-topics-3"));
}

#[test]
fn a_text_that_links_to_a_page_says_whether_only_a_series_box_does_and_which() {
    let page = "@-topics-4";
    let series = |title: &str| {
        format!(
            "```series\ntitle: {title}\n## Practices\n[Breath](topic:@-topics-4)\n[Scan](topic:@-topics-5)\n```"
        )
    };
    let cases: Vec<(String, bool, Option<&str>)> = vec![
        ("No link at all.".to_string(), false, None),
        ("See [breath](topic:@-topics-4).".to_string(), true, None),
        (series("Meditation"), false, Some("Meditation")),
        (
            format!("{}\n\nAnd [in words](topic:@-topics-4#12).", series("Meditation")),
            true,
            Some("Meditation"),
        ),
        (
            format!("```infobox\ntitle: Box\nSee: [b](topic:@-topics-4)\n```\n{}", series("Meditation")),
            true,
            Some("Meditation"),
        ),
        (
            "```series\n[Breath](topic:@-topics-4)\ntitle: Late title\n```".to_string(),
            false,
            Some("Late title"),
        ),
        ("```series\n[Breath](topic:@-topics-4)\n```".to_string(), false, Some("")),
        (
            format!("{}\n{}", series("Other").replace("@-topics-4", "@-topics-6"), series("Second")),
            false,
            Some("Second"),
        ),
        ("```\n[b](topic:@-topics-4)\n```".to_string(), false, None),
        (series("Meditation").replace("@-topics-4", "@-topics-40"), false, None),
    ];
    for (text, elsewhere, title) in cases {
        let said = linking_of(&text, page);
        assert_eq!((said.elsewhere, said.series.as_deref()), (elsewhere, title), "{text}");
        let only = if elsewhere { None } else { title };
        assert_eq!(said.series_only(), only, "{text}");
        assert_eq!(
            said.elsewhere || said.series.is_some(),
            links_to(&text, page),
            "a link anywhere is one of the two: {text}"
        );
    }
    let both = linking_of(&series("Meditation"), page).and(linking_of("[b](topic:@-topics-4)", page));
    assert_eq!(both.series_only(), None, "a page's summary that links counts with its post");
}
