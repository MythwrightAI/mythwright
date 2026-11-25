//! How an agent on the assistant that is built in is given a program that
//! can do work (`agents::real`): the sentence, for each thing an agent may
//! be. Every one of them names the command that does it, and none names a
//! command that would be said over.

use agents::real::{CELL, Whose, how};

#[test]
fn what_gives_an_agent_a_program_is_said_by_the_one_command_for_whose_the_agent_is() {
    let cli = brand::CLI;
    // Whose it is; what the sentence has to say; what it must not.
    let cases: Vec<(Whose, Vec<String>, Vec<&str>)> = vec![
        // Added by hand: said again, with what it runs on.
        (
            Whose::Hand,
            vec![format!("`{cli} agent add helper --adapter <package>`")],
            vec!["record set", "fleet"],
        ),
        // One of a fleet's row: the row, by its own id, and never the agent itself.
        (
            Whose::Row("shop-agents-2".into()),
            vec![
                format!("`{cli} record set shop-agents-2 {CELL}=<package>`"),
                format!("`{CELL}` cell"),
                "is one of a fleet's".into(),
            ],
            vec!["agent add", "<row>"],
        ),
        // Brought up by something running whose row was not found: the cell, and how its rows are listed.
        (
            Whose::Held(Some("shop".into())),
            vec![
                format!("`{cli} record list shop agents`"),
                format!("`{cli} record set <row> {CELL}=<package>`"),
            ],
            vec!["agent add"],
        ),
        (Whose::Held(None), vec![format!("`{cli} record list <project> agents`")], vec!["agent add"]),
        // Nobody to ask what is running: both, each said for what it is for.
        (
            Whose::Unasked,
            vec![
                format!("`{cli} agent add helper --adapter <package>`"),
                format!("`{cli} record set <row> {CELL}=<package>`"),
            ],
            vec![],
        ),
    ];
    for (whose, says, never) in cases {
        let said = how("helper", &whose);
        assert!(
            said.contains(&format!("(`{cli} help extensions`)")),
            "where a package comes from, of {whose:?}: {said}"
        );
        for part in says {
            assert!(said.contains(&part), "`{part}` of {whose:?}: {said}");
        }
        for part in never {
            assert!(!said.contains(part), "`{part}` is not said of {whose:?}: {said}");
        }
        assert!(!said.ends_with('.') && !said.contains('\n'), "a clause, to go on from another: {said}");
    }
}
