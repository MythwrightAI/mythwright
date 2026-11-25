//! Who is acting, and what they may do.
//!
//! Every request, command and blueprint run carries a principal. Identity
//! never comes from the network a request arrived on: reachability says who
//! *can* talk to the daemon, a credential says who *is* talking.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrincipalKind {
    User,
    Agent,
    Device,
    System,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Principal {
    pub kind: PrincipalKind,
    pub id: String,
    pub name: String,
    /// What this principal may do. A scope is `area` or `area:verb`; holding
    /// `area` grants every verb in it, and `*` grants everything.
    pub scopes: Vec<String>,
    /// Which key somebody made this is, by the key's id, when it is one:
    /// a key holds what it was given and may be withdrawn by itself, so
    /// what was done with one says which. What an instance was applied
    /// with is kept as its principal, and so says which key applied it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

/// Every scope the system checks, with what it allows.
pub const SCOPES: &[(&str, &str)] = &[
    ("admin", "manage accounts, credentials, devices and settings"),
    ("projects:read", "list projects and read their files"),
    ("projects:write", "create projects and change their files"),
    ("channels:read", "read channels and messages"),
    ("channels:write", "create channels and post messages"),
    ("inbox:read", "read one's own inbox: what was addressed to whoever holds this, and to nobody else"),
    (
        "inbox:write",
        "say that a line of one's own inbox was read, put one away, and have a blueprint tell somebody something",
    ),
    (
        "themes:read",
        "read one's own themes and which one is chosen, and how what one reads looks (its text, its width, light \
         or dark), which follow one to any browser, and the themes offered to everybody",
    ),
    (
        "themes:write",
        "make, change and take away one's own themes and choose one, and change how what one reads looks; \
         offering a theme to everybody, and withdrawing one, takes `admin` too",
    ),
    (
        "arranged:read",
        "read how one arranged the side panel for oneself (one's groups of channels, the channels pinned to the \
         top, what is pinned in Resources), which follows one to any browser",
    ),
    ("arranged:write", "change how one arranged the side panel for oneself"),
    (
        "clock:read",
        "read how a time is written for oneself (the zone, and a clock of twelve hours or twenty-four), which \
         follows one to any browser",
    ),
    ("clock:write", "change how a time is written for oneself"),
    ("board:read", "read work items"),
    ("board:write", "create, change and move work items"),
    ("records:read", "read the records of the collections blueprints declared"),
    ("records:write", "add, change and take away records of those collections, other than a board's cards"),
    (
        "columns",
        "add a column to a table for everybody who opens it, change what kind of thing it holds and what can be \
         chosen in it, and take it away; what a blueprint declares of a table is not changed by it",
    ),
    ("blueprints:read", "read blueprints, instances and runs"),
    ("blueprints:run", "start, apply and stop blueprint instances"),
    ("agents:read", "see agent slots"),
    ("agents:write", "create, change and talk to agent slots"),
    ("fleet:read", "see devices, services and metrics"),
    ("fleet:write", "change devices, services and routing"),
    (
        "private",
        "read, and change, what a table says is kept private; `private:<word>` is what is kept under one word",
    ),
    ("search", "search the knowledge base"),
    (
        "environments:secrets",
        "set, replace, take away and mark for agents the secrets of a project's environments, without ever being \
         shown one; the factory's owner holds it without being given it",
    ),
    (
        "secrets:use",
        "use what the factory keeps secret, without being shown it: tell an agent to draw its sign-in from those the \
         factory holds, and have a blueprint hand a secret to a program it runs",
    ),
    ("shell", "run commands on a device"),
    (
        "emit",
        "say an event of a blueprint's own topics from outside; `emit:<topic>` is one topic's, which is all a key \
         written into an address may hold",
    ),
    (
        "extensions",
        "run the programs of installed extension packages, as each package says they are run; \
         `extensions:<package>` is one package's",
    ),
];

/// One area of what can be held, as it is shown to somebody: the area,
/// and either what may be done in it, each with what that allows, or,
/// for an area that is one thing, what holding it allows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Area {
    pub area: &'static str,
    /// What holding it allows, where the area has no parts.
    pub does: Option<&'static str>,
    /// What may be done there. Holding the area holds every one of them.
    pub verbs: Vec<Verb>,
}

impl Area {
    /// What holding `scopes` comes to in one area, in words: what may be
    /// done there, `write` said as changing; nothing where nothing is held.
    pub fn held(&self, scopes: &[String]) -> Option<String> {
        let only: Vec<String> = scopes
            .iter()
            .filter_map(|scope| scope.strip_prefix(&format!("{}:", self.area)))
            .filter(|word| !self.verbs.iter().any(|verb| verb.verb == *word))
            .map(|word| format!("only {word}"))
            .collect();
        let mut held: Vec<String> = match (self.does, self.verbs.is_empty()) {
            (Some(does), true) => covers(scopes, self.area).then(|| does.to_string()).into_iter().collect(),
            _ => self
                .verbs
                .iter()
                .filter(|verb| covers(scopes, &format!("{}:{}", self.area, verb.verb)))
                .map(|verb| if verb.verb == "write" { "change".to_string() } else { verb.verb.to_string() })
                .collect(),
        };
        held.extend(only);
        match held.as_slice() {
            [] => None,
            [one] => Some(one.clone()),
            [first @ .., last] => Some(format!("{} and {last}", first.join(", "))),
        }
    }
}

/// One thing that may be done in an area (`read` of `board:read`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Verb {
    pub verb: &'static str,
    pub does: &'static str,
}

/// Every scope the system checks, by area and in the order they are
/// written above: what a page that says what somebody may do is drawn
/// from, so that no list of areas is written anywhere else.
pub fn areas() -> Vec<Area> {
    let mut all: Vec<Area> = Vec::new();
    for (scope, does) in SCOPES {
        let (area, verb) = match scope.split_once(':') {
            Some((area, verb)) => (area, Some(verb)),
            None => (*scope, None),
        };
        let at = match all.iter().position(|one| one.area == area) {
            Some(at) => at,
            None => {
                all.push(Area { area, does: None, verbs: Vec::new() });
                all.len() - 1
            }
        };
        match verb {
            Some(verb) => all[at].verbs.push(Verb { verb, does }),
            None => all[at].does = Some(does),
        }
    }
    all
}

/// The scopes whose last part is a word of somebody's own: what is kept
/// under one word, one topic, one package.
const NARROWED: &[&str] = &["private", "emit", "extensions"];

/// Whether `scope` is one the system checks, as it may be written down to
/// be held: a line of [`SCOPES`], an area of them, or one of those that
/// take a word of somebody's own after them (`private:<word>`,
/// `emit:<topic>`, `extensions:<package>`). Never `*`, which is the
/// system's alone.
pub fn named(scope: &str) -> bool {
    let area = |area: &str| SCOPES.iter().any(|(one, _)| *one == area || one.split(':').next() == Some(area));
    match scope.split_once(':') {
        None => area(scope),
        Some((first, rest)) => {
            SCOPES.iter().any(|(one, _)| *one == scope)
                || (NARROWED.contains(&first)
                    && !rest.is_empty()
                    && rest
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | '*')))
        }
    }
}

/// What a signed-in person gets by default.
pub const MEMBER_SCOPES: &[&str] = &[
    "projects",
    "channels",
    "inbox",
    "themes",
    "arranged",
    "clock",
    "board",
    "records",
    "columns",
    "blueprints",
    "agents",
    "fleet",
    "search",
    "secrets:use",
    "shell",
    "emit",
    "extensions",
];

/// What an agent gets unless its slot says otherwise: what its work is
/// done with. It reads what is running and starts and stops none of it:
/// whoever may apply may stop any instance, a fleet among them, and that
/// is given to an agent by saying so.
pub const AGENT_SCOPES: &[&str] =
    &["projects", "channels", "board", "records", "blueprints:read", "agents:read", "fleet:read", "search"];

/// How what an event takes begins where the thing it is of is kept to
/// those who are of it (`member:<what>`: a channel kept to its members).
/// It is no scope: nobody holds it, and who is of a thing is asked where
/// an event is handed to somebody, of whoever keeps that
/// (`journal::Journal::may_tell`).
pub const MEMBER: &str = "member:";

/// What `takes` says a thing is kept to the members of, where it says so.
pub fn kept_to(takes: &str) -> Option<&str> {
    takes.strip_prefix(MEMBER)
}

/// How what follows `member:` begins where a thing is one actor's alone
/// (`member:own:<actor>`: somebody's own themes). Nobody but that actor
/// is of it, and nothing has to be installed to say so: it is answered
/// where `member:` is read (`journal::Journal::may_tell`).
pub const OWN: &str = "own:";

/// What an event of something that is `actor`'s alone takes.
pub fn own(actor: &str) -> String {
    format!("{MEMBER}{OWN}{actor}")
}

/// Whether whoever holds `held` holds `scope`: the scope itself, the area
/// it is of, or everything. The one rule, for a credential and for a
/// list of scopes written down anywhere that is held against one.
///
/// Nothing held is being a member of something ([`MEMBER`]), everything
/// included: so whatever only knows that an event takes a scope keeps
/// such an event from everybody.
pub fn covers(held: &[String], scope: &str) -> bool {
    if kept_to(scope).is_some() {
        return false;
    }
    let area = scope.split(':').next().unwrap_or(scope);
    held.iter().any(|held| held == "*" || held == scope || held == area)
}

impl Principal {
    pub fn system() -> Self {
        Self {
            kind: PrincipalKind::System,
            id: "system".into(),
            name: "system".into(),
            scopes: vec!["*".into()],
            key: None,
        }
    }

    /// How this principal appears as an event's actor.
    pub fn actor(&self) -> String {
        match self.kind {
            PrincipalKind::User => format!("user:{}", self.id),
            PrincipalKind::Agent => format!("agent:{}", self.id),
            PrincipalKind::Device => format!("device:{}", self.id),
            PrincipalKind::System => "system".to_string(),
        }
    }

    pub fn can(&self, scope: &str) -> bool {
        covers(&self.scopes, scope)
    }

    /// This principal held to no more than `allow`: what both hold, a
    /// scope at a time, and the narrower of the two where one names a
    /// whole area and the other a part of it, as a hub's command is
    /// narrowed on a device. Who it is, and which key, stay as they were:
    /// what is done is still theirs, with less.
    pub fn within(&self, allow: &[String]) -> Principal {
        let mut scopes = Vec::new();
        for scope in &self.scopes {
            if scope == "*" {
                scopes.extend(allow.iter().cloned());
            } else if covers(allow, scope) {
                scopes.push(scope.clone());
            }
        }
        scopes.extend(allow.iter().filter(|scope| *scope != "*" && self.can(scope)).cloned());
        scopes.sort();
        scopes.dedup();
        Principal { scopes, ..self.clone() }
    }

    /// Errors unless the principal holds `scope`.
    pub fn require(&self, scope: &str) -> crate::Result<()> {
        if self.can(scope) {
            Ok(())
        } else {
            Err(crate::Error::denied(format!("{} is not allowed to do this (needs `{scope}`)", self.name)))
        }
    }
}
