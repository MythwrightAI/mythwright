//! What a package says about signing in: how its program is handed a
//! sign-in the factory holds, and how a limit shows.
//!
//! The application keeps sign-ins (`agents::signins`) and knows nothing
//! about any program that uses one. A package says, under `agent.signin`,
//! the variable its program reads each kind from, what else has to be so
//! for the program to use what it was handed and not the sign-in it would
//! find on the device, and what a turn looks like when the sign-in has
//! reached its limit. A package that says nothing cannot be handed one.
//!
//! This is the half that knows what the package said. It starts nothing
//! and keeps nothing: given a sign-in's kind and what it holds, it works
//! out what the program's environment gains and loses.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use blueprint::Cond;
use blueprint::template::Template;
use serde_json::{Map, Value, json};

use crate::extension::Check;

/// The kinds of sign-in there are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// A key for the provider's API: a value.
    Key,
    /// A subscription's long-lived token: a value, with nothing to renew.
    Token,
    /// A directory in which the provider's program keeps and renews its own
    /// sign-in. It is used where it is, and never copied.
    Folder,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Key, Kind::Token, Kind::Folder];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Key => "key",
            Kind::Token => "token",
            Kind::Folder => "folder",
        }
    }

    pub fn parse(name: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|kind| kind.name() == name)
    }

    /// Whether it is a subscription: a sign-in of a person's own plan, and
    /// not a key that is paid for by use.
    pub fn is_subscription(self) -> bool {
        self != Kind::Key
    }
}

/// How a limit shows in what a turn gave, and for how long it then holds.
struct LimitRule {
    when: Cond,
    /// When the sign-in is good again, in seconds since 1970, where the
    /// program says.
    until: Option<Template>,
    /// How long it is set aside where the program does not say.
    period: Duration,
}

/// What a turn that failed gave, for a package's `limit` to read.
pub struct Said<'a> {
    /// The sentence the turn failed with.
    pub error: &'a str,
    /// What the agent had answered by then, if anything.
    pub reply: &'a str,
    /// The code the program ended with; `None` if it was killed.
    pub code: Option<i32>,
    /// What it printed on standard error.
    pub stderr: &'a str,
    /// Now, in milliseconds since 1970.
    pub now_ms: i64,
}

/// A sign-in that has reached its limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limit {
    /// When it is good again, in milliseconds since 1970.
    pub until_ms: i64,
}

/// What a package says under `agent.signin`.
pub struct Handing {
    /// The variable each kind is handed in.
    vars: BTreeMap<Kind, String>,
    /// A variable pointed at a folder of the slot's own when a value is
    /// handed, for a program that would otherwise use the sign-in it finds
    /// in its usual folder.
    home: Option<String>,
    /// Other variables its program would sign in by: taken out of what the
    /// program is started with whenever it is handed a sign-in.
    without: Vec<String>,
    limit: Option<LimitRule>,
}

const FIELDS: &[&str] = &["key", "token", "folder", "home", "without", "limit"];

/// What a `limit` can read.
const LIMIT_READS: &[&str] = &["error", "reply", "code", "stderr", "now"];

/// Whether `name` can be a variable of a program's environment that a
/// package may say: one that is not the application's own to set.
fn is_variable(name: &str) -> bool {
    let shaped = name.starts_with(|first: char| first.is_ascii_alphabetic() || first == '_')
        && name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    let own = name == "PATH"
        || name.starts_with(&format!("{}_", brand::ENV_PREFIX))
        || name.starts_with("FACTORY_");
    shaped && !own
}

impl Handing {
    /// Reads what a manifest says under `agent.signin`.
    pub fn read(check: &mut Check, value: &Value) -> Option<Handing> {
        let map = check.fields("agent.signin", value, FIELDS)?;
        let variable = |check: &mut Check, field: &str| -> Option<String> {
            let at = format!("agent.signin.{field}");
            let name = check.text(&at, map.get(field), false);
            if name.is_empty() {
                return None;
            }
            if !is_variable(&name) {
                check.problem(
                    &at,
                    "must be the name of a variable of the program's environment, such as EXAMPLE_API_KEY, \
                     and not one the application sets itself",
                );
                return None;
            }
            Some(name)
        };
        let mut vars = BTreeMap::new();
        for kind in Kind::ALL {
            if let Some(name) = variable(check, kind.name()) {
                vars.insert(kind, name);
            }
        }
        let home = variable(check, "home");
        if vars.is_empty() {
            check.problem(
                "agent.signin",
                "says how none of `key`, `token` and `folder` is handed to the program; leave it out if the \
                 program cannot be handed a sign-in",
            );
        }
        let mut without = Vec::new();
        match map.get("without") {
            None | Some(Value::Null) => {}
            Some(Value::Array(names)) => {
                for name in names {
                    match name.as_str().filter(|name| is_variable(name)) {
                        Some(name) => without.push(name.to_string()),
                        None => check.problem(
                            "agent.signin.without",
                            format!("lists {name}, which is not the name of a variable the program reads"),
                        ),
                    }
                }
            }
            Some(_) => check.problem("agent.signin.without", "must be a list of variables' names"),
        }
        let limit = match map.get("limit") {
            None | Some(Value::Null) => None,
            Some(limit) => Self::limit(check, limit),
        };
        Some(Handing { vars, home, without, limit })
    }

    fn limit(check: &mut Check, value: &Value) -> Option<LimitRule> {
        let at = "agent.signin.limit";
        let map = check.fields(at, value, &["when", "until", "for"])?;
        let when = check.cond(&format!("{at}.when"), map.get("when"));
        let until = check.template(&format!("{at}.until"), map.get("until"));
        let period = match map.get("for") {
            Some(Value::String(period)) => match blueprint::duration::parse(period) {
                Ok(millis) if millis > 0 => Some(Duration::from_millis(millis)),
                Ok(_) => {
                    check.problem(&format!("{at}.for"), "must be longer than nothing");
                    None
                }
                Err(why) => {
                    check.problem(&format!("{at}.for"), why);
                    None
                }
            },
            _ => None,
        };
        let (Some(when), Some(period)) = (when, period) else {
            check.problem(
                at,
                "needs `when`, a condition on what the turn gave, and `for`, how long the sign-in is then \
                 set aside, such as 5h",
            );
            return None;
        };
        let reads = when.expr.paths().into_iter().chain(
            until.iter().flat_map(|until| until.exprs().flat_map(|expr| expr.paths()).collect::<Vec<_>>()),
        );
        for path in reads {
            if !LIMIT_READS.contains(&path[0].as_str()) {
                check.problem(
                    at,
                    format!(
                        "reads `{}`, which is not there to read; here there are: {}",
                        path[0],
                        LIMIT_READS.join(", ")
                    ),
                );
            }
        }
        Some(LimitRule { when, until, period })
    }

    /// The kinds this package's program can be handed.
    pub fn kinds(&self) -> Vec<Kind> {
        self.vars.keys().copied().collect()
    }

    /// Every variable its program would be told who to sign in as by, or
    /// where: what a program behind a wall keeps of the daemon's own
    /// environment when it signs in as whoever runs the daemon.
    pub fn variables(&self) -> Vec<String> {
        self.vars.values().chain(&self.home).chain(&self.without).cloned().collect()
    }

    /// Whether its program can be handed a sign-in of this kind.
    pub fn takes(&self, kind: Kind) -> bool {
        self.vars.contains_key(&kind)
    }

    /// What the program's environment gains and loses when it is handed a
    /// sign-in of `kind` that holds `held`: the value of a key or a token,
    /// or where a folder is. `home` is a folder of the slot's own.
    ///
    /// The variables of the other kinds are always among what it loses:
    /// the program is to find what it was handed and nothing else.
    pub fn dress(&self, kind: Kind, held: String, home: &Path) -> (BTreeMap<String, String>, Vec<String>) {
        let mut gains = BTreeMap::new();
        if let Some(variable) = self.vars.get(&kind) {
            gains.insert(variable.clone(), held);
        }
        // A folder is the program's own already; a value is given to a
        // program that may have a folder of the device's to fall back on.
        if kind != Kind::Folder
            && let Some(variable) = &self.home
        {
            gains.insert(variable.clone(), home.to_string_lossy().into_owned());
        }
        let loses = self
            .vars
            .values()
            .chain(&self.without)
            .filter(|variable| !gains.contains_key(*variable))
            .cloned()
            .collect();
        (gains, loses)
    }

    /// Whether the package says this turn reached its sign-in's limit, and
    /// if so when the sign-in is good again.
    pub fn limited(&self, said: &Said<'_>) -> Option<Limit> {
        let rule = self.limit.as_ref()?;
        let mut scope = Map::new();
        scope.insert("error".to_string(), json!(said.error));
        scope.insert("reply".to_string(), json!(said.reply));
        scope.insert("code".to_string(), json!(said.code));
        scope.insert("stderr".to_string(), json!(said.stderr));
        scope.insert("now".to_string(), json!(said.now_ms.div_euclid(1000)));
        if !rule.when.test(&scope).unwrap_or(false) {
            return None;
        }
        // The program's own word, where it gave one that can be read and
        // is still to come; the package's otherwise.
        let read =
            rule.until.as_ref().and_then(|until| until.render(&scope).ok()).and_then(|until| match until {
                Value::Number(seconds) => seconds.as_f64(),
                Value::String(seconds) => seconds.trim().parse::<f64>().ok(),
                _ => None,
            });
        let until_ms = read
            .map(|seconds| (seconds * 1000.0) as i64)
            .filter(|until| *until > said.now_ms)
            .unwrap_or(said.now_ms + rule.period.as_millis() as i64);
        Some(Limit { until_ms })
    }
}
