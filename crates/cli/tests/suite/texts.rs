//! `texts`: how what is running comes by new blueprints, said and changed
//! from the terminal.

use crate::Rig;

#[tokio::test(flavor = "multi_thread")]
async fn the_terminal_says_how_texts_are_come_by_and_changes_it_saying_what_following_gives_up() {
    let rig = Rig::signed_in().await;
    let usual = rig.run(&["texts"]).await.ok();
    assert!(
        usual.starts_with("nothing was said of how texts are come by, so it is as usual for "),
        "{usual}"
    );
    assert!(usual.contains("A text that is taken lands in place"), "{usual}");

    let held = rig.run(&["texts", "held", "--lands", "again"]).await.ok();
    assert!(held.starts_with("texts are held: what is running goes on with the blueprints it has"), "{held}");
    assert!(
        held.contains("lands by its steps being run again") && held.contains("\nsaid by user:"),
        "{held}"
    );

    // Following says what it gives up, to the owner who turns it on.
    let follow = rig.run(&["texts", "follow", "--lands", "in-place"]).await.ok();
    assert!(
        follow.starts_with("texts are followed: a blueprint that changes is taken as it changes"),
        "{follow}"
    );
    assert!(
        follow.contains("an agent among them, changes what runs with the authority of whoever applied it"),
        "{follow}"
    );
    assert!(follow.contains("lands in place"), "{follow}");
    // And with nothing held, there is nothing to put a tree back on.
    let unknown = rig.run(&["texts", "sometimes"]).await;
    assert_eq!(unknown.code, 1, "{}", unknown.out);
    assert!(unknown.err.contains("is not a way texts are come by"), "{}", unknown.err);
    let back = rig.run(&["texts", "usual"]).await.ok();
    assert!(back.starts_with("nothing was said of how texts are come by"), "{back}");
    assert!(rig.run(&["help", "texts"]).await.ok().contains("How texts are come by:"));
}

/// A package whose blueprint says its version and one thing it is handed.
fn package(version: u32, more: &str) -> String {
    format!(
        "extension: reach\ntitle: Reach\nblueprints:\n  round: |\n    blueprint: reach/round\n    version: {version}\n    \
         params:\n      id: {{ type: int, default: 0 }}\n{more}    steps:\n      made: {{ do: noop }}\n"
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn a_blueprint_that_ships_is_made_a_projects_own_looked_at_against_what_ships_and_given_back() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let folder = rig.dir.join("reach");
    std::fs::create_dir_all(&folder).unwrap();
    let release = |text: String| std::fs::write(folder.join("extension.yaml"), text).unwrap();
    release(package(1, ""));
    rig.run(&["extension", "add", folder.to_str().unwrap()]).await.ok();

    // Only what ships can be made somebody's own, and it has to be said whose.
    let nothing = rig.run(&["own", "reach/nothing", "--project", "shop"]).await;
    assert_eq!(nothing.code, 1, "{}", nothing.out);
    assert!(nothing.err.contains("no blueprint called `reach/nothing` ships here"), "{}", nothing.err);
    assert_eq!(rig.run(&["own", "reach/round"]).await.code, 1);

    let made = rig.run(&["own", "reach/round", "--project", "shop"]).await.ok();
    let file = format!("@blueprints/{}", brand::blueprint_file("reach/round"));
    assert!(
        made.starts_with(&format!("`reach/round` (version 1) is now `shop`'s own: `{file}`\n")),
        "{made}"
    );
    assert!(
        made.contains("no longer moves when a new version of this program brings") && made.contains("--back"),
        "{made}"
    );
    // Kept by the factory, and never in the project's folder.
    let own = rig.placed("shop").join(brand::blueprint_file("reach/round"));
    let text = std::fs::read_to_string(&own).unwrap();
    assert!(text.starts_with("blueprint: reach/round\nversion: 1\n"), "{text}");
    assert!(!rig.project("shop").join("blueprints").exists());
    // It is what runs there under the name, and it says so of itself.
    let applied = rig.run(&["apply", "reach/round", "--project", "shop", "--key", "one"]).await.ok();
    assert!(applied.contains("started"), "{applied}");
    let twice = rig.run(&["own", "reach/round", "--project", "shop"]).await;
    assert!(twice.err.contains("already has a file of its own"), "{}", twice.err);
    let same = rig.run(&["own", "reach/round", "--project", "shop", "--say"]).await.ok();
    assert!(same.contains("it says what the one that ships says"), "{same}");

    // The program brings another: the project's own stays, and is told where the two differ.
    release(package(2, "      tone: { type: string, default: plain }\n"));
    rig.run(&["extension", "add", folder.to_str().unwrap()]).await.ok();
    assert_eq!(std::fs::read_to_string(&own).unwrap(), text);
    let moved = rig.run(&["own", "reach/round", "--project", "shop", "--say"]).await.ok();
    assert!(
        moved.contains(
            "it differs from the one that ships now, version 2 where this says 1: the param `tone` is gone"
        ),
        "{moved}"
    );

    // Given back: refused while it says something else than what ships, unless told to lose that.
    let kept = rig.run(&["own", "reach/round", "--project", "shop", "--back"]).await;
    assert_eq!(kept.code, 1, "{}", kept.out);
    assert!(kept.err.contains("Nothing was taken away") && own.is_file(), "{}", kept.err);
    let back = rig.run(&["own", "reach/round", "--project", "shop", "--back", "--force"]).await.ok();
    assert!(back.contains("is the one that ships again in `shop`") && !own.exists(), "{back}");
    assert!(rig.run(&["help", "own"]).await.ok().contains("What follows from it being the project's"));
}

#[tokio::test(flavor = "multi_thread")]
async fn words_a_blueprint_links_are_made_a_projects_own_under_their_own_name() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let name = "std/prompts/lane-round.md";
    let made = rig.run(&["own", name, "--project", "shop"]).await.ok();
    let file = format!("@blueprints/{name}");
    assert!(made.starts_with(&format!("`{name}` is now `shop`'s own: `{file}`\n")), "{made}");
    // Words have no summary to explain them by: what was forked from is said by `--say`.
    assert!(made.contains(&format!("own {name} --project shop --say` says that")), "{made}");
    let own = rig.placed("shop").join(name);
    assert!(own.is_file(), "{made}");
    assert!(!rig.placed("shop").join(brand::blueprint_file(name)).exists());
    let same = rig.run(&["own", name, "--project", "shop", "--say"]).await.ok();
    assert!(same.contains("it says what the one that ships says"), "{same}");
    assert!(same.contains("Nowhere: it is the default as it was forked."), "{same}");
    // Another blank in the project's words: said as words, with no version.
    let text = std::fs::read_to_string(&own).unwrap();
    std::fs::write(&own, text.replacen("${why}", "${why} ${tone}", 1)).unwrap();
    let moved = rig.run(&["own", name, "--project", "shop", "--say"]).await.ok();
    assert!(moved.contains("; it was changed since it was forked"), "{moved}");
    let differs = "Where it differs from the default it was forked from";
    assert_eq!(part(&moved, differs), ["Its words changed.", "The blank `${tone}` is new."], "{moved}");
    assert!(!moved.contains("version"), "words say none: {moved}");
    let back = rig.run(&["own", name, "--project", "shop", "--back", "--force"]).await.ok();
    assert!(back.contains("is the one that ships again in `shop`") && !own.exists(), "{back}");
}

/// The part of what a terminal printed that is headed `title`: its lines, as far as the next heading.
fn part<'a>(printed: &'a str, title: &str) -> Vec<&'a str> {
    let after = printed.split(&format!("\n{title}\n")).nth(1).unwrap_or_default();
    after.lines().take_while(|line| line.starts_with("  ")).map(str::trim).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_fork_is_explained_by_what_it_was_forked_from_where_it_differs_and_how_the_default_has_moved() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let folder = rig.dir.join("reach");
    std::fs::create_dir_all(&folder).unwrap();
    let release = |text: String| std::fs::write(folder.join("extension.yaml"), text).unwrap();
    release(package(1, ""));
    rig.run(&["extension", "add", folder.to_str().unwrap()]).await.ok();

    // What ships says how what it does is changed, the last of it a fork, with the command for it.
    let ships = rig.run(&["explain", "reach/round"]).await.ok();
    let change = part(&ships, "How to change what it does");
    assert!(change[0].starts_with("1. Tell it something else: apply it again"), "{ships}");
    assert!(change[1].starts_with("2. Fork it: its text is copied into a project"), "{ships}");
    assert!(ships.contains("To fork it: `"), "{ships}");
    assert!(ships.contains(" own reach/round --project <project>`"), "{ships}");

    // Forked: it says what it was forked from, and that it is the default as it was forked.
    let made = rig.run(&["own", "reach/round", "--project", "shop"]).await.ok();
    assert!(made.contains("It is a fork of `reach/round`, and what it was forked from is kept"), "{made}");
    let fork = rig.run(&["explain", "reach/round", "--project", "shop"]).await.ok();
    assert!(part(&fork, "What it is")[0].starts_with("Forked from `reach/round` version 1 by "), "{fork}");
    let differs = "Where it differs from the default it was forked from";
    assert_eq!(part(&fork, differs), ["Nowhere: it is the default as it was forked."], "{fork}");
    assert!(!fork.contains("The default has moved since"), "{fork}");
    assert!(part(&fork, "Going back to the default")[0].contains("Nothing is lost by it"), "{fork}");
    assert!(fork.contains(" own reach/round --project shop --back`"), "{fork}");

    // Changed, as any file of the project's is: where it differs is said by step, the same by both commands.
    let own = rig.placed("shop").join(brand::blueprint_file("reach/round"));
    let text = std::fs::read_to_string(&own).unwrap();
    std::fs::write(&own, text.replace("made: { do: noop }", "made: { do: noop }\n  more: { do: noop }"))
        .unwrap();
    let fork = rig.run(&["explain", "reach/round", "--project", "shop"]).await.ok();
    assert_eq!(part(&fork, differs), ["The step `more` is new."], "{fork}");
    let say = rig.run(&["own", "reach/round", "--project", "shop", "--say"]).await.ok();
    assert!(say.contains(", forked from `reach/round` version 1 by "), "{say}");
    assert!(
        say.contains("; it was changed since it was forked, and the one that ships is as it was then"),
        "{say}"
    );
    assert_eq!(part(&say, differs), part(&fork, differs), "{say}");
    // Asked for as the file it is, it says the same: and as a file of no project, what the text alone says.
    let as_file = rig.run(&["explain", own.to_str().unwrap(), "--project", "shop"]).await.ok();
    assert_eq!(as_file, fork);
    let alone = rig.run(&["explain", own.to_str().unwrap()]).await.ok();
    assert!(!alone.contains("Forked from") && !alone.contains(differs), "{alone}");

    // The program brings another: the fork is as it was, and says how the default has moved since.
    release(package(2, "      tone: { type: string, default: plain }\n"));
    rig.run(&["extension", "add", folder.to_str().unwrap()]).await.ok();
    let fork = rig.run(&["explain", "reach/round", "--project", "shop"]).await.ok();
    assert!(part(&fork, "What it is")[0].starts_with("Forked from `reach/round` version 1 by "), "{fork}");
    assert_eq!(part(&fork, differs), ["The step `more` is new."], "{fork}");
    let moved = part(&fork, "The default has moved since");
    assert_eq!(
        moved[..2],
        [
            "`reach/round` 1 to 2: what ships now is not what this was forked from.",
            "The param `tone` is new."
        ],
        "{fork}"
    );
    assert!(moved[2].starts_with("The fork takes none of it by itself."), "{fork}");
    let say = rig.run(&["own", "reach/round", "--project", "shop", "--say"]).await.ok();
    assert!(say.contains("; it differs from the one that ships now, version 2 where this says 1: "), "{say}");
    assert_eq!(part(&say, "The default has moved since"), moved, "{say}");
    assert!(part(&fork, "Going back to the default")[0].contains("What the fork's text says is then gone"));

    // Gone back: what is found there is what ships, which says nothing of a fork.
    rig.run(&["own", "reach/round", "--project", "shop", "--back", "--force"]).await.ok();
    let ships = rig.run(&["explain", "reach/round", "--project", "shop"]).await.ok();
    assert!(!ships.contains("Forked from") && ships.contains("This is version 2 of it."), "{ships}");
    assert!(ships.contains(" own reach/round --project shop`"), "{ships}");
    let help = rig.run(&["help", "own"]).await.ok();
    assert!(
        help.contains("The copy is a fork of the") && help.contains("What a fork was forked from is kept")
    );
}
