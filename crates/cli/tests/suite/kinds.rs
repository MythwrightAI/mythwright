//! `events --kinds`: what can happen, and what each kind of event carries;
//! and the terminal's check holding a handler to it.

use crate::Rig;

#[tokio::test]
async fn the_kinds_of_event_there_are_and_what_one_carries_are_listed_from_the_terminal() {
    let rig = Rig::signed_in().await;

    let all = rig.run(&["events", "--kinds"]).await.ok();
    let line = |topic: &str| {
        all.lines()
            .find(|line| line.split_whitespace().next() == Some(topic))
            .unwrap_or_else(|| panic!("{topic} in\n{all}"))
    };
    assert!(line("board.item.moved").ends_with("A card was moved to another column."), "{all}");
    // What the runner says of its steps is passed on and kept only where
    // something hears it, which its sentence says.
    let failed = line("step.failed");
    assert!(failed.contains("A step failed.") && failed.ends_with("hears it or waits for it."), "{all}");
    assert!(line("log.line").ends_with("(passed on, not kept: no handler hears it)"), "{all}");

    let one = rig.run(&["events", "--kinds", "board.item.moved"]).await.ok();
    assert!(one.starts_with("board.item.moved: A card was moved to another column.\n"), "{one}");
    assert!(one.contains("carries, as `event.data.<name>`:"), "{one}");
    let field = |name: &str| {
        let found = one.lines().find(|line| line.split_whitespace().next() == Some(name));
        found.unwrap_or_else(|| panic!("{name} in\n{one}")).split_whitespace().collect::<Vec<_>>().join(" ")
    };
    assert_eq!(field("to"), "to string always the column it is in now");
    assert_eq!(field("note"), "note string what was said as it was moved");

    // An instance's state is under names of its blueprint's own.
    let state = rig.run(&["events", "--kinds", "instance.state"]).await.ok();
    assert!(state.ends_with("and whatever else its emitter put in it, under names of its own\n"), "{state}");

    let none = rig.run(&["events", "--kinds", "board.item.shuffled"]).await;
    assert_ne!(none.code, 0);
    assert!(
        none.err.contains(
            "there is no event called `board.item.shuffled`; under `board.` there are: board.columns"
        ),
        "{}",
        none.err
    );
    let theirs = rig.run(&["events", "--kinds", "shop.restocked"]).await;
    assert!(
        theirs.err.contains(
            "this program emits no event called `shop.restocked`. What a blueprint says with `emit` is not listed here"
        ),
        "{}",
        theirs.err
    );
}

#[tokio::test]
async fn the_terminals_check_says_what_a_handler_names_that_its_event_does_not_carry() {
    let rig = Rig::signed_in().await;
    let file = rig.dir.join("watch.bp");
    let source = "blueprint: watch\non:\n  moved:\n    event: board.item.moved\n    for: { card: c-1 }\n    when: ${ event.data.too == \"done\" }\n    do: noop\n";
    std::fs::write(&file, source).unwrap();
    let ran = rig.run(&["check", file.to_str().unwrap()]).await;
    assert_ne!(ran.code, 0);
    let said = format!("{}{}", ran.out, ran.err);
    assert!(said.contains("`for:` names `card`, which `board.item.moved` does not carry"), "{said}");
    assert!(
        said.contains("`event.data.too` is not something `board.item.moved` carries; did you mean `to`?"),
        "{said}"
    );
}
