//! A change talked over with an architect, planned as cards, worked and
//! reviewed, from the terminal: the fleet that ships (`std/fleet`), whose
//! rows are changed there as they would be in the client, with stand-ins
//! for its three kinds of agent. The architect's
//! stand-in does what a real one is told to and no more: it writes the
//! plan into the project, with its tickets at the end, and runs nothing.

use std::sync::Arc;

use api::{Config, Node};
use kernel::Event;

use crate::{Rig, soon};

/// A node with three programs to run agents on.
///
/// `planner` writes a plan into the project it is run in, with two
/// tickets, the second waiting on the first, and says where it is.
/// `writer` adds a line to a file. `critic` approves.
async fn rig() -> Rig {
    let dir = crate::scratch("fleets");
    // The plan, a line at a time: its words, and then its tickets.
    let plan = [
        "# A cart",
        "",
        "The floor, then the shelves.",
        "",
        "```tickets",
        r#"[{"title": "Lay the floor", "body": "Level, and dry."},"#,
        r#" {"title": "Put the shelves up", "after": [1]}]"#,
        "```",
    ];
    let mut plans = vec!["swallow"];
    for (nth, line) in plan.iter().enumerate() {
        plans.extend([if nth == 0 { "write" } else { "append" }, "docs/plans/cart.md", line]);
    }
    plans.extend(["say", "The plan is docs/plans/cart.md: the floor, and then the shelves."]);
    let writes = vec!["swallow", "append", "CHANGES.md", "changed", "say", "I changed a file."];
    let looks = vec!["swallow", "say", "It does what the card asks.", "say", "DECISION: approve"];
    let order = std::sync::Arc::new(journal::Order::default());
    let mut config = Config {
        listen: "127.0.0.1:0".parse().unwrap(),
        device: Some("test-device".into()),
        default_adapter: Some("builtin".into()),
        walls: Some(api::Walls::Open),
        stands_between: true,
        disk: journal::Disk::Unwaited,
        order: Some(order.clone()),
        ..Config::new(dir.join("data"))
    };
    for (name, does) in [("planner", plans), ("writer", writes), ("critic", looks)] {
        config.adapters.push((name.into(), crate::standin::argv(&does)));
    }
    let node = Node::start(config).await.unwrap();
    let rig = Rig { url: node.url(), node: Some(node), dir, order };
    rig.run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    rig
}

pub(crate) async fn event(rig: &Rig, topic: &str, wanted: impl Fn(&Event) -> bool) -> Arc<Event> {
    event_after(rig, Default::default(), topic, wanted).await
}

/// Where every journal of the rig's node is now: what is said from here
/// on is after it, whichever journal keeps it. A project that is added
/// has a journal of its own, so "after" is never told by two numbers.
pub(crate) fn now(rig: &Rig) -> journal::Places {
    rig.node.as_ref().unwrap().app.journal.now()
}

/// The first event on `topic` after `after`, in whichever journal, that
/// is the one `wanted`.
pub(crate) async fn event_after(
    rig: &Rig,
    after: journal::Places,
    topic: &str,
    wanted: impl Fn(&Event) -> bool,
) -> Arc<Event> {
    // What was said, kept or passed on.
    let mut follow = rig.order.follow_from(&rig.node.as_ref().unwrap().app.journal, after, topic);
    soon(&format!("an event on `{topic}`"), async {
        loop {
            let event = follow.next().await.unwrap();
            if wanted(&event) {
                return event;
            }
        }
    })
    .await
}

/// Changes a row of one of the fleet's tables from the terminal, and
/// waits for the fleet to have taken it.
async fn set(rig: &Rig, record: &str, fields: &[&str]) {
    let before = now(rig);
    rig.run(&[&["record", "set", record], fields].concat()).await.ok();
    taken(rig, before).await;
}

/// Waits for the fleet to have taken the first change to one of its
/// tables since `before`: the arrangement asked for by that change, or
/// by a later one of its project, which read the tables after it (the
/// API suite's `taken.rs` says why any `fleet.arranged` will not do).
async fn taken(rig: &Rig, before: journal::Places) {
    let tables = ["agents", "stages", "segments", "actions"];
    let written = event_after(rig, before.clone(), "record.*", |event| {
        event.data["collection"].as_str().is_some_and(|table| tables.contains(&table))
    })
    .await;
    let number = rig.order.number(written.seq).unwrap_or(written.seq);
    let project = written.data["project"].clone();
    event_after(rig, before, "fleet.arranged", move |event| {
        let asked = event.data["asked"].as_str().and_then(|asked| asked.strip_prefix("row "));
        event.data["project"] == project && asked.and_then(|asked| asked.parse::<u64>().ok()) >= Some(number)
    })
    .await;
}

/// The fleet that ships, applied to `shop` as it is first made, and then
/// each of its three kinds of agent given its stand-in: so many who work,
/// and so many who review.
async fn fleet(rig: &Rig, more: &[&str], workers: &str, reviewers: &str) {
    let mut fleet = vec!["apply", "std/fleet", "--project", "shop", "--param", "project=shop"];
    fleet.extend(more.iter().flat_map(|param| ["--param", *param]));
    rig.run(&fleet).await.ok();
    set(rig, "shop-agents-1", &["adapter=planner"]).await;
    set(rig, "shop-agents-2", &["adapter=writer", workers]).await;
    set(rig, "shop-agents-3", &["adapter=critic", reviewers]).await;
}

/// What is said in the project's channel by the fleet itself, once it begins `begins`.
async fn told(rig: &Rig, begins: &'static str) -> String {
    let said = event(rig, "channel.message", move |event| {
        event.data["author"] == "bot:conductor"
            && event.data["body"].as_str().unwrap_or_default().starts_with(begins)
    })
    .await;
    said.data["body"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn a_change_talked_over_with_the_architect_becomes_a_plan_and_cards_that_are_worked_in_their_order() {
    let rig = rig().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    rig.run(&["apply", "std/board", "--project", "shop"]).await.ok();
    // Who is in the fleet is a table of the project: two who work, one who reviews.
    fleet(&rig, &[], "count=2", "count=1").await;
    let rows = rig.run(&["record", "list", "shop", "agents"]).await.ok();
    for row in ["shop-agents-1", "shop-agents-2", "shop-agents-3"] {
        assert!(rows.contains(row), "{rows}");
    }
    // The fleet is there, each agent in its role, and only the architect listens in the channel.
    let agents = rig.run(&["agent", "list"]).await.ok();
    for one in ["shop-architect-1", "shop-worker-1", "shop-worker-2", "shop-reviewer-1"] {
        assert!(agents.lines().any(|line| line.starts_with(one) && line.contains("active")), "{agents}");
    }

    // Said in the channel with nobody named, it is for the architect, who
    // is told how a plan is written here: where, and how its tickets are listed.
    rig.run(&["post", "shop", "We", "need", "a", "cart.", "Plan", "it."]).await.ok();
    let answered =
        event(&rig, "channel.message", |event| event.data["author"] == "agent:shop-architect-1").await;
    assert_eq!(answered.data["body"], "The plan is docs/plans/cart.md: the floor, and then the shelves.");
    let architect = rig.run(&["agent", "list", "--json"]).await.ok();
    assert!(
        architect.contains("docs/plans/<a-few-words>.md") && architect.contains("```tickets"),
        "{architect}"
    );

    // The architect wrote a file and ran nothing. Its tickets are cards
    // now, the second saying what it waits on, and nothing is begun.
    let filed = told(&rig, "docs/plans/cart.md:").await;
    assert_eq!(
        filed,
        "docs/plans/cart.md: 2 cards were added from its tickets: shop-1, shop-2. They are in backlog, \
         where nothing is done with them. Move them to ready to have them worked."
    );
    // The second may already say on the board what it waits on: its card
    // looks at what it is after once it is there, a moment after it is added.
    let board = rig.run(&["board", "shop"]).await.ok();
    let lines: Vec<&str> = board.lines().collect();
    assert!(
        lines.len() > 3
            && lines[0] == "backlog (2)"
            && lines[1] == "  shop-1  Lay the floor"
            && lines[2].starts_with("  shop-2  Put the shelves up")
            && lines[3] == "ready (0)",
        "{board}"
    );
    let second = rig.run(&["record", "show", "shop-2"]).await.ok();
    assert!(second.lines().any(|line| line.contains("after") && line.contains("shop-1")), "{second}");
    let first = rig.run(&["record", "show", "shop-1"]).await.ok();
    assert!(
        first.contains("Level, and dry.") && first.contains("From the plan docs/plans/cart.md."),
        "{first}"
    );
    // And the plan says which cards came of it, where its tickets were listed.
    let plan = std::fs::read_to_string(rig.project("shop").join("docs/plans/cart.md")).unwrap();
    assert_eq!(
        plan,
        "# A cart\n\nThe floor, then the shelves.\n\nThe tickets of this plan are on the board: shop-1, shop-2.\n"
    );

    // Told to go ahead, both at once: the first is worked, reviewed and
    // done, and only then the second, which says meanwhile what it waits on.
    rig.run(&["board", "move", "shop-2", "ready"]).await.ok();
    rig.run(&["board", "move", "shop-1", "ready"]).await.ok();
    let moved = |card: &'static str, to: &'static str| {
        event(&rig, "board.item.moved", move |event| event.data["id"] == card && event.data["to"] == to)
    };
    let first_done = moved("shop-1", "done").await.seq;
    let second_begun = moved("shop-2", "doing").await.seq;
    assert!(first_done < second_begun, "the second was begun before the first was done");
    moved("shop-2", "done").await;
    let history = rig.run(&["board", "history", "shop-2"]).await.ok();
    for line in [
        "it waits its turn: after shop-1, not done yet",
        "what it waited on is done",
        "shop-reviewer-1 approved it",
        // What the fleet did is the factory's bot's doing, with the blueprint after it.
        "conductor moved it from ready to doing · via std/lane-round",
    ] {
        assert!(history.contains(line), "{history}");
    }
    // Both were worked where the project is, one after the other; and the
    // plan, written over once, added nothing more.
    assert_eq!(
        std::fs::read_to_string(rig.project("shop").join("CHANGES.md")).unwrap(),
        "changed\nchanged\n"
    );
    let board = rig.run(&["board", "shop"]).await.ok();
    assert!(board.contains("done (2)") && !board.contains("shop-3"), "{board}");
}

#[tokio::test]
async fn a_fleet_told_to_have_planned_cards_ready_has_them_begun_without_anyone_moving_them() {
    let rig = rig().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    rig.run(&["apply", "std/board", "--project", "shop"]).await.ok();
    fleet(&rig, &["planned=ready"], "count=1", "count=1").await;

    rig.run(&["post", "shop", "Plan", "the", "cart."]).await.ok();
    let filed = told(&rig, "docs/plans/cart.md:").await;
    assert!(filed.ends_with("They are ready: each is begun when the cards it waits on are done."), "{filed}");
    // Made ready once each said what it waits on, the first is worked at
    // once and the second after it: nobody moved either.
    let done = |card: &'static str| {
        event(&rig, "board.item.moved", move |event| event.data["id"] == card && event.data["to"] == "done")
    };
    let first_done = done("shop-1").await.seq;
    let second_begun =
        event(&rig, "board.item.moved", |event| event.data["id"] == "shop-2" && event.data["to"] == "doing")
            .await
            .seq;
    assert!(first_done < second_begun, "the second was begun before the first was done");
    done("shop-2").await;
}

#[tokio::test]
async fn a_plan_a_person_writes_is_taken_as_an_architects_is_and_one_that_cannot_be_read_is_said() {
    let rig = rig().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    rig.run(&["apply", "std/board", "--project", "shop"]).await.ok();
    rig.run(&["apply", "std/fleet", "--project", "shop", "--param", "project=shop"]).await.ok();

    // Written into the project by hand, and noticed when the project is looked over.
    let plans = rig.project("shop").join("docs/plans");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::write(plans.join("till.md"), "# A till\n\n```tickets\n[{\"title\": \"Buy a till\"}]\n```\n")
        .unwrap();
    std::fs::write(plans.join("broken.md"), "# Broken\n\n```tickets\n[{\"title\": \"Half a\n```\n").unwrap();
    let node = rig.node.as_ref().unwrap();
    node.app.studio.projects.scan("shop").await.unwrap();

    let filed = told(&rig, "docs/plans/till.md:").await;
    assert!(filed.starts_with("docs/plans/till.md: 1 cards were added from its tickets: shop-1."), "{filed}");
    let unread = told(&rig, "docs/plans/broken.md:").await;
    assert!(
        unread.starts_with("docs/plans/broken.md: its tickets could not be read, and no card was added (")
            && unread.contains("this is not JSON"),
        "{unread}"
    );
    // The one that could not be read is as it was written, for whoever wrote it to put right.
    assert!(std::fs::read_to_string(plans.join("broken.md")).unwrap().contains("```tickets"));
    let board = rig.run(&["board", "shop"]).await.ok();
    assert!(board.starts_with("backlog (1)\n  shop-1  Buy a till\nready (0)"), "{board}");
}

#[tokio::test]
async fn a_projects_fleet_is_given_reviewers_and_made_a_council_by_changing_a_row_while_it_runs() {
    let rig = rig().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    rig.run(&["apply", "std/board", "--project", "shop"]).await.ok();
    let written = rig.run(&["fleet", "init", "shop"]).await.ok();
    let name = brand::blueprint_file("fleet");
    assert!(
        written.starts_with(&format!("@blueprints/{name} is kept for `shop`."))
            && written.contains("start the fleet"),
        "{written}"
    );
    let file = rig.placed("shop").join(&name);
    let path = file.to_str().unwrap();

    // As it is first made: an architect and one worker, and nobody to look work over.
    rig.run(&["apply", path, "--project", "shop"]).await.ok();
    set(&rig, "shop-agents-1", &["adapter=planner"]).await;
    set(&rig, "shop-agents-2", &["adapter=writer"]).await;
    let active = |agents: &str| -> Vec<String> {
        let mut names: Vec<String> = agents
            .lines()
            .filter(|line| line.contains(" active"))
            .map(|line| line.split_whitespace().next().unwrap().to_string())
            .collect();
        names.sort();
        names
    };
    let agents = rig.run(&["agent", "list"]).await.ok();
    assert_eq!(active(&agents), ["shop-architect-1", "shop-worker-1"], "{agents}");
    // The two tables are the project's, there to be read: who, and what they do with a card.
    let rows = rig.run(&["record", "list", "shop", "stages"]).await.ok();
    assert!(rows.contains("shop-stages-1") && rows.contains("shop-stages-2"), "{rows}");
    // A card that is worked waits in review for a person.
    rig.run(&["board", "add", "shop", "Add", "a", "cart", "--status", "ready"]).await.ok();
    event(&rig, "board.item.moved", |event| event.data["id"] == "shop-1" && event.data["to"] == "review")
        .await;
    let board = rig.run(&["board", "shop"]).await.ok();
    assert!(board.contains("review (1)\n  shop-1  Add a cart"), "{board}");
    assert!(!rig.run(&["board", "history", "shop-1"]).await.ok().contains("approved"), "nobody was asked");

    // A reviewer from now on, said in the row of those who review: the
    // card that was waiting for a person is the reviewer's to look at,
    // and is done.
    set(&rig, "shop-agents-3", &["count=1", "adapter=critic"]).await;
    let passed = event(&rig, "review.passed", |_| true).await;
    assert_eq!(
        (&passed.data["card"], &passed.data["seats"]),
        (&serde_json::json!("shop-1"), &serde_json::json!(1))
    );
    event(&rig, "board.item.moved", |event| event.data["id"] == "shop-1" && event.data["to"] == "done").await;

    // Three of them from now on: a council of three, with nothing else said.
    set(&rig, "shop-agents-3", &["count=3"]).await;
    let told = event(&rig, "review.arranged", |event| event.data["arranged"] == "std/review/3/0").await;
    assert_eq!(told.data["stage"], "review");
    let agents = rig.run(&["agent", "list"]).await.ok();
    assert_eq!(
        active(&agents),
        ["shop-architect-1", "shop-reviewer-1", "shop-reviewer-2", "shop-reviewer-3", "shop-worker-1"],
        "{agents}"
    );
    rig.run(&["board", "add", "shop", "Add", "a", "till", "--status", "ready"]).await.ok();
    let passed = event(&rig, "review.passed", |event| event.data["card"] == "shop-2").await;
    assert_eq!(passed.data["seats"], 3);
    event(&rig, "board.item.moved", |event| event.data["id"] == "shop-2" && event.data["to"] == "done").await;

    // The file applied again is the fleet it already is: the rows are as they were left.
    rig.run(&["apply", path, "--project", "shop"]).await.ok();
    let agents = rig.run(&["agent", "list"]).await.ok();
    assert_eq!(active(&agents).len(), 5, "{agents}");
}

#[tokio::test]
async fn a_plan_whose_tickets_cannot_be_added_says_why_once_and_is_left_as_it_was_written() {
    let rig = rig().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    // A fleet for a project that has no board to add cards to.
    rig.run(&["apply", "std/fleet", "--project", "shop", "--param", "project=shop"]).await.ok();
    let plans = rig.project("shop").join("docs/plans");
    std::fs::create_dir_all(&plans).unwrap();
    let written = "# A till\n\n```tickets\n[{\"title\": \"Buy a till\"}]\n```\n";
    std::fs::write(plans.join("till.md"), written).unwrap();
    rig.node.as_ref().unwrap().app.studio.projects.scan("shop").await.unwrap();

    let said = told(&rig, "docs/plans/till.md:").await;
    assert!(said.starts_with("docs/plans/till.md: its tickets could not be added as cards:"), "{said}");
    assert!(said.contains("apply `std/board` to the project again, and then save the plan again"), "{said}");
    // Said by the plan, and not a second time as something that failed in the fleet.
    event(&rig, "run.finished", |event| event.data["flow"] == "on.planned").await;
    let all = rig.run(&["read", "shop"]).await.ok();
    assert!(!all.contains("Something in the fleet failed"), "{all}");
    assert_eq!(std::fs::read_to_string(plans.join("till.md")).unwrap(), written);
}

#[tokio::test]
async fn a_card_the_assistant_cannot_work_names_the_row_to_tell_and_is_worked_through_what_the_row_then_says()
{
    let rig = rig().await;
    let cli = brand::CLI;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    rig.run(&["apply", "std/board", "--project", "shop"]).await.ok();
    // The fleet as it first comes up where nothing but the assistant that
    // is built in can run an agent: no row says what its agents run on.
    rig.run(&["apply", "std/fleet", "--project", "shop", "--param", "project=shop"]).await.ok();
    let agent =
        |agents: &str| agents.lines().find(|line| line.starts_with("shop-worker-1")).unwrap().to_string();
    let before = agent(&rig.run(&["agent", "list"]).await.ok());
    assert!(before.contains("builtin") && before.contains("active"), "{before}");

    // A card it is given is not worked, and what is said where the fleet
    // reports is the one command that gives that agent a program which
    // can: its row, by the row's own id, and the cell.
    rig.run(&["board", "add", "shop", "Add", "a", "cart", "--status", "ready"]).await.ok();
    let said = told(&rig, "Something in the fleet failed").await;
    assert!(
        said.contains("shop-worker-1 could not do it: the built-in assistant cannot do this work; "),
        "{said}"
    );
    assert!(said.contains(&format!("`{cli} record set shop-agents-2 adapter=<package>`")), "{said}");
    assert!(
        said.contains(&format!("(`{cli} help extensions`)")) && said.contains("`adapter` cell"),
        "{said}"
    );
    assert!(!said.contains("agent add"), "an agent of a fleet is not said again by hand: {said}");
    assert!(rig.run(&["board", "shop"]).await.ok().contains("doing (1)"));

    // The row is told, as that sentence says, and the agent is the one it
    // was: asked for the card again, it is asked through what the row names.
    set(&rig, "shop-agents-2", &["adapter=writer"]).await;
    let after = agent(&rig.run(&["agent", "list"]).await.ok());
    assert!(after.contains("writer") && !after.contains("builtin"), "{after}");
    let from = now(&rig);
    rig.run(&["board", "move", "shop-1", "ready"]).await.ok();
    let did = event_after(&rig, from, "card.worked", |event| event.data["card"] == "shop-1").await;
    assert_eq!(did.data["agent"], "shop-worker-1");
    let files = rig.run(&["project", "tree", "shop"]).await.ok();
    assert!(files.contains("CHANGES.md"), "what the stand-in writes was written: {files}");
}

#[tokio::test]
async fn a_card_goes_as_the_stages_row_picks_and_one_given_to_an_agent_from_the_terminal_is_that_agents() {
    let rig = rig().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    rig.run(&["apply", "std/board", "--project", "shop"]).await.ok();
    fleet(&rig, &[], "count=2", "count=0").await;
    // A third who works, a row of its own that is ranked above the
    // others, and the stage told to go by that.
    let before = now(&rig);
    rig.run(&["record", "add", "shop", "agents", "title=ace", "role=worker", "adapter=writer", "rank=3"])
        .await
        .ok();
    taken(&rig, before).await;
    set(&rig, "shop-stages-1", &["picks=rank"]).await;
    let worked = |card: &'static str| {
        let rig = &rig;
        async move {
            let did = event(rig, "card.worked", move |event| event.data["card"] == card).await;
            did.data["agent"].as_str().unwrap().to_string()
        }
    };

    // Given to nobody, a card is the highest ranked one's.
    rig.run(&["board", "add", "shop", "Add", "a", "cart", "--status", "ready"]).await.ok();
    assert_eq!(worked("shop-1").await, "shop-ace-1");

    // Given to one of the others before it is made ready, it is that one's, whatever the row says.
    rig.run(&["board", "add", "shop", "Add", "a", "till"]).await.ok();
    let given = rig.run(&["board", "give", "shop-2", "shop-worker-2"]).await.ok();
    assert_eq!(given.trim(), "shop-2 is given to agent:shop-worker-2");
    let board = rig.run(&["board", "shop"]).await.ok();
    assert!(board.contains("shop-2  Add a till  [agent:shop-worker-2]"), "{board}");
    rig.run(&["board", "move", "shop-2", "ready"]).await.ok();
    assert_eq!(worked("shop-2").await, "shop-worker-2");

    // It can be given to nobody again, and a way of choosing that is none is refused where it is said.
    assert_eq!(
        rig.run(&["board", "give", "shop-2", "nobody"]).await.ok().trim(),
        "shop-2 is given to nobody"
    );
    // A person by their handle or email is that person; a word that is nobody's is refused, saying
    // who it could be, and the card stays given to nobody.
    for word in ["ada", "ada@example.com"] {
        let given = rig.run(&["board", "give", "shop-2", word]).await.ok();
        assert!(given.trim().starts_with("shop-2 is given to user:"), "{word}: {given}");
    }
    let nobody = rig.run(&["board", "give", "shop-2", "nemo"]).await;
    assert!(nobody.code != 0, "{}", nobody.out);
    for part in [
        "`nemo` is no agent's name and no person's handle or email here",
        "shop-worker-2",
        "ada (ada@example.com)",
    ] {
        assert!(nobody.err.contains(part), "{}", nobody.err);
    }
    let refused = rig.run(&["record", "set", "shop-stages-1", "picks=random"]).await;
    assert!(
        refused.code != 0
            && refused
                .err
                .contains("`random` is not a `picks` of `stages`; there are: first, turn, least, rank"),
        "{}",
        refused.err
    );
}

#[tokio::test]
async fn a_stage_is_not_arranged_from_the_terminal_with_a_key_that_holds_less_than_an_agent_of_the_fleet() {
    let rig = rig().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    rig.run(&["apply", "std/board", "--project", "shop"]).await.ok();
    fleet(&rig, &[], "count=1", "count=0").await;
    // The owner gives those who work more than a member holds.
    set(&rig, "shop-agents-2", &["holds=board,projects,admin"]).await;
    let scopes = ["projects", "channels", "board", "records", "blueprints", "agents", "fleet", "search"];
    let key: Vec<&str> =
        ["key", "narrow"].into_iter().chain(scopes.iter().flat_map(|scope| ["--scope", *scope])).collect();
    let made = rig.run(&key).await.ok();
    let token = made.lines().next().unwrap().to_string();

    // A stage would give that agent work as whoever applied the fleet:
    // changed, or added, by one who could not write its row.
    for asked in [
        &["record", "set", "shop-stages-1", "column=draft", "--token", &token][..],
        &["record", "add", "shop", "stages", "title=worker", "--token", &token],
    ] {
        let refused = rig.run(asked).await;
        assert_ne!(refused.code, 0, "{}", refused.out);
        assert!(
            refused.err.contains(
                "a record of `stages` steers the records of `agents`, and is added or changed only by whoever \
                 could write each of those as it stands"
            ) && refused.err.contains("whose `holds` says `admin` is changed only by whoever holds `admin`"),
            "{}",
            refused.err
        );
    }
    let rows = rig.run(&["record", "list", "shop", "stages"]).await.ok();
    assert!(!rows.contains("shop-stages-3"), "{rows}");
    // The owner's it is, as it was.
    set(&rig, "shop-stages-1", &["picks=turn"]).await;
}

#[tokio::test]
async fn what_a_card_cost_is_written_by_round_with_what_review_decided_and_says_what_its_turns_did_not() {
    let rig = rig().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    rig.run(&["apply", "std/board", "--project", "shop"]).await.ok();
    fleet(&rig, &[], "count=1", "count=1").await;
    rig.run(&["board", "add", "shop", "Add", "a", "cart", "--status", "ready"]).await.ok();
    rig.run(&["board", "add", "shop", "Add", "a", "till"]).await.ok();
    event(&rig, "board.item.moved", |event| event.data["id"] == "shop-1" && event.data["to"] == "done").await;
    // What decided it is read from its history, once that is written.
    let approved = "shop-reviewer-1 approved it: It does what the card asks.";
    soon("its approval in its history", async {
        loop {
            let before = now(&rig);
            if rig.run(&["board", "history", "shop-1"]).await.ok().contains(approved) {
                break;
            }
            event_after(&rig, before, "collection.history", |_| true).await;
        }
    })
    .await;

    // One round: the worker's turn and the reviewer's, passed, and what
    // it came to; stand-ins say nothing of what they took, which is said,
    // and their cost is not reported, and never nought.
    let cost = rig.run(&["board", "cost", "shop-1"]).await.ok();
    let lines: Vec<&str> = cost.lines().collect();
    assert!(
        lines[0].starts_with("shop-1: 1 round, 2 turns, 0 tokens sent and 0 written, cost not reported, "),
        "{cost}"
    );
    assert!(lines[0].ends_with("; none of them said what it took"), "{cost}");
    assert_eq!(lines[2], format!("round 1: passed: {approved}"), "{cost}");
    assert!(lines[3].trim_start().starts_with("shop-worker-1") && lines[3].contains("works"), "{cost}");
    assert!(lines[4].trim_start().starts_with("shop-reviewer-1") && lines[4].contains("looks"), "{cost}");
    assert!(lines[4].contains("approve") && lines[4].contains("said nothing of what it took"), "{cost}");
    // With --json, what the daemon laid out.
    let laid: serde_json::Value =
        serde_json::from_str(&rig.run(&["board", "cost", "shop-1", "--json"]).await.ok()).unwrap();
    assert_eq!(
        (&laid["rounds"][0]["decided"]["came"], &laid["spent"]["turns"]),
        (&"passed".into(), &2.into())
    );
    // A card nobody has worked, and one that is no card.
    assert_eq!(
        rig.run(&["board", "cost", "shop-2"]).await.ok().trim(),
        "no agent has taken a turn on shop-2"
    );
    let missing = rig.run(&["board", "cost", "shop-9"]).await;
    assert!(missing.code != 0 && missing.err.contains("there is no card `shop-9`"), "{}", missing.err);
    // And what it is, a command away.
    let help = rig.run(&["help", "cost"]).await.ok();
    assert!(help.contains("board cost <card>") && help.contains("never scored"), "{help}");
}
