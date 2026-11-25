//! An agent behind a wall: what its program is started with, who may take
//! the wall away, and what a package says its program needs there.
//!
//! The program is a stand-in, a script that writes down what it was
//! started with and tries what a test asks it to. Most of these hold the
//! wall with a stand-in too, which writes down what it was told to show
//! and then starts the command as it is: what is asked of the system is
//! the same wherever the tests run. One holds a real wall, where the
//! system will hold one.
//!
//! Unix's alone: a wall is held by a program of that system's
//! (`bubblewrap`), and what stands in for it and for the agent's program
//! are files of its shell. Nothing holds a wall on Windows.
#![cfg(unix)]

use std::path::{Path, PathBuf};

use agents::{Extension, SlotSpec, TurnInfo, TurnRequest};
use hosting::Walls;
use kernel::{ErrorKind, Principal, PrincipalKind};

use crate::{Rig, ada, soon};

fn executable(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    crate::program(path, text);
}

/// Something to hold walls with that writes down what it is told to show
/// and starts the command as it is, and where it writes it.
pub(crate) fn stand_in() -> (Walls, PathBuf) {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("holder-{}", &kernel::id::secret()[..12]));
    let told = dir.join("told");
    let script = format!(
        "#!/bin/sh\nfor word in \"$@\"; do printf '%s\\n' \"$word\"; done > {told}.$$\nmv {told}.$$ {told}\n\
         while [ \"$1\" != \"--\" ]; do shift; done\nshift\nexec \"$@\"\n",
        told = told.display()
    );
    executable(&dir.join("holder"), &script);
    (Walls::Held(dir.join("holder")), told)
}

/// The agent's program: it writes down what it was started with, where it
/// may write there, and tries what the last line it was asked says to.
const PROGRAM: &str = r#"#!/bin/sh
if [ "$1" = models ]; then
  test -r "$HOME/.needs/thing" || { echo "cannot find what I need in $HOME/.needs" >&2; exit 4; }
  printf '{"models":[{"key":"quick"},{"key":"careful"}]}\n'
  exit 0
fi
asked=$(cat)
{ env > "HERE/env"; printf '%s\n' "$*" > "HERE/args"; } 2>/dev/null
case "$asked" in *crash*) echo "cannot find my settings" >&2; exit 3 ;; esac
said=$(sh -c "$(printf '%s\n' "$asked" | tail -n 1)" 2>&1 | sed 's/\\/\\\\/g; s/"/\\"/g' | awk 'NR>1{printf "\\n"} {printf "%s", $0}')
printf '{"text":"%s"}\n' "$said"
"#;

const MANIFEST: &str = r#"
extension: standin
title: Stand-in
description: Not a real agent.
agent:
  program: PROGRAM
  args:
    - ["--about", "${ turn.system }"]
    - ["--own", "${ turn.cli_path }"]
    - when: ${ turn.web == true }
      args: ["--looks-things-up"]
  input: ${ turn.prompt }
  output:
    - when: ${ event.text != null }
      reply: ${ event.text }
  models:
    ask:
      args: ["models"]
      answer: ${ line.models != null }
      list: ${ line.models }
      id: ${ model.key }
  signin:
    key: STANDIN_KEY
    folder: STANDIN_FOLDER
    home: STANDIN_HOME
    without: [STANDIN_PROFILE]
  wall:
    beside: [".."]
    keeps: [".standin", ".standin.json"]
"#;

/// A rig that holds walls with `walls`, an agent `builder` on the stand-in
/// in the project `shop`, and where the stand-in writes things down.
pub(crate) struct Walled {
    pub(crate) rig: Rig,
    pub(crate) here: PathBuf,
    /// The home of whoever runs the daemon, as the rig says it is.
    pub(crate) home: PathBuf,
}

impl Walled {
    pub(crate) async fn holding(walls: Walls) -> Walled {
        let rig = Rig::holding(walls).await;
        let here = rig.dir.join("standin/1.0/bin");
        executable(&here.join("program"), &PROGRAM.replace("HERE", &here.to_string_lossy()));
        let manifest = MANIFEST.replace("PROGRAM", &here.join("program").to_string_lossy());
        rig.agents.add_extension(&manifest, &ada()).await.unwrap();
        let home = rig.dir.join("home");
        std::fs::create_dir_all(home.join(".standin")).unwrap();
        std::fs::write(home.join(".standin/credentials"), "the person's own sign-in").unwrap();
        std::fs::write(home.join(".standin.json"), "{}").unwrap();
        std::fs::create_dir_all(home.join(".ssh")).unwrap();
        std::fs::write(home.join(".ssh/id_rsa"), "a private key").unwrap();
        rig.agent("builder", "standin").await;
        Walled { rig, here, home }
    }

    pub(crate) fn project(&self) -> PathBuf {
        self.rig.studio.projects.dir_of("shop").unwrap().canonicalize().unwrap()
    }

    /// Asks `slot` for a turn whose last line is what its program tries.
    pub(crate) async fn does(&self, slot: &str, tries: &str, cwd: Option<&str>) -> TurnInfo {
        let request = TurnRequest {
            slot: slot.into(),
            kind: "task".into(),
            thread: format!("a-thread-{}", &kernel::id::secret()[..6]),
            prompt: format!("Please see to this.\n{tries}"),
            cwd: cwd.map(str::to_string),
            ..TurnRequest::default()
        };
        let turn = self.rig.agents.enqueue(request).await.unwrap();
        soon("the turn to end", self.rig.agents.wait(&turn)).await.unwrap()
    }

    pub(crate) fn env(&self) -> String {
        std::fs::read_to_string(self.here.join("env")).unwrap()
    }

    /// Has the stand-in's package say `wall` of what its program needs
    /// behind one, in place of what it said: the package is taken again.
    pub(crate) async fn package_says(&self, wall: &str) {
        let usual = "  wall:\n    beside: [\"..\"]\n    keeps: [\".standin\", \".standin.json\"]\n";
        assert!(MANIFEST.contains(usual), "the stand-in's manifest says its wall as it did");
        let program = self.here.join("program");
        let manifest = MANIFEST.replace(usual, wall).replace("PROGRAM", &program.to_string_lossy());
        self.rig.agents.add_extension(&manifest, &ada()).await.unwrap();
    }
}

/// What the holder was told, a word a line; and whether it was told to
/// show `what` this way.
pub(crate) fn words(told: &Path) -> Vec<String> {
    std::fs::read_to_string(told).unwrap().lines().map(str::to_string).collect()
}

pub(crate) fn shows(told: &[String], how: &str, what: &Path) -> bool {
    let what = what.to_string_lossy();
    told.windows(3).any(|three| three[0] == how && three[2] == what)
}

fn mentions(told: &[String], what: &Path) -> bool {
    let what = what.to_string_lossy();
    told.iter().any(|word| *word == what)
}

fn member(scopes: &[&str]) -> Principal {
    Principal {
        kind: PrincipalKind::User,
        id: "grace".into(),
        name: "Grace".into(),
        scopes: scopes.iter().map(|scope| scope.to_string()).collect(),
        key: None,
    }
}

#[tokio::test]
async fn a_new_agent_has_a_wall_and_one_from_before_goes_on_as_it_was_until_somebody_says() {
    let rig = Rig::new().await;
    let made =
        rig.agents.ensure(SlotSpec { name: "builder".into(), ..SlotSpec::default() }, &ada()).await.unwrap();
    assert!(made.wall && made.reads.is_empty() && made.changes.is_empty());
    let created = rig.event("agent.slot.created", |_| true).await;
    assert_eq!(created.data["wall"], true);

    // One that was here before there were walls has none.
    rig.journal
        .db()
        .call(|c| c.execute("UPDATE slots SET wall = 0 WHERE name = 'builder'", []))
        .await
        .unwrap();
    // Said nothing of, it stays as it was, whatever else is said of it.
    let again = SlotSpec { name: "builder".into(), model: Some("quick".into()), ..SlotSpec::default() };
    assert!(!rig.agents.ensure(again, &ada()).await.unwrap().wall);
    // And has one from when somebody says so.
    let walled = SlotSpec { name: "builder".into(), wall: Some(true), ..SlotSpec::default() };
    assert!(rig.agents.ensure(walled, &ada()).await.unwrap().wall);
    assert!(rig.agents.slot("builder").await.unwrap().wall);
}

#[tokio::test]
async fn taking_an_agents_wall_away_or_showing_it_more_of_the_device_takes_shell() {
    let rig = Rig::new().await;
    // May make agents, may not run commands.
    let grace = member(&[
        "agents",
        "projects",
        "channels",
        "board",
        "records",
        "blueprints:read",
        "fleet:read",
        "search",
    ]);
    let of = |name: &str| SlotSpec { name: name.into(), ..SlotSpec::default() };

    // One with a wall, they may make.
    assert!(rig.agents.ensure(of("walled"), &grace).await.unwrap().wall);

    let open = rig.agents.ensure(SlotSpec { wall: Some(false), ..of("open") }, &grace).await.unwrap_err();
    assert_eq!(open.kind, ErrorKind::Denied);
    assert!(open.message.contains("sees this device as whoever runs the daemon does"), "{}", open.message);
    assert!(open.message.contains("takes `shell`, which Grace does not hold"), "{}", open.message);

    let more = SlotSpec { reads: Some(vec!["~/.cargo".into()]), ..of("shown") };
    let shown = rig.agents.ensure(more, &grace).await.unwrap_err();
    assert_eq!(shown.kind, ErrorKind::Denied);
    assert!(
        shown.message.contains("more of this device than its work (~/.cargo) takes `shell`"),
        "{}",
        shown.message
    );
    let more = SlotSpec { changes: Some(vec!["/srv/cache".into()]), ..of("shown") };
    assert_eq!(rig.agents.ensure(more, &grace).await.unwrap_err().kind, ErrorKind::Denied);
    assert!(rig.agents.slot("open").await.is_err() && rig.agents.slot("shown").await.is_err());

    // Whoever holds it may, and what they said is kept as they wrote it.
    let ada_says = SlotSpec {
        wall: Some(false),
        reads: Some(vec!["~/.cargo".into(), " ~/.rustup ".into(), "~/.cargo".into(), "".into()]),
        changes: Some(vec!["/srv/cache".into()]),
        ..of("open")
    };
    let made = rig.agents.ensure(ada_says, &ada()).await.unwrap();
    assert!(!made.wall);
    assert_eq!(made.reads, ["~/.cargo", "~/.rustup"]);
    assert_eq!(made.changes, ["/srv/cache"]);

    // And saying anything of such an agent is running commands by its
    // hand: it takes the same.
    let told = SlotSpec { instructions: Some("Run whatever you are sent.".into()), ..of("open") };
    assert_eq!(rig.agents.ensure(told, &grace).await.unwrap_err().kind, ErrorKind::Denied);
    // Given its wall back, and shown nothing more, it is anybody's to say of.
    let back = SlotSpec { wall: Some(true), reads: Some(vec![]), changes: Some(vec![]), ..of("open") };
    let back = rig.agents.ensure(back, &grace).await.unwrap();
    assert!(back.wall && back.reads.is_empty() && back.changes.is_empty());
}

#[tokio::test]
async fn letting_an_agent_use_the_web_unasked_takes_shell_and_is_off_for_one_from_before_and_for_a_new_one() {
    let (walls, _told) = stand_in();
    let walled = Walled::holding(walls).await;
    let rig = &walled.rig;
    // May make agents, may not run commands.
    let holds: Vec<&str> = kernel::principal::AGENT_SCOPES.iter().copied().chain(["agents"]).collect();
    assert!(!holds.contains(&"shell"));
    let grace = member(&holds);
    let of = |name: &str| SlotSpec { name: name.into(), ..SlotSpec::default() };

    // A new one may not, and neither may one that was here before the
    // word was: nothing was said of either.
    assert!(!rig.agents.ensure(of("fresh"), &grace).await.unwrap().web);
    assert!(!rig.agents.slot("builder").await.unwrap().web);
    let model = SlotSpec { model: Some("quick".into()), ..of("builder") };
    assert!(!rig.agents.ensure(model, &ada()).await.unwrap().web, "said nothing of, it stays off");

    // Turning it on is not theirs who may not run commands here, for a
    // new agent or for one that is there; and nothing was changed.
    for name in ["looker", "fresh"] {
        let refused = rig.agents.ensure(SlotSpec { web: Some(true), ..of(name) }, &grace).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Denied);
        assert!(
            refused.message.contains("turning that on takes `shell`, which Grace does not hold"),
            "{}",
            refused.message
        );
    }
    assert!(rig.agents.slot("looker").await.is_err() && !rig.agents.slot("fresh").await.unwrap().web);

    // Whoever holds it may; the turn says so to the package, and its
    // program is started with what the package says that is.
    assert!(rig.agents.ensure(SlotSpec { web: Some(true), ..of("builder") }, &ada()).await.unwrap().web);
    let turn = walled.does("builder", "echo done", None).await;
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    let about = std::fs::read_to_string(walled.here.join("args")).unwrap();
    assert!(about.contains("--looks-things-up"), "{about}");

    // Saying something else of one that may is not turning anything on,
    // and turning it off takes no more than changing the agent.
    let told = SlotSpec { instructions: Some("Mind the till.".into()), ..of("builder") };
    assert!(rig.agents.ensure(told, &grace).await.unwrap().web);
    assert!(!rig.agents.ensure(SlotSpec { web: Some(false), ..of("builder") }, &grace).await.unwrap().web);
    let turn = walled.does("builder", "echo done", None).await;
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    let about = std::fs::read_to_string(walled.here.join("args")).unwrap();
    assert!(!about.contains("--looks-things-up"), "{about}");
}

#[tokio::test]
async fn what_an_agent_is_shown_is_written_from_the_top_or_from_the_home_and_never_climbs() {
    let rig = Rig::new().await;
    for wrong in ["cache", "./cache", "~other/.cargo", "/srv/../etc", "~/../elsewhere"] {
        let spec =
            SlotSpec { name: "builder".into(), reads: Some(vec![wrong.into()]), ..SlotSpec::default() };
        let refused = rig.agents.ensure(spec, &ada()).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Invalid, "{wrong}");
        assert!(
            refused.message.contains(&format!("`{wrong}` cannot be among what an agent may read")),
            "{}",
            refused.message
        );
        assert!(
            refused.message.contains("written from the top (`/opt/tools`) or from the home"),
            "{}",
            refused.message
        );
    }
    let spec =
        SlotSpec { name: "builder".into(), changes: Some(vec!["cache".into()]), ..SlotSpec::default() };
    let refused = rig.agents.ensure(spec, &ada()).await.unwrap_err();
    assert!(refused.message.contains("may change behind its wall"), "{}", refused.message);
    assert!(rig.agents.slot("builder").await.is_err());
}

#[test]
fn a_package_says_what_its_program_needs_behind_a_wall_and_is_told_what_is_wrong_with_how_it_said_it() {
    let said = Extension::load(&MANIFEST.replace("PROGRAM", "standin")).unwrap();
    assert_eq!(said.wall.beside, [".."]);
    assert_eq!(said.wall.keeps, [".standin", ".standin.json"]);
    // One that says nothing needs nothing more than any program does.
    let plain = MANIFEST.replace("PROGRAM", "standin");
    let plain = plain.split("  wall:").next().unwrap();
    assert_eq!(Extension::load(plain).unwrap().wall, Default::default());

    let wrong = |wall: &str| -> String {
        let manifest = format!("{plain}  wall:\n{wall}");
        Extension::load(&manifest).err().expect("a manifest that is wrong").join("; ")
    };
    let said = wrong("    keeps: [\"/etc\"]\n");
    assert!(
        said.contains("agent.wall.keeps")
            && said.contains("`/etc` is not a folder or file written from the home"),
        "{said}"
    );
    let said = wrong("    keeps: [\"../elsewhere\"]\n");
    assert!(said.contains("`../elsewhere` is not"), "{said}");
    let said = wrong("    beside: [\"/usr\"]\n");
    assert!(
        said.contains("agent.wall.beside") && said.contains("written from where the program's file is"),
        "{said}"
    );
    let said = wrong("    keeps: .standin\n");
    assert!(said.contains("agent.wall.keeps") && said.contains("must be a list"), "{said}");
    let said = wrong("    shows: [\".standin\"]\n");
    assert!(said.contains("has no `shows`; there are: beside, keeps, signin, instructions"), "{said}");

    // Which of what it keeps is its sign-in: one of them, or within one.
    let told = format!("{plain}  wall:\n    keeps: [\".standin\", \".standin.json\"]\n");
    let within = format!("{told}    signin: [\".standin/auth\", \".standin.json\"]\n");
    assert_eq!(Extension::load(&within).unwrap().wall.signin, [".standin/auth", ".standin.json"]);
    assert!(Extension::load(&told).unwrap().wall.signin.is_empty(), "one that says none has none");
    let said = wrong("    keeps: [\".standin\"]\n    signin: [\".elsewhere/auth\", \".standin-two\"]\n");
    assert!(
        said.contains("agent.wall.signin") && said.contains("`.elsewhere/auth` is not one of `keeps`"),
        "{said}"
    );
    assert!(
        said.contains("`.standin-two` is not one of `keeps`"),
        "a name that only begins as one does: {said}"
    );
    let said = wrong("    keeps: [\".standin\"]\n    signin: [\".standin/../.ssh\"]\n");
    assert!(
        said.contains("agent.wall.signin") && said.contains("`.standin/../.ssh` is not a folder or file"),
        "{said}"
    );
    let said = wrong("    keeps: [\".standin\"]\n    signin: [\"/etc/passwd\"]\n");
    assert!(
        said.contains("agent.wall.signin") && said.contains("`/etc/passwd` is not a folder or file"),
        "{said}"
    );

    // The file its program reads as its standing instructions: one,
    // written from the home, and none of its sign-in.
    let named = format!("{told}    instructions: \".standin/brief.md\"\n");
    assert_eq!(Extension::load(&named).unwrap().wall.instructions.as_deref(), Some(".standin/brief.md"));
    assert_eq!(Extension::load(&told).unwrap().wall.instructions, None, "one that says none has none");
    for wrongly in ["\"/etc/motd\"", "\"../brief.md\"", "\"\"", "[\".standin/brief.md\"]"] {
        let said = wrong(&format!("    keeps: [\".standin\"]\n    instructions: {wrongly}\n"));
        assert!(
            said.contains("agent.wall.instructions") && said.contains("is not a file written from the home"),
            "{wrongly}: {said}"
        );
    }
    let said = wrong(
        "    keeps: [\".standin\"]\n    signin: [\".standin/auth\"]\n    instructions: \".standin/auth/brief.md\"\n",
    );
    assert!(said.contains("agent.wall.instructions") && said.contains("is of the sign-in"), "{said}");
}

#[tokio::test]
async fn an_agents_program_is_started_behind_a_wall_that_shows_its_work_and_what_is_its_own() {
    let (walls, told) = stand_in();
    let walled = Walled::holding(walls).await;
    let (rig, project) = (&walled.rig, walled.project());
    // Shown a toolchain to read and a cache to change, by whoever holds `shell`.
    std::fs::create_dir_all(walled.home.join(".toolchain")).unwrap();
    std::fs::write(walled.home.join(".toolchain/credentials.toml"), "a token").unwrap();
    std::fs::create_dir_all(rig.dir.join("cache")).unwrap();
    let more = SlotSpec {
        name: "builder".into(),
        reads: Some(vec!["~/.toolchain".into(), "~/.nothing-there".into()]),
        changes: Some(vec![rig.dir.join("cache").to_string_lossy().into_owned()]),
        ..SlotSpec::default()
    };
    rig.agents.ensure(more, &ada()).await.unwrap();

    let turn = walled.does("builder", "echo done", None).await;
    assert_eq!((turn.status.as_str(), turn.reply.as_str()), ("done", "done"), "{:?}", turn.error);
    let told = words(&told);
    let slot = rig.dir.join("slots/builder");

    // Its work to change, and the repository's own folder only to read.
    assert!(shows(&told, "--bind", &project), "{told:#?}");
    let repository = project.join(".git");
    match repository.join("HEAD").exists() {
        true => assert!(shows(&told, "--ro-bind", &repository), "{told:#?}"),
        // Where there is none, none can be made.
        false => {
            assert!(told.windows(2).any(|two| two[0] == "--remount-ro" && Path::new(&two[1]) == repository))
        }
    }
    // A home of its own where the person's is, and at its own place.
    assert!(told.windows(3).any(|three| {
        three[0] == "--bind"
            && Path::new(&three[1]) == slot.join("home")
            && Path::new(&three[2]) == walled.home
    }));
    assert!(shows(&told, "--bind", &slot.join("home")));
    // Who it is to the factory, and the program it asks with, to read.
    assert!(shows(&told, "--ro-bind", &slot.join("token")), "{told:#?}");
    assert!(shows(&told, "--ro-bind", &rig.dir.join("bin")) || !rig.dir.join("bin").exists());
    // Its own program, and what its package says is beside it.
    assert!(shows(&told, "--ro-bind", &walled.here.join("program")), "{told:#?}");
    assert!(shows(&told, "--ro-bind", &rig.dir.join("standin/1.0")), "{told:#?}");
    // Signing in as whoever runs the daemon: what its package says it keeps.
    assert!(shows(&told, "--bind", &walled.home.join(".standin")), "{told:#?}");
    assert!(shows(&told, "--bind", &walled.home.join(".standin.json")));
    // What its slot added, less what is a credential by its name.
    assert!(shows(&told, "--ro-bind", &walled.home.join(".toolchain")));
    assert!(told.windows(3).any(|three| {
        three[0] == "--ro-bind"
            && three[1] == "/dev/null"
            && Path::new(&three[2]) == walled.home.join(".toolchain/credentials.toml")
    }));
    assert!(shows(&told, "--bind", &rig.dir.join("cache")));
    assert!(!told.iter().any(|word| word.contains("nothing-there")));
    // And nothing else of the home, or of the factory's own.
    assert!(!mentions(&told, &walled.home.join(".ssh")));
    for whole in [&rig.dir, &rig.dir.join("slots"), &rig.dir.join("projects")] {
        assert!(!mentions(&told, whole), "{} is shown whole: {told:#?}", whole.display());
    }
    // It is started where it works.
    let at = told.iter().position(|word| word == "--chdir").unwrap();
    assert_eq!(Path::new(&told[at + 1]), project);

    // It inherits nothing of the daemon's but what any program needs, and
    // is told what a turn is told.
    let env = walled.env();
    let has = |name: &str| env.lines().any(|line| line.starts_with(&format!("{name}=")));
    assert!(has("PATH") && has(&brand::env_var("URL")) && has(&brand::env_var("TOKEN_FILE")), "{env}");
    assert!(env.lines().any(|line| line == format!("HOME={}", walled.home.display())), "{env}");
    // Something cargo gives every test, which a program in the open is handed on.
    assert!(std::env::var("CARGO_MANIFEST_DIR").is_ok() && !has("CARGO_MANIFEST_DIR"), "{env}");
    // And it is told that it works behind one.
    let about = std::fs::read_to_string(walled.here.join("args")).unwrap();
    assert!(about.contains("Your program works behind a wall") && about.contains("do not commit"), "{about}");
    // What its package is told is the factory's own command is in the
    // folder it is shown to read, which is where its `PATH` begins: the
    // path it is told is the one its program finds.
    let own = rig.dir.join("bin").join(brand::CLI);
    assert!(about.contains(&format!("--own {}", own.display())), "{about}");
    let path = env.lines().find_map(|line| line.strip_prefix("PATH=")).unwrap();
    assert_eq!(path.split(':').next(), own.parent().map(|bin| bin.to_str().unwrap()), "{path}");
}

#[tokio::test]
async fn an_agent_that_draws_a_sign_in_the_factory_holds_is_not_shown_the_one_in_the_home() {
    let (walls, told) = stand_in();
    let walled = Walled::holding(walls).await;
    let rig = &walled.rig;
    let folder = rig.dir.join("agents-own-sign-in");
    std::fs::create_dir_all(&folder).unwrap();
    let new = agents::NewSignIn {
        name: "team".into(),
        package: "standin".into(),
        kind: "folder".into(),
        value: None,
        folder: Some(folder.to_string_lossy().into_owned()),
        at_once: Some(1),
    };
    rig.agents.signin_add(new, &ada()).await.unwrap();
    let draws =
        SlotSpec { name: "builder".into(), signins: Some(vec!["team".into()]), ..SlotSpec::default() };
    rig.agents.ensure(draws, &ada()).await.unwrap();

    let turn = walled.does("builder", "echo done", None).await;
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    let told = words(&told);
    // The folder it was handed, to change: its program renews it there.
    assert!(shows(&told, "--bind", &folder.canonicalize().unwrap()), "{told:#?}");
    // And nothing of what whoever runs the daemon signs in with.
    assert!(!mentions(&told, &walled.home.join(".standin")), "{told:#?}");
    assert!(!mentions(&told, &walled.home.join(".standin.json")));
    assert!(walled.env().lines().any(|line| line == format!("STANDIN_FOLDER={}", folder.display())));
}

#[tokio::test]
async fn a_cards_worktree_is_its_to_change_and_the_rest_of_the_project_to_read_and_one_held_to_reading_changes_neither()
 {
    let (walls, told) = stand_in();
    let walled = Walled::holding(walls).await;
    let (rig, project) = (&walled.rig, walled.project());
    let lane = project.join(".lanes/shop-1");
    std::fs::create_dir_all(&lane).unwrap();
    std::fs::write(lane.join(".git"), "gitdir: elsewhere").unwrap();

    let turn = walled.does("builder", "echo done", Some(".lanes/shop-1")).await;
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    let said = words(&told);
    assert!(shows(&said, "--ro-bind", &project), "{said:#?}");
    assert!(shows(&said, "--bind", &lane), "{said:#?}");
    // What says where its repository is cannot be put another way.
    assert!(shows(&said, "--ro-bind", &lane.join(".git")), "{said:#?}");
    assert!(!shows(&said, "--bind", &project));
    let at = said.iter().position(|word| word == "--chdir").unwrap();
    assert_eq!(Path::new(&said[at + 1]), lane);

    // In the project itself, the folders other cards are worked in are theirs.
    walled.does("builder", "echo done", None).await;
    let said = words(&told);
    assert!(
        shows(&said, "--bind", &project) && shows(&said, "--ro-bind", &project.join(".lanes")),
        "{said:#?}"
    );

    // Held to reading, it is shown its work to read: `read` is a wall and
    // not only a word its program is asked to keep.
    let reads =
        SlotSpec { name: "builder".into(), permission_mode: Some("read".into()), ..SlotSpec::default() };
    rig.agents.ensure(reads, &ada()).await.unwrap();
    walled.does("builder", "echo done", Some(".lanes/shop-1")).await;
    let said = words(&told);
    assert!(shows(&said, "--ro-bind", &lane) && shows(&said, "--ro-bind", &project), "{said:#?}");
    assert!(!shows(&said, "--bind", &lane) && !shows(&said, "--bind", &project));
}

#[tokio::test]
async fn an_agent_whose_wall_cannot_be_held_is_not_started_and_one_told_to_work_with_none_is() {
    let nowhere = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("no-such-holder");
    let walled = Walled::holding(Walls::Held(nowhere)).await;
    let rig = &walled.rig;

    let turn = walled.does("builder", "echo started > started; echo done", None).await;
    assert_eq!(turn.status, "failed");
    let why = turn.error.unwrap();
    assert!(why.contains("`builder` works behind a wall, and none can be held here"), "{why}");
    assert!(why.contains("bubblewrap") && why.contains("Nothing was started"), "{why}");
    assert!(why.contains(&format!("`{} agent add builder --no-wall`", brand::CLI)), "{why}");
    // Never the open device instead.
    assert!(!walled.project().join("started").exists());
    assert!(!walled.here.join("env").exists());

    // Told to work with none, it is started as everything was before
    // there were walls: seeing the device, and handed on what the daemon has.
    let open = SlotSpec { name: "builder".into(), wall: Some(false), ..SlotSpec::default() };
    rig.agents.ensure(open, &ada()).await.unwrap();
    let turn = walled.does("builder", "echo started > started; echo done", None).await;
    assert_eq!((turn.status.as_str(), turn.reply.as_str()), ("done", "done"), "{:?}", turn.error);
    assert!(walled.project().join("started").exists());
    assert!(walled.env().lines().any(|line| line.starts_with("CARGO_MANIFEST_DIR=")));
    let about = std::fs::read_to_string(walled.here.join("args")).unwrap();
    assert!(!about.contains("behind a wall"), "{about}");
}

#[tokio::test]
async fn a_program_that_fails_behind_a_wall_is_said_to_have_been_behind_one() {
    let (walls, _) = stand_in();
    let walled = Walled::holding(walls).await;
    let turn = walled.does("builder", "crash", None).await;
    assert_eq!(turn.status, "failed");
    let why = turn.error.unwrap();
    assert!(why.contains("cannot find my settings"), "{why}");
    assert!(why.contains("Its program works behind a wall"), "{why}");
    assert!(why.contains(&format!("`{} help wall` says how to show it", brand::CLI)), "{why}");
}

#[tokio::test]
async fn behind_a_real_wall_an_agent_acts_on_its_work_and_finds_nothing_of_the_factorys_or_the_persons() {
    let walls = Walls::system();
    if let Err(why) = walls.can() {
        println!("skipped: this system holds no wall ({why})");
        return;
    }
    let walled = Walled::holding(walls).await;
    let (rig, project) = (&walled.rig, walled.project());
    // Another agent, whose credential is its own; and the factory's own secret.
    rig.agents.ensure(SlotSpec { name: "other".into(), ..SlotSpec::default() }, &ada()).await.unwrap();
    let other = rig.dir.join("slots/other/token");
    assert!(other.exists() && rig.dir.join("installation.secret").exists());
    let at = |path: &Path| path.display().to_string();
    let tries = [
        "echo made > made-here && echo 'its work: changed'".to_string(),
        format!("test -s \"${}\" && echo 'its credential: there'", brand::env_var("TOKEN_FILE")),
        format!("cat {} && echo ': read'", at(&walled.home.join(".standin/credentials"))),
        "echo more > \"$HOME/kept\" && echo 'its home: changed'".to_string(),
        format!("cat {} 2>/dev/null || echo 'another agent: not there'", at(&other)),
        format!(
            "cat {} 2>/dev/null || echo 'the secret: not there'",
            at(&rig.dir.join("installation.secret"))
        ),
        format!("cat {} 2>/dev/null || echo 'the key: not there'", at(&walled.home.join(".ssh/id_rsa"))),
        format!(
            "ls {} 2>/dev/null | grep -v '^shop$' | grep . || echo 'other projects: not there'",
            at(project.parent().unwrap())
        ),
        format!("echo x 2>/dev/null >> {}/.git/config || echo 'the repository: not changed'", at(&project)),
    ]
    .join("; ");
    let turn = walled.does("builder", &tries, None).await;
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    for expected in [
        "its work: changed",
        "its credential: there",
        "the person's own sign-in: read",
        "its home: changed",
        "another agent: not there",
        "the secret: not there",
        "the key: not there",
        "other projects: not there",
        "the repository: not changed",
    ] {
        assert!(
            turn.reply.lines().any(|line| line.trim() == expected),
            "expected `{expected}` in:\n{}",
            turn.reply
        );
    }
    assert_eq!(std::fs::read_to_string(project.join("made-here")).unwrap(), "made\n");
    // Its home is a folder of its own, and the person's was not written in.
    assert_eq!(std::fs::read_to_string(rig.dir.join("slots/builder/home/kept")).unwrap(), "more\n");
    assert!(!walled.home.join("kept").exists());
}

#[tokio::test]
async fn an_agents_program_is_tried_as_a_turn_would_start_it_without_being_given_any_work() {
    let (walls, told) = stand_in();
    let walled = Walled::holding(walls).await;
    let rig = &walled.rig;
    // Something its program cannot do without, which the stand-in for a wall does not keep from it.
    std::fs::create_dir_all(walled.home.join(".needs")).unwrap();
    std::fs::write(walled.home.join(".needs/thing"), "what it needs").unwrap();

    let tried = rig.agents.try_program("builder", &ada()).await.unwrap();
    assert!(tried.ok && tried.walled, "{tried:?}");
    assert_eq!((tried.models, tried.error.as_deref()), (2, None));
    assert_eq!(tried.program, walled.here.join("program").to_string_lossy());
    // Started behind the wall a turn of its is started behind, and where it works.
    let said = words(&told);
    assert!(shows(&said, "--bind", &walled.project()), "{said:#?}");
    assert!(shows(&said, "--ro-bind", &rig.dir.join("slots/builder/token")));
    // No work was given: there is no turn, and nothing was said in its channel.
    assert!(rig.events("agent.turn.**").await.is_empty());
    assert!(!walled.here.join("env").exists());

    // With no wall it is tried with none, and said to have been.
    let open = SlotSpec { name: "builder".into(), wall: Some(false), ..SlotSpec::default() };
    rig.agents.ensure(open, &ada()).await.unwrap();
    let tried = rig.agents.try_program("builder", &ada()).await.unwrap();
    assert!(!tried.walled, "{tried:?}");
    // In the open its home is that of whoever runs these tests, where
    // nothing of a test's is: what it found missing there is what is said.
    assert!(!tried.ok && tried.error.unwrap().contains("cannot find what I need"));

    // What cannot be tried this way says why, and what to do instead.
    rig.agents
        .ensure(
            SlotSpec { name: "plain".into(), adapter: Some("fake".into()), ..SlotSpec::default() },
            &ada(),
        )
        .await
        .unwrap();
    let not_a_program = rig.agents.try_program("plain", &ada()).await.unwrap_err();
    assert_eq!(not_a_program.kind, ErrorKind::Invalid);
    assert!(not_a_program.message.contains("is not run by a package's program"), "{}", not_a_program.message);
    let nobody = rig.agents.try_program("nobody", &ada()).await.unwrap_err();
    assert_eq!(nobody.kind, ErrorKind::NotFound);
    let reader = member(&["agents:read"]);
    assert_eq!(rig.agents.try_program("builder", &reader).await.unwrap_err().kind, ErrorKind::Denied);
}

#[tokio::test]
async fn a_program_that_lacks_something_behind_its_wall_says_so_when_tried_and_answers_once_it_is_shown_it() {
    let walls = Walls::system();
    if let Err(why) = walls.can() {
        println!("skipped: this system holds no wall ({why})");
        return;
    }
    let walled = Walled::holding(walls).await;
    let rig = &walled.rig;
    // It is in the home of whoever runs the daemon, where a program in the open finds it.
    std::fs::create_dir_all(walled.home.join(".needs")).unwrap();
    std::fs::write(walled.home.join(".needs/thing"), "what it needs").unwrap();

    let tried = rig.agents.try_program("builder", &ada()).await.unwrap();
    assert!(!tried.ok && tried.walled && tried.models == 0, "{tried:?}");
    let why = tried.error.unwrap();
    assert!(why.contains("ended without saying which models it offers"), "{why}");
    assert!(why.contains(&format!("cannot find what I need in {}/.needs", walled.home.display())), "{why}");

    // Shown it, by whoever may show an agent more of the device, it answers.
    let more =
        SlotSpec { name: "builder".into(), reads: Some(vec!["~/.needs".into()]), ..SlotSpec::default() };
    rig.agents.ensure(more, &ada()).await.unwrap();
    let tried = rig.agents.try_program("builder", &ada()).await.unwrap();
    assert!(tried.ok && tried.walled, "{tried:?}");
    assert_eq!(tried.models, 2);
}

#[tokio::test]
async fn a_package_that_is_not_the_one_that_came_with_the_installation_is_said_to_be_and_replaced_is_kept() {
    let (walls, _) = stand_in();
    let walled = Walled::holding(walls).await;
    let rig = &walled.rig;
    let installed = std::fs::read_to_string(rig.dir.join("extensions/standin/extension.yaml")).unwrap();
    // Nothing of that name came with this installation: there is nothing to be behind.
    assert!(rig.agents.behind_what_came().is_empty());

    // The one that comes with it says more of its program than the copy
    // that was installed: here, one more thing it keeps in the home.
    let newer = installed.replace(
        "keeps: [\".standin\", \".standin.json\"]",
        "keeps: [\".standin\", \".standin.json\", \".needs\"]",
    );
    assert_ne!(newer, installed);
    std::fs::create_dir_all(rig.dir.join("offered/standin")).unwrap();
    std::fs::write(rig.dir.join("offered/standin/extension.yaml"), &newer).unwrap();
    assert_eq!(rig.agents.behind_what_came(), ["standin"]);
    // One that differs only in what is remarked beside its lines is no other package.
    let remarked = format!("# Said again, in other words.\n{installed}");
    std::fs::write(rig.dir.join("offered/standin/extension.yaml"), &remarked).unwrap();
    assert!(rig.agents.behind_what_came().is_empty());
    std::fs::write(rig.dir.join("offered/standin/extension.yaml"), &newer).unwrap();

    // A program that fails behind its wall, or does not answer when tried
    // there, is said to be on a package that may not say what it needs.
    let turn = walled.does("builder", "crash", None).await;
    let why = turn.error.unwrap();
    assert!(why.contains("Its package `standin` is not the one that came with this installation"), "{why}");
    assert!(why.contains(&format!("`{} extension update standin`", brand::CLI)), "{why}");
    let tried = rig.agents.try_program("builder", &ada()).await.unwrap();
    assert!(!tried.ok && tried.error.unwrap().contains("Its package `standin` is not the one that came"));

    // Taken, it is the one that came, and the copy it replaced is kept beside it.
    let taken = rig.agents.add_offered("standin", &ada()).await.unwrap();
    assert!(taken.wall.keeps.contains(&".needs".to_string()));
    assert!(rig.agents.behind_what_came().is_empty());
    let kept = std::fs::read_to_string(rig.dir.join("extensions/standin/extension.was.yaml")).unwrap();
    assert_eq!(kept, installed);
    // Installed again as it is, nothing is kept over that.
    rig.agents.add_offered("standin", &ada()).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(rig.dir.join("extensions/standin/extension.was.yaml")).unwrap(),
        installed
    );
    // And the turn that fails for a reason of its own is no longer put down to its package.
    let turn = walled.does("builder", "crash", None).await;
    assert!(!turn.error.unwrap().contains("is not the one that came"));
}

/// A library of the project `shop` whose documents are kept under
/// `private:journal`, one of them written, and beside it one that is not kept.
async fn journal(walled: &Walled) -> PathBuf {
    let project = walled.project();
    let fields = serde_json::json!({
        "title": { "type": "text", "required": true }, "body": { "type": "text" }, "path": { "type": "text" },
    });
    let spec = studio::collections::Spec::read(&fields, &serde_json::Value::Null, Some("documents"))
        .unwrap()
        .kept_at(Some("notes/days"))
        .read_by(Some("private:journal"))
        .unwrap();
    walled.rig.studio.collections.ensure("shop", "days", spec, None, &ada()).await.unwrap();
    for (file, text) in [
        ("notes/days/2026-11-14.md", "# Saturday\n\nThe marmalade is under the stairs.\n"),
        ("notes/guides/bread.md", "# Bread\n"),
        // And as a card's worktree has it, which is a copy of the project.
        (".lanes/shop-1/notes/days/2026-11-14.md", "# Saturday\n"),
    ] {
        std::fs::create_dir_all(project.join(file).parent().unwrap()).unwrap();
        std::fs::write(project.join(file), text).unwrap();
    }
    project
}

#[tokio::test]
async fn what_a_table_keeps_private_is_hidden_behind_the_wall_of_an_agent_that_does_not_hold_what_it_is_kept_under()
 {
    let (walls, told) = stand_in();
    let walled = Walled::holding(walls).await;
    let project = journal(&walled).await;
    let hidden = |said: &[String], folder: &Path| {
        let folder = folder.to_string_lossy();
        said.windows(4).any(|four| {
            four[0] == "--tmpfs" && four[1] == folder && four[2] == "--remount-ro" && four[3] == folder
        })
    };

    walled.does("builder", "echo done", None).await;
    let said = words(&told);
    // The folder its records are files in, and every copy of it.
    assert!(hidden(&said, &project.join("notes/days")), "{said:#?}");
    assert!(hidden(&said, &project.join(".lanes/shop-1/notes/days")), "{said:#?}");
    assert!(!hidden(&said, &project.join("notes/guides")) && !hidden(&said, &project.join("notes")));
    // The repository holds nothing of them yet (nothing of the folder is in a
    // commit or staged for one), so it is shown as it is to anybody.
    assert!(!hidden(&said, &project.join(".git")), "{said:#?}");
    assert!(shows(&said, "--ro-bind", &project.join(".git")), "{said:#?}");
    // Once it does, its history would show what the wall hides: it is not there either.
    crate::repositories::git(&project, &["add", "-f", "notes/days/2026-11-14.md"]);
    walled.does("builder", "echo done", None).await;
    let said = words(&told);
    assert!(hidden(&said, &project.join(".git")), "{said:#?}");
    let about = std::fs::read_to_string(walled.here.join("args")).unwrap();
    assert!(about.contains("The project's repository is not shown to you"), "{about}");

    // One that was given what they are kept under is shown them as the rest.
    let holds: Vec<String> = kernel::principal::AGENT_SCOPES
        .iter()
        .map(|scope| scope.to_string())
        .chain(["private:journal".to_string()])
        .collect();
    let trusted = SlotSpec { name: "builder".into(), scopes: Some(holds), ..SlotSpec::default() };
    walled.rig.agents.ensure(trusted, &ada()).await.unwrap();
    walled.does("builder", "echo done", None).await;
    let said = words(&told);
    assert!(!hidden(&said, &project.join("notes/days")), "{said:#?}");
    assert!(!hidden(&said, &project.join(".lanes/shop-1/notes/days")));
}

#[tokio::test]
async fn behind_a_real_wall_an_agent_reads_nothing_of_what_a_table_of_its_project_keeps_from_it() {
    let walls = Walls::system();
    if let Err(why) = walls.can() {
        println!("skipped: this system holds no wall ({why})");
        return;
    }
    let walled = Walled::holding(walls).await;
    journal(&walled).await;
    let tries = [
        "cat notes/days/2026-11-14.md 2>/dev/null || echo 'the day: not there'",
        "ls notes/days | grep . || echo 'its folder: empty'",
        "cat .lanes/shop-1/notes/days/2026-11-14.md 2>/dev/null || echo 'its copy: not there'",
        "echo mine > notes/days/planted.md 2>/dev/null || echo 'its folder: not written'",
        "cat notes/guides/bread.md && echo ': read'",
    ]
    .join("; ");
    let turn = walled.does("builder", &tries, None).await;
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    for expected in [
        "the day: not there",
        "its folder: empty",
        "its copy: not there",
        "its folder: not written",
        "# Bread",
        ": read",
    ] {
        assert!(
            turn.reply.lines().any(|line| line.trim() == expected),
            "expected `{expected}` in:\n{}",
            turn.reply
        );
    }
    assert!(!walled.project().join("notes/days/planted.md").exists());

    // Given what it is kept under, it reads the day.
    let holds: Vec<String> = kernel::principal::AGENT_SCOPES
        .iter()
        .map(|scope| scope.to_string())
        .chain(["private:journal".to_string()])
        .collect();
    let trusted = SlotSpec { name: "builder".into(), scopes: Some(holds), ..SlotSpec::default() };
    walled.rig.agents.ensure(trusted, &ada()).await.unwrap();
    let turn = walled.does("builder", "cat notes/days/2026-11-14.md", None).await;
    assert!(turn.reply.contains("The marmalade is under the stairs."), "{}", turn.reply);
}

#[tokio::test]
async fn an_agent_moved_to_another_project_is_shown_that_projects_folder_from_its_next_turn_and_none_moved_to_none()
 {
    let (walls, told) = stand_in();
    let walled = Walled::holding(walls).await;
    let (rig, shop) = (&walled.rig, walled.project());
    rig.studio.projects.create("Cafe", None, &ada()).await.unwrap();
    let cafe = rig.studio.projects.dir_of("cafe").unwrap().canonicalize().unwrap();
    let turn = walled.does("builder", "echo done", None).await;
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    assert!(shows(&words(&told), "--bind", &shop), "it begins in its own project");

    // Moved, its next turn is shown the other project's folder, and nothing of the one it left.
    let moved = rig.agents.move_to("builder", Some("cafe".into()), &ada(), &rig.engine).await.unwrap();
    assert_eq!(moved.project.as_deref(), Some("cafe"));
    let turn = walled.does("builder", "echo done", None).await;
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    let shown = words(&told);
    assert!(shows(&shown, "--bind", &cafe), "{shown:#?}");
    assert!(!mentions(&shown, &shop), "{shown:#?}");

    // Moved to none, it is shown no project's folder, and works in a folder of its own.
    rig.agents.move_to("builder", None, &ada(), &rig.engine).await.unwrap();
    let turn = walled.does("builder", "echo done", None).await;
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    let shown = words(&told);
    assert!(!mentions(&shown, &shop) && !mentions(&shown, &cafe), "{shown:#?}");
    assert!(
        shown.iter().any(|word| Path::new(word).starts_with(rig.dir.join("slots/builder"))),
        "{shown:#?}"
    );
}

#[tokio::test]
async fn a_turn_is_not_shown_the_folders_it_is_told_and_is_shown_its_own_place_whatever_it_is_told() {
    let (walls, told) = stand_in();
    let walled = Walled::holding(walls).await;
    let project = walled.project();
    for place in ["shop-1--a", "shop-1--b", "shop-1--c"] {
        std::fs::create_dir_all(project.join(".lanes").join(place)).unwrap();
    }
    let request = TurnRequest {
        slot: "builder".into(),
        kind: "task".into(),
        thread: "contend:shop-1:a".into(),
        prompt: "Please see to this.\necho done".into(),
        cwd: Some(".lanes/shop-1--a".into()),
        hidden: vec![".lanes/shop-1--b".into(), ".lanes/shop-1--c".into()],
        ..TurnRequest::default()
    };
    let turn = walled.rig.agents.enqueue(request).await.unwrap();
    let turn = soon("the turn to end", walled.rig.agents.wait(&turn)).await.unwrap();
    assert_eq!(turn.hidden, [".lanes/shop-1--b", ".lanes/shop-1--c"]);
    let said = words(&told);
    let hidden = |folder: &str| {
        let folder = project.join(folder).to_string_lossy().into_owned();
        said.windows(2).any(|two| two[0] == "--tmpfs" && two[1] == folder)
    };
    // The others' places are not shown: what is there reads as an empty folder that cannot be written.
    assert!(hidden(".lanes/shop-1--b") && hidden(".lanes/shop-1--c"), "{said:#?}");
    // Its own is, and the project it is in.
    assert!(!hidden(".lanes/shop-1--a"), "{said:#?}");
    assert!(shows(&said, "--bind", &project.join(".lanes/shop-1--a")), "{said:#?}");
    assert!(shows(&said, "--ro-bind", &project), "{said:#?}");

    // Told nothing, the others' places are there to read, as they always were.
    walled.does("builder", "echo done", Some(".lanes/shop-1--a")).await;
    let said = words(&told);
    assert!(
        !said.windows(2).any(|two| two[0] == "--tmpfs" && two[1].contains(".lanes/shop-1--")),
        "{said:#?}"
    );
}
