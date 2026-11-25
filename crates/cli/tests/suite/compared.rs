//! `compare`, from the terminal: what is counted of what comes of each
//! thing, said when one is started and set beside the rest by `show`.

use kernel::Source;
use serde_json::json;

use crate::{Rig, soon};

/// Has the work done for each thing that is wanted.
const SHOP: &str = "\
blueprint: shop
on:
  wanted:
    event: shop.wanted
    steps:
      done: { call: work, as: \"work-${ event.data.id }\", with: { item: ${ event.data.id } } }
";

/// The work: it says what it made, and how.
const WORK: &str = "\
blueprint: work
params:
  item: { type: int }
steps:
  words: { do: text.put, with: { parts: [{ from: work.md, with: { item: \"${ params.item }\" } }] } }
  spent: { do: measure, with: { name: shop.spent_usd, value: 0.25 } }
  made: { do: emit, with: { topic: shop.made, id: ${ params.item }, how: well } }
";

#[tokio::test(flavor = "multi_thread")]
async fn the_terminal_says_what_a_comparison_counts_of_each_thing_and_shows_it_under_each_way() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let blueprints = rig.placed("shop");
    std::fs::create_dir_all(&blueprints).unwrap();
    let file = |name: &str| blueprints.join(brand::blueprint_file(name));
    std::fs::write(file("shop"), SHOP).unwrap();
    std::fs::write(file("work"), WORK).unwrap();
    // What the work links, and other words for it.
    std::fs::create_dir_all(blueprints.join("prompts")).unwrap();
    std::fs::write(blueprints.join("prompts/work.md"), "Make ${item}.\n").unwrap();
    std::fs::write(blueprints.join("prompts/careful.md"), "Make ${item} with care.\n").unwrap();
    rig.run(&["apply", file("shop").to_str().unwrap(), "--project", "shop"]).await.ok();

    // What a way reads in place of a linked file is said as the way, the file and the other file.
    let unsaid = rig
        .run(&[
            "compare",
            "start",
            "x",
            "--project",
            "shop",
            "--of",
            "work",
            "--way",
            "a",
            "--prompt",
            "careful",
        ])
        .await;
    assert!(unsaid.err.contains("should be `<way>.<file>=<other file>`"), "{} {}", unsaid.out, unsaid.err);
    // A count is said as what it is called, the event, which field says the thing, and what it came to.
    let wrong = rig
        .run(&["compare", "start", "x", "--project", "shop", "--of", "work", "--way", "a", "--count", "made"])
        .await;
    assert_eq!(wrong.code, 1, "{}", wrong.out);
    assert!(
        wrong.err.contains("should be `<name>=<topic>:<field>[:<field>+<field>[:<word>+<word>]]`"),
        "{}",
        wrong.err
    );
    let started = rig
        .run(&[
            "compare",
            "start",
            "ways",
            "--project",
            "shop",
            "--of",
            "work",
            "--per",
            "item",
            "--way",
            "a",
            "--way",
            "b",
            "--prompt",
            "b.prompts/work.md=prompts/careful.md",
            "--count",
            "made=shop.made:id:how",
        ])
        .await
        .ok();
    assert!(started.contains("It counts what comes of each thing: made."), "{started}");

    let journal = &rig.node.as_ref().unwrap().app.journal;
    for id in [1, 2] {
        let mut made = journal.follow_every(journal.now(), "shop.made");
        journal.emit("shop.wanted", Source::default(), json!({ "id": id })).await.unwrap();
        soon("the work to be made", made.next()).await.unwrap();
    }
    // One screen: a column for each way with how many things it was given, what came of them with
    // how long after, and what was added up, in all and for each thing.
    let shown = soon("what came of each to be counted", async {
        loop {
            let shown = rig.run(&["compare", "show", "ways"]).await.ok();
            let made: Vec<&str> = line(&shown, "made well");
            if made.len() == 5 && made[1] == "1" && made[3] == "1" {
                return shown;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(shown.contains("it counts made\n"), "{shown}");
    // A way that reads other words where a file is linked says so, and its column says which words it ran.
    assert!(shown.contains("reads prompts/careful.md for prompts/work.md"), "{shown}");
    let read = line(&shown, "prompts/work.md");
    assert_eq!(
        (read[1], &read[2][..read[2].len() - 9]),
        ("as it is linked", "prompts/careful.md"),
        "{shown}"
    );
    assert_eq!(line(&shown, "things given")[1..], ["1", "1"], "{shown}");
    assert_eq!(line(&shown, "came of them")[1..], ["how many", "after", "how many", "after"], "{shown}");
    assert_eq!(line(&shown, "added up")[1..], ["in all", "a thing", "in all", "a thing"], "{shown}");
    assert_eq!(line(&shown, "shop.spent_usd")[1..], ["$0.25", "$0.25", "$0.25", "$0.25"], "{shown}");
    assert!(line(&shown, "step").len() == 5 && line(&shown, "run").len() == 5, "{shown}");
    assert!(shown.trim_end().ends_with("Which way did better is yours to say."), "{shown}");
    // What is not a time is written as what it is wherever it is asked for, with what it came to in all.
    let spent = rig.run(&["metrics", "shop.spent_usd", "--by", "way", "--label", "compared=ways"]).await.ok();
    assert!(line(&spent, "what").ends_with(&["in all"]), "{spent}");
    assert!(line(&spent, "way=a").ends_with(&["$0.25"]), "{spent}");

    rig.run(&["compare", "end", "ways"]).await.ok();
    // A way may be held to a version: the text that calls itself that, of those the factory has. The
    // project's own work says none, and nothing of a project's own file is kept.
    let held = ["compare", "start", "held", "--project", "shop", "--of", "work", "--way", "before=work@1"];
    let refused = rig.run(&held).await;
    assert_ne!(refused.code, 0, "{}", refused.out);
    for said in [
        "the way `before`: is `work` at version 1, and the factory has not kept that version",
        "What is there now says no version",
        "It is the project's own file here",
    ] {
        assert!(refused.err.contains(said), "{}", refused.err);
    }
    // Where a thing is a card and nothing is said, a board's cards are counted the usual way; or nothing, said so.
    let start =
        ["compare", "start", "cards", "--project", "shop", "--of", "work", "--per", "item", "--way", "a"];
    let cards = rig.run(&start).await.ok();
    assert!(
        cards.contains("It counts what comes of each thing: moved, verdict, asked, left, after."),
        "{cards}"
    );
    rig.run(&["compare", "end", "cards"]).await.ok();
    let mut bare = [&start[..], &["--count", "none"]].concat();
    bare[2] = "bare";
    let none = rig.run(&bare).await.ok();
    assert!(!none.contains("It counts"), "{none}");
}

/// The line of what was printed that begins with `what`, as its cells: those are two spaces apart or more.
pub(crate) fn line<'a>(printed: &'a str, what: &str) -> Vec<&'a str> {
    let found = printed.lines().find(|line| line.starts_with(what)).unwrap_or_default();
    found.split("  ").map(str::trim).filter(|cell| !cell.is_empty()).collect()
}

/// Starts the work for each thing that is wanted, naming the comparison
/// and the way the asking names.
const ASKS: &str = "\
blueprint: asks
on:
  wanted:
    event: shop.wanted
    steps:
      done:
        call: tuned
        as: \"tuned-${ event.data.id }\"
        with: { compared: \"${ event.data.compared }\", way: \"${ event.data.way }\" }
";

/// Work that takes the two params a start names a comparison by.
const TUNED: &str = "\
blueprint: tuned
params: { compared: '', way: '' }
steps:
  made: { do: emit, with: { topic: shop.tuned, way: \"${ params.way }\" } }
";

#[tokio::test(flavor = "multi_thread")]
async fn the_terminal_starts_a_comparison_whose_ways_are_named_by_the_start_beside_one_that_shares_them_out()
{
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let blueprints = rig.placed("shop");
    std::fs::create_dir_all(&blueprints).unwrap();
    let file = |name: &str| blueprints.join(brand::blueprint_file(name));
    std::fs::write(file("asks"), ASKS).unwrap();
    std::fs::write(file("tuned"), TUNED).unwrap();
    rig.run(&["apply", file("asks").to_str().unwrap(), "--project", "shop"]).await.ok();

    let start =
        ["compare", "start", "tones", "--project", "shop", "--of", "tuned", "--way", "a", "--way", "b"];
    let asked = rig.run(&[&start[..], &["--asked"]].concat()).await.ok();
    assert!(asked.contains("tones of tuned in shop, its ways named by the start: going"), "{asked}");
    assert!(asked.contains("whose params say `compared: tones` and `way: <one of its ways>`"), "{asked}");
    // One that shares its ways out may be in force for the same blueprint beside it.
    let shared = ["compare", "start", "shares", "--project", "shop", "--of", "tuned", "--way", "c"];
    let shares = rig.run(&shared).await.ok();
    assert!(shares.contains("each thing `tuned` is started for goes one of its ways"), "{shares}");
    let listed = rig.run(&["compare", "list", "--project", "shop"]).await.ok();
    assert!(listed.contains("tones of tuned in shop, its ways named by the start: going"), "{listed}");
    assert!(listed.contains("shares of tuned in shop: going"), "{listed}");
    // Asked for again with a way more, it is the same comparison with that way added.
    let more = rig.run(&[&start[..], &["--way", "c", "--asked"]].concat()).await.ok();
    assert!(more.contains("tones of tuned in shop, its ways named by the start"), "{more}");

    // A start that names it and its way `c` goes that way, though `shares` takes what names nothing.
    let journal = &rig.node.as_ref().unwrap().app.journal;
    let mut tuned = journal.follow_every(journal.now(), "shop.tuned");
    let asking = json!({ "id": 1, "compared": "tones", "way": "c" });
    journal.emit("shop.wanted", Source::default(), asking).await.unwrap();
    soon("the work to be done", tuned.next()).await.unwrap();
    let shown = rig.run(&["compare", "show", "tones", "--json"]).await.ok();
    let shown: serde_json::Value = serde_json::from_str(&shown).unwrap();
    let went: Vec<(String, u64)> = shown["comparison"]["ways"]
        .as_array()
        .unwrap()
        .iter()
        .map(|way| (way["name"].as_str().unwrap().to_string(), way["went"].as_u64().unwrap()))
        .collect();
    let wanted: Vec<(String, u64)> = vec![("a".into(), 0), ("b".into(), 0), ("c".into(), 1)];
    assert_eq!(went, wanted, "{shown}");
    assert_eq!(shown["comparison"]["asked"], true, "{shown}");
    // And `goes` is not said of it: each start names its way.
    let goes = rig.run(&["compare", "end", "tones", "--goes", "a"]).await;
    assert_ne!(goes.code, 0, "{}", goes.out);
    assert!(goes.err.contains("names its way"), "{}", goes.err);
}

/// Has the work done for each thing that is wanted, and says how it was made.
const MADE: &str = "\
blueprint: made
params:
  item: { type: int }
  how: { type: string, default: well }
steps:
  made: { do: emit, with: { topic: shop.made, id: ${ params.item }, how: \"${ params.how }\" } }
";

#[tokio::test(flavor = "multi_thread")]
async fn the_terminal_gives_a_comparison_a_rule_its_shares_grow_by_and_says_them_again_with_why() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let blueprints = rig.placed("shop");
    std::fs::create_dir_all(&blueprints).unwrap();
    let file = |name: &str| blueprints.join(brand::blueprint_file(name));
    std::fs::write(file("shop"), SHOP.replace("call: work", "call: made")).unwrap();
    std::fs::write(file("made"), MADE).unwrap();
    rig.run(&["apply", file("shop").to_str().unwrap(), "--project", "shop"]).await.ok();
    let start = [
        "compare",
        "start",
        "rollout",
        "--project",
        "shop",
        "--of",
        "made",
        "--per",
        "item",
        "--way",
        "usual",
        "--way",
        "new",
        "--with",
        "new.how=new",
        "--count",
        "made=shop.made:id:how",
    ];
    // A rule is said whole, or not at all.
    let half = rig.run(&[&start[..], &["--grows", "new"]].concat()).await;
    assert!(half.err.contains("a rule says each of `--grows <way> --against <way>"), "{}", half.err);
    let rule =
        ["--grows", "new", "--against", "usual", "--holds", "made=badly", "--over", "1", "--step", "25"];
    let wrong = rig.run(&[&start[..], &rule[..], &["--most", "100"]].concat()).await;
    assert!(wrong.err.contains("all of it is ending the comparison"), "{}", wrong.err);
    rig.run(&[&start[..], &rule[..], &["--most", "75"]].concat()).await.ok();

    // One thing each way, neither made badly: the new way is given three of every four from here, and `show` says why.
    let journal = &rig.node.as_ref().unwrap().app.journal;
    for id in [1, 2] {
        let mut made = journal.follow_every(journal.now(), "shop.made");
        journal.emit("shop.wanted", Source::default(), json!({ "id": id })).await.unwrap();
        soon("the work to be done", made.next()).await.unwrap();
    }
    let shown = soon("the shares to be said again", async {
        loop {
            let shown = rig.run(&["compare", "show", "rollout"]).await.ok();
            if shown.contains("Its things are shared out usual 25%, new 75%.") {
                return shown;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(
        shown.contains("by the rule: `new` came to `badly` of its `made` for 0 of the 1 things"),
        "{shown}"
    );
    assert!(shown.contains("It has taken its way as far as it goes."), "{shown}");

    // Said again by hand, with why; and the rule taken away, and given again.
    rig.run(&["compare", "share", "rollout", "usual=3", "new=1", "--why", "Not so fast."]).await.ok();
    let shown = rig.run(&["compare", "show", "rollout"]).await.ok();
    assert!(
        shown.contains("Its things are shared out usual 75%, new 25%.") && shown.contains("Not so fast."),
        "{shown}"
    );
    let bad = rig.run(&["compare", "share", "rollout", "usual"]).await;
    assert!(bad.err.contains("`usual` should be `<way>=<share>`"), "{}", bad.err);
    rig.run(&["compare", "rule", "rollout", "--none"]).await.ok();
    assert!(!rig.run(&["compare", "show", "rollout"]).await.ok().contains("Its rule:"));
    rig.run(&[&["compare", "rule", "rollout"][..], &rule[..], &["--most", "50", "--least", "10"]].concat())
        .await
        .ok();
    let shown = rig.run(&["compare", "show", "rollout"]).await.ok();
    assert!(
        shown.contains("Its rule: `new` is given 25 more of every hundred things at a time, up to 50"),
        "{shown}"
    );
    assert!(shown.contains("and goes back as much each time it did not, to 10 at least."), "{shown}");
}
