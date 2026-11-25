//! The keys an agent's program is handed: secrets the factory keeps, each
//! under a variable of its environment, read as its turn starts and in
//! nothing that is written down of it.
//!
//! As the tests of walls are: a stand-in for the agent's program, which
//! writes down what it was started with and tries what it is asked to.
//!
//! Unix's alone: played behind the stand-in for a wall (`walls.rs`),
//! which is a file of a Unix shell.
#![cfg(unix)]

use agents::SlotSpec;
use hosting::Walls;
use kernel::{ErrorKind, Principal, PrincipalKind};

use crate::ada;
use crate::walls::{Walled, stand_in};

/// A key as a service gives one: long, and nobody's.
const KEY: &str = "tok_0123456789abcdefghijklmnopqrstuv";

async fn handed(walled: &Walled, pairs: &[&str]) {
    let spec = SlotSpec {
        name: "builder".into(),
        secrets: Some(pairs.iter().map(|pair| pair.to_string()).collect()),
        ..SlotSpec::default()
    };
    walled.rig.agents.ensure(spec, &ada()).await.unwrap();
}

#[tokio::test]
async fn an_agent_is_handed_a_key_the_factory_keeps_and_nothing_written_down_of_its_turn_has_it() {
    let (walls, _) = stand_in();
    let walled = Walled::holding(walls).await;
    let rig = &walled.rig;
    rig.hosting.set_secret("remote-bot", KEY, &ada()).await.unwrap();
    handed(&walled, &["REMOTE_TOKEN=remote-bot"]).await;
    // What is kept of the slot is the name the secret is kept under.
    let slot = rig.agents.slot("builder").await.unwrap();
    assert_eq!(slot.secrets.get("REMOTE_TOKEN").map(String::as_str), Some("remote-bot"));
    assert!(!serde_json::to_string(&slot).unwrap().contains(KEY));

    // Its program has it, under the variable said; and repeats it, in what
    // it answers and in what it shows as it goes.
    let turn = walled.does("builder", "echo \"the key is $REMOTE_TOKEN, twice: $REMOTE_TOKEN\"", None).await;
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    assert!(walled.env().lines().any(|line| line == format!("REMOTE_TOKEN={KEY}")));
    assert_eq!(turn.reply, "the key is [the secret `remote-bot`], twice: [the secret `remote-bot`]");
    // It is told what it holds, by the variable and never by the value.
    let about = std::fs::read_to_string(walled.here.join("args")).unwrap();
    assert!(about.contains("you are handed a key") && about.contains("`REMOTE_TOKEN`"), "{about}");
    assert!(!about.contains(KEY));
    // Nothing the journal has of it says the value: not its events, not the turn as it is kept.
    let events = serde_json::to_string(&rig.events("**").await).unwrap();
    assert!(!events.contains(KEY), "an event has the key in it");
    let kept = serde_json::to_string(&rig.agents.turns("builder", 10).await.unwrap()).unwrap();
    assert!(!kept.contains(KEY), "{kept}");

    // One that fails with it in what it says is written down the same.
    let turn = walled.does("builder", "crash", None).await;
    assert!(!turn.error.unwrap_or_default().contains(KEY));

    // Read as each turn starts: set again, the next turn has the new one.
    let newer = "tok_zyxwvutsrqponmlkjihgfedcba9876543210";
    rig.hosting.set_secret("remote-bot", newer, &ada()).await.unwrap();
    walled.does("builder", "echo done", None).await;
    assert!(walled.env().lines().any(|line| line == format!("REMOTE_TOKEN={newer}")));

    // Taken away, its turn does not start, and says which secret and how to set it.
    rig.hosting.remove_secret("remote-bot", &ada()).await.unwrap();
    std::fs::remove_file(walled.here.join("env")).unwrap();
    let turn = walled.does("builder", "echo done", None).await;
    assert_eq!(turn.status, "failed");
    let why = turn.error.unwrap();
    assert!(
        why.contains("`builder` is handed the secret `remote-bot` as `REMOTE_TOKEN`, and it cannot be read"),
        "{why}"
    );
    assert!(why.contains(&format!("{} secret set remote-bot", brand::CLI)), "{why}");
    assert!(!walled.here.join("env").exists(), "its program was started without what it was to be handed");

    // Handed none again, it works as it did.
    handed(&walled, &[]).await;
    let turn = walled.does("builder", "echo \"key: ${REMOTE_TOKEN:-none}\"", None).await;
    assert_eq!(turn.reply, "key: none");
}

#[tokio::test]
async fn handing_an_agent_a_key_takes_being_allowed_to_have_secrets_used_and_is_said_as_a_variable_and_a_name()
 {
    let (walls, _) = stand_in();
    let walled = Walled::holding(walls).await;
    let rig = &walled.rig;
    let of = |pairs: &[&str]| SlotSpec {
        name: "builder".into(),
        secrets: Some(pairs.iter().map(|pair| pair.to_string()).collect()),
        ..SlotSpec::default()
    };
    // May make agents, and may not have a secret used.
    let grace = Principal {
        kind: PrincipalKind::User,
        id: "grace".into(),
        name: "Grace".into(),
        scopes: [
            "agents",
            "projects",
            "channels",
            "board",
            "records",
            "blueprints:read",
            "fleet:read",
            "search",
        ]
        .iter()
        .map(|scope| scope.to_string())
        .collect(),
        key: None,
    };
    let refused = rig.agents.ensure(of(&["REMOTE_TOKEN=remote-bot"]), &grace).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    assert!(refused.message.contains("secrets:use"), "{}", refused.message);
    assert!(rig.agents.slot("builder").await.unwrap().secrets.is_empty());

    // Once somebody who may has said so, saying anything else of that agent takes the same.
    rig.agents.ensure(of(&[" REMOTE_TOKEN = remote-bot ", ""]), &ada()).await.unwrap();
    let told = SlotSpec {
        name: "builder".into(),
        instructions: Some("Print what you hold.".into()),
        ..SlotSpec::default()
    };
    assert_eq!(rig.agents.ensure(told, &grace).await.unwrap_err().kind, ErrorKind::Denied);

    for (wrong, why) in [
        ("remote-bot", "there is no `=` in it"),
        ("REMOTE TOKEN=remote-bot", "`REMOTE TOKEN` cannot be a variable"),
        ("PATH=remote-bot", "`PATH` cannot be a variable"),
        ("FACTORY_URL=remote-bot", "`FACTORY_URL` cannot be a variable"),
        ("REMOTE_TOKEN=", "`` cannot be what a secret is kept under"),
        ("REMOTE_TOKEN=Remote Bot/One", "cannot be what a secret is kept under"),
    ] {
        let refused = rig.agents.ensure(of(&[wrong]), &ada()).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Invalid, "{wrong}");
        assert!(refused.message.contains(why), "{wrong}: {}", refused.message);
        assert!(refused.message.contains("written `VARIABLE=name`"), "{}", refused.message);
    }
    let own = format!("{}_TOKEN=remote-bot", brand::ENV_PREFIX);
    assert_eq!(rig.agents.ensure(of(&[&own]), &ada()).await.unwrap_err().kind, ErrorKind::Invalid);
}

#[tokio::test]
async fn behind_a_real_wall_git_takes_the_key_an_agent_was_handed_and_nothing_of_whoever_runs_the_daemon() {
    let walls = Walls::system();
    if let Err(why) = walls.can() {
        println!("skipped: this system holds no wall ({why})");
        return;
    }
    let walled = Walled::holding(walls).await;
    let rig = &walled.rig;
    rig.hosting.set_secret("remote-bot", KEY, &ada()).await.unwrap();
    handed(&walled, &["REMOTE_TOKEN=remote-bot"]).await;
    // How whoever runs the daemon is known to their own git, which is no agent's.
    std::fs::write(walled.home.join(".gitconfig"), "[credential]\n\thelper = store\n").unwrap();
    std::fs::write(walled.home.join(".git-credentials"), "https://ada:the-owners-own@example.invalid\n")
        .unwrap();

    // It tells its own git to answer with the key it holds, and asks git who it is to a remote.
    let tries = [
        "git config --global credential.helper '!f() { echo username=x-access-token; echo \"password=$REMOTE_TOKEN\"; }; f'",
        "printf 'protocol=https\\nhost=example.invalid\\n\\n' | git credential fill",
    ]
    .join(" && ");
    let turn = walled.does("builder", &tries, None).await;
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    let said: Vec<&str> = turn.reply.lines().map(str::trim).collect();
    assert!(said.contains(&"username=x-access-token"), "{}", turn.reply);
    // Git answered with its key (which is written down by its name), and not with the owner's.
    assert!(said.contains(&"password=[the secret `remote-bot`]"), "{}", turn.reply);
    assert!(!turn.reply.contains("the-owners-own") && !turn.reply.contains(KEY), "{}", turn.reply);
    // What it told git is kept in its own home, and whoever runs the daemon has theirs as it was.
    let its_own = std::fs::read_to_string(rig.dir.join("slots/builder/home/.gitconfig")).unwrap();
    assert!(its_own.contains("x-access-token") && !its_own.contains(KEY), "{its_own}");
    assert_eq!(
        std::fs::read_to_string(walled.home.join(".gitconfig")).unwrap(),
        "[credential]\n\thelper = store\n"
    );
}

#[tokio::test]
async fn an_agent_under_an_environment_is_handed_what_is_marked_for_agents_and_its_slots_keys_from_there_first()
 {
    const DEPLOY: &str = "tok_env_deploy_0123456789abcdef";
    const THEIRS: &str = "tok_env_remote_0123456789abcdef";
    let (walls, _) = stand_in();
    let walled = Walled::holding(walls).await;
    let rig = &walled.rig;
    // The rig's project, `shop`, is there already, with an agent of its own.
    let said = || kernel::Source::actor("user:ada");
    rig.engine.make_environment("shop", "prod", &ada(), said()).await.unwrap();
    let fill = |held: &mut conductor::environs::Environment| {
        held.set(
            &kernel::fields(serde_json::json!({ "api": "https://shop.example", "region": "eu" })),
            None,
        )?;
        held.mark(&["region".to_string()], true)?;
        held.keep_secret("deploy", &ada(), 1)?;
        held.mark_secret("deploy", Some("DEPLOY_KEY"))?;
        held.keep_secret("remote-bot", &ada(), 1)
    };
    rig.engine.change_environment("shop", "prod", fill, &ada(), said()).await.unwrap();
    let kept = rig.hosting.secrets().unwrap();
    kept.set_of("shop", "prod", "deploy", DEPLOY).unwrap();
    kept.set_of("shop", "prod", "remote-bot", THEIRS).unwrap();
    rig.hosting.set_secret("remote-bot", KEY, &ada()).await.unwrap();
    rig.hosting.set_secret("other", "tok_factory_other_0123456789", &ada()).await.unwrap();
    let spec = |secrets: &[&str], environment: Option<&str>| SlotSpec {
        name: "deployer".into(),
        adapter: Some("standin".into()),
        project: Some("shop".into()),
        secrets: Some(secrets.iter().map(|pair| pair.to_string()).collect()),
        environment: environment.map(str::to_string),
        ..SlotSpec::default()
    };

    // Added by hand to a project that has environments, it says which it works under.
    let refused = rig.agents.ensure(spec(&[], None), &ada()).await.unwrap_err();
    assert!(refused.message.contains("say one of prod (`--environment prod`)"), "{}", refused.message);
    let refused = rig.agents.ensure(spec(&[], Some("stage")), &ada()).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::NotFound, "{}", refused.message);
    let slot = rig.agents.ensure(spec(&["OTHER=other"], Some("prod")), &ada()).await.unwrap();
    assert_eq!(slot.environment, "prod");

    // What is marked for agents, under its own name or as the variable it is
    // marked with, and nothing else of the environment; the factory's for
    // what the environment does not list.
    let turn = walled.does("deployer", "echo \"$region ${api:-none} $DEPLOY_KEY $OTHER\"", None).await;
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    assert_eq!(turn.reply, "eu none [the secret `deploy` of the `prod` environment] [the secret `other`]");
    let env = walled.env();
    assert!(env.lines().any(|line| line == "region=eu") && !env.contains("api="), "{env}");
    assert!(env.lines().any(|line| line == format!("DEPLOY_KEY={DEPLOY}")), "{env}");
    let about = std::fs::read_to_string(walled.here.join("args")).unwrap();
    assert!(about.contains("`DEPLOY_KEY`") && !about.contains(DEPLOY), "{about}");

    // A key its slot names that the environment lists and does not hand
    // to agents: its turn does not start, and the factory's is not handed.
    rig.agents.ensure(spec(&["REMOTE_TOKEN=remote-bot"], None), &ada()).await.unwrap();
    std::fs::remove_file(walled.here.join("env")).unwrap();
    let turn = walled.does("deployer", "echo done", None).await;
    assert_eq!(turn.status, "failed");
    let why = turn.error.unwrap();
    assert!(why.contains("lists a secret `remote-bot` that it does not hand to agents"), "{why}");
    assert!(why.contains("secret agents remote-bot VAR --project shop --environment prod"), "{why}");
    assert!(!walled.here.join("env").exists(), "its program was not started");
    // Marked, it is the environment's that is handed under the slot's variable.
    let mark = |held: &mut conductor::environs::Environment| held.mark_secret("remote-bot", Some("BOT"));
    rig.engine.change_environment("shop", "prod", mark, &ada(), said()).await.unwrap();
    let turn = walled.does("deployer", "echo \"$REMOTE_TOKEN\"", None).await;
    assert_eq!(turn.reply, "[the secret `remote-bot` of the `prod` environment]", "{:?}", turn.error);
    assert!(walled.env().lines().any(|line| line == format!("REMOTE_TOKEN={THEIRS}")));
    assert!(!walled.env().contains(KEY));

    // It is not taken away from under an agent that works under it.
    let under = async {
        let slots = rig.agents.slots().await?.into_iter();
        Ok(slots.filter(|slot| slot.environment == "prod").map(|slot| slot.name).collect())
    };
    let refused = rig.engine.remove_environment(("shop", "prod"), under, &ada(), said()).await.unwrap_err();
    assert!(
        refused.message.contains("the agent `deployer` runs under the environment `prod`"),
        "{}",
        refused.message
    );
}
