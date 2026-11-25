//! A home of its own for each channel kept to its members, laid out as
//! the agent's ordinary home is (`agents::kept`, `agents::walled`): what
//! an agent's program keeps of a kept channel is there for that
//! channel's turns and for no other, with its sign-in carried; and an
//! agent whose sign-in cannot be told from what it is asked is in no
//! kept channel.
//!
//! Two ways of holding the wall, as in `walls.rs`. A stand-in that
//! writes down what it was told to show holds what the factory asks of
//! the system, wherever the tests run. And the system's own, where it
//! holds one, holds what a program then finds: what it wrote in one
//! home is not in the other, and a sign-in it renewed in one is the one
//! the other reads.
//!
//! Unix's alone: a wall is held by a program of that system's, and what
//! stands in for it and for the agent's program are files of its shell.
#![cfg(unix)]

use std::path::Path;

use agents::{SlotSpec, TurnInfo, TurnRequest};
use hosting::Walls;
use kernel::{ErrorKind, Principal, PrincipalKind};

use crate::walls::{Walled, shows, stand_in, words};
use crate::{ada, soon};

/// Somebody who holds what a person is given and may hand a sign-in out.
fn mira() -> Principal {
    let mut scopes: Vec<String> =
        kernel::principal::MEMBER_SCOPES.iter().map(|scope| scope.to_string()).collect();
    scopes.push("secrets:use".into());
    Principal { kind: PrincipalKind::User, id: "mira".into(), name: "mira".into(), scopes, key: None }
}

/// A channel kept to its members with `builder` in it: its id.
async fn kept(walled: &Walled, title: &str) -> String {
    let made = walled.rig.studio.channels.create_kept(title, None, "", &mira()).await.unwrap();
    let into = SlotSpec { name: "builder".into(), channel: Some(made.id.clone()), ..SlotSpec::default() };
    walled.rig.agents.ensure(into, &mira()).await.unwrap();
    made.id
}

/// Asks `builder` for a turn of the kept channel `kept`, or of none,
/// whose last line is what its program tries.
async fn does(walled: &Walled, kept: Option<&str>, tries: &str) -> kernel::Result<TurnInfo> {
    let request = TurnRequest {
        slot: "builder".into(),
        kind: "task".into(),
        thread: match kept {
            Some(kept) => format!("channel:{kept}"),
            None => "lane:shop-1".into(),
        },
        prompt: format!("Please see to this.\n{tries}"),
        ..TurnRequest::default()
    };
    let turn = walled.rig.agents.enqueue(request).await?;
    Ok(soon("the turn to end", walled.rig.agents.wait(&turn)).await.unwrap())
}

/// What its package says where its sign-in can be told from the rest of
/// what its program keeps: a folder of it, and a file of it.
const TELLS_ITS_SIGN_IN: &str = "  wall:\n    beside: [\"..\"]\n    keeps: [\".standin\", \".standin.json\"]\n    \
                                 signin: [\".standin/auth\", \".standin/credentials\", \
                                 \".standin/never-there\"]\n";

#[tokio::test]
async fn a_turn_of_a_kept_channel_is_shown_a_home_of_that_channels_own_with_the_sign_in_and_nothing_else_it_keeps()
 {
    let (walls, told) = stand_in();
    let walled = Walled::holding(walls).await;
    walled.package_says(TELLS_ITS_SIGN_IN).await;
    std::fs::create_dir_all(walled.home.join(".standin/auth")).unwrap();
    let kept = kept(&walled, "Pay review").await;
    let slot = walled.rig.dir.join("slots/builder");
    let home_shown = |told: &[String]| -> String {
        let at =
            told.windows(3).position(|three| three[0] == "--bind" && Path::new(&three[2]) == walled.home);
        told[at.expect("a home of its own is shown where the person's is") + 1].clone()
    };

    // Asked in the kept channel: the home that is that channel's, and of
    // what its program keeps in the person's home only its sign-in.
    let turn = does(&walled, Some(&kept), "echo done").await.unwrap();
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    let said = words(&told);
    let its_own = slot.join("kept").join(&kept).join("home");
    assert_eq!(Path::new(&home_shown(&said)), its_own, "{said:#?}");
    assert!(shows(&said, "--bind", &walled.home.join(".standin/auth")), "{said:#?}");
    assert!(shows(&said, "--bind", &walled.home.join(".standin/credentials")), "{said:#?}");
    assert!(!shows(&said, "--bind", &walled.home.join(".standin")), "what it keeps is not shown whole");
    // What its package says is its sign-in and is not there is not
    // shown, and is nothing wrong.
    assert!(!said.iter().any(|word| word.contains("never-there")), "{said:#?}");
    assert!(!shows(&said, "--bind", &walled.home.join(".standin.json")), "{said:#?}");
    assert!(!said.iter().any(|word| Path::new(word) == slot.join("home")), "nor its other home: {said:#?}");
    // It is told its home is where the person's is, as it always was.
    assert!(walled.env().lines().any(|line| line == format!("HOME={}", walled.home.display())));

    // Asked anywhere else: the home it always had, laid out the same
    // way, with its sign-in and nothing else it keeps; nothing of the
    // channel's.
    let turn = does(&walled, None, "echo done").await.unwrap();
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    let said = words(&told);
    assert_eq!(Path::new(&home_shown(&said)), slot.join("home"));
    assert!(shows(&said, "--bind", &walled.home.join(".standin/auth")), "{said:#?}");
    assert!(shows(&said, "--bind", &walled.home.join(".standin/credentials")), "{said:#?}");
    assert!(!shows(&said, "--bind", &walled.home.join(".standin")), "what it keeps is not shown whole");
    assert!(!shows(&said, "--bind", &walled.home.join(".standin.json")), "{said:#?}");
    assert!(!said.iter().any(|word| word.contains(&kept)), "nothing of the channel's is shown: {said:#?}");

    // Each kept channel has its own.
    let other = self::kept(&walled, "Hiring").await;
    does(&walled, Some(&other), "echo done").await.unwrap();
    let said = words(&told);
    assert_eq!(Path::new(&home_shown(&said)), slot.join("kept").join(&other).join("home"));
    assert!(!said.iter().any(|word| word.contains(&kept)), "{said:#?}");
}

/// Tries to bring `builder` into the kept channel `kept`.
async fn brought(walled: &Walled, kept: &str) -> kernel::Result<()> {
    let into = SlotSpec { name: "builder".into(), channel: Some(kept.to_string()), ..SlotSpec::default() };
    walled.rig.agents.ensure(into, &mira()).await.map(|_| ())
}

/// What the factory's bot last said in `channel`.
async fn noted(walled: &Walled, channel: &str) -> String {
    let said = walled.rig.studio.channels.messages(&crate::own(), channel, None, 20).await.unwrap();
    let last = said.iter().rev().find(|message| message.author.starts_with("bot:"));
    last.map(|message| message.body.clone()).unwrap_or_default()
}

/// A sign-in for the stand-in's package that the factory holds.
fn signin(name: &str, kind: &str, value: Option<&str>, folder: Option<&Path>) -> agents::NewSignIn {
    agents::NewSignIn {
        name: name.into(),
        package: "standin".into(),
        kind: kind.into(),
        value: value.map(str::to_string),
        folder: folder.map(|folder| folder.to_string_lossy().into_owned()),
        at_once: Some(1),
    }
}

#[tokio::test]
async fn an_agent_whose_sign_in_cannot_be_told_from_what_it_is_asked_is_refused_a_kept_channel_and_one_handed_a_key_has_a_home_for_each()
 {
    let (walls, told) = stand_in();
    let walled = Walled::holding(walls).await;
    let rig = &walled.rig;
    let (cli, slot) = (brand::CLI, rig.dir.join("slots/builder"));
    let made = rig.studio.channels.create_kept("Pay review", None, "", &mira()).await.unwrap();
    let kept = made.id.as_str();

    // Its package says nothing of which of what it keeps is its sign-in,
    // and it signs in as whoever runs the daemon: what it is asked is
    // kept beside its sign-in, where every agent signed in so finds it.
    // It is not brought in, and is told why and the two ways out.
    let refused = brought(&walled, kept).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    let said = &refused.message;
    for words in [
        "`builder` signs in as whoever runs the daemon, and its package `standin` does not say which",
        "an agent whose sign-in cannot be told from what it is asked is in no channel kept to its members",
        "its package says which of what it keeps is its sign-in (`agent.wall.signin`;",
        &format!("`{cli} help wall`"),
        &format!("`{cli} signin add <name> --for standin --kind key`"),
        &format!("`{cli} agent add builder --signin <name>`"),
    ] {
        assert!(said.contains(words), "`{words}` in: {said}");
    }
    assert!(!said.contains("earlier copy"), "nothing of that name came with this installation: {said}");
    assert_eq!(rig.studio.channels.kept_with("agent:builder"), Vec::<String>::new());
    // Where the package that came with the installation says it, and
    // the one installed is an earlier copy, that is said too.
    let installed = std::fs::read_to_string(rig.dir.join("extensions/standin/extension.yaml")).unwrap();
    let newer = installed.replace(
        "keeps: [\".standin\", \".standin.json\"]",
        "keeps: [\".standin\", \".standin.json\"]\n    signin: [\".standin/credentials\"]",
    );
    assert_ne!(newer, installed);
    std::fs::create_dir_all(rig.dir.join("offered/standin")).unwrap();
    std::fs::write(rig.dir.join("offered/standin/extension.yaml"), &newer).unwrap();
    let said = brought(&walled, kept).await.unwrap_err().message;
    assert!(said.contains("the one installed here is an earlier copy"), "{said}");
    assert!(said.contains(&format!("`{cli} extension update standin` takes the one that came")), "{said}");
    // Taken because somebody asked, the one that came says it and the
    // agent is brought in; and with the earlier copy installed over it
    // again the agent leaves, which is said there. What rests on what a
    // package says is asked again each time one is installed.
    let taken = rig.agents.take_what_came("standin", &ada()).await.unwrap();
    assert_eq!(taken.differs.unwrap().in_words(), "says `agent.wall.signin` besides");
    brought(&walled, kept).await.unwrap();
    assert_eq!(rig.studio.channels.kept_with("agent:builder"), [kept]);
    rig.agents.add_extension(&installed, &ada()).await.unwrap();
    assert_eq!(rig.studio.channels.kept_with("agent:builder"), Vec::<String>::new());
    let left = noted(&walled, kept).await;
    assert!(left.contains("does not say which"), "{left}");
    assert!(left.contains(&format!("`{cli} extension update standin` takes the one that came")), "{left}");
    // The daemon's own environment naming a folder for its program is
    // the same: everything it keeps is there. (Here a variable every
    // test is started with, which the package is made to name.)
    let names_a_folder = installed.replace("without: [STANDIN_PROFILE]", "without: [CARGO_MANIFEST_DIR]");
    assert_ne!(names_a_folder, installed);
    rig.agents
        .add_extension(&names_a_folder.replace("keeps:", "signin: [\".standin\"]\n    keeps:"), &ada())
        .await
        .unwrap();
    let said = brought(&walled, kept).await.unwrap_err().message;
    assert!(said.contains("the daemon was started with `CARGO_MANIFEST_DIR` naming a folder"), "{said}");
    rig.agents.add_extension(&installed, &ada()).await.unwrap();

    // Handed a key the factory holds, the sign-in is a value it is
    // handed each turn: it is brought in, has a home for each kept
    // channel, to which its program is pointed, and is asked anywhere.
    rig.agents
        .signin_add(signin("team-key", "key", Some("sk-a-value-of-the-factorys"), None), &ada())
        .await
        .unwrap();
    let draws =
        SlotSpec { name: "builder".into(), signins: Some(vec!["team-key".into()]), ..SlotSpec::default() };
    rig.agents.ensure(draws, &ada()).await.unwrap();
    brought(&walled, kept).await.unwrap();
    let turn = does(&walled, None, "echo done").await.unwrap();
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    assert!(walled.env().lines().any(|line| line == format!("STANDIN_HOME={}", slot.join("home").display())));
    let turn = does(&walled, Some(kept), "echo done").await.unwrap();
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    let its_own = slot.join("kept").join(kept).join("home");
    let env = walled.env();
    assert!(env.lines().any(|line| line == format!("STANDIN_HOME={}", its_own.display())), "{env}");
    assert!(env.lines().any(|line| line == "STANDIN_KEY=sk-a-value-of-the-factorys"), "{env}");
    let said = words(&told);
    assert!(shows(&said, "--bind", &its_own), "{said:#?}");
    assert!(!said.iter().any(|word| Path::new(word).starts_with(walled.home.join(".standin"))), "{said:#?}");
    let second = rig.studio.channels.create_kept("Hiring", None, "", &mira()).await.unwrap();
    brought(&walled, &second.id).await.unwrap();

    // A folder the factory holds is where its program keeps everything.
    // Drawing the key by its name, the agent is handed no folder, and
    // stays; told to draw whatever there is, it may be handed the
    // folder, and leaves both channels, which the factory says in each.
    let folder = rig.dir.join("a-sign-in-of-the-factorys");
    std::fs::create_dir_all(&folder).unwrap();
    rig.agents.signin_add(signin("team-folder", "folder", None, Some(&folder)), &ada()).await.unwrap();
    assert_eq!(rig.studio.channels.kept_with("agent:builder").len(), 2);
    let either = SlotSpec {
        name: "builder".into(),
        signins: Some(vec!["subscription".into(), "key".into()]),
        ..SlotSpec::default()
    };
    rig.agents.ensure(either.clone(), &ada()).await.unwrap();
    assert_eq!(rig.studio.channels.kept_with("agent:builder"), Vec::<String>::new());
    for channel in [kept, second.id.as_str()] {
        let note = noted(&walled, channel).await;
        assert!(
            note.contains("`builder` may be handed a sign-in that is a folder the factory holds"),
            "{note}"
        );
        assert!(
            note.contains("it has left this one") && note.contains("by its name, and no folder"),
            "{note}"
        );
        assert!(!slot.join("kept").join(channel).exists(), "what it kept of it went with it");
    }
    assert!(brought(&walled, kept).await.unwrap_err().message.contains("a folder the factory holds"));
    // It is asked elsewhere as it always was, and nothing in the kept one.
    assert_eq!(does(&walled, None, "echo done").await.unwrap().status, "done");
    assert!(does(&walled, Some(kept), "echo done").await.is_err());

    // And the other way round: in a channel while only a key could be
    // drawn, it leaves when a folder is added that it may be handed.
    rig.agents.signin_remove("team-folder", &ada()).await.unwrap();
    brought(&walled, kept).await.unwrap();
    rig.agents.signin_add(signin("team-folder", "folder", None, Some(&folder)), &ada()).await.unwrap();
    assert_eq!(rig.studio.channels.kept_with("agent:builder"), Vec::<String>::new());
    assert!(noted(&walled, kept).await.contains("it has left this one"));
}

#[tokio::test]
async fn an_agent_in_a_kept_channel_leaves_it_when_its_package_comes_to_say_less_of_its_sign_in() {
    let (walls, _) = stand_in();
    let walled = Walled::holding(walls).await;
    let rig = &walled.rig;
    let installed = std::fs::read_to_string(rig.dir.join("extensions/standin/extension.yaml")).unwrap();
    walled.package_says(TELLS_ITS_SIGN_IN).await;
    let kept = kept(&walled, "Pay review").await;
    assert_eq!(does(&walled, Some(&kept), "echo done").await.unwrap().status, "done");
    assert!(rig.dir.join("slots/builder/kept").join(&kept).exists());

    // The package is replaced by one that says no sign-in: its agent
    // may be in no kept channel, and is in none from then, which is
    // said there once with what would let it back.
    rig.agents.add_extension(&installed, &ada()).await.unwrap();
    assert_eq!(rig.studio.channels.kept_with("agent:builder"), Vec::<String>::new());
    let note = noted(&walled, &kept).await;
    assert!(note.contains("its package `standin` does not say which"), "{note}");
    assert!(note.contains("it has left this one") && note.contains("agent.wall.signin"), "{note}");
    assert!(!rig.dir.join("slots/builder/kept").join(&kept).exists());
    let left = rig.events("channel.left").await;
    assert_eq!(left.len(), 1, "it left once: {left:?}");
    // Taking the package again changes nothing more, and nothing more is said.
    let said = rig.studio.channels.messages(&crate::own(), &kept, None, 50).await.unwrap().len();
    rig.agents.add_extension(&installed, &ada()).await.unwrap();
    assert_eq!(rig.studio.channels.messages(&crate::own(), &kept, None, 50).await.unwrap().len(), said);

    // Its package saying it again, it is brought in again.
    walled.package_says(TELLS_ITS_SIGN_IN).await;
    brought(&walled, &kept).await.unwrap();
    assert_eq!(rig.studio.channels.kept_with("agent:builder"), std::slice::from_ref(&kept));
}

#[tokio::test]
async fn behind_a_real_wall_what_a_program_keeps_of_a_kept_channel_is_in_no_other_home_and_its_sign_in_is_in_both()
 {
    let walls = Walls::system();
    if let Err(why) = walls.can() {
        println!("skipped: this system holds no wall ({why})");
        return;
    }
    let walled = Walled::holding(walls).await;
    walled.package_says(TELLS_ITS_SIGN_IN).await;
    // The person's own sign-in: a file, and a folder with a file in it.
    std::fs::create_dir_all(walled.home.join(".standin/auth")).unwrap();
    std::fs::write(walled.home.join(".standin/auth/token"), "a token\n").unwrap();
    let kept = kept(&walled, "Pay review").await;
    let said = |turn: &TurnInfo, expected: &[&str]| {
        assert_eq!(turn.status, "done", "{:?}", turn.error);
        for expected in expected {
            assert!(
                turn.reply.lines().any(|line| line.trim() == *expected),
                "expected `{expected}` in:\n{}",
                turn.reply
            );
        }
    };

    // In the kept channel: it reads its sign-in, keeps what it was
    // asked, and renews its sign-in, the file where it is and the one in
    // the folder by putting another in its place.
    let there = [
        "cat \"$HOME/.standin/credentials\" && echo ': read there'",
        "cat \"$HOME/.standin/auth/token\"",
        "echo 'the medlar ledger' > \"$HOME/.standin/transcript\" && echo 'transcript: kept'",
        "echo 'what I made of it' > \"$HOME/notes\" && echo 'notes: kept'",
        "echo 'renewed where it is' > \"$HOME/.standin/credentials\" && echo 'the file: renewed'",
        "echo 'a newer token' > \"$HOME/.standin/auth/token.new\" && mv \"$HOME/.standin/auth/token.new\" \
         \"$HOME/.standin/auth/token\" && echo 'in the folder: replaced'",
        "ls \"$HOME/.standin.json\" 2>/dev/null || echo 'its settings: not there'",
    ]
    .join("; ");
    let turn = does(&walled, Some(&kept), &there).await.unwrap();
    let kept_there = [
        "the person's own sign-in: read there",
        "a token",
        "transcript: kept",
        "notes: kept",
        "the file: renewed",
        "in the folder: replaced",
        "its settings: not there",
    ];
    said(&turn, &kept_there);

    // Elsewhere: nothing it kept of the channel is there, and the
    // sign-in it reads is the one that was renewed.
    let elsewhere = [
        "cat \"$HOME/.standin/transcript\" 2>/dev/null || echo 'transcript: not there'",
        "cat \"$HOME/notes\" 2>/dev/null || echo 'notes: not there'",
        "cat \"$HOME/.standin/credentials\"",
        "cat \"$HOME/.standin/auth/token\"",
        "echo 'the open word' > \"$HOME/.standin/open\" && echo 'open: kept'",
        "echo 'more' > \"$HOME/open-notes\"",
    ]
    .join("; ");
    let turn = does(&walled, None, &elsewhere).await.unwrap();
    said(
        &turn,
        &["transcript: not there", "notes: not there", "renewed where it is", "a newer token", "open: kept"],
    );
    assert!(!turn.reply.contains("medlar") && !turn.reply.contains("what I made of it"), "{}", turn.reply);

    // And the other way round: in the kept channel again it finds what
    // it kept there, and nothing it kept elsewhere.
    let back = [
        "cat \"$HOME/.standin/transcript\"",
        "cat \"$HOME/notes\"",
        "cat \"$HOME/.standin/open\" 2>/dev/null || echo 'open: not there'",
        "cat \"$HOME/open-notes\" 2>/dev/null || echo 'open notes: not there'",
        // A file that is itself what is shown cannot have another put in
        // its place from behind the wall: said, where it is so.
        "(echo 'put in its place' > \"$HOME/.standin/credentials.new\" && mv \"$HOME/.standin/credentials.new\" \
         \"$HOME/.standin/credentials\") 2>/dev/null && echo 'the file: replaced' || echo 'the file: not replaced'",
    ]
    .join("; ");
    let turn = does(&walled, Some(&kept), &back).await.unwrap();
    said(&turn, &["the medlar ledger", "what I made of it", "open: not there", "open notes: not there"]);
    println!(
        "a sign-in that is one file, put in place of itself behind the wall: {}",
        turn.reply.lines().find(|line| line.starts_with("the file:")).unwrap_or("nothing said")
    );
    assert!(turn.reply.contains("the file: not replaced"), "{}", turn.reply);

    // On the device: the person's sign-in is the renewed one, what was
    // kept of the channel is in that channel's home, and nothing of it
    // is in the person's home or in the agent's other one.
    let slot = walled.rig.dir.join("slots/builder");
    let read = |path: &Path| std::fs::read_to_string(path).unwrap_or_default();
    assert_eq!(read(&walled.home.join(".standin/credentials")), "renewed where it is\n");
    assert_eq!(read(&walled.home.join(".standin/auth/token")), "a newer token\n");
    let its_own = slot.join("kept").join(&kept).join("home");
    assert_eq!(read(&its_own.join(".standin/transcript")), "the medlar ledger\n");
    assert_eq!(read(&its_own.join("notes")), "what I made of it\n");
    assert!(!walled.home.join(".standin/transcript").exists() && !walled.home.join("notes").exists());
    assert!(!slot.join("home/notes").exists() && !slot.join("home/.standin/transcript").exists());
    // What it kept in its ordinary home is that home's, as a kept
    // channel's is its own: nothing of it in the person's.
    assert_eq!(read(&slot.join("home/.standin/open")), "the open word\n");
    assert!(!walled.home.join(".standin/open").exists());
    assert_eq!(read(&slot.join("home/open-notes")), "more\n");
}

#[tokio::test]
async fn behind_a_real_wall_an_agent_that_works_in_no_project_finds_nothing_it_kept_of_a_kept_channel_where_it_works()
 {
    let walls = Walls::system();
    if let Err(why) = walls.can() {
        println!("skipped: this system holds no wall ({why})");
        return;
    }
    let walled = Walled::holding(walls).await;
    walled.package_says(TELLS_ITS_SIGN_IN).await;
    let rig = &walled.rig;
    // One that is no project's works in the folder that is its own.
    let loner = SlotSpec { name: "loner".into(), adapter: Some("standin".into()), ..SlotSpec::default() };
    rig.agents.ensure(loner, &ada()).await.unwrap();
    let made = rig.studio.channels.create_kept("Pay review", None, "", &mira()).await.unwrap();
    let into = SlotSpec { name: "loner".into(), channel: Some(made.id.clone()), ..SlotSpec::default() };
    rig.agents.ensure(into, &mira()).await.unwrap();
    let asks = |thread: String, tries: &str| TurnRequest {
        slot: "loner".into(),
        kind: "task".into(),
        thread,
        prompt: format!("Please see to this.\n{tries}"),
        ..TurnRequest::default()
    };
    let done = |request: TurnRequest| async {
        let turn = rig.agents.enqueue(request).await.unwrap();
        let turn = soon("the turn to end", rig.agents.wait(&turn)).await.unwrap();
        assert_eq!(turn.status, "done", "{:?}", turn.error);
        turn.reply
    };
    let there = "echo 'the medlar ledger' > working-notes && echo 'kept where it works'; pwd";
    let reply = done(asks(format!("channel:{}", made.id), there)).await;
    let place = rig.dir.join("slots/loner/kept").join(&made.id);
    assert!(reply.contains("kept where it works") && reply.contains(&place.to_string_lossy().into_owned()));
    let elsewhere = "cat working-notes 2>/dev/null || echo 'its notes: not there'; \
                     ls kept 2>/dev/null | grep . || echo 'of the channel: nothing'; \
                     find . -name working-notes 2>/dev/null | grep . || echo 'anywhere: nothing'";
    let reply = done(asks("lane:shop-1".into(), elsewhere)).await;
    for expected in ["its notes: not there", "of the channel: nothing", "anywhere: nothing"] {
        assert!(reply.lines().any(|line| line.trim() == expected), "expected `{expected}` in:\n{reply}");
    }
    assert_eq!(std::fs::read_to_string(place.join("working-notes")).unwrap(), "the medlar ledger\n");
}
