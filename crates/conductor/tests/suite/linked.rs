//! A part of a text that is read from a file (`from:` of `text.put`,
//! `conductor::words`): a table of what is handed and what it comes to,
//! what is refused, and which files are to be found and under what name.

use conductor::words::{Files, filled, linked_as, links, own_as, own_key, owned, put};
use serde_json::{Value, json};

fn files(entries: &[(&str, &str)]) -> Files {
    entries.iter().map(|(name, text)| (name.to_string(), text.to_string())).collect()
}

#[test]
fn a_part_read_from_a_file_is_the_files_words_with_its_blanks_filled_by_what_the_part_hands() {
    let kept = files(&[
        ("seat/card.md", "Card ${item}: ${title}\n\n${body}\n"),
        ("seat/among.md", "You are one of ${among} looking at it.\n"),
        ("plain.md", "Change nothing yourself.\n"),
        ("two-lines.md", "One.\nTwo.\n\n"),
    ]);
    let cases: Vec<(Value, &str)> = vec![
        // The file's words, its blanks filled, its one last newline not said.
        (
            json!([{ "from": "seat/card.md", "with": { "item": "shop-12", "title": "A cart", "body": "With tax." } }]),
            "Card shop-12: A cart\n\nWith tax.",
        ),
        // A file with no blanks is handed nothing, and what follows it follows its words.
        (json!([{ "from": "plain.md" }, " Then stop."]), "Change nothing yourself. Then stop."),
        // Only the last newline: a line left empty at its end is said.
        (json!([{ "from": "two-lines.md" }]), "One.\nTwo.\n"),
        // Said where its `if` says something, and with words in its place where it does not.
        (json!(["A.", { "if": true, "from": "plain.md" }]), "A.Change nothing yourself."),
        (json!(["A.", { "if": false, "from": "plain.md" }]), "A."),
        (json!(["A.", { "if": "", "from": "plain.md", "else": " Nothing." }]), "A. Nothing."),
        // A number handed is said as it is written; more handed than the file has blanks for is no harm.
        (
            json!([{ "if": true, "from": "seat/among.md", "with": { "among": 3, "unused": "x" } }]),
            "You are one of 3 looking at it.",
        ),
        // A file that is not said need not be there.
        (json!([{ "if": false, "from": "nowhere.md" }, "B"]), "B"),
    ];
    for (parts, comes_to) in cases {
        assert_eq!(put(&parts, None, &kept).map(|put| put.text), Ok(comes_to.to_string()), "{parts}");
    }
    // A blank may be a part of what is handed.
    let with = json!({ "item": { "id": "shop-12", "title": "A cart" } });
    assert_eq!(filled("${item.id}: ${item.title}\n", with.as_object()), Ok("shop-12: A cart".to_string()));
    // What is written `$${` is a dollar and a brace, and no blank.
    assert_eq!(filled("Say $${item} as it is.", None), Ok("Say ${item} as it is.".to_string()));
}

#[test]
fn a_part_that_links_a_file_wrongly_is_refused_saying_which_file_and_why() {
    let kept = files(&[
        ("card.md", "Card ${item}: ${title}\n"),
        ("sum.md", "There are ${ count + 1 } of them.\n"),
        ("called.md", "It is ${ upper(title) }.\n"),
        ("broken.md", "Card ${item\n"),
    ]);
    let cases: Vec<(Value, &str)> = vec![
        // Written wrongly as a part.
        (
            json!([{ "from": "card.md", "say": "x", "if": true }]),
            "part 1 of `parts`: it says more than one of `say`, `from` and `own`, and a part is said one way",
        ),
        (
            json!([{ "if": true }]),
            "part 1 of `parts`: it says neither `say` (words), `from` (a file of `prompts/`) nor `own` (a file of the project's own `prompts/`)",
        ),
        (
            json!([{ "say": "x", "if": true, "with": { "a": 1 } }]),
            "part 1 of `parts`: it says `with`, which fills the blanks of a file: words said here have their holes",
        ),
        (
            json!([{ "from": "card.md", "else": "x" }]),
            "part 1 of `parts`: it says `else` with no `if` for it to be the other side of",
        ),
        (
            json!([{ "from": 3 }]),
            "part 1 of `parts`: `from` is the name of a file of `prompts/`; it is the number 3",
        ),
        (
            json!([{ "from": "card.md", "with": ["shop-12"] }]),
            "part 1 of `parts`: `with` is what fills the file's blanks, each by its name; it is a list",
        ),
        // A name that is no file of the folder.
        (
            json!([{ "from": "../fleet.md" }]),
            "part 1 of `parts`: `from` names a file within `prompts/`, by its path from there with no going up out \
             of it; it is `../fleet.md`",
        ),
        (json!([{ "from": "/etc/passwd.md" }]), "part 1 of `parts`: `from` names a file within `prompts/`"),
        (json!([{ "from": "seat//card.md" }]), "part 1 of `parts`: `from` names a file within `prompts/`"),
        (json!([{ "from": "" }]), "part 1 of `parts`: `from` names a file within `prompts/`"),
        (
            json!([{ "from": "card.txt" }]),
            "part 1 of `parts`: `from` names a file that ends `.md`; it is `card.txt`",
        ),
        // A file that is not there.
        (
            json!(["a", { "from": "nowhere.md" }]),
            "part 2 of `parts`: the file `nowhere.md` it links was not found",
        ),
        // A blank nothing is handed for, and one that works something out.
        (
            json!([{ "from": "card.md", "with": { "item": "shop-12" } }]),
            "part 1 of `parts`: the file `card.md`: line 1: its blank `${title}` is handed nothing: say `title` under `with`",
        ),
        (
            json!([{ "from": "card.md" }]),
            "part 1 of `parts`: the file `card.md`: line 1: its blank `${item}` is handed nothing: say `item` under `with`",
        ),
        (
            json!([{ "from": "sum.md", "with": { "count": 2 } }]),
            "part 1 of `parts`: the file `sum.md`: line 1: its blank `${ count + 1 }` works something out",
        ),
        (
            json!([{ "from": "called.md", "with": { "title": "a" } }]),
            "part 1 of `parts`: the file `called.md`: line 1: its blank `${ upper(title) }` works something out",
        ),
        (
            json!([{ "from": "broken.md", "with": { "item": "a" } }]),
            "part 1 of `parts`: the file `broken.md`: it does not read: ",
        ),
    ];
    for (parts, why) in cases {
        let refused = put(&parts, None, &kept).unwrap_err();
        assert!(refused.starts_with(why), "{parts}: {refused}");
    }
    // A blank further down a file is said by the line it is on.
    let with = json!({ "item": "a" });
    assert_eq!(
        filled("Card ${item}.\n\nIt asks:\n${body}\n", with.as_object()),
        Err("line 4: its blank `${body}` is handed nothing: say `body` under `with`".to_string())
    );
    assert_eq!(
        filled("${item}\n${item} and ${ item + 1 }\n", with.as_object()).unwrap_err().split(':').next(),
        Some("line 2")
    );
}

#[test]
fn the_files_to_be_found_are_those_of_parts_that_are_said_each_once_and_by_the_name_of_their_place() {
    let parts = json!([
        "words",
        { "from": "seat/card.md", "with": { "item": "a" } },
        { "if": false, "from": "seat/changed.md" },
        { "if": "yes", "from": "seat/among.md" },
        { "if": true, "say": "said here" },
        { "from": "seat/card.md", "with": { "item": "b" } }
    ]);
    assert_eq!(links(&parts), Ok(vec!["seat/card.md".to_string(), "seat/among.md".to_string()]));
    assert_eq!(links(&json!([])), Ok(vec![]));
    // Parts said in the place of words: theirs are found where the part that holds them is said.
    let within = |said: bool| json!([{ "if": said, "say": [{ "if": true, "from": "seat/back.md" }, { "if": false, "from": "seat/new.md" }] }]);
    assert_eq!(links(&within(true)), Ok(vec!["seat/back.md".to_string()]));
    assert_eq!(links(&within(false)), Ok(vec![]));
    assert!(links(&json!("x")).unwrap_err().starts_with("`parts` is a list"));
    let outside = links(&json!([{ "from": "../x.md" }])).unwrap_err();
    assert!(outside.starts_with("part 1 of `parts`: `from` names a file within"), "{outside}");

    // A file is beside the blueprints of the place its blueprint is in.
    for (blueprint, from, name) in [
        ("std/review", "seat/card.md", "std/prompts/seat/card.md"),
        ("fleet", "worker.md", "prompts/worker.md"),
        ("shelf/restock", "opened.md", "shelf/prompts/opened.md"),
        ("teams/web/fleet", "told.md", "teams/web/prompts/told.md"),
    ] {
        assert_eq!(linked_as(blueprint, from), name, "`{from}` of `{blueprint}`");
    }
}

#[test]
fn a_file_of_the_projects_own_a_part_names_is_found_by_its_own_name_and_said_as_a_linked_file_is() {
    // Those of parts that are said, each once, and none that a part does not say.
    let parts = json!([
        { "own": "reviewer.md" },
        { "if": "", "own": "" },
        { "if": false, "own": "never.md" },
        { "if": true, "say": [{ "own": "reviewer.md" }, { "own": "notes/more.md" }] },
        { "from": "seat/card.md" }
    ]);
    assert_eq!(owned(&parts), Ok(vec!["reviewer.md".to_string(), "notes/more.md".to_string()]));
    assert_eq!(
        links(&parts),
        Ok(vec!["seat/card.md".to_string()]),
        "a file of the project's own is linked by nobody"
    );
    // A name said that is no name of a file is refused where it is said.
    let said = owned(&json!([{ "if": true, "own": "../away.md" }])).unwrap_err();
    assert!(said.starts_with("part 1 of `parts`: `own` names a file within"), "{said}");
    let said = owned(&json!([{ "own": "" }])).unwrap_err();
    assert!(said.starts_with("part 1 of `parts`: `own` names a file within"), "{said}");
    // It is found at the top of the project's own `prompts/`, whatever blueprint names it.
    assert_eq!(own_as("reviewer.md"), "prompts/reviewer.md");

    // Said with its blanks filled, kept under a key no `from` is taken for.
    let mut kept = files(&[("reviewer.md", "A file that ships beside the blueprint.\n")]);
    kept.insert(own_key("reviewer.md"), "You review ${what}.\n".to_string());
    let put_of = |parts: Value| put(&parts, None, &kept).map(|put| put.text);
    assert_eq!(
        put_of(json!([{ "own": "reviewer.md", "with": { "what": "the shop" } }, " Then stop."])),
        Ok("You review the shop. Then stop.".to_string())
    );
    assert_eq!(
        put_of(json!([{ "from": "reviewer.md" }])),
        Ok("A file that ships beside the blueprint.".to_string())
    );
    assert_eq!(
        put_of(json!([{ "if": "", "own": "", "else": "Nothing of its own." }])),
        Ok("Nothing of its own.".to_string())
    );
    // One part says one way.
    let both = put_of(json!([{ "own": "reviewer.md", "from": "reviewer.md" }])).unwrap_err();
    assert_eq!(
        both,
        "part 1 of `parts`: it says more than one of `say`, `from` and `own`, and a part is said one way"
    );
    let missing = put_of(json!([{ "own": "gone.md" }])).unwrap_err();
    assert_eq!(missing, "part 1 of `parts`: the project's own file `gone.md` it names was not found");
}
