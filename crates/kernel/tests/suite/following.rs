//! Following something, as plain values: how often, when a digest is due,
//! what a change is of and how what changed is said.

use kernel::following::{
    HOUR, Often, PageChange, ThingChange, at_of, counted, due, key, kind_of, line_said, of_key, page_change,
    said, thing_change, worded,
};
use serde_json::json;

#[test]
fn how_often_is_one_of_four_words_and_nothing_said_is_at_once() {
    for (said, often) in [
        ("", Often::Once),
        ("once", Often::Once),
        ("At once", Often::Once),
        ("hourly", Often::Hourly),
        ("DAILY", Often::Daily),
        ("off", Often::Off),
        ("never", Often::Off),
    ] {
        assert_eq!(Often::parse(said), Ok(often), "{said}");
        assert_eq!(Often::kept(often.as_str()), often, "kept as it is said");
    }
    let refused = Often::parse("weekly").unwrap_err();
    assert!(refused.contains("`hourly`, `daily` or `off`"), "{refused}");
    assert_eq!(Often::kept("weekly"), Often::Once, "what a later program wrote reads as at once");
    assert!(Often::Hourly.digests() && Often::Daily.digests());
    assert!(!Often::Once.digests() && !Often::Off.digests());
}

#[test]
fn what_may_be_followed_is_six_kinds_and_a_wiki_is_a_page() {
    for (said, kind) in
        [("page", "page"), ("Wiki", "page"), ("topic", "page"), ("board", "board"), ("project", "project")]
    {
        assert_eq!(kind_of(said), Ok(kind), "{said}");
    }
    assert!(kind_of("folder").unwrap_err().contains("page, board, table, library, channel, project"));
}

#[test]
fn a_daily_digest_comes_at_a_time_of_day_nine_where_none_is_said() {
    for (said, at) in [
        (None, "09:00"),
        (Some(""), "09:00"),
        (Some("7:05"), ""),
        (Some("07:05"), "07:05"),
        (Some("23:59"), "23:59"),
    ] {
        match at {
            "" => assert!(at_of(said).unwrap_err().contains("hours and minutes"), "{said:?}"),
            at => assert_eq!(at_of(said).as_deref(), Ok(at), "{said:?}"),
        }
    }
    for wrong in ["24:00", "12:60", "noon", "12"] {
        assert!(at_of(Some(wrong)).is_err(), "{wrong}");
    }
}

#[test]
fn a_digest_is_due_at_the_first_whole_hour_or_time_of_day_after_its_first_change() {
    let daily = |ms: i64| Some((ms / (24 * HOUR) + 1) * 24 * HOUR + 9 * HOUR);
    let none = |_: i64| None;
    for (often, first, expected) in [
        (Often::Hourly, 0, Some(HOUR)),
        (Often::Hourly, HOUR - 1, Some(HOUR)),
        // A change at the hour itself waits for the next.
        (Often::Hourly, HOUR, Some(2 * HOUR)),
        (Often::Hourly, 5 * HOUR + 30, Some(6 * HOUR)),
        (Often::Daily, 10 * HOUR, Some(24 * HOUR + 9 * HOUR)),
        (Often::Once, 10, None),
        (Often::Off, 10, None),
    ] {
        assert_eq!(due(often, first, daily), expected, "{often:?} {first}");
    }
    assert_eq!(due(Often::Daily, 10, none), None, "no time of day comes");
    assert_eq!(due(Often::Daily, 10, |_| Some(10)), None, "never at the change itself");
}

#[test]
fn a_page_tells_its_own_followers_what_it_always_did_and_those_above_it_of_pages_added_too() {
    let page = |own, above, noun, verb| Some(PageChange { own, above, noun, verb });
    let moved = json!({ "parent": "b", "was": { "parent": "a" } });
    let reordered = json!({ "parent": "a", "was": { "parent": "a" } });
    for (topic, data, under, expected) in [
        ("record.commented", json!({}), true, page(true, true, "reply", "said")),
        ("topic.revised", json!({ "nth": 2 }), true, page(true, true, "page", "changed")),
        ("topic.revised", json!({ "nth": 1 }), true, None),
        ("topic.closed", json!({}), false, page(true, true, "page", "changed")),
        ("topic.moved", moved, true, page(true, true, "page", "moved")),
        ("topic.moved", reordered, true, None),
        ("topic.placed", json!({}), false, page(true, true, "page", "moved")),
        ("topic.placed", json!({}), true, None),
        // A page made is followed by nobody yet: told to whoever follows a page above it.
        ("record.created", json!({}), true, page(false, true, "page", "added")),
        ("record.updated", json!({}), true, None),
        ("topic.followed", json!({}), true, None),
    ] {
        assert_eq!(page_change(topic, &data, under), expected, "{topic}");
    }
}

#[test]
fn a_change_of_a_row_is_of_its_table_and_project_and_a_message_of_its_channel_and_project() {
    let row = json!({ "project": "shop", "collection": "board", "id": "shop-board-1" });
    let thing = |things: &[(&'static str, &str)], table: Option<(&str, &str)>, noun, verb| {
        Some(ThingChange {
            things: things.iter().map(|(kind, id)| (*kind, id.to_string())).collect(),
            table: table.map(|(project, name)| (project.to_string(), name.to_string())),
            noun,
            verb,
        })
    };
    let of_row = [("table", "shop/board"), ("project", "shop")];
    for (topic, data, expected) in [
        ("record.created", row.clone(), thing(&of_row, Some(("shop", "board")), None, "added")),
        ("record.updated", row.clone(), thing(&of_row, Some(("shop", "board")), None, "changed")),
        ("record.removed", row.clone(), thing(&of_row, Some(("shop", "board")), None, "removed")),
        ("record.commented", row.clone(), thing(&of_row, Some(("shop", "board")), Some("comment"), "said")),
        ("record.attached", row.clone(), None),
        // The factory's own tables are no project's: their pages are followed as pages.
        ("record.updated", json!({ "project": "@", "collection": "topics" }), None),
        (
            "channel.message",
            json!({ "channel": "ch-1", "project": "shop" }),
            thing(&[("channel", "ch-1"), ("project", "shop")], None, Some("message"), "said"),
        ),
        (
            "channel.message",
            json!({ "channel": "ch-2" }),
            thing(&[("channel", "ch-2")], None, Some("message"), "said"),
        ),
        (
            "project.renamed",
            json!({ "project": "shop" }),
            thing(&[("project", "shop")], None, Some("project"), "renamed"),
        ),
        (
            "project.placed",
            json!({ "project": "shop" }),
            thing(&[("project", "shop")], None, Some("project"), "moved"),
        ),
        ("project.file.changed", json!({ "project": "shop" }), None),
    ] {
        assert_eq!(thing_change(topic, &data), expected, "{topic} {data}");
    }
}

#[test]
fn a_row_is_called_after_its_tables_kind_and_a_document_is_written() {
    for (kind, noun, verb, expected) in [
        ("board", None, "changed", ("card", "changed")),
        ("table", None, "added", ("row", "added")),
        ("library", None, "added", ("document", "written")),
        ("library", None, "changed", ("document", "written")),
        ("library", None, "removed", ("document", "removed")),
        ("board", Some("comment"), "said", ("comment", "said")),
        ("project", Some("message"), "said", ("message", "said")),
    ] {
        assert_eq!(worded(kind, noun, verb), expected, "{kind} {verb}");
    }
}

#[test]
fn what_changed_is_said_in_a_few_words_changes_first_and_new_things_after() {
    let counts = |all: &[(&str, &str)]| counted(all.iter().copied());
    for (changes, expected) in [
        (counts(&[]), ""),
        (counts(&[("page", "changed")]), "1 page changed"),
        (
            counts(&[
                ("page", "added"),
                ("reply", "said"),
                ("page", "changed"),
                ("reply", "said"),
                ("page", "changed"),
                ("reply", "said"),
                ("page", "changed"),
                ("page", "changed"),
                ("page", "changed"),
                ("page", "changed"),
            ]),
            "6 pages changed, 3 new replies, 1 page added",
        ),
        (counts(&[("card", "changed"), ("card", "changed"), ("card", "changed")]), "3 cards changed"),
        (counts(&[("document", "written")]), "1 document written"),
        (counts(&[("message", "said"), ("message", "said")]), "2 new messages"),
        (counts(&[("project", "renamed"), ("card", "added")]), "1 card added, renamed"),
    ] {
        assert_eq!(said(&changes), expected, "{changes:?}");
    }
    let frogs = counts(&[("page", "changed"), ("reply", "said")]);
    assert_eq!(
        line_said("Frogs: a field guide", &frogs),
        "Frogs: a field guide: 1 page changed, 1 new reply"
    );
    assert_eq!(line_said("", &frogs), "1 page changed, 1 new reply");
    assert_eq!(line_said("Frogs", &[]), "Frogs");
}

#[test]
fn a_thing_is_told_by_its_kind_and_id_and_a_digest_of_it_by_its_period_too() {
    assert_eq!(key("table", "shop/board"), "table:shop/board");
    assert_eq!(of_key("table:shop/board"), Some(("table", "shop/board")));
    assert_eq!(of_key("page:@-topics-3#7200000"), Some(("page", "@-topics-3")));
    assert_eq!(of_key("folder:x"), None);
    assert_eq!(of_key("table:"), None);
    assert_eq!(of_key("@-topics-3"), None, "a page's own line is about the page alone");
}
