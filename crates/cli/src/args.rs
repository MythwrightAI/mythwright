//! Command-line arguments, parsed by hand.
//!
//! The grammar is small (`<words...> [--flag] [--name value]`), so a parser
//! library would cost more to compile than this file is long.
//!
//! A flag is taken only by a command that reads it. What a command reads
//! is noted as it asks (`has`, `value`, `values`, `raw_of`), and a flag
//! that was given and never asked for is refused before anything is sent
//! (`Heed::check`, which the client asks before its first request) and,
//! for a command that sends nothing, as it ends: a program from before a
//! switch was answered as though all were well, having done what it was
//! asked without it. So a command reads every flag it takes before it
//! sends anything, and a program built to be tested holds it to that.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Flags that are followed by a value. Anything else starting with `--` is
/// a switch.
const TAKES_VALUE: &[&str] = &[
    "url",
    "workers",
    "env",
    "profile",
    "token",
    "data",
    "from",
    "db",
    "listen",
    "note",
    "attach",
    "out",
    "ui",
    "projects",
    "blueprints",
    "packages",
    "device",
    "adapter",
    "path",
    "without",
    "project",
    "environment",
    "workspace",
    "param",
    "key",
    "body",
    "status",
    "field",
    "channel",
    "model",
    "permission",
    "kind",
    "limit",
    "topics",
    "since",
    "by",
    "allow",
    "instance",
    "scope",
    "email",
    "name",
    "password",
    "invite",
    "before",
    "step",
    "grows",
    "against",
    "holds",
    "over",
    "most",
    "least",
    "why",
    "reply-to",
    "mark",
    "thread",
    "label",
    "instructions",
    "command",
    "signin",
    "reads",
    "git",
    "changes",
    "walls",
    "for",
    "file",
    "folder",
    "at-once",
    "at",
    "expires",
    "topic",
    "secret",
    "setup",
    "header",
    "prefix",
    "encoding",
    "signed",
    "time-header",
    "time-prefix",
    "within",
    "role",
    "of",
    "way",
    "share",
    "with",
    "often",
    "per",
    "also",
    "goes",
    "lands",
    "keep",
    "count",
    "prompt",
    "to",
    "member",
    "agent",
    "as",
    "subject",
    "on",
    "after",
    "under",
    "reason",
    "part",
    "why",
    "in",
    "rename",
    "remove",
];

/// The flags every command takes: which daemon, as whom, how it is said,
/// and how many threads this program works on. They are read before the
/// command is (`Client::new`, `serve::workers`), or by the commands that
/// talk to no daemon where they mean something to them. `--env` is the
/// old spelling of `--profile`, and is taken without being said.
/// They are said once, with the rest of what the command line says of
/// itself (`manual`), so that `help --json` says the same.
pub use manual::EVERYWHERE;

#[derive(Debug, Default)]
pub struct Args {
    words: Vec<String>,
    flags: Vec<(String, Option<String>)>,
    /// Everything as it was typed, for a command whose flags are done in
    /// the order they stand, each with the words after it.
    raw: Vec<String>,
    heed: Arc<Heed>,
}

/// Which flags the command being run has asked for, and whether it has
/// begun to act on them.
#[derive(Debug, Default)]
pub struct Heed {
    /// The command as it was typed, for the sentence that refuses.
    command: String,
    given: Vec<String>,
    asked: Mutex<BTreeSet<String>>,
    /// Something was sent, or the command began: from here on a flag that
    /// was not asked for already can no longer change what it does.
    begun: AtomicBool,
    refused: Mutex<Option<String>>,
}

impl Args {
    pub fn parse(raw: impl IntoIterator<Item = String>) -> Result<Args, String> {
        let typed: Vec<String> = raw.into_iter().collect();
        let mut args = Args { raw: typed.clone(), ..Args::default() };
        let mut given = Vec::new();
        let mut raw = typed.into_iter();
        while let Some(arg) = raw.next() {
            let Some(flag) = arg.strip_prefix("--") else {
                args.words.push(arg);
                continue;
            };
            if flag.is_empty() {
                // `--` ends the flags: the rest is words, whatever it looks like.
                args.words.extend(raw);
                break;
            }
            let (name, inline) = match flag.split_once('=') {
                Some((name, value)) => (name, Some(value.to_string())),
                None => (flag, None),
            };
            let value = match inline {
                Some(value) => Some(value),
                None if TAKES_VALUE.contains(&name) => {
                    Some(raw.next().ok_or_else(|| format!("`--{name}` needs a value"))?)
                }
                None => None,
            };
            if !given.iter().any(|said| said == name) {
                given.push(name.to_string());
            }
            args.flags.push((name.to_string(), value));
        }
        // The command is said by its word and the word after it where that
        // reads as one of its own (`agent add`), and not as a file, a
        // blueprint's name or anything else it is handed (`apply std/board`).
        let word = |word: &&String| word.chars().all(|letter| letter.is_ascii_lowercase() || letter == '-');
        let said: Vec<&str> = args
            .words
            .first()
            .into_iter()
            .chain(args.words.get(1).filter(word))
            .map(String::as_str)
            .collect();
        let command = said.join(" ");
        args.heed = Arc::new(Heed { command, given, ..Heed::default() });
        Ok(args)
    }

    /// What is noted of the flags this command asks for, which whatever
    /// sends for it is handed (`Client::new`).
    pub fn heed(&self) -> Arc<Heed> {
        Arc::clone(&self.heed)
    }

    /// Everything as it was typed, for a command that reads the flags
    /// `names` from it itself, in the order they stand.
    pub fn raw_of(&self, names: &[&str]) -> &[String] {
        for name in names {
            self.heed.note(name);
        }
        &self.raw
    }

    /// The word at `index`, counting from the command itself.
    pub fn word(&self, index: usize) -> Option<&str> {
        self.words.get(index).map(String::as_str)
    }

    /// The word at `index`, or an error saying what was expected there.
    pub fn need(&self, index: usize, what: &str) -> Result<&str, String> {
        self.word(index).ok_or_else(|| format!("missing {what}"))
    }

    /// Every word from `index` on, joined: for a message or a title typed
    /// without quotes.
    pub fn rest(&self, index: usize) -> String {
        self.words.get(index..).unwrap_or_default().join(" ")
    }

    /// Every word from `index` on, each by itself.
    pub fn words_from(&self, index: usize) -> Vec<String> {
        self.words.get(index..).unwrap_or_default().to_vec()
    }

    pub fn has(&self, name: &str) -> bool {
        self.heed.note(name);
        self.flags.iter().any(|(flag, _)| flag == name)
    }

    /// The value of the last `--name`.
    pub fn value(&self, name: &str) -> Option<&str> {
        self.heed.note(name);
        self.flags.iter().rev().find(|(flag, _)| flag == name).and_then(|(_, value)| value.as_deref())
    }

    /// The values of every `--name`, in order.
    pub fn values(&self, name: &str) -> Vec<&str> {
        self.heed.note(name);
        self.flags.iter().filter(|(flag, _)| flag == name).filter_map(|(_, value)| value.as_deref()).collect()
    }
}

impl Heed {
    /// Notes that the command asked for `--name`. Asked for the first time
    /// once the command has begun, it was asked too late to be refused
    /// where it was given and nobody read it: a program built to be tested
    /// says so, which is the hold on every command reading its flags first.
    fn note(&self, name: &str) {
        let first = self.asked.lock().unwrap_or_else(|held| held.into_inner()).insert(name.to_string());
        if first
            && cfg!(debug_assertions)
            && self.begun.load(Ordering::Relaxed)
            && !EVERYWHERE.contains(&name)
        {
            panic!(
                "`{}` asked for `--{name}` after it had begun: a command reads every flag it takes before it \
                 sends anything, or one that is given is refused as never asked for (crates/cli/src/args.rs)",
                self.named()
            );
        }
    }

    /// Whether every flag given was asked for. Asked before anything is
    /// sent, and from then on the command has begun: what it asks for
    /// afterwards was asked too late. Once it refuses it refuses always,
    /// so that a command that went on past a failure sends nothing more.
    pub fn check(&self) -> Result<(), String> {
        if let Some(refused) = self.refused() {
            return Err(refused);
        }
        let unasked = self.unasked();
        if unasked.is_empty() {
            self.begun.store(true, Ordering::Relaxed);
            return Ok(());
        }
        let refusal = self.refusal(&unasked);
        *self.refused.lock().unwrap_or_else(|held| held.into_inner()) = Some(refusal.clone());
        Err(refusal)
    }

    /// The sentence that refused, where something was refused.
    pub fn refused(&self) -> Option<String> {
        self.refused.lock().unwrap_or_else(|held| held.into_inner()).clone()
    }

    fn unasked(&self) -> Vec<&str> {
        let asked = self.asked.lock().unwrap_or_else(|held| held.into_inner());
        self.given
            .iter()
            .map(String::as_str)
            .filter(|name| !EVERYWHERE.contains(name) && !asked.contains(*name))
            .collect()
    }

    fn refusal(&self, unasked: &[&str]) -> String {
        let flags = |names: &[&str]| names.iter().map(|name| format!("`--{name}`")).collect::<Vec<_>>();
        let said = |names: Vec<String>, and: &str| match names.split_last() {
            Some((last, [])) => last.clone(),
            Some((last, rest)) => format!("{} {and} {last}", rest.join(", ")),
            None => String::new(),
        };
        let everywhere: Vec<&str> = EVERYWHERE.iter().copied().filter(|name| *name != "env").collect();
        let everywhere = said(flags(&everywhere), "and");
        let asked = self.asked.lock().unwrap_or_else(|held| held.into_inner());
        let own: Vec<&str> =
            asked.iter().map(String::as_str).filter(|name| !EVERYWHERE.contains(name)).collect();
        let takes = match own.is_empty() {
            true => format!("it takes only the flags every command takes ({everywhere})"),
            false => format!(
                "it takes {}, besides the flags every command takes ({everywhere})",
                said(flags(&own), "and")
            ),
        };
        let help = match self.command.split(' ').next().filter(|word| !word.is_empty()) {
            Some(word) => format!("`{} help {word}` says what it does", brand::CLI),
            None => format!("`{} help` lists every command", brand::CLI),
        };
        format!(
            "`{}` takes no {}, so nothing was sent: {takes}. {help}",
            self.named(),
            said(flags(unasked), "or")
        )
    }

    /// The command as it was typed, by its first two words.
    fn named(&self) -> String {
        format!("{} {}", brand::CLI, self.command).trim_end().to_string()
    }
}
