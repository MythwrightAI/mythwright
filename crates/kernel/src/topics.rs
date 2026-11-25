//! A forum's topics, as plain values: what a topic
//! says of itself, how its title is told from another's, what a change of
//! it comes to, and where a reply goes among the others.
//!
//! A topic is a row of the factory's own table of topics (at
//! [`crate::place::FACTORY`]), its body a document kept in the row, and
//! its replies the row's comments, each of which may answer another
//! (`parent`). What is decided of a topic is decided here, with its cases
//! in the kernel's suite, and the keeper of topics (the `topics` crate)
//! does what it is told.

use serde_json::{Map, Value};

pub mod linked;
pub mod tree;

pub use linked::{LINKED, Link, Linking, link_of, linking_of, links_in, links_to};
pub use tree::{Landing, Line, PARENT, Refused, Tree, between, landing};

/// The table every topic is a row of, at the factory's own place.
pub const TOPICS: &str = "topics";
/// The table of the categories a topic is filed under.
pub const CATEGORIES: &str = "categories";
/// The model a topic's table is, which the daemon reads by name.
pub const MODEL: &str = "topic";

/// Where a topic stands, in the order they are offered: `open` until it
/// is answered or closed, and `standing` for one that is never answered
/// but kept up to date. A topic may stand nowhere: one that is no question
/// (a wiki's page, a thread that gathers what is known) has no status, and
/// is made so where nothing is said.
pub const STATUSES: &[&str] = &["open", "answered", "closed", "standing"];
/// Where a topic stands once it is a question again: what taking its
/// answer back leaves it.
pub const OPEN: &str = "open";
/// What a topic may be made as, beside no status: what needs nothing
/// done to it first.
pub const MADE_AS: &[&str] = &["open", "standing"];

/// The fields whose change is a revision of a topic: what it says.
pub const REVISED: &[&str] = &["title", "body", "summary"];

/// How the keeper of topics' own tables are made, a step at a time (its
/// list, migrated as `topics`): steps are only ever added at the end. Here,
/// as plain values, so that what a copy of the factory knows of them
/// (`studio::copy::KNOWS`) is said below the keeper.
pub const STEPS: &[&str] = &["
    -- Each revision of a topic, whole: what it said, who said it and
    -- through what. The first is the topic as it was made.
    CREATE TABLE topic_revisions (
        topic TEXT NOT NULL,
        nth INTEGER NOT NULL,
        title TEXT NOT NULL,
        body TEXT NOT NULL,
        summary TEXT NOT NULL,
        by TEXT,
        through TEXT,
        at INTEGER NOT NULL,
        PRIMARY KEY (topic, nth)
    ) WITHOUT ROWID;
"];

/// How deep replies nest: a reply to one this deep is put beside it.
pub const DEPTH: usize = 8;

/// The most characters a topic's body or summary holds.
pub const MOST_DOCUMENT: usize = 512 * 1024;
/// The most characters a topic's title holds.
pub const MOST_TITLE: usize = 200;

/// A title as it is told from another: its words with the spaces between
/// them folded and their capitals taken away, so that two topics are not
/// called the same by a capital or a space.
pub fn title_key(title: &str) -> String {
    title.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// A title as it is written, or why it cannot be one.
pub fn titled(title: &str) -> Result<String, String> {
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.is_empty() {
        return Err("a topic is called something: its title is words".to_string());
    }
    if title.chars().count() > MOST_TITLE {
        return Err(format!("a topic's title is {MOST_TITLE} characters at most"));
    }
    Ok(title)
}

/// Whether a topic's change is a revision: its title, body or summary
/// is another than it was. A topic made is its first revision.
pub fn revised(before: Option<&Map<String, Value>>, after: &Map<String, Value>) -> bool {
    let Some(before) = before else { return true };
    REVISED.iter().any(|field| said(before.get(*field)) != said(after.get(*field)))
}

/// What a field says, nothing and empty words being the same.
fn said(value: Option<&Value>) -> &str {
    value.and_then(Value::as_str).unwrap_or_default()
}

/// Where a topic stands as its row says: nothing where it says nothing,
/// a topic that is not tracked as a question.
pub fn status_of(data: &Map<String, Value>) -> Option<&str> {
    Some(said(data.get("status"))).filter(|status| !status.is_empty())
}

/// The status a topic is made with, of what was said: none where nothing
/// was, or why it cannot be made so.
pub fn made_status(said: Option<&str>) -> Result<Option<&'static str>, String> {
    let said = said.map(str::trim).unwrap_or_default();
    if said.is_empty() {
        return Ok(None);
    }
    if let Some(status) = MADE_AS.iter().find(|status| **status == said) {
        return Ok(Some(*status));
    }
    let answered = if said == "answered" {
        " A topic is answered by marking one of its replies as the answer, once it has one."
    } else {
        ""
    };
    Err(format!(
        "a topic is made `open` or `standing`, or with no status where nothing is said, and not `{said}`.{answered}"
    ))
}

/// What is said where a topic that is not tracked as a question is to be
/// answered, or its answer taken back: what makes it one, with the
/// command line called `cli`.
pub fn not_a_question(cli: &str, title: &str, id: &str) -> String {
    format!(
        "`{title}` is not tracked as a question, so no reply answers it: give it a status first, with \
         `{cli} record set {id} status=open` or the status at the head of its page"
    )
}

/// What is said of a topic whose status came to another: the event's
/// topic, or nothing where it stands where it stood. A topic made with no
/// status says nothing of it; one made with a status, or given one, says
/// what it came to; one whose status is taken away is `topic.untracked`.
pub fn status_said(before: Option<&Map<String, Value>>, after: &Map<String, Value>) -> Option<&'static str> {
    let now = status_of(after);
    let was = before.map(status_of);
    if was == Some(now) || (was.is_none() && now.is_none()) {
        return None;
    }
    Some(match now {
        None => "topic.untracked",
        Some("answered") => "topic.answered",
        Some("closed") => "topic.closed",
        Some("standing") => "topic.standing",
        Some(_) => "topic.opened",
    })
}

/// Whether whoever follows a page is told of an event of it (`topic` and
/// what it carries, `data`), where the page is `under` another now: a
/// reply on it, a change of what it says or of how it stands, and its
/// being put somewhere else, under another page or at the top
/// (`topic.moved` with another page above it than before), or, for a
/// wiki, in another workspace or project (`topic.placed` of a page that
/// is under none). A page put in another order among the pages beside
/// it, and one carried along with the page above it, is where it was in
/// its wiki, and tells nobody; nor does anything else of it.
pub fn tells_followers(topic: &str, data: &Value, under: bool) -> bool {
    fn parent(data: Option<&Value>) -> Option<&str> {
        data.and_then(|data| data.get(PARENT)).and_then(Value::as_str).filter(|said| !said.is_empty())
    }
    match topic {
        "record.commented" | "topic.revised" | "topic.answered" | "topic.closed" | "topic.opened"
        | "topic.standing" | "topic.untracked" => true,
        "topic.moved" => parent(Some(data)) != parent(data.get("was")),
        "topic.placed" => !under,
        _ => false,
    }
}

/// Who of those who follow a page now (`now`) followed it as one of its
/// changes was made, of whoever began or stopped following it since, each
/// as it was said, in the order it was (`since`: who, and whether they
/// follow it from then on): somebody whose first word since was that they
/// began did not follow it then. Who stopped is not among `now`, and is
/// told nothing more of it.
pub fn followed_then(now: &[String], since: &[(String, bool)]) -> Vec<String> {
    let began = |who: &String| since.iter().find(|(one, _)| one == who).is_some_and(|(_, follows)| *follows);
    now.iter().filter(|who| !began(who)).cloned().collect()
}

/// What a change of a page a follower is told of did, in a few words, as
/// their line of the page says it: nothing for a reply, whose own words
/// say it, nor for anything a follower is not told of.
pub fn change_said(topic: &str) -> Option<&'static str> {
    Some(match topic {
        "topic.revised" => "Revised the page",
        "topic.answered" => "Marked it answered",
        "topic.closed" => "Closed it",
        // Said both of a topic opened again and of one first tracked as a question.
        "topic.opened" => "Marked it open",
        "topic.standing" => "Marked it standing",
        "topic.untracked" => "Removed its status",
        "topic.moved" | "topic.placed" => "Moved it",
        _ => return None,
    })
}

/// Which reply a new one is put under, of the chain from the top of its
/// chain down to the reply it answers (`chain`, the top first): that one
/// where the new reply is no deeper than [`DEPTH`], and otherwise the one
/// at the depth before the last, so that it stands beside the reply it
/// answers at the deepest there is. Nothing where it answers none.
pub fn placed_under(chain: &[u64]) -> Option<u64> {
    if chain.len() < DEPTH { chain.last().copied() } else { chain.get(DEPTH - 2).copied() }
}

/// The chain from the top down to `reply`, of the replies of one topic
/// (each its number and the reply it answers): the top first, `reply`
/// last; nothing where `reply` is none of them. A reply that answers one
/// that is not there is at the top of its chain.
pub fn chain(replies: &[(u64, Option<u64>)], reply: u64) -> Vec<u64> {
    let parent_of = |seq: u64| replies.iter().find(|(one, _)| *one == seq).map(|(_, parent)| *parent);
    let mut chain = Vec::new();
    let mut at = Some(reply);
    while let Some(seq) = at {
        if chain.contains(&seq) || chain.len() > replies.len() {
            break;
        }
        match parent_of(seq) {
            Some(parent) => {
                chain.push(seq);
                at = parent;
            }
            None => break,
        }
    }
    chain.reverse();
    chain
}

/// Where a topic is: a workspace's own, or a project's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    Workspace(String),
    Project(String),
}

impl Place {
    /// The place a topic's row says: exactly one of `workspace` and
    /// `project`, and nothing where it says neither or both.
    pub fn of(data: &Map<String, Value>) -> Option<Place> {
        match (said(data.get("workspace")), said(data.get("project"))) {
            (workspace, "") if !workspace.is_empty() => Some(Place::Workspace(workspace.to_string())),
            ("", project) if !project.is_empty() => Some(Place::Project(project.to_string())),
            _ => None,
        }
    }

    /// The two cells of a row that say it, as they are written.
    pub fn cells(&self) -> (Value, Value) {
        match self {
            Place::Workspace(id) => (Value::String(id.clone()), Value::Null),
            Place::Project(id) => (Value::Null, Value::String(id.clone())),
        }
    }

    /// The project it is, where it is a project's.
    pub fn project(&self) -> Option<&str> {
        match self {
            Place::Project(id) => Some(id),
            Place::Workspace(_) => None,
        }
    }
}
