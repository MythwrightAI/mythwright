//! A column somebody added to a table, and what somebody chose of any
//! column, as plain values (`kernel::added`): what one is called and
//! named, which added ones stand beside a declaration, and what order a
//! choice's options are in. Each a function with its cases.

use kernel::added::{
    KINDS, LONGEST, RESERVED, added, copy, free, label, name, offered, ordered, placed, reads, same, standing,
};
use kernel::principal::{AGENT_SCOPES, MEMBER_SCOPES, SCOPES, covers};

fn words(all: &[&str]) -> Vec<String> {
    all.iter().map(|one| one.to_string()).collect()
}

#[test]
fn a_column_is_called_by_a_few_words_on_one_line() {
    for (written, kept) in [("Points", "Points"), ("  Due date ", "Due date"), ("Größe", "Größe"), ("a", "a")]
    {
        assert_eq!(label(written).unwrap(), kept, "{written:?}");
    }
    let most = "x".repeat(LONGEST);
    assert_eq!(label(&most).unwrap(), most);
    for wrong in ["", "   ", "two\nlines", "a\tb", &"x".repeat(LONGEST + 1)] {
        let refused = label(wrong).unwrap_err().to_string();
        assert!(refused.contains("a column is called by a word or a few"), "{wrong:?}: {refused}");
        // What is said back is no longer than a label may be, and is one line.
        assert!(refused.chars().count() < 200 && !refused.contains('\n'), "{refused}");
    }
}

#[test]
fn a_columns_name_is_made_once_from_what_it_is_first_called() {
    for (called, named) in [
        ("Points", "points"),
        ("Due date", "due_date"),
        ("  Story   Points!  ", "story_points"),
        ("due-date", "due_date"),
        ("In_Flight", "in_flight"),
        ("Größe", "gr_e"),
        // It reads as a word in an expression: it begins with a letter.
        ("2026 goals", "column_2026_goals"),
        ("7", "column_7"),
        // Nothing of it can be written so: the label is still what it is called.
        ("日本", "column"),
        ("!!!", "column"),
    ] {
        let made = name(called);
        assert_eq!(made, named, "{called:?}");
        assert!(made.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'));
        assert!(made.as_bytes()[0].is_ascii_lowercase(), "{made}");
    }
    // No longer than a name may be, and never ending where a word was cut at its join.
    let long = name(&format!("{} tail", "x".repeat(LONGEST - 1)));
    assert_eq!(long, "x".repeat(LONGEST - 1));
    // Called what its name reads as, a column needs no label: the two are the same words.
    for called in ["Points", "Due date", "Story points"] {
        assert_eq!(reads(&name(called)), called);
    }
    assert_eq!(reads("part_of"), "Part of");
    assert_eq!(reads("in-flight"), "In flight");
    assert_eq!(reads(""), "");
}

#[test]
fn a_name_that_is_taken_or_is_a_records_own_is_numbered() {
    assert_eq!(free("points", &["title", "size"]), "points");
    assert_eq!(free("points", &["title", "points"]), "points_2");
    assert_eq!(free("points", &["points", "points_2", "points_3"]), "points_4");
    // What every record has, and what a record is known by, is no column somebody adds.
    for own in RESERVED {
        assert_eq!(free(own, &[]), format!("{own}_2"), "{own}");
    }
    assert!(RESERVED.contains(&"title") && RESERVED.contains(&"created") && RESERVED.contains(&"position"));
}

#[test]
fn a_copy_is_called_after_what_it_is_a_copy_of() {
    assert_eq!(copy("Points", &["Title", "Points"]), "Points copy");
    assert_eq!(copy("Points", &["Points", "points COPY"]), "Points copy 2");
    assert_eq!(copy("Points", &["Points copy", "Points copy 2"]), "Points copy 3");
}

#[test]
fn what_somebody_added_stands_beside_a_declaration_unless_the_declaration_takes_its_name() {
    assert_eq!(standing(&["title", "size"], &["notes", "weight"]), ["notes", "weight"]);
    // A field declared under a name somebody's column has is the declaration's from then on.
    assert_eq!(standing(&["title", "notes"], &["notes", "weight"]), ["weight"]);
    assert_eq!(standing(&["title"], &[]), Vec::<&str>::new());
    assert_eq!(standing(&[], &["notes"]), ["notes"]);
}

#[test]
fn a_choices_options_are_in_the_order_somebody_chose_with_what_is_declared_since_put_where_it_follows() {
    let declared = words(&["draft", "ready", "doing", "review", "done"]);
    // Nothing chosen: the declaration's order.
    assert_eq!(ordered(&declared, &[]), declared);
    // Chosen whole: as chosen.
    let chosen = words(&["done", "review", "doing", "ready", "draft"]);
    assert_eq!(ordered(&declared, &chosen), chosen);
    // What the declaration no longer has is passed over, and what was chosen twice counts once.
    assert_eq!(
        ordered(&words(&["ready", "doing", "done"]), &words(&["done", "gone", "doing", "done", "ready"])),
        words(&["done", "doing", "ready"])
    );
    // What the declaration has since is put after the one it follows there, and first where it follows none.
    let chosen = words(&["done", "doing", "ready"]);
    assert_eq!(
        ordered(&words(&["ready", "doing", "testing", "done"]), &chosen),
        words(&["done", "doing", "testing", "ready"])
    );
    assert_eq!(
        ordered(&words(&["triage", "ready", "doing", "done"]), &chosen),
        words(&["triage", "done", "doing", "ready"])
    );
    // Two new ones that follow each other stay together, in the declaration's order.
    assert_eq!(
        ordered(&words(&["ready", "doing", "testing", "staging", "done"]), &chosen),
        words(&["done", "doing", "testing", "staging", "ready"])
    );
    // Nothing of what was chosen is declared any more: the declaration's order.
    assert_eq!(ordered(&words(&["a", "b"]), &words(&["x", "y"])), words(&["a", "b"]));
    // Every option is there once whatever was chosen.
    for chosen in [&[][..], &["doing"][..], &["done", "draft"][..], &["x"][..]] {
        let mut all = ordered(&declared, &words(chosen));
        all.sort();
        let mut every = declared.clone();
        every.sort();
        assert_eq!(all, every, "{chosen:?}");
    }
    assert!(same(&words(&["a", "b"]), &words(&["b", "a"])));
    assert!(!same(&words(&["a", "b"]), &words(&["a"])) && !same(&words(&["a", "b"]), &words(&["a", "c"])));
}

#[test]
fn what_somebody_added_to_a_choice_stands_through_a_declaration_and_only_what_a_declaration_offered_goes() {
    let before = words(&["backlog", "ready", "doing", "done"]);
    // A board arranged with two columns more, one between two of its own and one before the last.
    let chosen = words(&["backlog", "ready", "doing", "reviewed", "staging", "done"]);
    let theirs = added(&chosen, &before, &[]);
    assert_eq!(theirs, words(&["reviewed", "staging"]));
    let theirs = Some(theirs.as_slice());
    // Declared again as it was: as they arranged it.
    assert_eq!(offered(&before, theirs, &chosen), chosen);
    // A new text that offers one more puts it after the one it follows there, beside what they added.
    let declared = words(&["backlog", "ready", "doing", "review", "done"]);
    assert_eq!(
        offered(&declared, theirs, &chosen),
        words(&["backlog", "ready", "doing", "review", "reviewed", "staging", "done"])
    );
    // One the declaration offered and no longer does goes; what they added stays.
    assert_eq!(
        offered(&words(&["ready", "doing", "done"]), theirs, &chosen),
        words(&["ready", "doing", "reviewed", "staging", "done"])
    );
    // Once a declaration comes to offer what they added, it is still theirs:
    // the next declaration that does not offer it does not take it away.
    let said = words(&["backlog", "ready", "doing", "reviewed", "staging", "done"]);
    assert_eq!(added(&chosen, &said, &words(&["reviewed", "staging"])), words(&["reviewed", "staging"]));
    assert_eq!(offered(&before, theirs, &chosen), chosen);
    // Arranged again without one, it is no longer theirs; arranged with one more, that one is.
    assert_eq!(
        added(
            &words(&["backlog", "ready", "staging", "qa", "done"]),
            &before,
            &words(&["reviewed", "staging"])
        ),
        words(&["staging", "qa"])
    );
    // What they chose the order of alone is as `ordered` has it.
    let reordered = words(&["done", "doing", "ready", "backlog"]);
    assert!(added(&reordered, &before, &[]).is_empty());
    for declared in [&before, &declared, &words(&["triage", "ready", "done"])] {
        assert_eq!(offered(declared, Some(&[]), &reordered), ordered(declared, &reordered), "{declared:?}");
    }
    // Not known what they added (a table kept before it was said): everything chosen stands.
    assert_eq!(offered(&declared, None, &chosen), offered(&declared, theirs, &chosen));
    assert_eq!(
        offered(&words(&["ready", "done"]), None, &words(&["done", "backlog", "ready"])),
        words(&["done", "backlog", "ready"])
    );
    // Nothing chosen: the declaration's order; what was chosen twice counts once.
    assert_eq!(offered(&declared, theirs, &[]), declared);
    let twice = words(&["x", "done", "x"]);
    assert_eq!(added(&twice, &before, &[]), words(&["x"]));
    assert_eq!(
        offered(&before, Some(&words(&["x"])), &twice),
        words(&["backlog", "ready", "doing", "x", "done"])
    );
}

#[test]
fn an_added_column_stands_before_the_one_it_was_put_beside_or_after_the_last() {
    let order = words(&["title", "points", "due"]);
    assert_eq!(placed(&order, "notes", Some("points")), words(&["title", "notes", "points", "due"]));
    assert_eq!(placed(&order, "notes", None), words(&["title", "points", "due", "notes"]));
    // A column that is not there is no place: it stands after the last.
    assert_eq!(placed(&order, "notes", Some("gone")), words(&["title", "points", "due", "notes"]));
    // It stands once, wherever it was said to stand already.
    assert_eq!(placed(&words(&["notes", "title", "title"]), "notes", None), words(&["title", "notes"]));
    assert_eq!(placed(&[], "notes", Some("title")), words(&["notes"]));
}

#[test]
fn what_a_column_somebody_adds_may_hold_has_no_authority_in_it_and_changing_columns_is_a_scope_of_its_own() {
    assert_eq!(KINDS, ["text", "number", "date", "flag", "choice", "list", "record", "records"]);
    assert!(!KINDS.contains(&"actor"));
    // A person holds it unless it was taken away, and an agent does not unless it was given:
    // an area of its own, since an agent holds the whole of `records`.
    assert!(SCOPES.iter().any(|(scope, _)| *scope == kernel::added::SCOPE));
    let held = |scopes: &[&str]| covers(&words(scopes), kernel::added::SCOPE);
    assert!(held(MEMBER_SCOPES) && held(&["*"]));
    assert!(!held(AGENT_SCOPES) && !held(&["records", "board", "projects"]));
}
