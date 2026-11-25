//! An agent a factory does not start, with the real program: invited by
//! whoever is signed in, joined from a home of its own with the
//! invitation read from standard input, its credential kept where only
//! whoever ran it reads and apart from any person's session there, its
//! commands asked as it by name, and leaving.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::io::AsyncWriteExt;

use crate::{BINARY, Ran, Rig, command_in, kept_on_windows, run_in, scratch, soon};

/// Runs the program in the home `dir`, told where the daemon is, and
/// handed `said` on standard input.
pub(crate) async fn handed(dir: &Path, url: &str, args: &[&str], said: &str) -> Ran {
    let mut command = command_in(dir, url, args);
    command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    input.write_all(format!("{said}\n").as_bytes()).await.unwrap();
    drop(input);
    let output = soon(&format!("`{}`", args.join(" ")), child.wait_with_output()).await.unwrap();
    Ran {
        code: output.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&output.stdout).into_owned(),
        err: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Runs the program in the home `dir` told nothing of where any daemon
/// is: no address in a flag or in the environment.
async fn told_nowhere(dir: &Path, env: &[(&str, &str)], args: &[&str]) -> Ran {
    let mut command = tokio::process::Command::new(BINARY);
    command
        .args(args)
        .env_clear()
        .envs(kept_on_windows())
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_DATA_HOME", dir.join("share"))
        .envs(env.iter().copied())
        .stdin(Stdio::null());
    let output = soon(&format!("`{}`", args.join(" ")), command.output()).await.unwrap();
    Ran {
        code: output.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&output.stdout).into_owned(),
        err: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// The invitation a command that made one printed: the line of its own.
pub(crate) fn invitation(printed: &str) -> String {
    let line = printed.lines().find(|line| line.trim().starts_with("enrol_"));
    line.unwrap_or_else(|| panic!("no invitation was printed:\n{printed}")).trim().to_string()
}

/// The files kept for agents that joined from the home `dir`.
fn kept(dir: &Path) -> Vec<PathBuf> {
    let folder = dir.join("config").join(brand::ID).join("agents");
    let mut files = Vec::new();
    for agent in std::fs::read_dir(folder).into_iter().flatten().flatten() {
        files.extend(std::fs::read_dir(agent.path()).into_iter().flatten().flatten().map(|file| file.path()));
    }
    files.sort();
    files
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_from_outside_is_invited_joins_from_its_own_home_and_its_commands_say_its_name() {
    let rig = Rig::signed_in().await;
    let cli = brand::CLI;
    rig.run(&["project", "create", "shop", "--bare"]).await.ok();
    rig.run(&["apply", "std/board", "--project", "shop"]).await.ok();
    let card = rig.run(&["board", "add", "shop", "Hang", "the", "gate"]).await.ok();
    let card = card.split_whitespace().nth(1).unwrap().to_string();

    // Invited by whoever is signed in: the invitation is printed once,
    // with the command that reads it from standard input.
    let printed = rig.run(&["agent", "invite", "walls", "--project", "shop"]).await.ok();
    assert!(printed.starts_with("`walls` is invited, as an agent this factory does not start."), "{printed}");
    assert!(printed.contains(&format!("  {cli} agent join --url {}", rig.url)), "{printed}");
    assert!(printed.contains("within the next 24 hours"), "{printed}");
    let first = invitation(&printed);
    assert!(rig.run(&["agent", "list"]).await.ok().contains("invited, has yet to join"));

    // Joined from another home, the invitation handed on standard input:
    // the credential is kept and never printed.
    let home = scratch("outside");
    let joined = handed(&home, &rig.url, &["agent", "join"], &first).await;
    let said = format!("{}{}", joined.out, joined.err);
    assert_eq!(joined.code, 0, "{said}");
    assert!(joined.out.starts_with(&format!("joined {} as the agent `walls`", rig.url)), "{said}");
    assert!(!said.contains("agt_") && !said.contains(&first), "a secret was printed: {said}");
    let files = kept(&home);
    assert_eq!(files.len(), 1, "{files:?}");
    assert!(joined.out.contains(&files[0].display().to_string()), "{said}");
    let token = std::fs::read_to_string(&files[0]).unwrap();
    assert!(
        token.contains("\"token\":\"agt_") && token.contains(&rig.url),
        "what is kept is its credential there"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&files[0]).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "only whoever ran it reads its credential");
    }
    // Nobody signed in there: a command with no name is nobody's, and
    // one asked as the agent is the agent's.
    let nobody = run_in(&home, &rig.url, &[], &["whoami"]).await;
    assert!(nobody.code != 0 && nobody.err.contains("sign in first"), "{}", nobody.err);
    let who = run_in(&home, &rig.url, &[], &["--as", "walls", "whoami"]).await.ok();
    assert!(who.starts_with("walls (agent:walls)"), "{who}");
    // By the environment, too; and with no address said anywhere it is
    // the one factory that agent joined from here.
    let agent = brand::env_var("AGENT");
    let named = [(agent.as_str(), "walls".to_string())];
    assert!(run_in(&home, &rig.url, &named, &["whoami"]).await.ok().starts_with("walls (agent:walls)"));
    assert!(
        told_nowhere(&home, &[], &["--as", "walls", "whoami"]).await.ok().starts_with("walls (agent:walls)")
    );
    assert!(
        told_nowhere(&home, &[(&agent, "walls")], &["whoami"]).await.ok().starts_with("walls (agent:walls)")
    );

    // It gives a card to itself, takes it in hand and says something on
    // it, and each says it did.
    let as_walls = |args: &[&str]| {
        let (home, url) = (home.clone(), rig.url.clone());
        let args: Vec<String> = ["--as", "walls"].iter().chain(args).map(|word| word.to_string()).collect();
        async move {
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            run_in(&home, &url, &[], &args).await
        }
    };
    assert_eq!(
        as_walls(&["board", "give", &card, "walls"]).await.ok().trim(),
        format!("{card} is given to agent:walls")
    );
    as_walls(&["board", "move", &card, "doing"]).await.ok();
    as_walls(&["record", "comment", &card, "The", "hinges", "are", "on."]).await.ok();
    let history = rig.run(&["board", "history", &card]).await.ok();
    for line in ["walls moved it from backlog to doing", "walls said: The hinges are on."] {
        assert!(history.contains(line), "{history}");
    }
    assert!(
        rig.run(&["board", "shop"]).await.ok().contains(&format!("{card}  Hang the gate  [agent:walls]"))
    );
    let listed = rig.run(&["agent", "list"]).await.ok();
    let line = listed.lines().find(|line| line.starts_with("walls")).unwrap();
    let words: Vec<&str> = line.split_whitespace().collect();
    assert_eq!(words[..5], ["walls", "external", "shop", "active", "joined;"], "{listed}");
    assert!(line.ends_with("joined; holds what an agent usually does, and its own inbox"), "{listed}");

    // What it holds is changed where any agent's is, and its next
    // command is held to it: the credential it keeps is not replaced.
    let changed =
        rig.run(&["agent", "add", "walls", "--scope", "board:read", "--scope", "projects:read"]).await.ok();
    assert!(changed.starts_with("walls is an external agent, which this factory does not start. It holds: board:read, projects:read"), "{changed}");
    let refused = as_walls(&["board", "move", &card, "review"]).await;
    assert!(refused.code != 0 && refused.err.contains("needs `board:write`"), "{}", refused.err);
    as_walls(&["board", "shop"]).await.ok();
    assert_eq!(std::fs::read_to_string(&files[0]).unwrap(), token);
    assert!(rig.run(&["agent", "list"]).await.ok().contains("joined; holds board:read, projects:read"));
    // There is nothing of it to try.
    let tried = rig.run(&["agent", "try", "walls"]).await;
    assert!(tried.code != 0 && tried.err.contains("there is nothing to try"), "{}", tried.err);

    // Used, the invitation lets nobody in; and a person's invitation is none.
    let again = handed(&home, &rig.url, &["agent", "join"], &first).await;
    assert!(again.code != 0 && again.err.contains("that invitation lets no agent in"), "{}", again.err);
    assert_eq!(std::fs::read_to_string(&files[0]).unwrap(), token, "a refusal left what was kept");
    let empty = handed(&home, &rig.url, &["agent", "join"], "").await;
    assert!(empty.code != 0 && empty.err.contains("an invitation is needed"), "{}", empty.err);

    // Stopped, its credential is not taken, and the command says so for
    // what it is and nothing of signing in.
    rig.run(&["agent", "stop", "walls"]).await.ok();
    let refused = as_walls(&["whoami"]).await;
    assert!(refused.code != 0, "{}", refused.out);
    assert!(
        refused.err.contains("the credential kept on this machine for the agent `walls` is not taken at"),
        "{}",
        refused.err
    );
    assert!(!refused.err.contains("login"), "{}", refused.err);
    // Invited again, it joins again, the invitation given as a word this
    // time: the credential kept is the new one.
    let second = invitation(&rig.run(&["agent", "invite", "walls", "--expires", "2h"]).await.ok());
    run_in(&home, &rig.url, &[], &["agent", "join", &second]).await.ok();
    assert_eq!(kept(&home), files);
    assert_ne!(std::fs::read_to_string(&files[0]).unwrap(), token);
    assert!(as_walls(&["whoami"]).await.ok().contains("may: board:read, projects:read"));

    // It leaves by its own word: the factory withdraws the credential,
    // and it is forgotten here.
    let unnamed = run_in(&home, &rig.url, &[], &["agent", "leave"]).await;
    assert!(
        unnamed.code != 0 && unnamed.err.contains("which agent leaves is said by its name"),
        "{}",
        unnamed.err
    );
    let left = as_walls(&["agent", "leave"]).await.ok();
    assert!(left.starts_with(&format!("`walls` has left {}", rig.url)), "{left}");
    assert!(kept(&home).is_empty());
    let gone = as_walls(&["whoami"]).await;
    assert!(
        gone.code != 0
            && gone.err.contains("no credential is kept on this machine for an agent called `walls`"),
        "{}",
        gone.err
    );
    assert!(rig.run(&["agent", "list"]).await.ok().contains("not joined"));
    let _ = std::fs::remove_dir_all(home);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_person_and_an_agent_on_one_machine_never_use_each_others_credential() {
    let rig = Rig::signed_in().await;
    let cli = brand::CLI;
    let first = invitation(&rig.run(&["agent", "invite", "walls"]).await.ok());
    // The agent joins from the home Ada is signed in from.
    let session = rig.dir.join("config").join(brand::ID).join("session.json");
    let before = std::fs::read_to_string(&session).unwrap();
    handed(&rig.dir, &rig.url, &["agent", "join"], &first).await.ok();
    assert_eq!(std::fs::read_to_string(&session).unwrap(), before, "joining touched nobody's session");
    // A command is Ada's unless it says the agent, and the agent's where it does.
    assert!(rig.run(&["whoami"]).await.ok().starts_with("Ada"));
    assert!(rig.run(&["--as", "walls", "whoami"]).await.ok().starts_with("walls (agent:walls)"));
    // The agent may not add agents; Ada may.
    let refused = rig.run(&["--as", "walls", "agent", "invite", "gate"]).await;
    assert!(refused.code != 0 && refused.err.contains("needs `agents:write`"), "{}", refused.err);
    // An agent that never joined from here is not Ada: the command says
    // nothing is kept for it, and uses nothing else.
    let other = rig.run(&["--as", "gate", "whoami"]).await;
    assert!(other.code != 0, "{}", other.out);
    assert!(
        other.err.contains("no credential is kept on this machine for an agent called `gate`"),
        "{}",
        other.err
    );
    assert!(other.err.contains(&format!("`{cli} agent join --url {}`", rig.url)), "{}", other.err);
    let (agent, token) = (brand::env_var("AGENT"), brand::env_var("TOKEN"));
    let named = [(agent.as_str(), "gate".to_string())];
    let other = rig.run_with(&named, &["whoami"]).await;
    assert!(other.code != 0 && other.err.contains("no credential is kept"), "{}{}", other.out, other.err);
    // A name that is no agent's leads nowhere.
    let odd = rig.run(&["--as", "../session", "whoami"]).await;
    assert!(odd.code != 0 && odd.err.contains("is not an agent's name"), "{}", odd.err);
    // A credential given whole and a name are one or the other.
    let both = rig.run(&["--as", "walls", "--token", "key_x", "whoami"]).await;
    assert!(both.code != 0 && both.err.contains("give one"), "{}", both.err);
    // A credential in the environment is its own, and no name beside it changes whose.
    let key = rig.run(&["key", "ci", "--scope", "board:read"]).await.ok();
    let key = key.split_whitespace().find(|word| word.starts_with("key_")).unwrap().to_string();
    let env = [(agent.as_str(), "walls".to_string()), (token.as_str(), key)];
    assert!(rig.run_with(&env, &["whoami"]).await.ok().contains("may: board:read"));
    // Signing out leaves the agent's credential, and its leaving leaves Ada's session.
    rig.run(&["--as", "walls", "agent", "leave"]).await.ok();
    assert_eq!(std::fs::read_to_string(&session).unwrap(), before);
    assert!(rig.run(&["whoami"]).await.ok().starts_with("Ada"));

    // What the program says of all this, and what it will not be told.
    let help = rig.run(&["help", "invite"]).await.ok();
    assert!(help.contains(&format!("{cli} agent join [<invitation>] --url ADDRESS")), "{help}");
    assert_eq!(rig.run(&["help", "join"]).await.ok(), help);
    assert!(rig.run(&["help", "agent"]).await.ok().contains(&format!("`{cli} help invite` has the rest")));
    let odd = rig
        .run(&["serve", "--data", &rig.dir.join("never").to_string_lossy(), "--command", "external=true"])
        .await;
    assert!(odd.code != 0 && odd.err.contains("a program cannot be called `external`"), "{}", odd.err);
    assert!(!rig.dir.join("never").exists(), "a daemon was started all the same");
}
