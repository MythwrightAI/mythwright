//! What an agent asked in a topic is given: the topic and the chain it
//! answers, each bounded, and the reply that asked it last.

use topics::context::{About, MOST_BODY, MOST_CHAIN, MOST_PAGES, May, Said, prompt};

/// What an agent that may read and write records may do.
const ALL: May = May { read: true, write: true };

fn said(who: &str, body: &str) -> Said {
    Said { who: who.into(), body: body.into() }
}

fn about(body: &str) -> About {
    About {
        id: "@-topics-3".into(),
        title: "Which linker".into(),
        status: Some("open".into()),
        place: "the project shop".into(),
        summary: "Two are left.".into(),
        body: body.into(),
        path: Vec::new(),
        pages: Vec::new(),
    }
}

#[test]
fn an_agent_is_given_the_topic_the_chain_from_its_top_and_what_asked_it_last() {
    let given = prompt(
        &about("It links for minutes."),
        &[said("ada", "Linking."), said("grace", "Which?")],
        &said("ada", "@scout say"),
        ALL,
    );
    let order = [
        "\"Which linker\" (open, in the project shop)",
        "What is known now:\nTwo are left.",
        "The topic:\nIt links for minutes.",
        "[ada] Linking.\n[grace] Which?",
        "from ada, is for you:\n@scout say",
    ];
    let mut at = 0;
    for part in order {
        let found = given[at..].find(part).unwrap_or_else(|| panic!("{part} after {at}: {given}"));
        at += found;
    }
    assert!(given.contains("topic show @-topics-3"), "{given}");
}

#[test]
fn a_topic_with_no_status_is_named_with_none() {
    let untracked = About { status: None, ..about("") };
    let given = prompt(&untracked, &[], &said("ada", "@scout say"), ALL);
    assert!(given.contains("\"Which linker\" (in the project shop)"), "{given}");
}

#[test]
fn a_long_chain_is_its_first_and_its_last_with_how_many_between_and_a_long_body_its_headings() {
    let chain: Vec<Said> = (0..30).map(|n| said("ada", &format!("reply {n}"))).collect();
    let given = prompt(&about(""), &chain, &said("ada", "now"), ALL);
    assert!(given.contains("[ada] reply 0\n[9 more replies"), "{given}");
    assert!(given.contains("reply 29") && !given.contains("reply 5\n"), "{given}");
    let body = format!("# One\n{}\n## Two\n{}", "x".repeat(MOST_BODY), "y".repeat(100));
    let given = prompt(&about(&body), &[], &said("ada", "now"), ALL);
    assert!(given.contains("Its headings:\n# One\n## Two"), "{given}");
    assert!(given.len() < MOST_BODY, "only its start");
}

#[test]
fn a_page_says_where_it_is_in_its_wiki_and_the_pages_under_it_by_title_and_a_wiki_says_neither() {
    let mut page = about("Roll back by the tag.");
    page.title = "Rolling back".into();
    page.path = vec!["Handbook".into(), "Deploying".into()];
    page.pages = vec!["With a database".into(), "Without one".into()];
    let given = prompt(&page, &[], &said("ada", "@scout which?"), ALL);
    assert!(
        given.contains("It is a page of the wiki \"Handbook\", under Handbook › Deploying.\n"),
        "{given}"
    );
    assert!(given.contains("The pages under it: \"With a database\", \"Without one\";"), "{given}");
    assert!(given.contains("topic make <title> --under @-topics-3"), "{given}");
    // Where it is comes before what it says, and the pages under it are named and never read.
    assert!(given.find("under Handbook").unwrap() < given.find("What is known now").unwrap(), "{given}");

    // A wiki says nothing of a place in a wiki, and one with no pages nothing of pages.
    let given = prompt(&about("x"), &[], &said("ada", "now"), ALL);
    assert!(!given.contains("a page of the wiki") && !given.contains("pages under it"), "{given}");

    // Past so many, the rest are counted.
    page.pages = (0..MOST_PAGES + 3).map(|n| format!("Page {n}")).collect();
    let given = prompt(&page, &[], &said("ada", "now"), ALL);
    assert!(given.contains(&format!("\"Page {}\" and 3 more;", MOST_PAGES - 1)), "{given}");
    assert!(!given.contains(&format!("Page {}\"", MOST_PAGES)), "{given}");
}

#[test]
fn an_agent_is_offered_only_the_commands_it_may_run_and_told_its_answer_is_said_for_it() {
    let mut page = about(&"x".repeat(MOST_BODY + 1));
    page.pages = vec!["Gold".into()];
    let chain: Vec<Said> = (0..MOST_CHAIN + 1).map(|n| said("ada", &format!("reply {n}"))).collect();
    let show = "topic show @-topics-3";
    let reply = "topic reply @-topics-3 --to <reply> <words>";
    let read_page = "topic show <page>";
    let make = "topic make <title> --under @-topics-3";
    let cut = "left out here";
    let alone = "you may not read topics or say anything in them yourself";
    let none = May::default();
    let read = May { read: true, write: false };
    // Writing without reading runs nothing: each finds the topic by reading it first.
    let write = May { read: false, write: true };
    // What it may do, what it is offered, and what it is not.
    let cases: [(May, &[&str], &[&str]); 4] = [
        (ALL, &[show, reply, read_page, make], &[cut, alone]),
        (read, &[show, read_page], &[reply, make, cut, alone]),
        (none, &[cut, alone], &[show, reply, read_page, make]),
        (write, &[cut, alone], &[show, reply, read_page, make]),
    ];
    for (may, offered, not) in cases {
        let given = prompt(&page, &chain, &said("ada", "@scout which?"), may);
        assert!(given.contains("Your answer is said for you as a reply"), "{may:?}: {given}");
        assert!(given.contains("so do not post it yourself."), "{may:?}: {given}");
        for part in offered {
            assert!(given.contains(part), "{may:?} is offered {part}: {given}");
        }
        for part in not {
            assert!(!given.contains(part), "{may:?} is not offered {part}: {given}");
        }
        // Whatever it may do, it is given the pages under it by title.
        assert!(given.contains("The pages under it: \"Gold\""), "{may:?}: {given}");
    }
}

#[test]
fn an_agent_is_asked_to_say_where_its_facts_come_from_and_to_link_the_pages_it_names() {
    let given = prompt(&about("It links for minutes."), &[], &said("ada", "@scout say"), ALL);
    assert!(given.contains("[^1]: Author, *Title*, Publisher, Year. https://"), "{given}");
    assert!(given.contains("`[its title](topic:<its id>)`"), "{given}");
    assert!(given.contains("this page is `[Which linker](topic:@-topics-3)`"), "{given}");
    assert!(given.contains("topic list` gives each page's id"), "where it may read topics: {given}");
    let unread = prompt(&about(""), &[], &said("ada", "@scout say"), May { read: false, write: false });
    assert!(unread.contains("[^1]: Author"), "asked whatever it may do: {unread}");
    assert!(!unread.contains("topic list"), "{unread}");
    // Asked before what it is given, so that it reads as how to answer and not as part of the topic.
    assert!(given.find("[^1]:").unwrap() < given.find("What is known now").unwrap(), "{given}");
}

#[test]
fn an_agent_is_told_the_forms_of_an_articles_boxes() {
    let given = prompt(&about(""), &[], &said("ada", "@scout say"), May { read: false, write: false });
    for form in ["```infobox (`title:`", "```series (`title:`", "```note ("] {
        assert!(given.contains(form), "{form}: {given}");
    }
}
