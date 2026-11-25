//! A user's own script, and what has their system's shell read it
//! (`hosting::script`).
//!
//! What is Windows' is a function of text (the line its shell is given)
//! and of a folder (where that shell is), played here on every system.
//! What a shell then makes of a line is run on the system whose shell it
//! is: the tests marked for Windows are run there, and the others
//! everywhere else.

use std::path::PathBuf;

use hosting::script::{SHELL, command, line, refused, shell};

#[test]
fn the_line_the_windows_shell_is_given_is_the_script_as_it_was_written_between_two_quotation_marks() {
    let cases: &[(&str, &str, &str)] = &[
        // (what it is, the script, the line)
        ("a plain line", "echo hello", r#""echo hello""#),
        ("quotation marks, which are the shell's to read", r#"echo "a b""#, r#""echo "a b"""#),
        (
            "one that begins and ends with a quotation mark",
            r#""my tool.cmd" "a b""#,
            r#"""my tool.cmd" "a b"""#,
        ),
        (
            "what joins commands and sends what they say somewhere",
            r#"dir /b & type notes.txt | find "x" > found.txt 2>&1"#,
            r#""dir /b & type notes.txt | find "x" > found.txt 2>&1""#,
        ),
        ("a variable of the environment", "echo %FACTORY_URL% %PORT%", r#""echo %FACTORY_URL% %PORT%""#),
        (
            "a caret, which is how that shell says a sign is only a sign",
            "echo a^&b ^> c",
            r#""echo a^&b ^> c""#,
        ),
        (
            "a backslash before a quotation mark, which is two signs there",
            r#"dir "C:\My Work\""#,
            r#""dir "C:\My Work\"""#,
        ),
        ("an empty one, which does nothing", "", r#""""#),
        ("one of nothing but room", "  \n\t\n", r#""""#),
        ("one line and its end, as a block of a blueprint is read", "echo hello\n", r#""echo hello""#),
        ("the same as that system ends a line", "echo hello\r\n", r#""echo hello""#),
        ("one line among lines with nothing on them", "\n\n  echo hello  \n\n", r#""  echo hello  ""#),
    ];
    for (what, script, given) in cases {
        assert_eq!(line(script).as_deref(), Ok(*given), "{what}");
    }
}

#[test]
fn a_script_of_several_lines_is_refused_for_the_windows_shell_in_a_sentence_that_says_what_to_write() {
    for (script, lines) in [
        ("echo one\necho two", 2),
        ("echo one\r\necho two\r\n", 2),
        ("if exist a (\n  echo a\n)\n", 3),
        ("\necho one\n\necho two\n\necho three\n", 3),
    ] {
        let why = line(script).expect_err(script);
        for words in [
            "several lines",
            "reads the one line it is started with (`cmd /C`)",
            &format!("this one has {lines}"),
            "Nothing was started",
            "`run: build.cmd`",
            "`&`",
        ] {
            assert!(why.contains(words), "`{words}` is not in: {why}");
        }
    }
    // Only there: every other shell is handed the script whole.
    assert_eq!(refused("echo one\necho two").is_some(), cfg!(windows));
    assert_eq!(refused("echo one"), None);
}

#[test]
fn the_windows_shell_is_where_the_system_keeps_it_and_is_named_only_where_that_is_not_found() {
    let top =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("script-{}", &kernel::id::secret()[..12]));
    let kept = top.join("system/System32");
    std::fs::create_dir_all(&kept).unwrap();
    std::fs::write(kept.join(SHELL), "").unwrap();
    std::fs::create_dir_all(top.join("empty/System32")).unwrap();
    std::fs::create_dir_all(top.join("folder/System32").join(SHELL)).unwrap();

    let named = PathBuf::from(SHELL);
    let cases: &[(&str, Option<PathBuf>, &PathBuf)] = &[
        ("the system's folder holds it", Some(top.join("system")), &kept.join(SHELL)),
        ("the system's folder holds none", Some(top.join("empty")), &named),
        ("what is called so there is a folder", Some(top.join("folder")), &named),
        ("the system's folder is not there", Some(top.join("gone")), &named),
        ("the system's folder was not said", None, &named),
        ("it was said as nothing", Some(PathBuf::new()), &named),
    ];
    for (what, system, found) in cases {
        assert_eq!(&shell(system.as_deref()), *found, "{what}");
    }
    let _ = std::fs::remove_dir_all(&top);
}

/// What `made` would start, as the words it was given.
fn words(made: &std::process::Command) -> Vec<String> {
    let program = made.get_program().to_string_lossy().into_owned();
    std::iter::once(program).chain(made.get_args().map(|word| word.to_string_lossy().into_owned())).collect()
}

#[cfg(not(windows))]
#[test]
fn every_other_shell_is_handed_the_script_whole_as_one_argument() {
    let script = "echo \"a b\"\necho two";
    assert_eq!(words(&command(script).unwrap()), ["/bin/sh", "-c", script]);
    assert_eq!(words(&command("").unwrap()), ["/bin/sh", "-c", ""]);
}

/// Windows' alone: what its shell is started with.
#[cfg(windows)]
#[test]
fn the_windows_shell_is_started_with_its_three_words_and_then_the_line_as_it_is() {
    let made = words(&command(r#"echo "a b""#).unwrap());
    assert!(std::path::Path::new(&made[0]).ends_with(SHELL), "{made:?}");
    let [first, second, third] = hosting::script::READS;
    assert_eq!(made[1..], [first, second, third, r#""echo "a b"""#]);
    assert!(command("echo one\necho two").is_err());
}

/// Windows' alone: what that shell then makes of each line, which is
/// what the line is written for.
#[cfg(windows)]
#[test]
fn the_windows_shell_reads_a_line_as_it_would_be_typed() {
    for (script, said) in [
        (r#"echo "a b""#, "\"a b\"\r\n"),
        ("echo one& echo two", "one\r\ntwo\r\n"),
        ("echo a^&b", "a&b\r\n"),
        ("echo %SCRIPT_SAYS%", "said\r\n"),
        (r#"echo one| findstr "o n""#, "one\r\n"),
        ("", ""),
    ] {
        let ended = command(script).unwrap().env("SCRIPT_SAYS", "said").output().unwrap();
        assert_eq!(
            (ended.status.code(), String::from_utf8_lossy(&ended.stdout).as_ref()),
            (Some(0), said),
            "{script}"
        );
    }
}

/// Windows' alone: that shell puts what a variable holds into its line
/// before it reads the line, so what the variable holds is read as part
/// of the command. A line of a user's own that writes `%NAME%` of
/// something somebody else said (a bot's `%MESSAGE%`) has that run. A
/// program that reads the variable itself, which the line only names,
/// is handed it as it is.
#[cfg(windows)]
#[test]
fn the_windows_shell_reads_what_a_variable_holds_as_part_of_its_line_and_a_program_named_there_does_not() {
    let holds = "this& echo and ran that";
    let said = |script: &str| {
        let ended = command(script).unwrap().env("SCRIPT_SAYS", holds).output().unwrap();
        String::from_utf8_lossy(&ended.stdout).into_owned()
    };
    assert_eq!(said("echo heard %SCRIPT_SAYS%"), "heard this\r\nand ran that\r\n");
    let names = crate::standin::line(&["put", "heard ", "var", "SCRIPT_SAYS"]);
    assert_eq!(said(&names), "heard this& echo and ran that");
}

#[tokio::test]
async fn a_step_and_a_health_check_have_the_shell_read_a_script_with_quotation_marks_as_it_was_written() {
    use crate::{Rig, step};
    use serde_json::json;
    let rig = Rig::new().await;
    // The same three things for each shell, each as that shell writes
    // them: a step that says words between quotation marks, a service
    // that writes some into a file and goes on, and a check that reads
    // the file for them.
    #[cfg(not(windows))]
    let (source, said) = (
        r#"
blueprint: read
permissions: [shell, fleet]
steps:
  said:
    do: shell
    with:
      run: |
        echo "a  b" 'c  d' "$FACTORY_ENVIRONMENT."
  served:
    do: service.start
    with:
      name: read-web
      run: |
        echo "up  here" > up.txt; i=0; while [ $i -lt 3000 ]; do i=$((i+1)); sleep 0.2; done
      health:
        cmd: test "$(cat up.txt)" = "up  here"
        interval: 100ms
  ready: { do: service.wait_healthy, with: { service: read-web, timeout: 60s } }
"#
        .to_string(),
        "a  b c  d .",
    );
    // That shell says quotation marks as it was given them, and has no
    // command that goes on: what goes on is the stand-in, named by its
    // file between quotation marks of its own.
    #[cfg(windows)]
    let (source, said) = (
        format!(
            r#"
blueprint: read
permissions: [shell, fleet]
steps:
  said:
    do: shell
    with:
      run: |
        echo "a  b" 'c  d'
  served:
    do: service.start
    with:
      name: read-web
      run: |
        echo "up  here"> up.txt& "{}" idle
      health:
        cmd: findstr /C:"up  here" up.txt
        interval: 100ms
  ready: {{ do: service.wait_healthy, with: {{ service: read-web, timeout: 60s }} }}
"#,
            crate::STANDIN
        ),
        r#""a  b" 'c  d'"#,
    );
    let applied = rig.apply(&source, json!({})).await;
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, conductor::RunStatus::Ok, "{:?}", run.error);
    assert_eq!(step(&run, "said").output["stdout"].as_str(), Some(said));
    assert_eq!(step(&run, "ready").output["healthy"], true);
    rig.engine.stop(&applied.instance).await.unwrap();
    rig.event("service.stopped", |_| true).await;
}
