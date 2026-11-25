//! The tickets a document lists (`deciding::tickets`, asked by `std/plan`
//! as `tickets.read` and `tickets.filed`): tables of what is handed and
//! what it comes to, with no word of `read` left unreached.

use deciding::tickets::{Came, Filed, filed, read};
use serde_json::{Value, json};

const PLAN: &str = "# A cart\n\nThe floor, then the shelves.\n\n```tickets\n[{\"title\": \"Lay the floor\"}, {\"title\": \"Put up shelves\", \"after\": [1]}]\n```\n";

#[test]
fn a_document_lists_no_tickets_or_tickets_that_are_read_or_a_block_that_cannot_be() {
    // The document; what reading it came to; how many tickets; and what is said of why not.
    let cases: [(&str, Came, usize, &str); 16] = [
        // No block: nothing at all, words alone, another kind of block, and
        // one whose tickets were filed.
        ("", Came::None, 0, ""),
        ("# A cart\n\nThe floor, then the shelves.\n", Came::None, 0, ""),
        ("```json\n[{\"title\": \"x\"}]\n```\n", Came::None, 0, ""),
        ("# A cart\n\nThe tickets of this plan are on the board: shop-1, shop-2.\n", Came::None, 0, ""),
        // A block with tickets: several, one, none; with words after it,
        // and as the last thing with no line after.
        (PLAN, Came::Read, 2, ""),
        ("```tickets\n[{\"title\": \"Buy a till\"}]\n```\n", Came::Read, 1, ""),
        ("```tickets\n[]\n```\n", Came::Read, 0, ""),
        ("Before.\n```tickets\n[{\"title\": \"a\"}]\n```\nAfter, with ``` in it.\n", Came::Read, 1, ""),
        ("```tickets\n[{\"title\": \"a\"}]\n```", Came::Read, 1, ""),
        // The first block is the one that is read.
        ("```tickets\n[{\"title\": \"a\"}]\n```\n```tickets\nnot json\n```\n", Came::Read, 1, ""),
        // A block that is not JSON, or is cut short, or was never closed.
        ("```tickets\n[{\"title\": \"Half a\n```\n", Came::Unread, 0, "this is not JSON: "),
        ("```tickets\n- Lay the floor\n```\n", Came::Unread, 0, "this is not JSON: "),
        ("```tickets\n[{\"title\": \"a\"}]\n", Came::Read, 1, ""),
        ("```tickets\n```\n", Came::Unread, 0, "this is not JSON: "),
        // JSON that is not a list of tickets.
        (
            "```tickets\n{\"title\": \"one\"}\n```\n",
            Came::Unread,
            0,
            "they are a list, and what is written is one mapping",
        ),
        (
            "```tickets\n[{\"title\": \"a\"}, \"b\"]\n```\n",
            Came::Unread,
            0,
            "ticket 2 is \"b\", and a ticket is a mapping with a `title`",
        ),
    ];
    let mut reached = Vec::new();
    for (content, came, tickets, why) in cases {
        let answer = read(content);
        assert_eq!((answer.came, answer.tickets.len()), (came, tickets), "{content:?}: {}", answer.why);
        match why {
            "" => assert_eq!(answer.why, "", "{content:?}"),
            why => assert!(answer.why.starts_with(why), "{content:?}: {}", answer.why),
        }
        reached.push(answer.came);
    }
    for word in Came::ALL {
        assert!(reached.contains(word), "no case reaches `{}`", word.word());
    }
    // What is read is the tickets as they are written, in their order.
    assert_eq!(
        read(PLAN).tickets,
        [json!({ "title": "Lay the floor" }), json!({ "title": "Put up shelves", "after": [1] })]
    );
}

fn made(ids: &[&str]) -> Vec<Value> {
    ids.iter().map(|id| json!({ "id": id, "status": "backlog" })).collect()
}

#[test]
fn tickets_that_came_to_things_say_what_each_waits_on_and_the_document_says_what_they_came_to() {
    let tickets = read(PLAN).tickets;
    let says = "The tickets of this plan are on the board: {ids}.";
    assert_eq!(
        filed(PLAN, &tickets, &made(&["shop-1", "shop-2"]), says),
        Ok(Filed {
            links: vec![json!({ "id": "shop-1", "after": [] }), json!({ "id": "shop-2", "after": ["shop-1"] })],
            ids: vec!["shop-1".to_string(), "shop-2".to_string()],
            content: "# A cart\n\nThe floor, then the shelves.\n\nThe tickets of this plan are on the board: shop-1, \
                      shop-2.\n"
                .to_string(),
        })
    );
    // What it comes to reads as a document that lists none.
    let content = filed(PLAN, &tickets, &made(&["shop-1", "shop-2"]), says).unwrap().content;
    assert_eq!(read(&content).came, Came::None);

    // What each waits on, by the places its ticket named.
    let after = |after: Value| -> Vec<Value> {
        let tickets =
            [json!({ "title": "a" }), json!({ "title": "b" }), json!({ "title": "c", "after": after })];
        let filed = filed("", &tickets, &made(&["s-1", "s-2", "s-3"]), says).unwrap();
        filed.links[2]["after"].as_array().unwrap().clone()
    };
    let cases: [(Value, &[&str]); 8] = [
        (json!([1, 2]), &["s-1", "s-2"]),
        // In the order asked, and as often as asked.
        (json!([2, 1]), &["s-2", "s-1"]),
        (json!([1, 1]), &["s-1", "s-1"]),
        // None named, and nothing said.
        (json!([]), &[]),
        (json!(null), &[]),
        // A place no ticket is at is left out: before the first, past the
        // last, and what is no place at all.
        (json!([0, 1, 9]), &["s-1"]),
        (json!([-1, 2, "3", 2.5]), &["s-2"]),
        // A ticket may name itself, which is the plan's to get right.
        (json!([3]), &["s-3"]),
    ];
    for (asked, comes_to) in cases {
        assert_eq!(after(asked.clone()), *comes_to, "{asked}");
    }

    // A document with no block is left as it is; with none of the words
    // that stand for the ids, the words are written as they are.
    let unchanged = filed("# Nothing listed\n", &[], &[], says).unwrap();
    assert_eq!((unchanged.content.as_str(), unchanged.links.len()), ("# Nothing listed\n", 0));
    let plain = filed("```tickets\n[]\n```\n", &[], &[], "Filed.").unwrap();
    assert_eq!(plain.content, "Filed.\n");
    // A block never closed is not found again to be replaced, as it never was.
    let open = "```tickets\n[{\"title\": \"a\"}]\n";
    assert_eq!(filed(open, &read(open).tickets, &made(&["s-1"]), says).unwrap().content, open);
}

#[test]
fn tickets_and_what_they_came_to_that_do_not_fit_each_other_are_refused_in_a_sentence() {
    let ticket = || json!({ "title": "a" });
    let cases: Vec<(Vec<Value>, Vec<Value>, &str)> = vec![
        (
            vec![ticket(), ticket()],
            made(&["s-1"]),
            "there are 2 tickets and 1 things they came to: one each, in the same order",
        ),
        (
            vec![],
            made(&["s-1"]),
            "there are 0 tickets and 1 things they came to: one each, in the same order",
        ),
        (
            vec![ticket()],
            vec![json!({ "status": "backlog" })],
            "what ticket 1 came to has no `id`: {\"status\":\"backlog\"}",
        ),
        (vec![ticket()], vec![json!({ "id": "" })], "what ticket 1 came to has no `id`"),
        (vec![ticket()], vec![json!("s-1")], "what ticket 1 came to has no `id`: \"s-1\""),
        (
            vec![json!({ "title": "a", "after": 1 })],
            made(&["s-1"]),
            "`after` of ticket 1 is the places in the list of the tickets it waits on ([1, 2]); it is 1",
        ),
        (
            vec![ticket(), json!({ "title": "b", "after": "the first" })],
            made(&["s-1", "s-2"]),
            "`after` of ticket 2 is the places in the list of the tickets it waits on ([1, 2]); it is \"the first\"",
        ),
    ];
    for (tickets, came_to, why) in cases {
        let refused = filed("", &tickets, &came_to, "").unwrap_err();
        assert!(refused.starts_with(why), "{tickets:?} and {came_to:?}: {refused}");
    }
}
