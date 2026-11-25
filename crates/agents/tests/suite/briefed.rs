//! A home of an agent's own, and the agent briefed in it
//! (`agents::walled`, `agents::told`): behind a wall every turn of an
//! agent whose package says which of what its program keeps is its
//! sign-in is in a home of the agent's own, its ordinary one or a kept
//! channel's, shown only that sign-in of the home of whoever runs the
//! daemon. Where the package names the file its program reads as its
//! own standing instructions, what the agent is and how it stands is
//! written there before each turn, and what it is handed with a turn is
//! what it was asked and one line more. An agent whose sign-in cannot be
//! told from the rest is shown what its program keeps there whole, as it
//! always was, and told everything ahead of what it is asked.
//!
//! The agent's program is a stand-in that writes down what it was
//! started with and handed, answers with what is in the file under its
//! home, and carries a conversation on by a handle it makes up. The wall
//! is held by a stand-in that writes down what it was told to show and
//! shows the program its home as a wall would, by telling it that its
//! home is the folder that would be shown there; and by the system's
//! own, where it holds one.
//!
//! Unix's alone: what stands in for the wall and for the agent's program
//! are files of a Unix shell.
#![cfg(unix)]

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use agents::{SlotSpec, TurnInfo, TurnRequest};
use hosting::Walls;
use kernel::{Principal, PrincipalKind};
use serde_json::json;

use crate::walls::{shows, words};
use crate::{Rig, ada, soon};

/// Something to hold walls with that writes down what it is told to
/// show, and shows the program its home as a wall would: the folder it
/// is told to show where the person's home is, which it then says is the
/// program's home. Where it writes down what it was told.
fn showing_its_home() -> (Walls, PathBuf) {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("holder-{}", &kernel::id::secret()[..12]));
    std::fs::create_dir_all(&dir).unwrap();
    let told = dir.join("told");
    let script = format!(
        "#!/bin/sh\nfor word in \"$@\"; do printf '%s\\n' \"$word\"; done > {told}.$$\nmv {told}.$$ {told}\n\
         own=\nwhile [ \"$1\" != \"--\" ]; do\n  \
         if [ \"$1\" = \"--bind\" ] && [ \"$3\" = \"$HOME\" ]; then own=$2; fi\n  shift\ndone\nshift\n\
         if [ -n \"$own\" ]; then HOME=$own; export HOME; fi\nexec \"$@\"\n",
        told = told.display()
    );
    crate::program(&dir.join("holder"), &script);
    (Walls::Held(dir.join("holder")), told)
}

/// The agent's program: it writes down what it was started with, what it
/// was handed ahead of what it was asked (`--about`) and what it was
/// asked, and answers with what is in the file under its home that its
/// package names, or that there is none, under a handle on the
/// conversation of its own making.
const PROGRAM: &str = r#"#!/bin/sh
asked=$(cat)
{ printf '%s\n' "$*" > "HERE/args"; printf '%s' "$2" > "HERE/system"; printf '%s' "$asked" > "HERE/asked"; } 2>/dev/null
tries=$(printf '%s\n' "$asked" | tail -n 1)
case "$tries" in sh:*) extra=$(sh -c "${tries#sh:}" 2>&1) ;; *) extra= ;; esac
if [ -f "$HOME/.standin/brief.md" ]; then said=$(cat "$HOME/.standin/brief.md"); else said="no brief"; fi
if [ -n "$extra" ]; then said=$(printf '%s\n%s' "$said" "$extra"); fi
said=$(printf '%s\n' "$said" | sed 's/\\/\\\\/g; s/"/\\"/g' | awk 'NR>1{printf "\\n"} {printf "%s", $0}')
printf '{"session":"s-%s","text":"%s"}\n' "$$" "$said"
"#;

const MANIFEST: &str = r#"
extension: standin
title: Stand-in
description: Not a real agent.
agent:
  program: PROGRAM
  args:
    - ["--about", "${ turn.system }"]
    - when: ${ turn.session != null }
      args: ["--resume", "${ turn.session }"]
  input: ${ turn.prompt }
  output:
    - when: ${ event.session != null }
      session: ${ event.session }
    - when: ${ event.text != null }
      reply: ${ event.text }
  wall:
WALL
"#;

/// What the stand-in's package says of its wall where its sign-in can be
/// told from the rest of what its program keeps, and the file its
/// program reads as its standing instructions.
const APART: &str = "    beside: [\"..\"]\n    keeps: [\".standin\", \".standin.json\"]\n    \
                     signin: [\".standin/credentials\"]\n    instructions: \".standin/brief.md\"\n";

/// The same without its standing instructions.
const UNBRIEFED: &str = "    beside: [\"..\"]\n    keeps: [\".standin\", \".standin.json\"]\n    \
                         signin: [\".standin/credentials\"]\n";

/// The same where its sign-in cannot be told from the rest.
const ONE_PLACE: &str = "    beside: [\"..\"]\n    keeps: [\".standin\", \".standin.json\"]\n    \
                         instructions: \".standin/brief.md\"\n";

struct Briefed {
    rig: Rig,
    here: PathBuf,
    /// The home of whoever runs the daemon, as the rig says it is.
    home: PathBuf,
    /// The agent's ordinary home of its own.
    own: PathBuf,
}

impl Briefed {
    async fn holding(walls: Walls, wall: &str) -> Briefed {
        let rig = Rig::holding(walls).await;
        let here = rig.dir.join("standin/1.0/bin");
        std::fs::create_dir_all(&here).unwrap();
        crate::program(&here.join("program"), &PROGRAM.replace("HERE", &here.to_string_lossy()));
        let home = rig.dir.join("home");
        std::fs::create_dir_all(home.join(".standin")).unwrap();
        std::fs::write(home.join(".standin/credentials"), "the person's own sign-in").unwrap();
        std::fs::write(home.join(".standin/history"), "what the person asked their own program").unwrap();
        std::fs::write(home.join(".standin.json"), "{\"the person's\": \"settings\"}").unwrap();
        let briefed = Briefed { own: rig.dir.join("slots/builder/home"), rig, here, home };
        briefed.package_says(wall).await;
        briefed.rig.agent("builder", "standin").await;
        briefed
    }

    async fn package_says(&self, wall: &str) {
        let manifest =
            MANIFEST.replace("PROGRAM", &self.here.join("program").to_string_lossy()).replace("WALL\n", wall);
        self.rig.agents.add_extension(&manifest, &ada()).await.unwrap();
    }

    fn read(&self, what: &str) -> String {
        std::fs::read_to_string(self.here.join(what)).unwrap()
    }

    /// Asks `builder` for a turn of the conversation `thread`.
    async fn asks(&self, thread: &str, prompt: &str) -> TurnInfo {
        let request = TurnRequest {
            slot: "builder".into(),
            kind: "task".into(),
            thread: thread.into(),
            prompt: prompt.into(),
            ..TurnRequest::default()
        };
        let turn = self.rig.agents.enqueue(request).await.unwrap();
        let turn = soon("the turn to end", self.rig.agents.wait(&turn)).await.unwrap();
        assert_eq!(turn.status, "done", "{:?}", turn.error);
        turn
    }

    async fn instructed(&self, instructions: &str) {
        let spec = SlotSpec {
            name: "builder".into(),
            instructions: Some(instructions.into()),
            ..SlotSpec::default()
        };
        self.rig.agents.ensure(spec, &ada()).await.unwrap();
    }

    /// What its program was handed as its handle on the conversation.
    fn resumed(&self) -> Option<String> {
        let args = self.read("args");
        args.split_once("--resume ").map(|(_, rest)| rest.trim().to_string())
    }
}

/// What every brief says, whoever it is of.
fn the_factorys(text: &str) -> bool {
    text.starts_with("You are `builder`, an agent in a ")
        && text.contains(&format!("You act on the factory with the `{}` command", brand::CLI))
}

#[tokio::test]
async fn an_agent_is_briefed_in_its_own_home_on_every_turn_and_is_handed_only_what_was_asked() {
    let (walls, told) = showing_its_home();
    let briefed = Briefed::holding(walls, APART).await;
    briefed.instructed("Write in short sentences.").await;
    let file = briefed.own.join(".standin/brief.md");

    let turn = briefed.asks("lane:shop-1", "Please look at the board.").await;
    // In the file under the home of its own: what it is, how it stands,
    // and what it was told when it was added, under a word that says it
    // is the factory's and written over.
    let written = std::fs::read_to_string(&file).unwrap();
    assert_eq!(turn.reply, written.trim_end(), "its program read what was written");
    let (head, brief) = written.split_once("\n\n").unwrap();
    assert!(head.starts_with("<!--") && head.ends_with("-->"), "{head}");
    assert!(head.contains("before each turn of `builder`") && head.contains("written over"), "{head}");
    assert!(the_factorys(brief), "{brief}");
    assert!(brief.contains("the project `shop`"), "{brief}");
    assert!(brief.contains("Your program works behind a wall"), "{brief}");
    assert!(brief.trim_end().ends_with("Write in short sentences."), "{brief}");
    // What it was handed is what it was asked, with one line ahead of it
    // that says it was asked through the factory, and none of the rest.
    assert_eq!(briefed.read("asked"), "Please look at the board.");
    let system = briefed.read("system");
    assert!(!system.contains('\n') && system.contains("standing instructions"), "{system}");
    assert!(!system.contains("You act on the factory") && !system.contains("short sentences"), "{system}");
    // Of what its program keeps in the person's home, it is shown its
    // sign-in where it is and nothing else; and nothing of the person's
    // home was written in.
    let said = words(&told);
    assert!(shows(&said, "--bind", &briefed.home.join(".standin/credentials")), "{said:#?}");
    assert!(!shows(&said, "--bind", &briefed.home.join(".standin")), "what it keeps is not shown whole");
    assert!(!shows(&said, "--bind", &briefed.home.join(".standin.json")), "{said:#?}");
    assert!(!said.iter().any(|word| word.contains(".standin/history")), "{said:#?}");
    assert!(!briefed.home.join(".standin/brief.md").exists());

    // The same again is not written again.
    let was = std::fs::metadata(&file).unwrap().ino();
    briefed.asks("lane:shop-1", "And again.").await;
    assert_eq!(std::fs::metadata(&file).unwrap().ino(), was, "written only where it differs");

    // Told otherwise, the next turn's file says so.
    briefed.instructed("Write in long sentences.").await;
    let turn = briefed.asks("lane:shop-1", "Once more.").await;
    let written = std::fs::read_to_string(&file).unwrap();
    assert!(
        written.contains("Write in long sentences.") && !written.contains("short sentences"),
        "{written}"
    );
    assert_eq!(turn.reply, written.trim_end());
    assert_eq!(briefed.read("asked"), "Once more.");
}

#[tokio::test]
async fn a_conversation_its_program_held_in_the_persons_home_is_begun_again_once_in_its_own() {
    let (walls, _) = showing_its_home();
    let briefed = Briefed::holding(walls, APART).await;
    let kept = |key: &str, value: serde_json::Value| {
        let journal = briefed.rig.journal.clone();
        let key = key.to_string();
        async move { journal.kv_set("agent.session", &key, &value).await.unwrap() }
    };
    // What a program from before kept: one held in the person's home, and
    // one held under a sign-in the factory handed, which was in a home of
    // the agent's own.
    kept("builder/lane:shop-1", json!("held-in-theirs")).await;
    kept("builder/lane:shop-2", json!({ "session": "held-handed", "signin": "pro" })).await;

    briefed.asks("lane:shop-1", "Go on.").await;
    assert_eq!(briefed.resumed(), None, "what was held in the person's home is not carried on");
    let first = briefed.rig.journal.kv_get("agent.session", "builder/lane:shop-1").await.unwrap().unwrap();
    assert_eq!(first["home"], "own", "{first}");
    // Once: the next turn carries on the one begun in its own home.
    briefed.asks("lane:shop-1", "Go on.").await;
    assert_eq!(briefed.resumed().as_deref(), first["session"].as_str());

    // One held handed a sign-in was its own already, and is only not
    // carried on under another, as it never was.
    briefed.asks("lane:shop-2", "Go on.").await;
    assert_eq!(briefed.resumed(), None, "it was held under a sign-in this one does not draw");
}

#[tokio::test]
async fn an_agent_whose_sign_in_cannot_be_told_from_the_rest_is_shown_what_it_keeps_whole_and_told_everything_as_before()
 {
    let (walls, told) = showing_its_home();
    let briefed = Briefed::holding(walls, ONE_PLACE).await;
    briefed.instructed("Write in short sentences.").await;
    briefed
        .rig
        .journal
        .kv_set("agent.session", "builder/lane:shop-1", &json!("held-in-theirs"))
        .await
        .unwrap();

    let turn = briefed.asks("lane:shop-1", "Please look at the board.").await;
    // What it keeps is shown whole, in the home that is the person's
    // there, which nothing is written in.
    let said = words(&told);
    assert!(shows(&said, "--bind", &briefed.home.join(".standin")), "{said:#?}");
    assert!(shows(&said, "--bind", &briefed.home.join(".standin.json")), "{said:#?}");
    assert!(
        !briefed.own.join(".standin/brief.md").exists() && !briefed.home.join(".standin/brief.md").exists()
    );
    assert!(turn.reply.starts_with("no brief"), "{}", turn.reply);
    // It is told everything ahead of what it is asked, as it always was,
    // and carries on what it held there.
    let system = briefed.read("system");
    assert!(the_factorys(&system) && system.trim_end().ends_with("Write in short sentences."), "{system}");
    assert_eq!(briefed.read("asked"), "Please look at the board.");
    assert_eq!(briefed.resumed().as_deref(), Some("held-in-theirs"));
}

#[tokio::test]
async fn a_brief_the_factory_wrote_that_is_no_longer_its_programs_is_taken_away_and_nothing_else_is() {
    let (walls, _) = showing_its_home();
    let briefed = Briefed::holding(walls, APART).await;
    let file = briefed.own.join(".standin/brief.md");
    briefed.asks("lane:shop-1", "Hello.").await;
    assert!(file.exists());

    // Its package names the file no longer: what the factory wrote there
    // goes, and it is told everything ahead of what it is asked.
    briefed.package_says(UNBRIEFED).await;
    let turn = briefed.asks("lane:shop-1", "Hello.").await;
    assert!(!file.exists(), "the factory's brief was taken away");
    assert!(turn.reply.starts_with("no brief"), "{}", turn.reply);
    assert!(the_factorys(&briefed.read("system")));

    // A file of that name that somebody else wrote is theirs, and stays,
    // wherever the factory would have written one and could not.
    briefed.package_says(APART).await;
    std::fs::write(&file, "What I keep for myself.\n").unwrap();
    let open = SlotSpec { name: "builder".into(), wall: Some(false), ..SlotSpec::default() };
    briefed.rig.agents.ensure(open, &ada()).await.unwrap();
    briefed.asks("lane:shop-1", "Hello.").await;
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "What I keep for myself.\n");
    assert!(the_factorys(&briefed.read("system")), "with no wall it is told everything as before");
    // And one the factory wrote where it could not write one this turn
    // goes: the agent's next turn with no wall is not briefed there.
    let wrote =
        format!("<!-- Written by {} before each turn of `builder`, once. -->\n\nOld.\n", brand::DISPLAY);
    std::fs::write(&file, wrote).unwrap();
    briefed.asks("lane:shop-1", "Hello.").await;
    assert!(!file.exists(), "a brief the factory wrote that is not this turn's was taken away");
    assert!(!briefed.home.join(".standin/brief.md").exists());
}

/// Somebody who holds what a person is given and may hand a sign-in out.
fn mira() -> Principal {
    let mut scopes: Vec<String> =
        kernel::principal::MEMBER_SCOPES.iter().map(|scope| scope.to_string()).collect();
    scopes.push("secrets:use".into());
    Principal { kind: PrincipalKind::User, id: "mira".into(), name: "mira".into(), scopes, key: None }
}

#[tokio::test]
async fn in_a_kept_channel_an_agent_is_briefed_in_that_channels_home_and_in_its_own_elsewhere() {
    let (walls, _) = showing_its_home();
    let briefed = Briefed::holding(walls, APART).await;
    briefed.instructed("Keep to the point.").await;
    let made = briefed.rig.studio.channels.create_kept("Pay review", None, "", &mira()).await.unwrap();
    let into = SlotSpec { name: "builder".into(), channel: Some(made.id.clone()), ..SlotSpec::default() };
    briefed.rig.agents.ensure(into, &mira()).await.unwrap();
    let slot = briefed.rig.dir.join("slots/builder");

    let turn = briefed.asks(&format!("channel:{}", made.id), "What is due?").await;
    let there = slot.join("kept").join(&made.id).join("home/.standin/brief.md");
    let written = std::fs::read_to_string(&there).unwrap();
    assert_eq!(turn.reply, written.trim_end(), "its program read what was written in that home");
    assert!(written.contains("Keep to the point."), "{written}");
    assert_eq!(briefed.read("asked"), "What is due?");
    assert!(!briefed.read("system").contains("You act on the factory"));

    // Asked anywhere else: its own, in the home it always has.
    let turn = briefed.asks("lane:shop-1", "And here?").await;
    let elsewhere = std::fs::read_to_string(briefed.own.join(".standin/brief.md")).unwrap();
    assert_eq!(turn.reply, elsewhere.trim_end());
    assert!(!briefed.home.join(".standin/brief.md").exists(), "the person's home is not written in");
}

#[tokio::test]
async fn no_key_an_agent_is_handed_is_in_the_file_of_its_standing_instructions() {
    const KEY: &str = "tok_0123456789abcdefghijklmnopqrstuv";
    let (walls, _) = showing_its_home();
    let briefed = Briefed::holding(walls, APART).await;
    briefed.rig.hosting.set_secret("remote-bot", KEY, &ada()).await.unwrap();
    // Handed a key, and told something that happens to hold it.
    let spec = SlotSpec {
        name: "builder".into(),
        secrets: Some(vec!["REMOTE_TOKEN=remote-bot".into()]),
        instructions: Some(format!("The remote takes {KEY} as its key.")),
        ..SlotSpec::default()
    };
    briefed.rig.agents.ensure(spec, &ada()).await.unwrap();
    let turn = briefed.asks("lane:shop-1", "Push the branch.").await;
    let written = std::fs::read_to_string(briefed.own.join(".standin/brief.md")).unwrap();
    // It says which variable holds a key, and never what it holds.
    assert!(written.contains("you are handed a key") && written.contains("`REMOTE_TOKEN`"), "{written}");
    assert!(written.contains("The remote takes [the secret `remote-bot`] as its key."), "{written}");
    assert!(!written.contains(KEY) && !turn.reply.contains(KEY));
    assert!(!briefed.read("system").contains(KEY));
}

#[tokio::test]
async fn behind_a_real_wall_its_program_finds_its_brief_and_its_sign_in_in_its_own_home_and_nothing_else_of_the_persons()
 {
    let walls = Walls::system();
    if let Err(why) = walls.can() {
        println!("skipped: this system holds no wall ({why})");
        return;
    }
    let briefed = Briefed::holding(walls, APART).await;
    briefed.instructed("Write in short sentences.").await;
    let tries = [
        "cat \"$HOME/.standin/credentials\" && echo ': read'",
        "cat \"$HOME/.standin/history\" 2>/dev/null || echo 'their history: not there'",
        "cat \"$HOME/.standin.json\" 2>/dev/null || echo 'their settings: not there'",
        "echo 'mine' > \"$HOME/.standin/history\" && echo 'its history: kept'",
    ]
    .join("; ");
    let turn = briefed.asks("lane:shop-1", &format!("Please look.\nsh:{tries}")).await;
    let written = std::fs::read_to_string(briefed.own.join(".standin/brief.md")).unwrap();
    assert!(turn.reply.starts_with(written.trim_end()), "{}", turn.reply);
    assert!(turn.reply.contains("Write in short sentences."), "{}", turn.reply);
    for expected in [
        "the person's own sign-in: read",
        "their history: not there",
        "their settings: not there",
        "its history: kept",
    ] {
        assert!(turn.reply.lines().any(|line| line.trim() == expected), "`{expected}` in:\n{}", turn.reply);
    }
    let read = |path: &Path| std::fs::read_to_string(path).unwrap_or_default();
    assert_eq!(read(&briefed.home.join(".standin/history")), "what the person asked their own program");
    assert_eq!(read(&briefed.own.join(".standin/history")), "mine\n");
    assert!(!briefed.home.join(".standin/brief.md").exists());
}

#[tokio::test]
async fn an_agent_is_told_in_its_brief_what_its_slot_lets_it_do_and_told_again_when_that_changes() {
    let (walls, _) = showing_its_home();
    let briefed = Briefed::holding(walls, APART).await;
    let file = briefed.own.join(".standin/brief.md");
    briefed.asks("lane:shop-1", "Look.").await;
    let written = std::fs::read_to_string(&file).unwrap();
    // What it may ask of the factory, by area, as the Tools panel says it,
    // and what it holds nothing of.
    assert!(
        written.contains("Of the factory you may ask, by area:\n  projects: read and change\n"),
        "{written}"
    );
    assert!(written.contains("  agents: read\n") && written.contains("  search: search the knowledge base"));
    let nothing = written.lines().find(|line| line.starts_with("And nothing of:")).unwrap();
    assert!(nothing.contains("`admin`") && nothing.contains("`shell`") && !nothing.contains("`board`"));
    // What its program may do on the device, what it signs in with, and
    // who changes any of it: never itself.
    assert!(written.contains("Your program works behind a wall."), "{written}");
    assert!(
        written.contains("It is held to `edit`") && written.contains("asks leave before it uses the web")
    );
    assert!(written.contains("It signs in to its model's provider as whoever runs the daemon."), "{written}");
    assert!(written.contains("you cannot change your own slot, even within what you hold"), "{written}");
    // And how a change it may not make is asked for, in place of more for itself.
    let asks =
        format!("ask for it rather than for more yourself: `{} ask <record> <field>=<value>...", brand::CLI);
    assert!(written.contains(&asks) && written.contains("Your own slot is not asked for so."), "{written}");

    // Given less of the factory and the web, the next turn is told so.
    let spec = SlotSpec {
        name: "builder".into(),
        scopes: Some(vec!["projects:read".into(), "board".into()]),
        web: Some(true),
        ..SlotSpec::default()
    };
    briefed.rig.agents.ensure(spec, &ada()).await.unwrap();
    briefed.asks("lane:shop-1", "Look again.").await;
    let written = std::fs::read_to_string(&file).unwrap();
    assert!(
        written.contains("by area:\n  projects: read\n  board: read and change\nAnd nothing of:"),
        "{written}"
    );
    assert!(written.contains("`channels`") && written.contains("It may use the web unasked."), "{written}");

    // Put in a group, it holds what the group holds beside its slot's, and is told which is the group's.
    let identity = &briefed.rig.studio.identity;
    identity.make_group("crew", &["search".to_string()], &ada()).await.unwrap();
    identity.join_group("crew", "agent:builder", &ada()).await.unwrap();
    briefed.asks("lane:shop-1", "Look once more.").await;
    let written = std::fs::read_to_string(&file).unwrap();
    assert!(written.contains("  search: search the knowledge base\n"), "{written}");
    assert!(written.contains("Of that you hold `search` by the groups you are in"), "{written}");
}
