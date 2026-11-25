//! What a summary says of where its text stands among the blueprints
//! that ship: one that ships, and how what it does is changed; a fork of
//! one, what it was forked from, where it differs and how the default
//! has moved since. By its cases, each handed over as plain values.

use blueprint::summary::forked::{
    Fork, Is, Made, Own, Running, Standing, Usual, Usually, Ways, day, parts, usual,
};
use blueprint::summary::{Known, Line, Summary, plain, summary_as};
use blueprint::{ActionSpec, ValueType};

/// The default as the fork was made from it.
const DEFAULT: &str = "\
blueprint: shop/round
version: 3
description: Works a card.
params:
  tone: { type: string, default: plain }
steps:
  rows: { do: table.make, with: { name: stages, beside: board } }
  claim: { do: emit, with: { topic: shop.claimed } }
on:
  poked:
    event: shop.poke
    steps:
      say: { do: emit, with: { topic: shop.heard } }
";

fn actions() -> Vec<ActionSpec> {
    vec![
        ActionSpec::new("table.make", "Make sure a table is there.")
            .making("a table", "name")
            .setting("beside")
            .required("name", ValueType::String, "what it is called")
            .arg("beside", ValueType::String, "the table it sits beside"),
        ActionSpec::new("thing.make", "Make sure a thing is there.").making("a thing", "name").required(
            "name",
            ValueType::String,
            "what it is called",
        ),
    ]
}

fn said(source: &str, standing: Option<&Standing>) -> Summary {
    let actions = actions();
    let known = Known { actions: &actions, events: &[], scopes: &[] };
    let checked = blueprint::check(source, &blueprint::ActionList::new(actions.clone()));
    let errors: Vec<String> = checked.errors().map(|one| one.message.clone()).collect();
    assert!(errors.is_empty(), "the case checks: {errors:?}\n{source}");
    summary_as(&checked.blueprint.expect("it reads"), &known, standing)
}

/// A fork of the default made by Ada on the twentieth of November 2026.
fn fork<'a>(text: &'a str, ships: Option<&'a str>) -> Fork<'a> {
    let made = Made { from: DEFAULT, by: "Ada", at: 1_795_132_800_000 + 3_600_000 };
    Fork { of: "shop/round", text, made: Some(made), taken: None, ships, own: None }
}

fn part<'a>(summary: &'a Summary, kind: &str) -> Option<&'a [Line]> {
    summary.parts.iter().find(|part| part.kind == kind).map(|part| part.lines.as_slice())
}

fn texts(lines: &[Line]) -> Vec<&str> {
    lines.iter().map(|line| line.text.as_str()).collect()
}

fn kinds(summary: &Summary) -> Vec<&str> {
    summary.parts.iter().map(|part| part.kind.as_str()).collect()
}

#[test]
fn a_day_is_written_as_the_calendar_has_it() {
    for (at, written) in [
        (0, "1970-01-01"),
        (-1, "1969-12-31"),
        (946_598_400_000, "1999-12-31"),
        (951_868_800_000, "2000-03-01"),
        (1_709_164_800_000, "2024-02-29"),
        (1_795_132_800_000, "2026-11-20"),
        (1_795_132_800_000 + 86_399_999, "2026-11-20"),
    ] {
        assert_eq!(day(at), written, "{at}");
    }
}

#[test]
fn a_fork_nobody_has_changed_says_what_it_was_forked_from_and_that_it_is_the_default_as_forked() {
    let summary = said(DEFAULT, Some(&Standing::Fork(fork(DEFAULT, Some(DEFAULT)))));
    assert_eq!(kinds(&summary), ["is", "differs", "makes", "does", "hears", "may", "back"]);
    let first = &part(&summary, "is").unwrap()[0];
    assert_eq!(first.text, "Forked from `shop/round` version 3 by Ada on 2026-11-20.");
    let opens = first.opens.as_ref().expect("it leads to the default");
    assert_eq!((opens.what.as_str(), opens.name.as_deref()), ("default", Some("shop/round")));
    assert_eq!(texts(part(&summary, "differs").unwrap()), ["Nowhere: it is the default as it was forked."]);
    let back = part(&summary, "back").unwrap();
    assert_eq!(
        texts(back),
        ["Take the fork away, and the one that ships is what runs here under the name again, once what is \
          running is applied. Nothing is lost by it: the fork says what ships now."]
    );
    assert_eq!(back[0].opens.as_ref().map(|opens| opens.what.as_str()), Some("back"));
}

/// The fork with a step changed in a handler, a step added and a default changed.
fn changed() -> String {
    DEFAULT
        .replace("default: plain", "default: warm")
        .replace("topic: shop.heard", "topic: shop.said")
        .replace("  claim:", "  first: { do: noop }\n  claim:")
}

#[test]
fn a_fork_that_was_changed_says_where_it_differs_with_what_each_line_is_about() {
    let text = changed();
    let summary = said(&text, Some(&Standing::Fork(fork(&text, Some(DEFAULT)))));
    let differs = part(&summary, "differs").unwrap();
    assert_eq!(
        texts(differs),
        ["The param `tone` is changed.", "The step `first` is new.", "The handler `poked` is changed."]
    );
    // A step is shown where it is written in the fork; what moved inside a handler by the step that did.
    assert_eq!(differs[1].at.as_deref(), Some("main/first"));
    assert_eq!(differs[2].at, None);
    assert_eq!(texts(&differs[2].under), ["The step `say` is changed."]);
    assert_eq!(differs[2].under[0].at.as_deref(), Some("on.poked/say"));
    // The default is where it was: nothing is said of its having moved, and going back loses the fork's text.
    assert!(part(&summary, "moved").is_none());
    let back = &part(&summary, "back").unwrap()[0];
    assert!(
        back.text
            .ends_with("What the fork's text says is then gone, unless the project keeps it somewhere else.")
    );
}

#[test]
fn a_fork_whose_default_has_moved_says_so_by_version_first_and_that_it_takes_none_of_it() {
    let later =
        DEFAULT.replace("version: 3", "version: 4").replace("  claim:", "  tidy: { do: noop }\n  claim:");
    let text = changed();
    let summary = said(&text, Some(&Standing::Fork(fork(&text, Some(&later)))));
    assert_eq!(kinds(&summary), ["is", "differs", "moved", "makes", "does", "hears", "may", "back"]);
    let moved = part(&summary, "moved").unwrap();
    assert_eq!(
        texts(moved),
        [
            "`shop/round` 3 to 4: what ships now is not what this was forked from.",
            "The step `tidy` is new.",
            "The fork takes none of it by itself. Its text is this project's to change, with what moved said here \
             beside it.",
        ]
    );
    // What moved in the default is in the default's text: no line of it leads into the fork's.
    assert!(moved.iter().all(|line| line.at.is_none()));
    assert_eq!(moved[0].opens.as_ref().map(|opens| opens.what.as_str()), Some("default"));
    // Where the fork differs is still said against what it was forked from, not against what ships now.
    assert_eq!(texts(part(&summary, "differs").unwrap()).len(), 3);
    // A default that only says another version of the same has moved, and nothing in it is said to be another.
    let renumbered = DEFAULT.replace("version: 3", "version: 4");
    let summary = said(&text, Some(&Standing::Fork(fork(&text, Some(&renumbered)))));
    assert_eq!(
        texts(part(&summary, "moved").unwrap())[..2],
        [
            "`shop/round` 3 to 4: what ships now is not what this was forked from.",
            "Only in the version it says of itself, or in the order its parts are written.",
        ]
    );
}

#[test]
fn a_fork_from_before_says_only_what_its_texts_show() {
    let text = changed();
    // Nothing kept, and a text of the default that calls itself what the fork does.
    let taken = Fork {
        of: "shop/round",
        text: &text,
        made: None,
        taken: Some(DEFAULT),
        ships: Some(DEFAULT),
        own: None,
    };
    let summary = said(&text, Some(&Standing::Fork(taken)));
    assert_eq!(
        part(&summary, "is").unwrap()[0].text,
        "Forked from `shop/round` version 3. That was before what a fork is made from was kept: by whom and \
         when is not known, and the version is the one its own text says."
    );
    let titles: Vec<&str> = summary.parts.iter().map(|part| part.title.as_str()).collect();
    assert!(titles.contains(&"Where it differs from the default it was forked from"), "{titles:?}");
    // Nothing kept and no such text: no version is said, and it is set beside what ships now.
    let later = DEFAULT.replace("version: 3", "version: 5");
    let unknown =
        Fork { of: "shop/round", text: &text, made: None, taken: None, ships: Some(&later), own: None };
    let summary = said(&text, Some(&Standing::Fork(unknown)));
    assert_eq!(
        part(&summary, "is").unwrap()[0].text,
        "Forked from `shop/round`. That was before what a fork is made from was kept: from which version, by \
         whom and when is not known."
    );
    let titles: Vec<&str> = summary.parts.iter().map(|part| part.title.as_str()).collect();
    assert!(titles.contains(&"Where it differs from the default as it ships now"), "{titles:?}");
    assert!(part(&summary, "moved").is_none(), "what the default moved from is not known: {titles:?}");
    assert_eq!(texts(part(&summary, "differs").unwrap()).len(), 3);
}

#[test]
fn a_fork_laid_out_another_way_or_given_a_comment_comes_to_the_same_summary() {
    let text = changed();
    let later =
        DEFAULT.replace("version: 3", "version: 4").replace("  claim:", "  tidy: { do: noop }\n  claim:");
    let laid = blueprint::format(&text, &blueprint::Settings::default()).expect("it is laid out");
    let noted = format!("# Ours.\n\n{}", laid.replace("steps:\n", "steps:\n  # What it does.\n"));
    assert_ne!(noted, text);
    assert!(blueprint::means_the_same(&text, &noted));
    for ships in [DEFAULT, later.as_str()] {
        let one = said(&text, Some(&Standing::Fork(fork(&text, Some(ships)))));
        let other = said(&noted, Some(&Standing::Fork(fork(&noted, Some(ships)))));
        assert_eq!(one, other);
        // And the default laid out another way has not moved.
        let laid = blueprint::format(ships, &blueprint::Settings::default()).unwrap();
        assert_eq!(one, said(&text, Some(&Standing::Fork(fork(&text, Some(&laid))))));
    }
    // One that is neither a fork nor known to ship is said as it always was.
    let plainly = said(&text, None);
    assert_eq!(kinds(&plainly), ["is", "makes", "does", "hears", "may"]);
    assert_eq!(serde_json::to_string(&plainly).unwrap().matches("opens").count(), 0);
}

#[test]
fn one_that_ships_says_how_what_it_does_is_changed_in_the_order_to_reach_for_them() {
    let summary = said(DEFAULT, Some(&Standing::Ships { own: None, part_of: &[], ways: &Ways::default() }));
    assert_eq!(kinds(&summary), ["is", "makes", "does", "hears", "may", "change"]);
    let change = summary.parts.last().unwrap();
    assert!(change.ordered && change.title == "How to change what it does");
    let opens: Vec<&str> =
        change.lines.iter().map(|line| line.opens.as_ref().map_or("", |opens| opens.what.as_str())).collect();
    assert_eq!(opens, ["apply", "", "forking"]);
    assert!(change.lines[0].text.starts_with("Tell it something else: apply it again"));
    assert!(change.lines[1].text.starts_with("Change the rows of the table it makes"));
    let table = &change.lines[1].under[0];
    assert_eq!(
        (table.text.as_str(), table.opens.as_ref().unwrap().name.as_deref()),
        ("`stages`", Some("stages"))
    );
    assert!(change.lines[2].text.starts_with("Fork it: its text is copied into a project"));
    // Nothing of it is in the part that says what it is.
    assert!(part(&summary, "is").unwrap().iter().all(|line| line.opens.is_none()));

    // One with nothing to be told and no table says only the last; a thing that is no table is none.
    let bare = "blueprint: shop/bare\nsteps:\n  one: { do: thing.make, with: { name: shelf } }\n";
    let summary = said(bare, Some(&Standing::Ships { own: None, part_of: &[], ways: &Ways::default() }));
    assert_eq!(part(&summary, "change").unwrap().len(), 1);
}

#[test]
fn one_that_ships_offers_no_fork_where_the_project_keeps_no_blueprints_of_its_own_and_says_how_to_keep_them()
{
    let command = "mw project blueprints shop --usual".to_string();
    let ways = Ways { keeps_none: Some(command), ..Ways::default() };
    let summary = said(DEFAULT, Some(&Standing::Ships { own: None, part_of: &[], ways: &ways }));
    let change = part(&summary, "change").unwrap();
    let opens: Vec<&str> =
        change.iter().map(|line| line.opens.as_ref().map_or("", |opens| opens.what.as_str())).collect();
    // What it may be told and the rows of its table are as they were; no fork is offered, and nothing a project writes.
    assert_eq!(opens, ["apply", "", ""]);
    assert_eq!(
        change[2].text,
        "This project keeps no blueprints of its own, so none of its files is read in place of this and it has no \
         fork of it. To have it keep them: `mw project blueprints shop --usual`."
    );
}

#[test]
fn the_files_it_links_are_read_from_the_folder_the_project_keeps_its_own_in() {
    let linking =
        "blueprint: shop/asks\nsteps:\n  put: { do: text.put, with: { parts: [{ from: ask.md }] } }\n";
    let ways = Ways { own_in: Some("tools/bp".to_string()), ..Ways::default() };
    let actions = vec![ActionSpec::new("text.put", "Put words together.").arg(
        "parts",
        ValueType::List,
        "what it is put together from",
    )];
    let known = Known { actions: &actions, events: &[], scopes: &[] };
    let checked = blueprint::check(linking, &blueprint::ActionList::new(actions.clone()));
    let summary = summary_as(
        &checked.blueprint.expect("it reads"),
        &known,
        Some(&Standing::Ships { own: None, part_of: &[], ways: &ways }),
    );
    let under: Vec<&str> = part(&summary, "change")
        .unwrap()
        .iter()
        .flat_map(|line| line.under.iter().map(|under| under.text.as_str()))
        .collect();
    assert!(under.iter().any(|line| line.starts_with("`tools/bp/shop/prompts/ask.md`")), "{under:?}");
}

#[test]
fn one_that_ships_says_first_that_the_project_runs_its_own_fork_where_it_has_one() {
    let file = "blueprints/shop/round.bp";
    let summary = said(
        DEFAULT,
        Some(&Standing::Ships {
            own: Some(Own { file, is: Is::InUse }),
            part_of: &[],
            ways: &Ways::default(),
        }),
    );
    let first = &part(&summary, "is").unwrap()[0];
    assert_eq!(
        first.text,
        "This project runs its own fork of it, `blueprints/shop/round.bp`: that file, and not this text, is what \
         runs here under the name."
    );
    let opens = first.opens.as_ref().unwrap();
    assert_eq!((opens.what.as_str(), opens.name.as_deref()), ("fork", Some(file)));
    // And what is offered last is the fork there is, not another.
    let last = part(&summary, "change").unwrap().last().unwrap();
    assert_eq!(last.text, "Change this project's fork of it: its text is the project's own to change.");
    assert_eq!(last.opens.as_ref().map(|opens| opens.what.as_str()), Some("fork"));
}

#[test]
fn a_forks_own_parts_are_what_its_summary_says_and_a_terminal_prints_them_as_words() {
    let later =
        DEFAULT.replace("version: 3", "version: 4").replace("  claim:", "  tidy: { do: noop }\n  claim:");
    let text = changed();
    let summary = said(&text, Some(&Standing::Fork(fork(&text, Some(&later)))));
    let own = parts(&fork(&text, Some(&later)));
    for part in &own {
        assert!(summary.parts.contains(part), "`{}` is a part of the summary", part.title);
    }
    let printed = plain(&Summary { name: "shop/round".to_string(), parts: own });
    assert_eq!(
        printed,
        "shop/round

Where it differs from the default it was forked from
  The param `tone` is changed.
  The step `first` is new.
  The handler `poked` is changed.
    - The step `say` is changed.

The default has moved since
  `shop/round` 3 to 4: what ships now is not what this was forked from.
  The step `tidy` is new.
  The fork takes none of it by itself. Its text is this project's to change, with what moved said here beside it.

Going back to the default
  Take the fork away, and the one that ships is what runs here under the name again, once what is running is \
         applied. What the fork's text says is then gone, unless the project keeps it somewhere else.
"
    );
    // Read back from what it is handed out as, it is the same.
    let handed = serde_json::to_value(&summary).unwrap();
    assert_eq!(serde_json::from_value::<Summary>(handed).unwrap(), summary);
}

#[test]
fn one_that_ships_and_is_a_part_says_after_what_it_is_what_it_is_a_part_of_and_leads_there() {
    let opens = |line: &Line| {
        let opens = line.opens.as_ref()?;
        Some((opens.what.clone(), opens.name.clone().unwrap_or_default()))
    };
    // A part of one: one line, which opens what runs it.
    let one = ["shop/floor".to_string()];
    let summary = said(DEFAULT, Some(&Standing::Ships { own: None, part_of: &one, ways: &Ways::default() }));
    let is = part(&summary, "is").unwrap();
    assert_eq!(texts(is)[..2], ["Works a card.", "A part of `shop/floor`."]);
    assert_eq!(opens(&is[1]), Some(("default".to_string(), "shop/floor".to_string())));
    assert!(is[1].under.is_empty());

    // A part of several: how many, and each on a line of its own under it that opens it.
    let two = ["shop/floor".to_string(), "shop/yard".to_string()];
    let summary = said(DEFAULT, Some(&Standing::Ships { own: None, part_of: &two, ways: &Ways::default() }));
    let line = &part(&summary, "is").unwrap()[1];
    assert_eq!((line.text.as_str(), opens(line)), ("A part of 2 blueprints:", None));
    assert_eq!(texts(&line.under), ["`shop/floor`", "`shop/yard`"]);
    assert_eq!(opens(&line.under[1]), Some(("default".to_string(), "shop/yard".to_string())));
    let printed = blueprint::summary::plain(&summary);
    assert!(
        printed.contains("\n  A part of 2 blueprints:\n    - `shop/floor`\n    - `shop/yard`\n"),
        "{printed}"
    );

    // Under what says a fork runs here, which stays first; and first of all for a text that says nothing of itself.
    let forked = Standing::Ships {
        own: Some(Own { file: "blueprints/shop/round.bp", is: Is::InUse }),
        part_of: &one,
        ways: &Ways::default(),
    };
    let summary = said(DEFAULT, Some(&forked));
    let is = texts(part(&summary, "is").unwrap());
    assert!(is[0].starts_with("This project runs its own fork of it"));
    assert_eq!(is[1..3], ["Works a card.", "A part of `shop/floor`."]);
    let bare = "blueprint: shop/bare\nsteps:\n  one: { do: thing.make, with: { name: shelf } }\n";
    let summary = said(bare, Some(&Standing::Ships { own: None, part_of: &one, ways: &Ways::default() }));
    assert_eq!(texts(part(&summary, "is").unwrap())[0], "A part of `shop/floor`.");

    // And but for that line the summary is what it was.
    let mut with = said(DEFAULT, Some(&Standing::Ships { own: None, part_of: &one, ways: &Ways::default() }));
    with.parts[0].lines.remove(1);
    assert_eq!(
        with,
        said(DEFAULT, Some(&Standing::Ships { own: None, part_of: &[], ways: &Ways::default() }))
    );
}

/// A text that goes through a list of things, of which a row of a
/// project's table lays something into it by its order: two named and
/// one worked out as it runs, handed twice.
const ROUND: &str = "\
blueprint: shop/turn
description: Goes through what is done with a thing.
params:
  first: { type: string, default: shop/look }
steps:
  next:
    do: list.next
    with:
      usual:
        - { order: 10, does: shop/work }
        - { order: 20, does: shop/keep }
        - { order: 30, does: \"${params.first}\" }
  again:
    do: list.next
    with:
      usual:
        - { order: 10, does: shop/work }
";

/// A rule: asked and never run.
const RULE: &str = "\
blueprint: shop/choose
description: Say who takes it.
params:
  agents: { type: list, default: [] }
  about: { type: any, default: {} }
outputs:
  take: \"${ params.agents[0] ?? '' }\"
";

/// A text that links files of words.
const SAYS: &str = "\
blueprint: shop/ask
description: Ask for it.
steps:
  words:
    do: text.put
    with:
      parts:
        - from: ask.md
          with:
            why:
              - { if: true, from: ask/again.md }
        - from: ask.md
";

fn with_lists(source: &str, standing: Option<&Standing>) -> Summary {
    let mut actions = actions();
    actions.push(ActionSpec::new("list.next", "Say what comes next.").arg(
        "usual",
        ValueType::List,
        "what is usually done",
    ));
    actions.push(ActionSpec::new("text.put", "Put a text together.").arg(
        "parts",
        ValueType::List,
        "the parts",
    ));
    let known = Known { actions: &actions, events: &[], scopes: &[] };
    let checked = blueprint::check(source, &blueprint::ActionList::new(actions.clone()));
    let errors: Vec<String> = checked.errors().map(|one| one.message.clone()).collect();
    assert!(errors.is_empty(), "the case checks: {errors:?}\n{source}");
    summary_as(&checked.blueprint.expect("it reads"), &known, standing)
}

/// The table whose rows lay something into a list, and the command that adds one.
fn rows() -> Ways {
    Ways { rows: Some(("steps".to_string(), "add a row to steps".to_string())), ..Ways::default() }
}

fn opens(line: &Line) -> (&str, &str) {
    let opens = line.opens.as_ref().expect("the line leads somewhere");
    (opens.what.as_str(), opens.name.as_deref().unwrap_or(""))
}

#[test]
fn the_usual_things_of_the_lists_a_text_goes_through_are_what_it_hands_each_once_in_order() {
    let read = blueprint::check(ROUND, &blueprint::NoCatalog).blueprint.unwrap();
    let found = usual(&read);
    let said: Vec<(Option<&str>, f64)> = found.iter().map(|one| (one.does.as_deref(), one.order)).collect();
    assert_eq!(said, [(Some("shop/work"), 10.0), (Some("shop/keep"), 20.0), (None, 30.0)]);
    // A text that hands none goes through none.
    let read = blueprint::check(DEFAULT, &blueprint::NoCatalog).blueprint.unwrap();
    assert_eq!(usual(&read), Vec::<Usual>::new());
}

#[test]
fn one_that_goes_through_a_list_says_first_that_a_row_of_the_table_lays_something_into_it() {
    let ways = rows();
    let summary = with_lists(ROUND, Some(&Standing::Ships { own: None, part_of: &[], ways: &ways }));
    let change = part(&summary, "change").unwrap();
    assert_eq!(
        change[0].text,
        "Change what it goes through by a row of the project's `steps` table: as it ships it does `shop/work` \
         at 10, `shop/keep` at 20 and what it is told at 30, and a row lays something before any of them by a \
         lesser order, in its place by the same order, or after it by a greater: `add a row to steps`."
    );
    assert_eq!(opens(&change[0]), ("table", "steps"));
    assert!(change[1].text.starts_with("Tell it something else"));
    assert_eq!(opens(change.last().unwrap()).0, "forking");
    // With no such table handed, nothing is said of one.
    let summary =
        with_lists(ROUND, Some(&Standing::Ships { own: None, part_of: &[], ways: &Ways::default() }));
    assert!(part(&summary, "change").unwrap()[0].text.starts_with("Tell it something else"));
}

#[test]
fn one_of_the_usual_things_of_a_list_says_first_where_it_is_done_and_that_a_row_lays_something_there() {
    let usually = vec![
        Usually { by: "shop/turn".to_string(), order: 20.0 },
        Usually { by: "shop/other".to_string(), order: 2.5 },
    ];
    let ways = Ways { usually, ..rows() };
    let summary = said(DEFAULT, Some(&Standing::Ships { own: None, part_of: &[], ways: &ways }));
    let change = part(&summary, "change").unwrap();
    assert_eq!(
        change[0].text,
        "Lay something before it, in its place or after it by a row of the project's `steps` table: it is done \
         at 20 in what `shop/turn` goes through and at 2.5 in what `shop/other` goes through, so a row there at \
         a lesser order is done before it, one at the same order in its place, and one at a greater after it: \
         `add a row to steps`."
    );
    assert_eq!(opens(&change[0]), ("table", "steps"));
    assert_eq!(texts(change).len(), 4, "the row, what it is told, its table and a fork: {:?}", texts(change));
}

#[test]
fn a_rule_says_how_a_project_writes_its_own_and_not_that_it_is_applied_with_something_else() {
    let ways = Ways { own_as: Some("blueprints/shop/choose.bp".to_string()), ..Ways::default() };
    let summary = said(RULE, Some(&Standing::Ships { own: None, part_of: &[], ways: &ways }));
    let change = part(&summary, "change").unwrap();
    assert_eq!(
        texts(change)[0],
        "Write the project's own: it is asked and never run, so a file of the project's own under its name, \
         `blueprints/shop/choose.bp`, is asked in its place and says `take` from what it is shown (`agents` and \
         `about`). Saved through the factory it is asked from then on; written in an editor, once what is \
         running there is applied again saying to take the project's files as they are; and in a folder \
         somebody gave, only once the project is trusted."
    );
    assert_eq!(change.len(), 2, "{:?}", texts(change));
    assert_eq!(opens(&change[1]).0, "forking");
}

#[test]
fn each_file_a_text_links_is_a_line_with_the_projects_own_file_read_in_its_place() {
    let linking = vec![("shop/turn".to_string(), 3), ("shop/seat".to_string(), 1)];
    let ways = Ways { linking, ..Ways::default() };
    let summary = with_lists(SAYS, Some(&Standing::Ships { own: None, part_of: &[], ways: &ways }));
    let change = part(&summary, "change").unwrap();
    assert!(change[0].text.starts_with("Change the words it puts together: each file it links is read"));
    assert_eq!(
        texts(&change[0].under),
        [
            "`blueprints/shop/prompts/ask.md`, in place of `shop/prompts/ask.md`",
            "`blueprints/shop/prompts/ask/again.md`, in place of `shop/prompts/ask/again.md`",
        ]
    );
    assert!(change[1].text.starts_with("Change the words what it runs puts together: 4 files are linked"));
    assert_eq!(texts(&change[1].under), ["`shop/turn`: 3 files", "`shop/seat`: 1 file"]);
    assert_eq!(opens(&change[1].under[0]), ("default", "shop/turn"));
    assert_eq!(opens(change.last().unwrap()).0, "forking");
}

#[test]
fn a_table_it_makes_is_said_with_what_the_step_that_makes_it_says_it_is() {
    let described = DEFAULT.replace(
        "  rows: { do: table.make",
        "  rows: { description: What is done. A row is one thing., do: table.make",
    );
    let summary =
        said(&described, Some(&Standing::Ships { own: None, part_of: &[], ways: &Ways::default() }));
    let table = &part(&summary, "change").unwrap()[1].under[0];
    assert_eq!((table.text.as_str(), opens(table)), ("`stages`: What is done.", ("table", "stages")));
}

#[test]
fn only_a_table_whose_rows_set_what_something_does_is_offered_to_be_changed_by_its_rows() {
    let ships = Standing::Ships { own: None, part_of: &[], ways: &Ways::default() };
    let offered = |text: &str| -> Vec<String> {
        let summary = said(text, Some(&ships));
        let change = part(&summary, "change").unwrap();
        change
            .iter()
            .flat_map(|line| line.under.iter())
            .filter_map(|line| line.opens.as_ref().filter(|opens| opens.what == "table")?.name.clone())
            .collect()
    };
    // A table whose step says what the action says makes its rows set something: it sits beside another.
    let two = "blueprint: shop/two\nsteps:\n  cards: { do: table.make, with: { name: board } }\n  \
               rows: { do: table.make, with: { name: stages, beside: board } }\n";
    assert_eq!(offered(two), ["stages"], "the board's own rows are what it holds, and are not offered");
    let summary = said(two, Some(&ships));
    assert!(
        part(&summary, "change").unwrap()[0].text.starts_with("Change the rows of the table it makes"),
        "one table is offered, and is said as one"
    );
    // A text that makes only what its rows are said nothing of, of the rows.
    let one = "blueprint: shop/one\nsteps:\n  cards: { do: table.make, with: { name: board } }\n";
    assert!(offered(one).is_empty());
    assert_eq!(texts(part(&said(one, Some(&ships)), "change").unwrap()).len(), 1, "a fork alone");
    // What makes a table and says nothing of which argument has its rows set something offers none.
    let silent = vec![
        ActionSpec::new("table.make", "Make sure a table is there.")
            .making("a table", "name")
            .required("name", ValueType::String, "what it is called")
            .arg("beside", ValueType::String, "the table it sits beside"),
    ];
    let known = Known { actions: &silent, events: &[], scopes: &[] };
    let checked = blueprint::check(two, &blueprint::ActionList::new(silent.clone()));
    let summary = summary_as(&checked.blueprint.unwrap(), &known, Some(&ships));
    assert_eq!(texts(part(&summary, "change").unwrap()).len(), 1, "a fork alone");
}

#[test]
fn a_projects_own_file_under_the_name_says_first_where_it_stands_there() {
    let file = "blueprints/shop/round.bp";
    let running = [Running { key: "floor".to_string(), take: "apply floor again, taking".to_string() }];
    let ways = Ways::default();
    let ships = |is| Standing::Ships { own: Some(Own { file, is }), part_of: &[], ways: &ways };

    // Not in use yet: what runs there, and both ways to put it in use.
    let summary = said(DEFAULT, Some(&ships(Is::NotYet { running: &running })));
    let first = &part(&summary, "is").unwrap()[0];
    assert_eq!(
        first.text,
        "This project's own `blueprints/shop/round.bp`, for `shop/round`, is not in use yet: it was written \
         outside the factory, or changed there, since `floor` was applied, which goes on with what was there \
         under the name then. Saved here by somebody who holds what that runs with, it is in use; or apply that \
         again saying to take the project's files as they are: `apply floor again, taking`."
    );
    assert_eq!(opens(first), ("fork", file));
    assert_eq!(opens(part(&summary, "change").unwrap().last().unwrap()), ("fork", file));

    // Passed over: the one that ships runs, and the command that trusts the project.
    let summary = said(DEFAULT, Some(&ships(Is::PassedOver { trust: "trust shop" })));
    let first = &part(&summary, "is").unwrap()[0];
    assert!(
        first
            .text
            .starts_with("This project's own `blueprints/shop/round.bp`, for `shop/round`, is passed over")
    );
    assert!(first.text.ends_with("until the factory's owner trusts it: `trust shop`."), "{}", first.text);
    // A project that is not trusted is offered the fork it would make, which the daemon refuses in its words.
    assert_eq!(opens(part(&summary, "change").unwrap().last().unwrap()).0, "forking");

    // The file itself, passed over: that, first, and no part of a fork.
    let own = Own { file, is: Is::PassedOver { trust: "trust shop" } };
    let summary = said(DEFAULT, Some(&Standing::Passed(own)));
    assert_eq!(kinds(&summary), ["is", "makes", "does", "hears", "may"]);
    assert_eq!(part(&summary, "is").unwrap()[0].text, first.text);

    // A fork that is not in use yet says so under what it was forked from.
    let text = changed();
    let mut forked = fork(&text, Some(DEFAULT));
    forked.own = Some(Own { file, is: Is::NotYet { running: &running } });
    let summary = said(&text, Some(&Standing::Fork(forked)));
    let is = texts(part(&summary, "is").unwrap());
    assert!(is[0].starts_with("Forked from `shop/round` version 3 by Ada"));
    assert!(
        is[1].starts_with(
            "This project's own `blueprints/shop/round.bp`, for `shop/round`, is not in use yet"
        )
    );
    // One in use says nothing more than it did.
    let mut forked = fork(&text, Some(DEFAULT));
    forked.own = Some(Own { file, is: Is::InUse });
    assert_eq!(
        said(&text, Some(&Standing::Fork(forked))),
        said(&text, Some(&Standing::Fork(fork(&text, Some(DEFAULT)))))
    );
}

#[test]
fn where_a_projects_own_file_stands_is_one_sentence_wherever_else_it_is_said() {
    use blueprint::summary::forked::{own_line, own_sentence};
    let running = [
        Running { key: "floor".to_string(), take: "take floor".to_string() },
        Running { key: "yard".to_string(), take: "take yard".to_string() },
    ];
    let not_yet = Own { file: "blueprints/std/prompts/ask.md", is: Is::NotYet { running: &running } };
    let sentence = own_sentence(&not_yet, "std/prompts/ask.md");
    assert_eq!(sentence, own_line(&not_yet, "std/prompts/ask.md").text);
    assert!(sentence.contains("since `floor` and `yard` were applied, which go on with"), "{sentence}");
    assert!(sentence.ends_with("or apply each again saying to take the project's files as they are: `take floor` and `take yard`."), "{sentence}");
    let passed = Own { file: "blueprints/std/x.bp", is: Is::PassedOver { trust: "trust it" } };
    assert_eq!(own_sentence(&passed, "std/x"), own_line(&passed, "std/x").text);
    let used = Own { file: "blueprints/std/x.bp", is: Is::InUse };
    assert_eq!(
        own_sentence(&used, "std/x"),
        "`blueprints/std/x.bp` is in use: it is what runs in this project under `std/x`, in place of the one that \
         ships."
    );
}
