//! Agents of a channel talking something over, as plain values: what a
//! channel offers (`Settings`), whether a message starts a discussion, is
//! one's or is neither (`takes`), whose turn it is and whether it is over
//! (`next`), the sentences said as it ends (`ending`) and what each
//! agent is told of the others (`brief`).
//!
//! A discussion is a thread. It is begun by one of three things, each a
//! setting: an agent naming another that takes part (`tagged`), an agent
//! marking what it said as a plan (`plans`), or a person marking what they
//! said as something to discuss and naming two or more who take part
//! (`asked`). Marks are explicit words kept on a message, never read
//! from its words. It goes a turn at a time, each agent ending its turn
//! with one of three words ([`WORDS`]), and stops agreed, needing a
//! person, or at the cap of turns in a stretch; a person's message in the
//! thread then begins the next stretch.
//!
//! A channel may have a lead: what a person says there naming no agent is
//! the lead's alone, its advisors answer the lead in a thread, and an
//! agreement ends with the lead answering the person in the channel.
//!
//! Everything here is a function of what it is handed. Who is in the
//! channel, what was said and who may read it are whoever keeps them.

use serde_json::{Value, json};

/// What may begin a discussion, in the order the settings offer them.
pub const ON: &[&str] = &["tagged", "plans", "asked"];
/// How a turn of a discussion may end: it goes on, it agrees, or a person
/// is needed.
pub const WORDS: &[&str] = &["reply", "agree", "needs"];
/// The marks a message may carry, which nothing reads from its words: a
/// plan, and something a person asks to be discussed.
pub const MARKS: &[&str] = &["plan", "discuss"];
/// What an ending may be told of, in the person's inbox.
pub const TOLD_ON: &[&str] = &["agree", "needs", "cap"];
/// Turns of agents in one stretch where nobody says, and the most.
pub const TURNS: i64 = 6;
pub const MOST_TURNS: i64 = 20;
/// The most agents that take part in a channel's discussions.
pub const MOST_AGENTS: usize = 12;

/// One that takes part, with what it is for, said to the others.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Taking {
    pub name: String,
    pub does: String,
}

/// What a channel offers: who takes part, what begins a discussion, how
/// long one goes, how it is agreed and what is told.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Who discuss with each other as equals.
    pub agents: Vec<Taking>,
    /// The agent a person talks to, where there is one.
    pub lead: String,
    /// Whom the lead consults, each with what it is for.
    pub advisors: Vec<Taking>,
    pub on: Vec<String>,
    pub turns: i64,
    /// `one` (one agree, once everyone has spoken) or `all`.
    pub agreeing: String,
    pub tell_on: Vec<String>,
}

fn word(said: &Value, key: &str) -> Result<String, String> {
    match &said[key] {
        Value::Null => Ok(String::new()),
        Value::String(text) => Ok(text.trim().to_string()),
        other => Err(format!("`{key}` is a word; it was given {other}")),
    }
}

fn words(said: &Value, key: &str, usual: &[&str], known: &[&str]) -> Result<Vec<String>, String> {
    let listed = match &said[key] {
        Value::Null => return Ok(usual.iter().map(|one| one.to_string()).collect()),
        Value::Array(listed) => listed,
        other => return Err(format!("`{key}` is a list of words; it was given {other}")),
    };
    let mut out: Vec<String> = Vec::new();
    for one in listed {
        let Some(one) = one.as_str().map(str::trim) else {
            return Err(format!("`{key}` is a list of words; {one} is none"));
        };
        if !known.contains(&one) {
            return Err(format!("`{one}` in `{key}` is none of {}", said_as(known)));
        }
        if !out.iter().any(|was| was == one) {
            out.push(one.to_string());
        }
    }
    Ok(out)
}

fn said_as(known: &[&str]) -> String {
    known.iter().map(|one| format!("`{one}`")).collect::<Vec<_>>().join(", ")
}

/// Who take part as a list says them: each a name, or `{name, does}` (or
/// `for`, as advisors say it).
fn takings(said: &Value, key: &str) -> Result<Vec<Taking>, String> {
    let listed = match &said[key] {
        Value::Null => return Ok(Vec::new()),
        Value::Array(listed) => listed,
        other => return Err(format!("`{key}` is a list of agents; it was given {other}")),
    };
    listed
        .iter()
        .map(|one| match one {
            // `name`, or `name: what it is for`, as a line of a form says it.
            Value::String(said) => {
                let (name, does) = said.split_once(':').unwrap_or((said, ""));
                Ok(Taking { name: name.trim().to_string(), does: does.trim().to_string() })
            }
            Value::Object(_) => {
                let does = word(one, "does")?;
                let does = if does.is_empty() { word(one, "for")? } else { does };
                Ok(Taking { name: word(one, "name")?, does })
            }
            other => {
                Err(format!("an agent of `{key}` is a name, or `name` and `does`; it was given {other}"))
            }
        })
        .collect()
}

impl Settings {
    /// Read from what the text was handed, checked: why not, in a sentence.
    pub fn read(said: &Value) -> Result<Settings, String> {
        let settings = Settings {
            agents: takings(said, "agents")?,
            lead: word(said, "lead")?,
            advisors: takings(said, "advisors")?,
            on: words(said, "on", ON, ON)?,
            turns: match &said["turns"] {
                Value::Null => TURNS,
                turns => {
                    turns.as_i64().ok_or_else(|| format!("`turns` is a number; it was given {turns}"))?
                }
            },
            agreeing: match word(said, "agreeing")?.as_str() {
                "" => "one".into(),
                agreeing @ ("one" | "all") => agreeing.into(),
                other => return Err(format!("`agreeing` is `one` or `all`; it was given `{other}`")),
            },
            tell_on: words(said, "tell_on", TOLD_ON, TOLD_ON)?,
        };
        if !(1..=MOST_TURNS).contains(&settings.turns) {
            return Err(format!(
                "`turns` is from 1 to {MOST_TURNS}: how many turns agents take before a person is asked"
            ));
        }
        let taking = settings.taking();
        if taking.iter().any(|name| name.is_empty()) {
            return Err("an agent that takes part is said by its name, and one was said with none".into());
        }
        if taking.len() < 2 {
            return Err(
                "a discussion is between two agents at least: say them in `agents`, or a `lead` and its `advisors`".into(),
            );
        }
        if taking.len() > MOST_AGENTS {
            return Err(format!("{MOST_AGENTS} agents at most take part in a channel's discussions"));
        }
        let mut names =
            settings.agents.iter().chain(&settings.advisors).map(|one| &one.name).collect::<Vec<_>>();
        names.push(&settings.lead);
        if let Some(twice) =
            names.iter().enumerate().find(|(at, name)| !name.is_empty() && names[..*at].contains(name))
        {
            return Err(format!("`{}` is said twice among who take part", twice.1));
        }
        if !settings.lead.is_empty() && settings.advisors.is_empty() {
            return Err(format!(
                "a lead consults its advisors: say who advises `{}` in `advisors`",
                settings.lead
            ));
        }
        Ok(settings)
    }

    /// Everybody who takes part, the lead first, each once.
    pub fn taking(&self) -> Vec<String> {
        let lead = (!self.lead.is_empty()).then(|| self.lead.clone());
        lead.into_iter().chain(self.advisors.iter().chain(&self.agents).map(|one| one.name.clone())).collect()
    }

    pub fn takes_part(&self, name: &str) -> bool {
        self.taking().iter().any(|one| one == name)
    }

    /// As a text hands it on and the keeper keeps it.
    pub fn said(&self) -> Value {
        let takings = |list: &[Taking], key: &str| -> Vec<Value> {
            list.iter().map(|one| json!({ "name": one.name, key: one.does })).collect()
        };
        json!({
            "agents": takings(&self.agents, "does"), "lead": self.lead,
            "advisors": takings(&self.advisors, "for"), "on": self.on, "turns": self.turns,
            "agreeing": self.agreeing, "tell_on": self.tell_on,
        })
    }

    fn on(&self, trigger: &str) -> bool {
        self.on.iter().any(|one| one == trigger)
    }

    /// Whom `name` discusses with when it begins one: the lead its
    /// advisors, an advisor the lead, and among equals everybody else.
    fn others(&self, name: &str) -> Vec<String> {
        if self.lead == name {
            return self.advisors.iter().map(|one| one.name.clone()).collect();
        }
        if self.advisors.iter().any(|one| one.name == name) {
            return vec![self.lead.clone()];
        }
        self.agents.iter().map(|one| one.name.clone()).filter(|one| one != name).collect()
    }

    fn does(&self, name: &str) -> &str {
        let found = self.agents.iter().chain(&self.advisors).find(|one| one.name == name);
        found.map(|one| one.does.as_str()).unwrap_or_default()
    }
}

/// How the thread a message is in stands, where it is a discussion's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Thread {
    /// No discussion.
    Free,
    Going,
    /// One that waits for a person.
    Waiting,
    /// One that ended agreed: what agents say under it begins none again.
    Agreed,
}

/// What is known of a message said in a channel that offers discussions.
#[derive(Clone, Debug, Default)]
pub struct Message {
    pub id: String,
    /// `user:<id>`, `agent:<name>`, `bot:<name>`.
    pub author: String,
    /// The agents of the channel it names.
    pub names: Vec<String>,
    pub mark: String,
    /// The message the thread it is in hangs from.
    pub thread: Option<String>,
    /// The message it answers, and whether a person said that one.
    pub reply_to: Option<String>,
    pub answers: Option<String>,
}

/// What a message comes to in a channel that offers discussions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Takes {
    /// Neither: whoever it is for answers it as anywhere else.
    Not,
    /// The lead alone answers it: a person's that names no agent.
    Lead(String),
    /// It is said in a discussion's thread, which is the discussion's;
    /// `resumes`, where it waits and a person said it.
    Within { resumes: bool },
    /// It begins one, under `root`, between `agents` (whoever speaks
    /// first first), of which `spoken` have said something already.
    Starts {
        root: String,
        agents: Vec<String>,
        spoken: Vec<String>,
        asker: Option<String>,
        why: &'static str,
    },
}

/// What `message` comes to in a channel that offers `settings`, where
/// the thread that hangs from a message stands as `stands` says.
pub fn takes(settings: &Settings, message: &Message, stands: impl Fn(&str) -> Thread) -> Takes {
    let person = message.author.starts_with("user:");
    let agent = message.author.strip_prefix("agent:").filter(|name| settings.takes_part(name));
    let thread = message.thread.as_deref().map_or(Thread::Free, &stands);
    if matches!(thread, Thread::Going | Thread::Waiting) {
        return Takes::Within { resumes: thread == Thread::Waiting && person };
    }
    let named: Vec<String> = message.names.iter().filter(|name| settings.takes_part(name)).cloned().collect();
    // Where it hangs: in lead's way under what the person asked, so that
    // the lead's answer and what it consulted on are one place.
    let root = || match (&message.thread, &message.reply_to) {
        (Some(thread), _) => thread.clone(),
        (None, Some(asked)) if !settings.lead.is_empty() && message.answers.is_some() => asked.clone(),
        _ => message.id.clone(),
    };
    if person && message.mark == "discuss" && settings.on("asked") && named.len() >= 2 {
        let asker = Some(message.author.clone());
        return Takes::Starts { root: root(), agents: named, spoken: Vec::new(), asker, why: "asked" };
    }
    // What agents say under a discussion that was agreed (the lead's
    // answer to the person, say) begins none again, and asks nobody it
    // names: a person may begin one there.
    if agent.is_some() && !named.is_empty() && stands(&root()) == Thread::Agreed {
        return Takes::Within { resumes: false };
    }
    if let Some(author) = agent {
        let others = settings.others(author);
        let tagged: Vec<String> = named.iter().filter(|name| others.contains(name)).cloned().collect();
        let (why, with) = match (message.mark == "plan" && settings.on("plans"), settings.on("tagged")) {
            (true, _) if !others.is_empty() => ("plans", if tagged.is_empty() { others } else { tagged }),
            (_, true) if !tagged.is_empty() => ("tagged", tagged),
            _ => return Takes::Not,
        };
        let mut agents = with;
        agents.push(author.to_string());
        return Takes::Starts {
            root: root(),
            agents,
            spoken: vec![author.to_string()],
            asker: message.answers.clone(),
            why,
        };
    }
    if person && message.names.is_empty() && !settings.lead.is_empty() {
        return Takes::Lead(settings.lead.clone());
    }
    Takes::Not
}

/// Where a stretch of a discussion has come to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stretch {
    pub agents: Vec<String>,
    /// Who has said something in the discussion.
    pub spoken: Vec<String>,
    /// Whose last turn ended `agree`, since anybody's last went on.
    pub agreed: Vec<String>,
    /// Turns taken in this stretch, the one just over among them.
    pub nth: i64,
    /// Whose turn was just over, and what it ended with; nothing before the first.
    pub last: String,
    pub word: String,
    /// Who is to speak first, where nobody has yet in this stretch.
    pub first: String,
}

/// What a stretch comes to after a turn, or before its first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Next {
    /// Another turn, of `slot`, with the stretch as it then stands.
    Turn {
        slot: String,
        then: Stretch,
    },
    Agreed,
    /// A person is needed.
    Needs,
    /// The cap was reached with nobody agreeing.
    Capped,
}

impl Next {
    pub fn came(&self) -> &'static str {
        match self {
            Next::Turn { .. } => "turn",
            Next::Agreed => "agreed",
            Next::Needs => "needs",
            Next::Capped => "capped",
        }
    }
}

/// What comes of `stretch` under `settings`: before its first turn
/// (`last` empty), the first; after one, the next turn or how it ended.
pub fn next(settings: &Settings, stretch: &Stretch) -> Next {
    let mut then = stretch.clone();
    if stretch.last.is_empty() {
        let slot = if stretch.first.is_empty() {
            stretch.agents.first().cloned().unwrap_or_default()
        } else {
            stretch.first.clone()
        };
        then.nth = 0;
        return Next::Turn { slot, then };
    }
    if !then.spoken.contains(&stretch.last) {
        then.spoken.push(stretch.last.clone());
    }
    match stretch.word.as_str() {
        "needs" => return Next::Needs,
        "agree" => {
            if !then.agreed.contains(&stretch.last) {
                then.agreed.push(stretch.last.clone());
            }
            let everyone = |said: &[String]| stretch.agents.iter().all(|one| said.contains(one));
            let agreed = match settings.agreeing.as_str() {
                "all" => everyone(&then.agreed),
                _ => everyone(&then.spoken),
            };
            if agreed {
                return Next::Agreed;
            }
        }
        // What chose nothing goes on, as what said `reply` does.
        _ => then.agreed.clear(),
    }
    if stretch.nth >= settings.turns {
        return Next::Capped;
    }
    let at = stretch.agents.iter().position(|one| *one == stretch.last).unwrap_or(stretch.agents.len() - 1);
    let slot = stretch.agents[(at + 1) % stretch.agents.len()].clone();
    then.last.clear();
    then.word.clear();
    Next::Turn { slot, then }
}

fn listed(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// What the agent whose turn it is is asked: who it discusses with, what
/// each is for, and how it ends its turn.
pub fn asked(settings: &Settings, stretch: &Stretch, slot: &str) -> String {
    let others: Vec<String> = stretch.agents.iter().filter(|one| *one != slot).cloned().collect();
    let with: Vec<String> = others
        .iter()
        .map(|one| match settings.does(one) {
            "" => one.clone(),
            does => format!("{one} ({does})"),
        })
        .collect();
    format!(
        "You are discussing what this thread is about with {with}. Read what was said in it, and say what you \
         think in a few lines. End your turn with `reply` to answer and let them go on, `agree` when you agree \
         with what was said and there is nothing more to settle, or `needs` when only a person can decide, \
         saying in a sentence what they must decide. This is turn {nth} of at most {most}.",
        with = listed(&with),
        nth = stretch.nth + 1,
        most = settings.turns,
    )
}

/// What is said as a stretch ends: a line in the thread, whether the
/// person is told and in what words, and in lead's way what the lead is
/// asked to answer the person with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ending {
    /// What the thread is left as: `agreed`, or `waiting` for a person.
    pub status: &'static str,
    pub line: String,
    pub tells: bool,
    pub says: String,
    /// Where the lead answers the person in the channel: what it is asked.
    pub lead: String,
}

/// The ending of a stretch between `agents` that came to `came` after
/// `turns`, in `channel`; `words` is what the last turn said, which for
/// `needs` is what is to be decided.
pub fn ending(
    settings: &Settings,
    came: &str,
    agents: &[String],
    turns: i64,
    channel: &str,
    words: &str,
) -> Ending {
    let who = listed(agents);
    let words = words.trim();
    let (status, told, line, says) = match came {
        "agreed" => (
            "agreed",
            "agree",
            format!("{who} agreed, after {}.", turns_said(turns)),
            format!("{who} agreed in a discussion in #{channel}."),
        ),
        "needs" => (
            "waiting",
            "needs",
            format!("{who} need a person to decide: {words}"),
            format!("{who} need you to decide, in #{channel}: {words}"),
        ),
        _ => (
            "waiting",
            "cap",
            format!("{who} did not agree in {turns} turns; a person's answer here lets them go on."),
            format!(
                "{who} did not agree in {turns} turns in #{channel}; your answer in the thread lets them go on."
            ),
        ),
    };
    let lead = if came == "agreed" && !settings.lead.is_empty() {
        format!(
            "Your discussion in the thread under the person's message has ended: you agreed. Answer the person now, \
             in a few lines, with what was agreed and what you will do. {words}"
        )
    } else {
        String::new()
    };
    Ending { status, tells: settings.tell_on.iter().any(|one| one == told), line, says, lead }
}

fn turns_said(turns: i64) -> String {
    if turns == 1 { "1 turn".into() } else { format!("{turns} turns") }
}

/// What `name` is told in a channel that offers `settings`: who else
/// takes part and what each is for, how it asks them and when. Nothing
/// for one that takes no part.
pub fn brief(settings: &Settings, name: &str) -> String {
    if !settings.takes_part(name) {
        return String::new();
    }
    let said =
        |list: &[Taking]| -> Vec<String> {
            list.iter()
                .filter(|one| one.name != name)
                .map(|one| {
                    if one.does.is_empty() {
                        one.name.clone()
                    } else {
                        format!("`{}`: {}", one.name, one.does)
                    }
                })
                .collect()
        };
    let mut lines = Vec::new();
    if settings.lead == name {
        lines.push(format!(
            "In this channel people talk to you. You may consult {}, for technical advice or to confirm a plan \
             before you answer: name one with `@` in what you say, and the two of you talk it over in a thread.",
            listed(&said(&settings.advisors))
        ));
    } else if settings.advisors.iter().any(|one| one.name == name) {
        lines.push(format!(
            "In this channel you advise `{}`, who talks to the people: you answer it in the thread it consults you \
             in, and answer a person only where they name you.",
            settings.lead
        ));
    }
    let peers = said(&settings.agents);
    if settings.agents.iter().any(|one| one.name == name) && !peers.is_empty() {
        lines.push(format!(
            "Others here you may talk things over with: {}. Name one with `@` to ask for a second opinion, \
             and the two of you discuss it in a thread.",
            listed(&peers)
        ));
    }
    if settings.on("plans") {
        lines.push("When what you answer is a plan or a proposal, end your turn with `plan`, and it is discussed before anything is done.".into());
    }
    lines.push(format!(
        "A discussion is {} turns at most, and a person is asked only for what they must decide.",
        settings.turns
    ));
    lines.join(" ")
}
