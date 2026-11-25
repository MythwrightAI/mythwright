//! The stand-in for a program (`crates/hosting/standin`): how a test has
//! a blueprint start it, how one is held until the test lets it go, and
//! that it does what its words say.
//!
//! It is a program and not a line of a shell, so that what is held here
//! of programs being started, stopped and found again is held on every
//! system the factory is built for. A test that is about a script of a
//! user's own being read by their shell still writes a script.

use std::process::{Command, Stdio};

use crate::{STANDIN, soon};

// What every suite that starts the stand-in says to it is beside the
// program, and named from here.
#[path = "../../standin/cmd.rs"]
mod cmd;
#[path = "../../standin/hold.rs"]
mod hold;
#[path = "../../standin/holds.rs"]
mod holds;
#[path = "../../standin/let_go.rs"]
mod let_go;
#[path = "../../standin/line.rs"]
mod line;
#[path = "../../standin/listening.rs"]
mod listening;
pub use cmd::{cmd, word};
pub use hold::Hold;
pub use line::line;

/// What the stand-in said and how it ended, having done `words`.
fn does(words: &[&str]) -> (Option<i32>, String, String) {
    let ended = Command::new(STANDIN).args(words).stdin(Stdio::null()).output().unwrap();
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
    (ended.status.code(), text(&ended.stdout), text(&ended.stderr))
}

fn scratch(label: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("standin-{label}-{}", &kernel::id::secret()[..12]));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_stand_in_says_what_it_is_told_where_it_is_told_and_ends_with_the_code_it_is_given() {
    assert_eq!(
        does(&["say", "hello", "err", "oops", "say", "done"]),
        (Some(0), "hello\ndone\n".into(), "oops\n".into())
    );
    assert_eq!(does(&["say", "last words", "exit", "7", "say", "never"]).0, Some(7));
    assert_eq!(does(&[]), (Some(0), String::new(), String::new()));
    // Its arguments as they arrived: nothing read into any of them.
    let (_, said, _) = does(&["args", "a b", "$HOME", "%PATH%", "", "it's \"quoted\""]);
    assert_eq!(said, "a b|$HOME|%PATH%||it's \"quoted\"\n");
    // A word it does not know, or one with too little after it, is said
    // and is not a way of ending well.
    let (code, _, why) = does(&["say", "so far", "dance"]);
    assert_eq!((code, why.trim()), (Some(2), "the stand-in: `dance` is no word it knows"));
    let (code, _, why) = does(&["write", "only-a-file"]);
    assert_eq!((code, why.trim()), (Some(2), "the stand-in: `write` takes a file and a text after it"));
}

#[test]
fn the_stand_in_reads_its_variables_and_what_it_is_handed() {
    let ran = |words: &[&str], given: &str| {
        use std::io::Write;
        let mut child = Command::new(STANDIN)
            .args(words)
            .env("STANDIN_SAYS", "said")
            .env("STANDIN_EMPTY", "")
            .env_remove("STANDIN_UNSET")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(given.as_bytes()).unwrap();
        let ended = child.wait_with_output().unwrap();
        (ended.status.code(), String::from_utf8_lossy(&ended.stdout).into_owned())
    };
    // A variable, one that is empty, one that is not there, and what is
    // said in its place where it is not there at all.
    let (_, said) = ran(&["say", "[%STANDIN_SAYS%] [%STANDIN_EMPTY%] [%STANDIN_UNSET%] 100%% of 1%"], "");
    assert_eq!(said, "[said] [] [] 100% of 1%\n");
    let (_, said) = ran(&["say", "[%STANDIN_SAYS?none%] [%STANDIN_EMPTY?none%] [%STANDIN_UNSET?none%]"], "");
    assert_eq!(said, "[said] [] [none]\n");
    assert_eq!(
        ran(&["lacking", "STANDIN_UNSET", "handed nothing", "say", "never"], ""),
        (Some(0), "handed nothing\n".into())
    );
    assert_eq!(
        ran(&["lacking", "STANDIN_EMPTY", "handed nothing", "say", "there"], ""),
        (Some(0), "there\n".into())
    );
    // Its own number, which is the number of the process that was started.
    let child = Command::new(STANDIN).args(["say", "%:pid%"]).stdout(Stdio::piped()).spawn().unwrap();
    let number = child.id().to_string();
    assert_eq!(String::from_utf8_lossy(&child.wait_with_output().unwrap().stdout).trim(), number);
    // What it is handed to read: said back as it is, or read and not said.
    assert_eq!(
        ran(&["say", "first", "cat", "say", "last"], "one\ntwo"),
        (Some(0), "first\none\ntwolast\n".into())
    );
    assert_eq!(ran(&["swallow", "say", "read"], "one\ntwo\n"), (Some(0), "read\n".into()));
    // A variable said by its name, with nothing a shell would read, and
    // a line put together a piece at a time.
    let (_, said) = ran(
        &["put", "heard ", "var", "STANDIN_SAYS", "put", " and ", "var", "STANDIN_UNSET", "say", "."],
        "",
    );
    assert_eq!(said, "heard said and .\n");
    // It goes on where a variable holds something, and not where it is
    // empty or is not there.
    assert_eq!(ran(&["given", "STANDIN_SAYS", "say", "handed"], ""), (Some(0), "handed\n".into()));
    assert_eq!(ran(&["given", "STANDIN_EMPTY", "say", "handed"], ""), (Some(1), String::new()));
    assert_eq!(ran(&["given", "STANDIN_UNSET", "say", "handed"], ""), (Some(1), String::new()));
    // What it was started with, a line each: by a part of a name or of
    // what one holds, and with no part all of it.
    assert_eq!(
        ran(&["env", "STANDIN_S", "env", "=sai", "env", "STANDIN_UNSET"], "").1,
        "STANDIN_SAYS=said\n".repeat(2)
    );
    let (_, all) = ran(&["env", ""], "");
    assert!(all.lines().any(|line| line == "STANDIN_EMPTY=") && all.lines().count() > 2, "{all}");
}

#[test]
fn the_stand_in_writes_files_and_goes_by_what_is_in_them() {
    let dir = scratch("files");
    let at = |name: &str| dir.join(name).to_string_lossy().into_owned();
    let (notes, flag) = (at("notes"), at("flag"));
    assert_eq!(does(&["write", &notes, "one", "write", &notes, "two", "append", &notes, "three"]).0, Some(0));
    assert_eq!(std::fs::read_to_string(&notes).unwrap(), "two\nthree\n");
    // It goes on where a file is there, or says what was expected, and
    // ends there where not.
    assert_eq!(does(&["has", &notes, "say", "there"]), (Some(0), "there\n".into(), String::new()));
    assert_eq!(does(&["has", &flag, "say", "there"]), (Some(1), String::new(), String::new()));
    assert_eq!(does(&["lacks", &flag, "say", "not there"]).1, "not there\n");
    assert_eq!(does(&["lacks", &notes, "say", "not there"]), (Some(1), String::new(), String::new()));
    assert_eq!(
        does(&["needs", &flag, "there is no flag", "say", "there"]),
        (Some(1), "there is no flag\n".into(), String::new())
    );
    assert_eq!(does(&["needs", &notes, "there are no notes", "say", "there"]).1, "there\n");
    std::fs::write(&flag, "up  here\n").unwrap();
    assert_eq!(does(&["holds", &flag, "up  here", "say", "same"]).1, "same\n");
    assert_eq!(does(&["holds", &flag, "up here", "say", "same"]), (Some(1), String::new(), String::new()));
    assert_eq!(does(&["holds", &at("nothing"), "up  here"]).0, Some(1));
    // And where it has fewer lines than it is to have.
    assert_eq!(
        does(&["fewer", &notes, "3", "only two", "say", "enough"]),
        (Some(1), "only two\n".into(), String::new())
    );
    assert_eq!(does(&["fewer", &notes, "2", "only two", "say", "enough"]).1, "enough\n");
    // Something done where a file is there, and then nothing more.
    let badly = ["(", "err", "it went badly", "exit", "3", ")", "say", "it went well"];
    let went = |file: &str| does(&[&["there", file][..], &badly[..]].concat());
    assert_eq!(went(&notes), (Some(3), String::new(), "it went badly\n".into()));
    assert_eq!(went(&at("nothing")), (Some(0), "it went well\n".into(), String::new()));
    // The first time, it makes the file and ends; after that it goes on.
    let again = at("again");
    assert_eq!(does(&["first", &again, "3", "say", "second life"]), (Some(3), String::new(), String::new()));
    assert_eq!(
        does(&["first", &again, "3", "say", "second life"]),
        (Some(0), "second life\n".into(), String::new())
    );
    // Where it is: what the folder it was started in is called.
    let inside = |words: &[&str]| {
        let ended = Command::new(STANDIN).args(words).current_dir(&dir).output().unwrap();
        (ended.status.code(), String::from_utf8_lossy(&ended.stdout).trim().to_string())
    };
    let called = dir.file_name().unwrap().to_string_lossy().into_owned();
    assert_eq!(inside(&["say", "%:dir%"]), (Some(0), called));
    assert_eq!(inside(&["say", "%:cwd%"]).1, dir.display().to_string());
    // And whether anything there is called something that starts so.
    assert_eq!(inside(&["any", "not", "say", "some"]), (Some(0), "some".into()));
    assert_eq!(inside(&["any", "nothing-", "say", "some"]), (Some(1), String::new()));
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn a_stand_in_is_held_until_it_is_told_and_a_copy_it_started_goes_on_without_it() {
    let hold = Hold::new().await;
    let at = hold.address();
    // It starts a copy of itself, which is held twice; says the copy's
    // number; and ends. What it said is read to its end, which is when
    // the copy has let go of it too.
    let words =
        ["also", "(", "held", &at, "copy", "say", "told %:told%", "held", &at, "again", "say", "over", ")"];
    let child = tokio::process::Command::new(STANDIN)
        .args(words)
        .args(["say", "copy %:child%"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let said = tokio::spawn(child.wait_with_output());
    let first = hold.came().await;
    assert_eq!(first.who, "copy");
    first.told("this").await;
    // Let go of with no word, it goes on as well.
    let second = hold.came().await;
    assert_eq!(second.who, "again");
    drop(second);
    let ended = soon("the stand-in and its copy to have said everything", said).await.unwrap().unwrap();
    let said = String::from_utf8_lossy(&ended.stdout).into_owned();
    assert_eq!(ended.status.code(), Some(0));
    // The two say what they say side by side, so which line of the one
    // comes before which of the other is the machine's to say: each is
    // read for itself.
    let (own, copys): (Vec<&str>, Vec<&str>) = said.lines().partition(|line| line.starts_with("copy "));
    assert!(matches!(own[..], [line] if line["copy ".len()..].parse::<u32>().is_ok()), "{said}");
    assert_eq!(copys, ["told this", "over"], "{said}");
}

#[tokio::test]
async fn a_stand_in_told_to_wait_where_nothing_listens_is_held_by_nobody_and_goes_on() {
    // An address nothing listens at for as long as the socket is kept: it
    // has the port, and does not listen.
    let nobody = tokio::net::TcpSocket::new_v4().unwrap();
    nobody.bind("127.0.0.1:0".parse().unwrap()).unwrap();
    let at = nobody.local_addr().unwrap().to_string();
    let ended = tokio::process::Command::new(STANDIN)
        .args(["held", &at, "anybody", "say", "went on [%:told%]"])
        .stdin(Stdio::null())
        .output();
    let ended = soon("the stand-in to go on", ended).await.unwrap();
    assert_eq!(String::from_utf8_lossy(&ended.stdout), "went on []\n");
}

#[test]
fn the_stand_in_goes_by_what_it_read_and_says_a_file_where_there_is_one() {
    use std::io::Write;
    let dir = scratch("read");
    let ran = |words: &[&str], given: &str| {
        let mut child = Command::new(STANDIN)
            .args(words)
            .current_dir(&dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(given.as_bytes()).unwrap();
        let ended = child.wait_with_output().unwrap();
        (ended.status.code(), String::from_utf8_lossy(&ended.stdout).into_owned())
    };
    // Where what it read has a part in it, it does one thing and ends;
    // where not, it goes on to the other.
    let asks = ["read", "where", "has been answered", "(", "say", "done", "exit", "4", ")", "say", "which?"];
    assert_eq!(ran(&asks, "It has been answered: with tax."), (Some(4), "done\n".into()));
    assert_eq!(ran(&asks, "Add a cart."), (Some(0), "which?\n".into()));
    assert_eq!(ran(&["read", "put", "%:read%"], "as it was\nhanded"), (Some(0), "as it was\nhanded".into()));
    // What is between two things, in the first line that begins with the one.
    let card = ["read", "pick", "Card ", ":", "say", "[%:picked%]"];
    assert_eq!(ran(&card, "A Card no: 1\nCard shop-7: Add a cart: now\nCard shop-8: next").1, "[shop-7]\n");
    assert_eq!(ran(&card, "nothing of the kind").1, "[]\n");
    // A number of what it read: the same for the same, and another for another.
    let sum = |given: &str| ran(&["read", "say", "%:sum%"], given).1;
    assert_eq!(sum("one thing"), sum("one thing"));
    assert_ne!(sum("one thing"), sum("another"));
    assert!(sum("").trim().parse::<u64>().is_ok());
    // A file where there is one, as it is, and then nothing more; and a
    // file written where its folder was not there yet.
    let said = ["says", "words/said", "say", "nothing was written for it"];
    assert_eq!(ran(&said, ""), (Some(0), "nothing was written for it\n".into()));
    assert_eq!(ran(&["write", "words/said", "what was written"], "").0, Some(0));
    assert_eq!(ran(&said, ""), (Some(0), "what was written\n".into()));
    let _ = std::fs::remove_dir_all(dir);
}

/// The stand-in doing `words`, begun and not waited for: what it says.
fn begun(words: &[&str]) -> tokio::task::JoinHandle<std::io::Result<std::process::Output>> {
    let child = tokio::process::Command::new(STANDIN)
        .args(words)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    tokio::spawn(child.wait_with_output())
}

async fn said(ended: tokio::task::JoinHandle<std::io::Result<std::process::Output>>) -> String {
    let ended = soon("a stand-in to have ended", ended).await.unwrap().unwrap();
    String::from_utf8_lossy(&ended.stdout).into_owned()
}

#[tokio::test]
async fn a_stand_in_waits_at_a_file_a_test_made_and_at_none_that_is_not_there() {
    let dir = scratch("holds");
    let at = |name: &str| dir.join(name);
    let place = |name: &str| at(name).to_string_lossy().into_owned();
    // Told to wait where nothing was made, nobody holds it.
    assert_eq!(said(begun(&["held", &place("own"), "it", "say", "went on"])).await, "went on\n");

    // One hold for everybody, and one of somebody's own, which that one
    // waits at in place of the other.
    holds::hold(&at("all"));
    holds::hold(&at("own"));
    let either = format!("{}|{}", place("own"), place("all"));
    let mut heard = listening::Listening::at(&at("began"));
    let waits = |who: &str, at: &str| begun(&["tell", &place("began"), who, "held", at, who, "say", who]);
    let (first, second, own) =
        (waits("first", &place("all")), waits("second", &place("all")), waits("own", &either));
    let mut came = Vec::new();
    for _ in 0..3 {
        came.push(soon("a stand-in to say it began", heard.next_line()).await.unwrap().unwrap());
    }
    came.sort();
    assert_eq!(came, ["first", "own", "second"]);
    // So many are let go at the one, whether they wait there already
    // or have yet to come; the one at its own is not.
    holds::pass(&at("all"), 2);
    assert_eq!((said(first).await, said(second).await), ("first\n".into(), "second\n".into()));
    assert!(!own.is_finished());
    holds::pass(&at("own"), 2);
    assert_eq!(said(own).await, "own\n");
    assert_eq!(said(waits("later", &either)).await, "later\n");
    // And whoever is waiting is let go, once somebody is: asked before
    // anybody has come, that is waited for.
    let third = waits("third", &either);
    let_go::let_go("the third to be waiting", &at("own")).await;
    assert_eq!(said(third).await, "third\n");
    let _ = std::fs::remove_dir_all(dir);
}
