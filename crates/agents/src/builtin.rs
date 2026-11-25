//! The built-in assistant: an adapter that needs no model.
//!
//! It understands a handful of plainly worded requests and carries them out
//! with the same authority, through the same services, as any other agent in
//! its slot. It exists so the product does something useful before an agent
//! CLI is installed or signed in, and so the whole path from "a message in a
//! channel" to "files in a project" can be tested without one.
//!
//! It is honest about what it is: anything it does not understand, it says
//! so, and says how to bring in a real agent.

use kernel::{BoxFut, Principal};
use studio::Studio;

use crate::adapter::{Adapter, Reply, Turn};

pub(crate) struct Builtin {
    studio: Studio,
}

impl Builtin {
    pub fn new(studio: Studio) -> Self {
        Self { studio }
    }
}

/// The request itself, without the channel context a message turn is given.
fn request(prompt: &str) -> &str {
    let body = prompt.rsplit_once("is for you:\n").map_or(prompt, |(_, body)| body);
    body.split("\n\nWhen you have done what you can, end your reply").next().unwrap_or(body).trim()
}

/// What follows `keyword` in `text`, as a name: without lead-in words, quotes
/// or the rest of the sentence.
fn name_after(text: &str, keyword: &str) -> Option<String> {
    let at = text.to_lowercase().find(keyword)?;
    let mut rest = text[at + keyword.len()..].trim_start_matches([' ', ':', '-']);
    for lead in ["called ", "named ", "for ", "of ", "the "] {
        if rest.to_lowercase().starts_with(lead) {
            rest = &rest[lead.len()..];
        }
    }
    let rest = rest.split(['\n', '.', '!', '?']).next().unwrap_or_default();
    let name = rest.trim().trim_matches(['"', '\'', '`', '*']).trim_end_matches(" please").trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// Whether the first line of a request asks for `noun` with one of `verbs`,
/// in that order: "set up a project", not "the project I set up".
///
/// Deliberately narrow. This assistant acts on what it matches, so a loose
/// match is worse than no match: "please take the card called Add a cart"
/// must not add a card.
fn asks_for(request: &str, verbs: &[&str], noun: &str) -> bool {
    let first = request.lines().next().unwrap_or_default().to_lowercase();
    let Some(noun_at) = first.find(noun) else { return false };
    verbs.iter().any(|verb| first[..noun_at].contains(verb))
}

const START: &[&str] = &["set up", "setup", "create", "start", "new", "make", "scaffold", "init"];
const ADD: &[&str] = &["add", "create", "new", "file"];

impl Builtin {
    /// The project a request is about: the one it names, else the slot's
    /// own, else the only one there is.
    async fn project(&self, named: Option<String>, slot_project: Option<&str>) -> Result<String, String> {
        let projects = self.studio.projects.list().await.map_err(|err| err.message)?;
        if let Some(named) = named {
            let slug = blueprint::expr::slug(&named);
            return match projects.iter().find(|project| project.id == slug) {
                Some(project) => Ok(project.id.clone()),
                None => Err(format!("There is no project called `{slug}`. Ask me to set one up first.")),
            };
        }
        if let Some(project) = slot_project {
            return Ok(project.to_string());
        }
        match projects.as_slice() {
            [only] => Ok(only.id.clone()),
            [] => {
                Err("There are no projects yet. Ask me to set one up: \"set up a new project <name>\"."
                    .into())
            }
            _ => Err("Which project? Say \"set up the fleet for <name>\".".into()),
        }
    }

    /// Carries out a request it understands, or returns `None`.
    async fn answer(&self, text: &str, by: &Principal, slot_project: Option<&str>) -> Option<String> {
        let cli = brand::CLI;

        if asks_for(text, START, "fleet") {
            let named = name_after(text, " for ").or_else(|| name_after(text, " of "));
            let project = match self.project(named, slot_project).await {
                Ok(project) => project,
                Err(message) => return Some(message),
            };
            // A fleet works a board: asked for one where there is none,
            // it says so before anybody starts agents with nothing to work.
            let board = match self.studio.board.columns(&project).await {
                Ok(_) => String::new(),
                Err(_) => format!(
                    " `{project}` has no board yet, and a fleet works one: give it its board first, by applying \
                     `std/board` to it (`{cli} apply std/board --project {project}`)."
                ),
            };
            return Some(match self.studio.projects.init_fleet(&project, by).await {
                Ok(entry) => format!(
                    "The fleet for `{project}` is set up: `{}` is kept for the project now. Open it to read the blueprint; \
                     its Node view shows the same thing as a graph. To start the fleet, press Apply there \
                     (or run `{cli} apply fleet --project {project}`).{board}",
                    entry.path
                ),
                Err(err) => format!("I could not set the fleet up: {}", err.message),
            });
        }

        if asks_for(text, START, "project") {
            let Some(name) = name_after(text, "project") else {
                return Some(
                    "What should the project be called? Say \"set up a new project <name>\".".into(),
                );
            };
            return Some(match self.studio.projects.create(&name, None, by).await {
                Ok(project) => {
                    let files: Vec<String> = self
                        .studio
                        .projects
                        .tree(&project.id)
                        .await
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|entry| !entry.dir)
                        .map(|entry| format!("`{}`", entry.path))
                        .collect();
                    // Its board before its fleet: a fleet works a board,
                    // and one asked for first comes up with none to work.
                    format!(
                        "Created the project **{}** (`{}`) at {}. It starts with {}. \
                         Next it needs a board, which a blueprint makes: apply `std/board` to it (it is among the \
                         blueprints under Factory, or `{cli} apply std/board --project {}`). Then ask me to set up \
                         its fleet, which works that board.",
                        project.name,
                        project.id,
                        project.path,
                        files.join(", "),
                        project.id
                    )
                }
                Err(err) if err.kind == kernel::ErrorKind::Conflict => err.message,
                Err(err) => format!("I could not create the project: {}", err.message),
            });
        }

        for noun in ["card", "ticket"] {
            if asks_for(text, ADD, noun) {
                let (Some(title), Ok(project)) =
                    (name_after(text, noun), self.project(None, slot_project).await)
                else {
                    return Some(
                        "What should the card say, and in which project? Say \"add a card <title>\".".into(),
                    );
                };
                return Some(match self.studio.board.add(&project, &title, "", None, Vec::new(), by).await {
                    Ok(item) => format!("Added `{}` to the {project} board: {}.", item.id, item.title),
                    Err(err) => format!("I could not add the card: {}", err.message),
                });
            }
        }
        None
    }
}

/// What it says to a person who asks for something it cannot do: what it
/// can, and the command by which `slot` comes to run on something that
/// does the rest.
fn introduction(slot: &str) -> String {
    let cli = brand::CLI;
    format!(
        "I am the built-in assistant. I work without a language model, so I only understand a few requests:\n\
         - \"set up a new project <name>\"\n\
         - \"set up the fleet for <name>\"\n\
         - \"add a card <title>\"\n\n\
         For anything else this slot needs a real agent: {}. `{cli} agent adapters` lists what this device has.",
        crate::real::how(slot, &crate::real::Whose::Unasked)
    )
}

/// How it begins when it is given work it cannot do. What follows says
/// how its slot is given a program that can ([`cannot`]), and whoever
/// knows whose the slot is says that more exactly ([`crate::real`]).
pub(crate) const CANNOT: &str = "the built-in assistant cannot do this work";

/// Why work given to `slot` was not done, with what to do about it.
pub(crate) fn cannot(slot: &str, whose: &crate::real::Whose) -> String {
    format!("{CANNOT}; {}", crate::real::how(slot, whose))
}

impl Adapter for Builtin {
    fn run(&self, turn: Turn) -> BoxFut<'static, Result<Reply, crate::adapter::Failure>> {
        let assistant = Builtin { studio: self.studio.clone() };
        Box::pin(async move {
            let slot_project = turn.env.get(&brand::env_var("PROJECT")).cloned();
            let in_conversation = turn.prompt.contains("is for you:\n");
            let understood =
                assistant.answer(request(&turn.prompt), &turn.principal, slot_project.as_deref()).await;
            // Asked to choose, it cannot judge, so it chooses the option that
            // changes least: it never says "retry" or "continue" on a guess.
            // Only where choosing is all that is asked: work that may end
            // more than one way is still work, which it cannot do.
            let asked_to_choose = turn
                .prompt
                .rsplit_once("choosing one:\nDECISION: <")
                .map(|(_, rest)| rest.trim_end().trim_end_matches('>'));
            let text = match (understood, asked_to_choose) {
                (_, Some(options)) => {
                    let options: Vec<&str> = options.split('|').map(str::trim).collect();
                    let cautious = ["fail", "stop", "wait", "no", "abort"];
                    let choice = options
                        .iter()
                        .find(|option| cautious.contains(&option.to_lowercase().as_str()))
                        .or(options.last());
                    format!(
                        "I cannot judge this without a language model, so I am choosing the cautious option.\nDECISION: {}",
                        choice.copied().unwrap_or_default()
                    )
                }
                (Some(text), None) => text,
                // In a conversation it explains itself. Given work to do, it
                // says plainly that it did not do it: a reply would read as
                // the work having been done.
                (None, None) if in_conversation => introduction(&turn.slot),
                (None, None) => return Err(cannot(&turn.slot, &crate::real::Whose::Unasked).into()),
            };
            (turn.progress)(&text);
            Ok(Reply { text, ..Reply::default() })
        })
    }
}
