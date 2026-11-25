//! What the factory's bot takes, in words: the list of the commands it
//! takes (each as the command line's help writes it, which is the grammar
//! it is read by: [`crate::taken`]), what it says of itself, and the few
//! sentences and pieces of words every command's answer is made of. What
//! each command does is asked of the daemon's routes ([`crate::bidden`],
//! and `asking` for the rest): nothing here asks anything, and all of it
//! is a function of words.

use kernel::Principal;
use serde_json::{Map, Value};

use crate::taken::Said;

/// The most lines one answer lists; the command line has the rest.
pub const LINES: usize = 30;

/// What the bot's commands reach, by the scopes their routes ask: what it
/// may ask where nobody narrowed it.
pub const REACHES: &[&str] = &[
    "agents:read",
    "agents:write",
    "blueprints:read",
    "blueprints:run",
    "board:read",
    "board:write",
    "channels:read",
    "channels:write",
    "emit",
    "inbox:read",
    "inbox:write",
    "projects:read",
    "records:read",
    "records:write",
    "search",
];

/// A refusal of what the bot was asked, said as the bot's limit where it
/// is: the route asked for a scope whoever asked holds and the bot may not
/// ask, so it is the bot's ceiling that stands in the way, and whoever
/// manages the factory is who can change that.
pub fn beyond(name: &str, who: &Principal, asking: &Principal, holds: &[String], refused: String) -> String {
    let needs =
        refused.split_once("(needs `").and_then(|(_, rest)| rest.split_once('`')).map(|(scope, _)| scope);
    match needs.filter(|scope| who.can(scope) && !asking.can(scope)) {
        Some(scope) => format!(
            "I may not do that: what `{name}` may ask is held to {}, and that needs `{scope}`, which you hold. \
             Whoever manages the factory gives me more where I am read about, under Channel members.",
            match holds.is_empty() {
                true => "nothing".to_string(),
                false => holds.iter().map(|one| format!("`{one}`")).collect::<Vec<_>>().join(", "),
            }
        ),
        None => refused,
    }
}

/// What the bot takes: how each is written, in the command line's own
/// words to the letter (the command line's suite holds each to its help),
/// and what it does. The one list of them: what is said to the bot is
/// read by these lines and nothing else (`taken`), and what the bot says
/// of itself (`help`) and what is shown of it where it is written to
/// (`/api/system/speaker`) are both this, so a command is a line here
/// before it is anything else. Kept together by the command they begin
/// with, which is how `help` groups them.
pub const TAKES: &[(&str, &str)] = &[
    ("status", "what is running here"),
    ("instances", "what is alive, and which are behind the text now there"),
    ("instance <id>", "one instance: what it runs and how it stands"),
    ("stop <id>", "stop it and what it holds"),
    ("apply <file|name> [--project P [--environment E]] [--param k=v]...", "apply a blueprint, by its name"),
    ("board <project>", "a board"),
    ("board columns <project> [<column>...]", "a board's columns, or them set in the order given"),
    (
        "board add <project> <title...> [--body TEXT] [--status COLUMN] [--label L]... [--field name=value]...",
        "add a card",
    ),
    ("board move <card> <column> [--note TEXT]", "move a card"),
    ("board give <card> <agent>|<person>|nobody", "give a card to an agent or a person, or to nobody"),
    (
        "board set <card> <field=value>...",
        "set what a card holds: its title, body, labels and its board's fields",
    ),
    ("board press <card> <action> [--back]", "press what a card's column offers on it, or take it back"),
    ("board history <card>", "what has happened to a card, the latest last"),
    ("board cost <card>", "what each round of a card cost"),
    ("record list <project> <collection>", "the records of one of a project's tables"),
    ("record show <record>", "one record"),
    ("record add <project> <collection> <field=value>...", "add a record"),
    ("record set <record> <field=value>...", "set what a record holds"),
    ("record comment <record> <words...>", "say something on a record"),
    ("record remove <record>", "move a record to the Trash"),
    ("agent list", "the agents"),
    ("agent stop <name> [--idle]", "stop an agent; with `--idle`, when it has finished what it has in hand"),
    ("agent move <name> <project>|--none", "have an agent work in another project, or in none"),
    ("read <channel> [--limit N]", "what was said lately in a channel"),
    ("post <channel> <message...>", "say something in a channel"),
    ("channel rename <channel> <name>", "call a channel something else"),
    ("inbox [--all]", "how much is in your inbox"),
    ("inbox read <line>|--all", "say a line of your inbox was read, or all of them"),
    ("inbox away <line>", "put a line of your inbox away"),
    ("project environment <project>", "a project's environments"),
    ("project environment <project> <env> run <branch>", "load an environment from a branch"),
    ("compare [list]", "what is being compared"),
    ("compare show <name>", "one comparison"),
    ("compare end <name> [--goes <way>]", "end a comparison"),
    ("sandbox <file|name> [--project P] [--param k=v]...", "run a blueprint where nothing is real"),
    ("search <words...> [--kind KIND] [--project P]", "find anything you may read, of one kind where said"),
    ("emit <topic> [k=v]...", "say an event"),
    ("help", "what it takes, said in the channel"),
];

/// What the bot takes, said to whoever said something else; and, in a
/// project's channel, that it is that project's there.
pub fn help(name: &str, within: Option<&str>) -> String {
    let cli = brand::CLI;
    // Those that begin with the same word together, set apart by a blank line, so that a board's are read as one.
    let mut takes = String::new();
    let first = |written: &str| written.split(' ').next().unwrap_or_default().to_string();
    let grouped =
        |written: &str| TAKES.iter().filter(|(other, _)| first(other) == first(written)).count() > 1;
    for (at, (written, does)) in TAKES.iter().enumerate() {
        let before = at.checked_sub(1).map(|at| TAKES[at].0);
        if before
            .is_some_and(|before| first(before) != first(written) && (grouped(before) || grouped(written)))
        {
            takes.push('\n');
        }
        takes.push_str(&format!("- `{written}`: {does}\n"));
    }
    let here = match within {
        Some(project) => format!(
            "This is the channel of `{project}`: here I am asked about that project, and do nothing to \
             another's. Where a command names a project (`<project>`, `--project`), it is left out here: \
             it is `{project}`.\n\n"
        ),
        None => String::new(),
    };
    format!(
        "{here}I take these, in the words of `{cli}`, and do each as whoever asks, with what their key holds:\n\
         {takes}\
         I have no model and do not guess what is meant. To be understood in ordinary words, ask an agent \
         in this channel by its name; `@{name}` is for the commands above."
    )
}

/// What is said of something asked in one project's channel about another's.
pub fn elsewhere(within: &str, what: &str) -> String {
    format!(
        "This is the channel of `{within}`, and {what}. Here I am asked about `{within}` and do nothing to \
         another project's: ask in that project's own channel."
    )
}

/// What follows the bot's name in a message: what is said to it.
pub fn after<'a>(body: &'a str, name: &str) -> &'a str {
    let lower = body.to_lowercase();
    let named = format!("@{name}");
    // Lowercasing can change how long a letter is; then nothing is cut.
    match lower.find(&named).filter(|_| lower.len() == body.len()) {
        Some(at) => &body[at + named.len()..],
        None => "",
    }
}

/// The words of what was said: parted by spaces, with what is between
/// double quotes kept as one. Nothing else means anything.
pub fn words(said: &str) -> Vec<String> {
    let (mut all, mut word, mut quoted, mut begun) = (Vec::new(), String::new(), false, false);
    for letter in said.chars() {
        match letter {
            '"' => (quoted, begun) = (!quoted, true),
            space if space.is_whitespace() && !quoted => {
                if begun {
                    all.push(std::mem::take(&mut word));
                }
                begun = false;
            }
            other => {
                word.push(other);
                begun = true;
            }
        }
    }
    if begun {
        all.push(word);
    }
    all
}

/// A word that is to be part of an address: what an id, a name or a topic
/// is made of, and nothing an address would read as more than a name.
pub fn part<'a>(word: &'a str, what: &str) -> Result<&'a str, String> {
    let plain = |letter: char| letter.is_ascii_alphanumeric() || matches!(letter, '-' | '_' | '.' | ':');
    if word.is_empty() || !word.chars().all(plain) || word.chars().all(|letter| letter == '.') {
        return Err(format!(
            "`{word}` is not {what}: one is letters, digits, `-`, `_`, `.` and `:`, with nothing else in it."
        ));
    }
    Ok(word)
}

/// A value said after `=`, as JSON where it reads as JSON and as words otherwise.
pub fn value_of(said: &str) -> Value {
    serde_json::from_str(said).unwrap_or_else(|_| Value::String(said.into()))
}

/// The last lines of many, with how many came before them and where they are.
pub fn latest(mut lines: Vec<String>, none: &str, command: &str) -> String {
    if lines.is_empty() {
        return none.to_string();
    }
    let before = lines.len().saturating_sub(LINES);
    let mut kept = lines.split_off(before);
    if before > 0 {
        kept.insert(0, format!("{before} earlier: `{} {command}` has them all.", brand::CLI));
    }
    kept.join("\n")
}

pub fn list(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

pub fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or_default()
}

pub fn count(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// Lines as one answer: all of a few, and of many the first, with where
/// the rest are.
pub fn listed(mut lines: Vec<String>, none: &str, command: &str) -> String {
    if lines.is_empty() {
        return none.to_string();
    }
    let more = lines.len().saturating_sub(LINES);
    lines.truncate(LINES);
    if more > 0 {
        lines.push(format!("and {more} more: `{} {command}` lists them all.", brand::CLI));
    }
    lines.join("\n")
}

/// Refuses what is another project's than the channel's, where the
/// channel has one, saying so: asked once whoever asks has read it, so
/// that nothing is said of something they may not read.
pub fn held(within: Option<&str>, found: &Value, what: &str) -> Result<(), String> {
    match (within, found["project"].as_str()) {
        (Some(project), Some(theirs)) if theirs != project => Err(elsewhere(project, what)),
        (Some(project), None) => Err(elsewhere(project, &format!("{what}, or no project's"))),
        _ => Ok(()),
    }
}

/// The project a `<project>` names, held to the channel's.
pub fn named_project(said: &Said, within: Option<&str>) -> Result<String, String> {
    let named = part(said.word("project"), "a project")?;
    match within.filter(|project| *project != named) {
        Some(project) => Err(elsewhere(project, &format!("`{named}` is another project"))),
        None => Ok(named.to_string()),
    }
}

/// The project a `--project` says, or in a project's channel that one,
/// which no other may be.
pub fn project(said: &Said, within: Option<&str>) -> Result<Option<String>, String> {
    match (said.flag("project"), within) {
        (Some(named), Some(project)) if named != project => {
            Err(elsewhere(project, &format!("`{named}` is another project")))
        }
        (Some(named), _) => Ok(Some(part(named, "a project")?.to_string())),
        (None, project) => Ok(project.map(str::to_string)),
    }
}

/// What a record holds, in a few words: its title where it has one.
pub fn brief(data: &Value) -> String {
    match data.get("title").and_then(Value::as_str) {
        Some(title) => title.to_string(),
        None => data
            .as_object()
            .into_iter()
            .flatten()
            .take(3)
            .map(|(name, value)| format!("{name} {}", plain(value)))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// A value as words.
pub fn plain(value: &Value) -> String {
    match value {
        Value::String(text) => text.lines().next().unwrap_or_default().to_string(),
        Value::Array(items) => items.iter().map(plain).collect::<Vec<_>>().join(", "),
        Value::Null => "nothing".to_string(),
        other => other.to_string(),
    }
}

/// Words made part of an address's query: what is not a letter or a digit written by its bytes.
pub fn encode(words: &str) -> String {
    words
        .bytes()
        .map(|byte| match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (byte as char).to_string(),
            other => format!("%{other:02X}"),
        })
        .collect()
}

/// Why words said to the bot are no command it does here.
pub enum Unread {
    /// They are none of what it takes: answered with what it does take.
    Unknown(String),
    /// They are one, about another project than the channel's: answered
    /// with that, and nothing more.
    Elsewhere(String),
}

impl From<String> for Unread {
    fn from(why: String) -> Unread {
        Unread::Unknown(why)
    }
}

/// One of the things the bot does.
#[derive(Debug)]
pub enum Command {
    Help,
    Status,
    Instances,
    Board {
        project: String,
    },
    Add {
        project: String,
        title: String,
        body: Option<String>,
        status: Option<String>,
        labels: Vec<String>,
        fields: Map<String, Value>,
    },
    Move {
        card: String,
        column: String,
        note: Option<String>,
    },
    Give {
        card: String,
        to: String,
    },
    Set {
        card: String,
        pairs: Vec<(String, String)>,
    },
    Press {
        card: String,
        action: String,
        back: bool,
    },
    History {
        card: String,
    },
    Cost {
        card: String,
    },
    Agents,
    Stop {
        agent: String,
        idle: bool,
    },
    Emit {
        topic: String,
        data: Map<String, Value>,
    },
    /// One of the rest, done by `asking` with what was said.
    Other {
        key: String,
        said: crate::taken::Said,
    },
}

impl Command {
    /// The command these words are, or why they are none here: `project`
    /// is the channel's, where it has one. Which command is read from
    /// how each is written (`TAKES`, through `taken`): the one whose fixed
    /// words are the most of what was said, so that `board move` is
    /// `board move` and not the board called `move`. A leading `mw`, as
    /// the command line is typed, is the same words.
    pub fn read(words: &[String], project: Option<&str>) -> Result<Command, Unread> {
        let mut words: Vec<&str> = words.iter().map(String::as_str).collect();
        if words.first() == Some(&brand::CLI) {
            words.remove(0);
        }
        if words.is_empty() {
            return Ok(Command::Help);
        }
        // Of those it begins with, the one with the most fixed words first, and of
        // two with as many the one with more parts: the first that reads it is it.
        let mut those: Vec<(usize, &str)> = TAKES
            .iter()
            .filter_map(|(written, _)| crate::taken::begins(written, &words).map(|fixed| (fixed, *written)))
            .collect();
        those.sort_by_key(|(fixed, written)| std::cmp::Reverse((*fixed, crate::taken::parts(written))));
        // Only those that begin with the most of it: `board move` said alone is `board move` wanting its card,
        // and never the board of a project called `move`.
        let most = those.first().map_or(0, |(fixed, _)| *fixed);
        those.retain(|(fixed, _)| *fixed == most);
        let mut why = None;
        let mut found = None;
        for (_, written) in &those {
            match crate::taken::read(written, &words, project) {
                Ok(said) => {
                    found = Some((*written, said));
                    break;
                }
                Err(not) => drop(why.get_or_insert(not)),
            }
        }
        let Some((written, said)) = found else {
            return Err(Unread::Unknown(why.unwrap_or_else(|| "I did not take that as a command.".into())));
        };
        let card = || part(said.word("card"), "a card").map(str::to_string);
        // A project named in another project's channel is said to be another's, and nothing more.
        let project_of = || {
            let named = part(said.word("project"), "a project")?;
            match project.filter(|within| *within != named) {
                Some(within) => {
                    Err(Unread::Elsewhere(elsewhere(within, &format!("`{named}` is another project"))))
                }
                None => Ok(named.to_string()),
            }
        };
        Ok(match crate::taken::key(written).as_str() {
            "help" => Command::Help,
            "status" => Command::Status,
            "instances" => Command::Instances,
            "board" => Command::Board { project: project_of()? },
            "board add" => {
                let mut fields = Map::new();
                for pair in said.every("field") {
                    let (name, value) = pair
                        .split_once('=')
                        .ok_or_else(|| format!("`--field {pair}` should be `--field name=value`."))?;
                    fields.insert(name.to_string(), value_of(value));
                }
                Command::Add {
                    project: project_of()?,
                    title: said.word("title").to_string(),
                    body: said.flag("body").map(str::to_string),
                    status: said.flag("status").map(str::to_string),
                    labels: said.every("label").to_vec(),
                    fields,
                }
            }
            "board move" => Command::Move {
                card: card()?,
                column: said.word("column").to_string(),
                note: said.flag("note").map(str::to_string),
            },
            "board give" => Command::Give { card: card()?, to: said.word("agent").to_string() },
            "board set" => Command::Set { card: card()?, pairs: said.pairs.clone() },
            "board press" => Command::Press {
                card: card()?,
                action: said.word("action").to_string(),
                back: said.has("back"),
            },
            "board history" => Command::History { card: card()? },
            "board cost" => Command::Cost { card: card()? },
            "agent list" => Command::Agents,
            "agent stop" => Command::Stop {
                agent: part(said.word("name"), "an agent")?.to_string(),
                idle: said.has("idle"),
            },
            "emit" => {
                let data = said.pairs.iter().map(|(key, value)| (key.clone(), value_of(value))).collect();
                Command::Emit { topic: part(said.word("topic"), "a topic")?.to_string(), data }
            }
            other => Command::Other { key: other.to_string(), said },
        })
    }
}
