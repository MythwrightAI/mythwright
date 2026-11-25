//! What a process knows of the groups it may be asked to stop, where a
//! group cannot be reached by its leader's number (Windows), and how a
//! wrapper there hears that it is to end.
//!
//! None of that system's own is here: this is the table `stop` goes by
//! there, and the looking a wrapper does, each of which is the same
//! wherever it is built. What only that system can say (that a job ends
//! what is in it when it is let go) is held where this suite is run
//! there, by every test that stops a program: `shell.rs`, `services.rs`.

use std::path::PathBuf;
use std::time::Duration;

use hosting::group::{Known, Stops, hears, word};

/// What is said of a number, in the order it is said.
enum Said {
    /// A program started with no wrapper, and the hold on its group.
    Led(u32, &'static str),
    /// A wrapper, and where word is left for it.
    Wrapped(u32, &'static str),
    /// Seen to end.
    Ended(u32),
    /// Stopped.
    Stop(u32),
}
use Said::{Ended, Led, Stop, Wrapped};

struct Case {
    what: &'static str,
    said: &'static [Said],
    /// The holds that were handed back to be let go along the way.
    let_go: &'static [&'static str],
    /// What stopping a number then comes to.
    then: (u32, Stops<&'static str>),
}

fn leaves(at: &str) -> Stops<&'static str> {
    Stops::Leaves(PathBuf::from(at))
}

const NOTHING: Stops<&'static str> = Stops::Nothing;

fn cases() -> Vec<Case> {
    vec![
        Case { what: "a number nothing was said of was stopped", said: &[], let_go: &[], then: (7, NOTHING) },
        Case {
            what: "a group that is held ends by its hold being let go",
            said: &[Led(7, "job")],
            let_go: &[],
            then: (7, Stops::Ends("job")),
        },
        Case {
            what: "and is known no longer once it was stopped",
            said: &[Led(7, "job"), Stop(7)],
            let_go: &["job"],
            then: (7, NOTHING),
        },
        Case {
            what: "a wrapper is left word where it hears",
            said: &[Wrapped(7, "run/a.exit.stop")],
            let_go: &[],
            then: (7, leaves("run/a.exit.stop")),
        },
        Case {
            what: "and may be left word again: asked and then not asked",
            said: &[Wrapped(7, "run/a.exit.stop"), Stop(7)],
            let_go: &[],
            then: (7, leaves("run/a.exit.stop")),
        },
        Case {
            what: "one of them stopped leaves the other as it was",
            said: &[Led(7, "job"), Wrapped(8, "run/a.exit.stop"), Stop(8)],
            let_go: &[],
            then: (7, Stops::Ends("job")),
        },
        Case {
            what: "a group whose leader was seen to end is let go then, and was stopped",
            said: &[Led(7, "job"), Ended(7)],
            let_go: &["job"],
            then: (7, NOTHING),
        },
        Case {
            what: "a wrapper that was seen to end is left no word: its number may be anybody's",
            said: &[Wrapped(7, "run/a.exit.stop"), Ended(7)],
            let_go: &[],
            then: (7, NOTHING),
        },
        Case {
            what: "a number given again to a program on pipes is that program's, and the hold before it is let go",
            said: &[Led(7, "before"), Led(7, "now")],
            let_go: &["before"],
            then: (7, Stops::Ends("now")),
        },
        Case {
            what: "a wrapper's number given to a program on pipes is no wrapper's any more",
            said: &[Wrapped(7, "run/a.exit.stop"), Led(7, "job"), Stop(7)],
            let_go: &["job"],
            then: (7, NOTHING),
        },
        Case {
            what: "a held group's number given to a wrapper is the wrapper's, and the hold is let go",
            said: &[Led(7, "job"), Wrapped(7, "run/a.exit.stop")],
            let_go: &["job"],
            then: (7, leaves("run/a.exit.stop")),
        },
        Case {
            what: "a wrapper found again where it hears elsewhere is left word there",
            said: &[Wrapped(7, "run/a.exit.stop"), Wrapped(7, "run/b.exit.stop")],
            let_go: &[],
            then: (7, leaves("run/b.exit.stop")),
        },
    ]
}

#[test]
fn what_stopping_a_group_comes_to_is_what_was_last_said_of_its_number() {
    for case in cases() {
        let mut known: Known<&'static str> = Known::new();
        let mut let_go = Vec::new();
        for said in case.said {
            let handed = match said {
                Led(leader, job) => known.led(*leader, job),
                Wrapped(leader, at) => known.wrapped(*leader, PathBuf::from(at)),
                Ended(leader) => known.ended(*leader),
                Stop(leader) => match known.stop(*leader) {
                    Stops::Ends(job) => Some(job),
                    Stops::Leaves(_) | Stops::Nothing => None,
                },
            };
            let_go.extend(handed);
        }
        assert_eq!(let_go, case.let_go, "{}: what was let go along the way", case.what);
        let (leader, comes_to) = case.then;
        assert_eq!(known.stop(leader), comes_to, "{}", case.what);
    }
}

#[test]
fn a_wrappers_word_is_a_file_beside_the_one_it_says_how_its_program_ended_in_called_by_its_number() {
    assert_eq!(word(&PathBuf::from("run/step.exit"), 4242), PathBuf::from("run/step.exit.4242.stop"));
    // The next wrapper under the same files has another number, and
    // does not end on a word that was left for this one.
    assert_ne!(word(&PathBuf::from("run/step.exit"), 4243), word(&PathBuf::from("run/step.exit"), 4242));
}

#[test]
fn a_wrapper_goes_on_until_word_is_left_for_it_and_takes_the_word_away_as_it_hears_it() {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("word-{}", &kernel::id::secret()[..12]));
    std::fs::create_dir_all(&dir).unwrap();
    let at = word(&dir.join("step.exit"), 4242);
    let (heard, hearing) = std::sync::mpsc::channel();
    let listens = {
        let at = at.clone();
        std::thread::spawn(move || {
            hears(&at, Duration::from_millis(1));
            let _ = heard.send(());
        })
    };
    // Nothing was left for it: neither the file beside it nor a word
    // for another wrapper under the same files is its word.
    std::fs::write(dir.join("step.exit"), "0").unwrap();
    std::fs::write(word(&dir.join("step.exit"), 4243), "").unwrap();
    assert!(hearing.try_recv().is_err(), "it heard a word nobody left for it");
    std::fs::write(&at, "").unwrap();
    hearing.recv_timeout(Duration::from_secs(30)).expect("it never heard the word that was left for it");
    listens.join().unwrap();
    assert!(!at.exists(), "the word is still there, for whatever has this number next");
    assert!(word(&dir.join("step.exit"), 4243).exists(), "another wrapper's word was taken away");
    // A word that is there before the looking begins is heard at once.
    std::fs::write(&at, "").unwrap();
    hears(&at, Duration::from_secs(3600));
    let _ = std::fs::remove_dir_all(&dir);
}
