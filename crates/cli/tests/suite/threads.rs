//! An agent posts a file with this program's own command, and it lands
//! where the agent was asked.
//!
//! The agent here is a stand-in that does what an agent's program does
//! when it has made something for whoever asked: it runs the factory's
//! command, as the real binary, signed in as its own slot, and says no
//! thread. Asked in a thread it lands in that thread; told `--top`, in the
//! channel itself.
//!
//! Unix's alone: the daemon here is the real program as a process of its
//! own (`daemon::Daemon`), and its agent a line of a Unix shell that runs
//! the factory's command.
#![cfg(unix)]

use api::{Config, Node};
use serde_json::Value;

use crate::fleets::event_after;
use crate::{BINARY, Rig};

/// A node with one program to run an agent on. It writes a page, posts it
/// to `studio` with the command and no thread said, posts it again saying
/// the channel itself where the newest thing it was asked says "for
/// everybody", and answers.
async fn rig() -> Rig {
    let posts = format!(
        "asked=$(cat); newest=${{asked##*is for you:}}; printf '<h1>Six ways</h1>' > page.html; \
         \"{BINARY}\" post studio --attach page.html The page. >/dev/null || echo The page could not be posted.; \
         case \"$newest\" in *'for everybody'*) \
           \"{BINARY}\" post studio --top --attach page.html For everybody. >/dev/null ;; esac; \
         echo Posted."
    );
    rig_running("threads", posts).await
}

/// A node with one program to run an agent on, `poster`, which is the line of shell `posts`.
pub(crate) async fn rig_running(name: &str, posts: String) -> Rig {
    let dir = crate::scratch(name);
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
    config.adapters.push(("poster".into(), vec!["sh".into(), "-c".into(), posts]));
    let node = Node::start(config).await.unwrap();
    let rig = Rig { url: node.url(), node: Some(node), dir, order };
    rig.run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    rig
}

/// Says something in `studio` and waits for the agent to have answered it.
async fn asks(rig: &Rig, args: &[&str]) -> Value {
    let before = crate::fleets::now(rig);
    let said = rig.run(&[&["post", "studio"], args, &["--json"]].concat()).await.ok();
    event_after(rig, before, "agent.turn.finished", |event| event.data["slot"] == "maker").await;
    serde_json::from_str(&said).unwrap()
}

#[tokio::test]
async fn an_agent_asked_in_a_thread_that_posts_a_file_with_the_command_has_it_said_in_that_thread() {
    let rig = rig().await;
    rig.run(&["channel", "create", "studio"]).await.ok();
    rig.run(&["agent", "add", "maker", "--adapter", "poster", "--channel", "studio"]).await.ok();

    // Asked in the channel itself, the page is in the channel, as its answer is.
    let asked = asks(&rig, &["@maker", "three", "palettes,", "please"]).await;
    let id = asked["id"].as_str().unwrap();
    let channel = rig.run(&["read", "studio"]).await.ok();
    assert!(channel.contains("[maker] The page. [attached: page.html (att_"), "{channel}");
    assert!(channel.contains("[maker] Posted."), "{channel}");

    // Asked under that message, the page is under it, with the answer:
    // the command was told no thread, and it is where the agent was asked.
    asks(&rig, &["@maker", "three", "more", "of", "each", "--thread", id]).await;
    let thread = rig.run(&["read", "studio", "--thread", id]).await.ok();
    // What is attached is named with its id, and how it is read is said once under it all.
    let lines: Vec<&str> = thread.lines().collect();
    assert_eq!(
        [lines[0], lines[1], lines[3]],
        [
            format!("[Ada] @maker three palettes, please  (3 replies: --thread {id})").as_str(),
            "[Ada] @maker three more of each",
            "[maker] Posted.",
        ],
        "{thread}"
    );
    assert!(
        lines[2].starts_with("[maker] The page. [attached: page.html (att_") && lines[2].ends_with(")]"),
        "{thread}"
    );
    let how = format!("(what is attached is read by its id: {} attachment <id> [--out FILE])", brand::CLI);
    assert_eq!(lines[4..], [how.as_str()], "{thread}");
    let channel = rig.run(&["read", "studio"]).await.ok();
    assert_eq!(
        channel.matches("[maker] The page.").count(),
        1,
        "nothing more in the channel itself: {channel}"
    );

    // Told the channel itself, that one is said there, and the other still under the message.
    asks(&rig, &["@maker", "and", "one", "for", "everybody", "--thread", id]).await;
    let channel = rig.run(&["read", "studio"]).await.ok();
    assert!(channel.contains("[maker] For everybody. [attached: page.html (att_"), "{channel}");
    assert_eq!(channel.matches("[maker] The page.").count(), 1, "{channel}");
    let thread = rig.run(&["read", "studio", "--thread", id]).await.ok();
    assert_eq!(thread.matches("[maker] The page.").count(), 2, "{thread}");
    assert!(!thread.contains("For everybody."), "{thread}");
}

#[tokio::test]
async fn posting_says_where_it_went_and_the_channel_itself_and_a_thread_are_not_said_together() {
    let rig = Rig::signed_in().await;
    rig.run(&["channel", "create", "studio"]).await.ok();
    let first: Value =
        serde_json::from_str(&rig.run(&["post", "studio", "Morning", "all", "--json"]).await.ok()).unwrap();
    let id = first["id"].as_str().unwrap();

    // A person posts where they say: in the channel, under a message, and
    // in the channel when they say the channel itself.
    assert_eq!(rig.run(&["post", "studio", "Again"]).await.ok().trim(), "posted to #studio");
    assert_eq!(
        rig.run(&["post", "studio", "Under", "it", "--thread", id]).await.ok().trim(),
        format!("posted to #studio, in the thread under {id}")
    );
    assert_eq!(
        rig.run(&["post", "studio", "At", "the", "top", "--top"]).await.ok().trim(),
        "posted to #studio"
    );

    // Both at once is refused, and nothing is said.
    let both = rig.run(&["post", "studio", "Which?", "--top", "--thread", id]).await;
    assert_ne!(both.code, 0);
    assert!(
        both.err.contains("`--top` says it in the channel itself and `--thread` under a message"),
        "{}",
        both.err
    );
    assert!(!rig.run(&["read", "studio"]).await.ok().contains("Which?"));

    // The help says what an agent's own post does, under the command's own name.
    let help = rig.run(&["help", "post"]).await.ok();
    assert!(help.contains("[--thread <message> | --top] [--attach FILE]..."), "{help}");
    assert!(help.contains("is said in\nthat thread without being told so"), "{help}");
    assert!(help.contains("`--top` says it in\nthe channel itself"), "{help}");
}

/// What is attached to the newest message of `channel` that says `body`, as the command reads it.
async fn attached(rig: &Rig, channel: &str, body: &str) -> Value {
    let read: Value = serde_json::from_str(&rig.run(&["read", channel, "--json"]).await.ok()).unwrap();
    let messages = read["messages"].as_array().unwrap();
    let found = messages.iter().rev().find(|message| message["body"] == body);
    found.unwrap_or_else(|| panic!("`{body}` was said: {read}"))["attachments"][0].clone()
}

#[tokio::test]
async fn a_file_an_agent_writes_into_its_project_and_attaches_with_the_command_is_posted_as_that_file() {
    // It writes what it was asked for among its project's files, where it
    // works, and posts it with the command, as an agent was seen to do.
    let posts = format!(
        "cat >/dev/null; mkdir -p docs; printf '# Distribution\\n\\nBy the river.\\n' > docs/distribution.md; \
         \"{BINARY}\" post shop-talk --attach docs/distribution.md The distribution. >/dev/null \
           || echo It could not be posted.; \
         echo Posted."
    );
    let rig = rig_running("posted", posts).await;
    rig.run(&["project", "create", "Shop", "--bare"]).await.ok();
    rig.run(&["channel", "create", "shop-talk", "--project", "shop"]).await.ok();
    rig.run(&[
        "agent",
        "add",
        "writer",
        "--adapter",
        "poster",
        "--project",
        "shop",
        "--channel",
        "shop-talk",
    ])
    .await
    .ok();
    let before = crate::fleets::now(&rig);
    rig.run(&["post", "shop-talk", "@writer", "the", "distribution,", "please"]).await.ok();
    event_after(&rig, before, "agent.turn.finished", |event| event.data["slot"] == "writer").await;

    // The message names the project's file, as it is, and keeps no copy of it.
    let channel = rig.run(&["read", "shop-talk"]).await.ok();
    assert!(channel.contains("[writer] The distribution. [attached: distribution.md (att_"), "{channel}");
    let file = attached(&rig, "shop-talk", "The distribution.").await;
    assert_eq!(file["file"]["project"], "shop", "{file}");
    assert_eq!(file["file"]["path"], "docs/distribution.md", "{file}");
    assert_eq!(file["file"]["state"], "there", "{file}");
    let id = file["id"].as_str().unwrap();
    assert!(!rig.dir.join("data/attachments").join(id).exists(), "no copy of it is kept");

    // A person posting the same file is told it went as the project's file; `--copy` posts a copy.
    let folder = rig.node.as_ref().unwrap().app.studio.projects.dir_of("shop").unwrap();
    let path = folder.join("docs/distribution.md");
    let path = path.to_str().unwrap();
    let said = rig.run(&["post", "shop-talk", "--attach", path, "Again."]).await.ok();
    assert_eq!(
        said.trim(),
        "posted to #shop-talk, with the project's file `docs/distribution.md` of shop as it is and no copy"
    );
    let said = rig.run(&["post", "shop-talk", "--copy", "--attach", path, "A copy."]).await.ok();
    assert_eq!(said.trim(), "posted to #shop-talk");
    assert_eq!(attached(&rig, "shop-talk", "A copy.").await["file"], Value::Null);
}
