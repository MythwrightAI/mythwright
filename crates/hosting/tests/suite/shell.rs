use conductor::{RunStatus, StepStatus};
use serde_json::json;

use crate::standin::{Hold, cmd};
use crate::{Rig, alive, gone, line, pids, step};

/// A script of a user's own, which their system's shell reads: written
/// for each of the two shells where they read the same thing otherwise.
fn script(sh: &'static str, windows: &'static str) -> &'static str {
    if cfg!(windows) { windows } else { sh }
}

#[tokio::test]
async fn shell_runs_a_script_and_reports_what_it_printed() {
    let rig = Rig::new().await;
    // Three lines for a shell that reads as many, and where the shell is
    // started with one line (Windows) the same three things on it.
    let source = script(
        "\
blueprint: sh
permissions: [shell]
steps:
  hello:
    do: shell
    with:
      run: |
        echo hello
        echo oops >&2
        echo done
",
        "\
blueprint: sh
permissions: [shell]
steps:
  hello: { do: shell, with: { run: 'echo hello& echo oops>&2& echo done' } }
",
    );
    let run = rig.run(source, json!({})).await;
    let output = &step(&run, "hello").output;
    assert_eq!(
        (output["code"].as_i64(), output["stdout"].as_str(), output["stderr"].as_str()),
        (Some(0), Some("hello\ndone"), Some("oops"))
    );

    let mut logged: Vec<(char, String)> =
        rig.engine.step_log(&run.id, "hello", 0, 10).into_iter().map(|l| (l.stream, l.text)).collect();
    logged.sort();
    assert_eq!(logged, [('e', "oops".into()), ('o', "done".into()), ('o', "hello".into())]);
}

#[tokio::test]
async fn what_a_program_says_while_it_goes_on_is_heard_as_it_says_it_and_not_when_it_ends() {
    let rig = Rig::new().await;
    let mut lines = rig.lines();
    // It says a line, waits to be told, and says another: twice, with its
    // output held open throughout, as a build that takes its time has it.
    let hold = Hold::new().await;
    let at = hold.address();
    let works =
        cmd(&["say", "one", "held", &at, "first", "say", "two", "held", &at, "second", "say", "three"]);
    let applied = rig
        .apply(
            &format!("blueprint: slow\npermissions: [shell]\nsteps:\n  work: {{ do: shell, with: {{ cmd: {works} }} }}\n"),
            json!({}),
        )
        .await;
    line(&mut lines, "one").await;
    hold.came().await.go().await;
    // The second is heard while the program is still at work: it is held
    // again, and is let go only once that line has been read.
    line(&mut lines, "two").await;
    hold.came().await.go().await;
    let run = rig.finished(&applied).await;
    assert_eq!(step(&run, "work").output["stdout"], "one\ntwo\nthree");
}

#[tokio::test]
async fn a_non_zero_exit_fails_the_step_with_the_code_and_the_last_thing_it_said() {
    let rig = Rig::new().await;
    let source = script(
        "\
blueprint: sh
permissions: [shell]
params: { ok: [0] }
steps:
  bad: { do: shell, with: { run: \"echo no such table >&2; exit 3\", ok_codes: ${params.ok} } }
",
        "\
blueprint: sh
permissions: [shell]
params: { ok: [0] }
steps:
  bad: { do: shell, with: { run: \"echo no such table>&2& exit 3\", ok_codes: ${params.ok} } }
",
    );
    let run = rig.run(source, json!({})).await;
    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(step(&run, "bad").error.as_deref(), Some("exited with code 3: no such table"));
    assert_eq!(step(&run, "bad").output["code"], 3);

    let run = rig.run(source, json!({ "ok": [0, 3] })).await;
    assert_eq!(step(&run, "bad").status, StepStatus::Ok, "3 was declared acceptable");
}

#[tokio::test]
async fn cmd_runs_a_program_without_a_shell_in_between() {
    let rig = Rig::new().await;
    // What either system's shell would read something into: a space, a
    // variable as each of them writes one, and quotation marks.
    let says = cmd(&["args", "a b", "$HOME", "%PATH%", "it's \"here\""]);
    let run = rig
        .run(
            &format!("blueprint: argv\npermissions: [shell]\nsteps:\n  p: {{ do: shell, with: {{ cmd: {says} }} }}\n"),
            json!({}),
        )
        .await;
    assert_eq!(
        step(&run, "p").output["stdout"],
        "a b|$HOME|%PATH%|it's \"here\"",
        "arguments are passed as they are, unexpanded"
    );
}

#[tokio::test]
async fn cli_runs_this_programs_own_command_line_without_a_shell_in_between() {
    let rig = Rig::new().await;
    // The program here is this test's own, which can be asked what tests
    // it has; the product's is asked for its store the same way.
    let name = "cli_runs_this_programs_own_command_line_without_a_shell_in_between";
    let run = rig
        .run(
            &format!(
                "blueprint: own\npermissions: [shell]\nsteps:\n  p: {{ do: shell, with: {{ cli: [--list, {name}] }} }}\n"
            ),
            json!({}),
        )
        .await;
    let listed = step(&run, "p").output["stdout"].as_str().unwrap_or_default().to_string();
    assert!(listed.contains(&format!("shell::{name}: test")), "{listed}");
}

#[tokio::test]
async fn a_step_says_what_to_run_one_way_and_is_refused_with_two_or_with_none() {
    let rig = Rig::new().await;
    for (with, why) in [
        ("{ cmd: [printf, hello], cli: [help] }", "give one of `run`, `cmd` and `cli`, not more"),
        ("{ run: echo hello, cli: [help] }", "give one of `run`, `cmd` and `cli`, not more"),
        ("{ cwd: . }", "needs `run: <script>`, `cmd: [program, args...]` or `cli: [args...]`"),
    ] {
        let run = rig
            .run(
                &format!(
                    "blueprint: which\npermissions: [shell]\nsteps:\n  p: {{ do: shell, with: {with} }}\n"
                ),
                json!({}),
            )
            .await;
        assert_eq!(run.status, RunStatus::Failed, "{with}");
        assert_eq!(step(&run, "p").error.as_deref(), Some(why), "{with}");
    }
}

#[tokio::test]
async fn environment_and_directory_come_from_the_blueprint_the_step_and_the_action() {
    let rig = Rig::new().await;
    std::fs::create_dir_all(rig.path("sub")).unwrap();
    // A script of a user's own reads them as its shell does, and a
    // program started from a list of arguments has them as they are.
    let reads = script(
        r#"'echo "$GREETING $WHO$PUNCT"; basename "$PWD"'"#,
        "'echo %GREETING% %WHO%%PUNCT%& for %I in (.) do @echo %~nxI'",
    );
    let says = cmd(&["say", "%GREETING% %WHO%%PUNCT%", "say", "%:dir%"]);
    let run = rig
        .run(
            &format!(
                "\
blueprint: env
permissions: [shell]
params: {{ who: factory }}
env: {{ GREETING: hello, WHO: nobody }}
steps:
  say:
    do: shell
    env: {{ WHO: \"${{params.who}}\" }}
    with:
      cwd: sub
      env: {{ PUNCT: \"!\" }}
      run: {reads}
  told:
    do: shell
    env: {{ WHO: \"${{params.who}}\" }}
    with:
      cwd: sub
      env: {{ PUNCT: \"!\" }}
      cmd: {says}
"
            ),
            json!({}),
        )
        .await;
    assert_eq!(step(&run, "say").output["stdout"], "hello factory!\nsub");
    assert_eq!(step(&run, "told").output["stdout"], "hello factory!\nsub");
}

#[tokio::test]
async fn a_missing_directory_is_reported_before_anything_runs() {
    let rig = Rig::new().await;
    let run = rig.run("blueprint: lost\npermissions: [shell]\nsteps:\n  a: { do: shell, with: { run: ls, cwd: nowhere } }\n", json!({})).await;
    let error = step(&run, "a").error.clone().unwrap();
    assert!(
        error.starts_with("the working directory ") && error.ends_with("nowhere does not exist"),
        "{error}"
    );
}

#[tokio::test]
async fn a_step_that_times_out_leaves_no_process_behind() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let mut lines = rig.lines();
    // A program that starts another, which goes on by itself, and then
    // goes on too: the number of each, and then that both are there.
    let runs = cmd(&[
        "write", "pids", "%:pid%", "also", "(", "idle", ")", "append", "pids", "%:child%", "say", "ready",
        "idle",
    ]);
    let applied = rig
        .apply(
            &format!(
                "\
blueprint: runaway
permissions: [shell]
steps:
  build:
    do: shell
    timeout: 1m
    with:
      cmd: {runs}
"
            ),
            json!({}),
        )
        .await;
    line(&mut lines, "ready").await;
    let started = pids(&rig.path("pids"));
    assert_eq!(started.len(), 2);
    assert!(started.iter().all(|pid| alive(*pid)));

    rig.clock.sleeps_started(since + 1).await;
    rig.clock.advance(std::time::Duration::from_secs(60));
    let run = rig.finished(&applied).await;
    assert_eq!(step(&run, "build").error.as_deref(), Some("timed out after 1m"));
    gone(&started).await;
}

#[tokio::test]
async fn stopping_an_instance_stops_the_command_it_was_running() {
    let rig = Rig::new().await;
    let mut lines = rig.lines();
    let runs = cmd(&["write", "pids", "%:pid%", "say", "ready", "idle"]);
    let applied = rig
        .apply(
            &format!("blueprint: long\npermissions: [shell]\nsteps:\n  work: {{ do: shell, with: {{ cmd: {runs} }} }}\n"),
            json!({}),
        )
        .await;
    line(&mut lines, "ready").await;
    let started = pids(&rig.path("pids"));
    rig.engine.stop(&applied.instance).await.unwrap();
    assert_eq!(step(&rig.finished(&applied).await, "work").status, StepStatus::Cancelled);
    gone(&started).await;
}

#[tokio::test]
async fn a_command_survives_a_restart_of_the_conductor_and_is_picked_up_again() {
    let mut rig = Rig::new().await;
    let mut lines = rig.lines();
    // A program that says it has started and then waits to be told to
    // finish, which the test tells it once the conductor is another.
    let hold = Hold::new().await;
    let builds = cmd(&[
        "write",
        "pids",
        "%:pid%",
        "say",
        "started",
        "held",
        &hold.address(),
        "build",
        "say",
        "finished",
        "exit",
        "0",
    ]);
    let applied = rig
        .apply(
            &format!(
                "\
blueprint: survivor
permissions: [shell]
steps:
  build: {{ do: shell, with: {{ cmd: {builds} }} }}
  after: {{ do: shell, with: {{ run: \"echo ${{steps.build.out.code}}\" }} }}
"
            ),
            json!({}),
        )
        .await;
    line(&mut lines, "started").await;
    let started = pids(&rig.path("pids"));
    let held = hold.came().await;

    rig.restart().await;
    assert!(alive(started[0]), "the command must not die with the conductor");
    let mut lines = rig.lines();
    held.go().await;
    line(&mut lines, "finished").await;

    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok);
    assert_eq!(step(&run, "build").attempt, 1, "picked up, not run again");
    assert_eq!(step(&run, "build").output["code"], 0);
    assert_eq!(step(&run, "after").output["stdout"], "0");
    let log: Vec<String> = rig.engine.step_log(&run.id, "build", 0, 20).into_iter().map(|l| l.text).collect();
    assert!(
        log.iter().any(|text| text == &format!("picked up process {} after a restart", started[0])),
        "{log:?}"
    );
    assert_eq!(log.iter().filter(|text| *text == "started").count(), 1, "output is not replayed");
}

#[tokio::test]
async fn system_check_reports_resources_and_judges_requirements() {
    let rig = Rig::new().await;
    let source = "\
blueprint: room
permissions: [fleet]
params: { mem: 1, fail: false }
steps:
  check: { do: system.check, with: { min_free_mem_mb: ${params.mem}, min_free_disk_mb: 1, fail: ${params.fail} } }
  go: { when: ${ steps.check.out.ok }, do: noop }
";
    let run = rig.run(source, json!({})).await;
    let found = &step(&run, "check").output;
    assert_eq!(found["ok"], true);
    assert!(found["cpus"].as_u64().unwrap() >= 1 && found["total_mem_mb"].as_u64().unwrap() > 0);
    assert!(found["free_disk_mb"].as_u64().unwrap() > 0, "{found}");
    assert_eq!(step(&run, "go").status, StepStatus::Ok);

    let run = rig.run(source, json!({ "mem": 999_999_999 })).await;
    let found = &step(&run, "check").output;
    assert_eq!(found["ok"], false);
    assert!(found["reasons"][0].as_str().unwrap().ends_with("999999999 MB is needed"), "{found}");
    assert_eq!(step(&run, "go").status, StepStatus::Skipped, "the blueprint decides what to do about it");

    let run = rig.run(source, json!({ "mem": 999_999_999, "fail": true })).await;
    assert_eq!(run.status, RunStatus::Failed);
}

#[tokio::test]
async fn what_a_blueprint_runs_is_told_where_the_products_own_program_is() {
    let rig = Rig::new().await;
    let variable = hosting::PROGRAM_ENV;
    let says = cmd(&["say", &format!("%{variable}%")]);
    // A script of a user's own, as each system's shell reads a variable.
    let reads =
        if cfg!(windows) { format!("'echo %{variable}%'") } else { format!("'echo \"${variable}\"'") };
    let run = rig
        .run(
            &format!(
                "\
blueprint: itself
permissions: [shell]
steps:
  told: {{ do: shell, with: {{ cmd: {says} }} }}
  bare: {{ do: shell, with: {{ cmd: {says}, clear_env: true }} }}
  own: {{ do: shell, with: {{ cmd: {says}, env: {{ {variable}: /somewhere/else }} }} }}
  read: {{ do: shell, with: {{ run: {reads}, clear_env: true }} }}
"
            ),
            json!({}),
        )
        .await;
    // Here that is this test program; in a daemon it is the daemon, which
    // is also the command line.
    let here = std::env::current_exe().unwrap().to_string_lossy().into_owned();
    assert_eq!(step(&run, "told").output["stdout"], here);
    assert_eq!(step(&run, "bare").output["stdout"], here, "with an environment that is otherwise empty too");
    assert_eq!(
        step(&run, "read").output["stdout"],
        here,
        "and a script reads it there as its shell reads any"
    );
    assert_eq!(step(&run, "own").output["stdout"], "/somewhere/else", "unless the blueprint says otherwise");
}

#[tokio::test]
async fn what_a_blueprint_runs_is_told_where_the_daemon_answers() {
    let rig = Rig::new().await;
    let variable = hosting::ADDRESS_ENV;
    let says = cmd(&["say", &format!("[%{variable}%]")]);
    let source = format!(
        "\
blueprint: whereabouts
permissions: [shell]
params:
  again: false
steps:
  told: {{ do: shell, with: {{ cmd: {says} }} }}
  bare: {{ do: shell, with: {{ cmd: {says}, clear_env: true }} }}
  own: {{ do: shell, with: {{ cmd: {says}, env: {{ {variable}: http://elsewhere:1 }} }} }}
"
    );
    // Hosting listens for nobody. Until whoever does has said where, there
    // is nothing to tell.
    let run = rig.run(&source, json!({})).await;
    assert_eq!(step(&run, "told").output["stdout"], "[]");

    rig.hosting.answers_at("http://127.0.0.1:7700");
    let run = rig.run(&source, json!({ "again": true })).await;
    // The address and nothing with it: whatever goes there signs in.
    assert_eq!(step(&run, "told").output["stdout"], "[http://127.0.0.1:7700]");
    assert_eq!(step(&run, "bare").output["stdout"], "[http://127.0.0.1:7700]", "in an empty environment too");
    assert_eq!(
        step(&run, "own").output["stdout"],
        "[http://elsewhere:1]",
        "unless the blueprint says otherwise"
    );
}

#[tokio::test]
async fn what_a_blueprint_runs_is_told_what_the_factorys_profile_is_called() {
    let rig = Rig::new().await;
    let variable = hosting::PROFILE_ENV;
    // What it holds, or `nothing` where it is not there at all.
    let says = cmd(&["say", &format!("[%{variable}?nothing%]")]);
    let source = format!(
        "\
blueprint: whereabouts
permissions: [shell]
params:
  again: false
steps:
  told: {{ do: shell, with: {{ cmd: {says} }} }}
  bare: {{ do: shell, with: {{ cmd: {says}, clear_env: true }} }}
  own: {{ do: shell, with: {{ cmd: {says}, env: {{ {variable}: elsewhere }} }} }}
"
    );
    // Hosting does not know what the factory is. Until whoever started it
    // has said, the variable is not there at all.
    let run = rig.run(&source, json!({})).await;
    assert_eq!(step(&run, "told").output["stdout"], "[nothing]");

    rig.hosting.in_profile("dev1");
    let run = rig.run(&source, json!({ "again": true })).await;
    assert_eq!(step(&run, "told").output["stdout"], "[dev1]");
    assert_eq!(step(&run, "bare").output["stdout"], "[dev1]", "in an empty environment too");
    assert_eq!(step(&run, "own").output["stdout"], "[elsewhere]", "unless the blueprint says otherwise");

    // A service is a process hosting starts itself, and starts again: it is told too.
    let mut lines = rig.journal.follow(rig.journal.last_seq(), "log.line");
    let serves = cmd(&["say", &format!("in %{variable}%"), "idle"]);
    let service = format!(
        "\
blueprint: kept
permissions: [fleet]
steps:
  start: {{ do: service.start, with: {{ name: kept, cmd: {serves} }} }}
"
    );
    rig.apply(&service, json!({})).await;
    crate::soon("the service to say where it is", async {
        loop {
            let line = lines.next().await.unwrap();
            if line.data["text"].as_str().is_some_and(|text| text.contains("in dev1")) {
                return;
            }
        }
    })
    .await;
}

#[tokio::test]
async fn a_factory_with_no_profile_tells_what_it_runs_the_empty_text() {
    let rig = Rig::new().await;
    rig.hosting.in_profile("");
    let says = cmd(&["say", &format!("[%{}?nothing%]", hosting::PROFILE_ENV)]);
    let source = format!(
        "\
blueprint: unnamed
permissions: [shell]
steps:
  told: {{ do: shell, with: {{ cmd: {says} }} }}
"
    );
    // There, and empty: a program that is strict about what is not set can still ask.
    let run = rig.run(&source, json!({})).await;
    assert_eq!(step(&run, "told").output["stdout"], "[]");
}
