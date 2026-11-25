//! What an agent asked in a topic is given: the topic and
//! the chain of replies it answers, and nothing of any channel; and, the
//! topic being a page of a wiki, where it is in its wiki
//! and the pages directly under it, by title and never what they say. A
//! function of plain values, so that whoever hands an agent its turn only
//! hands it on. It offers the agent only the commands it may run, and
//! says that its answer is said for it, so that it neither tries what it
//! would be refused nor posts its answer itself.

/// The most of a summary given whole.
pub const MOST_SUMMARY: usize = 16 * 1024;
/// The most of a body given whole; past it, its headings and its start.
pub const MOST_BODY: usize = 48 * 1024;
/// How many replies of a chain are given whole: past it, the first and
/// the last so many, with how many were left out between.
pub const MOST_CHAIN: usize = 24;
/// How many of the last of a long chain are given.
const LAST: usize = 20;
/// How many of the pages under a page are named: past it, how many more.
pub const MOST_PAGES: usize = 20;

/// Something said in a topic: who said it, and the words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Said {
    pub who: String,
    pub body: String,
}

/// The topic, as an agent asked there is told of it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct About {
    pub id: String,
    pub title: String,
    /// Where it stands: nothing for a topic that is not tracked as a question.
    pub status: Option<String>,
    /// Where it is, in words: `the project shop`, `the workspace Research`.
    pub place: String,
    pub summary: String,
    pub body: String,
    /// The pages above it, its wiki first, by title: nothing for a wiki.
    pub path: Vec<String>,
    /// The pages directly under it, in the order they stand, by title.
    pub pages: Vec<String>,
}

/// What the agent asked may do with topics itself, which is what says
/// which commands it is offered: none it would be refused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct May {
    /// It may read a topic whole, and the pages under it (`records:read`).
    pub read: bool,
    /// It may say something in a topic and make a page under it
    /// (`records:write`): each finds the topic first, so it is offered
    /// only with `read`.
    pub write: bool,
}

/// What an agent is asked of what it writes in a topic, always: where each
/// thing it states as a fact comes from, as a footnote, and every other
/// page it names linked by its id, in the forms the client draws
/// (`docs/topics.md`, *Links and sources*); and the forms an article's
/// boxes are written in (*Boxes and notes*). It is asked in every topic: a
/// topic that did not want it would need a setting kept, and none is.
fn cite(about: &About, read: bool) -> String {
    let cli = brand::CLI;
    let ids = if read { format!(" (`{cli} topic list` gives each page's id)") } else { String::new() };
    format!(
        "Say where what you state as a fact comes from: put a footnote marker after it, [^1], [^2] in the order \
         you first use them, and end your answer with one line for each, as \
         `[^1]: Author, *Title*, Publisher, Year. https://… (checked <the date you read it>)`; write only sources \
         you have read. When you name another page, link it by its id, `[its title](topic:<its id>)`{ids}: this \
         page is `[{title}](topic:{id})`. An article's boxes are blocks of code: ```infobox (`title:`, then \
         `Key: value` lines), ```series (`title:`, then a page's link a line) and ```note (a line pointing \
         elsewhere).\n",
        title = about.title,
        id = about.id,
    )
}

/// How what is cut short is said: with the command that reads it whole,
/// where the agent may run it, and otherwise that it was left out.
fn rest(show: Option<&str>) -> String {
    match show {
        Some(show) => format!("which {show} reads whole"),
        None => "left out here".to_string(),
    }
}

/// Cuts `text` to `most` bytes at a character's edge, saying so.
fn bounded(text: &str, most: usize, show: Option<&str>) -> String {
    if text.len() <= most {
        return text.to_string();
    }
    let mut end = most;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n[… more, {}]", &text[..end], rest(show))
}

/// A body past [`MOST_BODY`]: its headings, and its start.
fn body_of(body: &str, show: Option<&str>) -> String {
    if body.len() <= MOST_BODY {
        return body.to_string();
    }
    let headings: Vec<&str> = body.lines().filter(|line| line.trim_start().starts_with('#')).collect();
    format!("Its headings:\n{}\n\nIts start:\n{}", headings.join("\n"), bounded(body, MOST_BODY / 2, show))
}

/// What an agent asked in a topic is given: where it is answering, that
/// its answer is said for it, and the commands it may run there, as it
/// `may`; the summary, the body, the chain it answers from the top down to
/// what was said before the reply that asked it, and that reply, last, as
/// what is for it.
pub fn prompt(about: &About, chain: &[Said], newest: &Said, may: May) -> String {
    let cli = brand::CLI;
    let (read, write) = (may.read, may.read && may.write);
    let show = format!("`{cli} topic show {}`", about.id);
    let show = read.then_some(show.as_str());
    let mut out = format!(
        "You are answering in the topic \"{title}\" ({status}in {place}). Your answer is said for you as a \
         reply under the one that asked you, so do not post it yourself.",
        title = about.title,
        status = about.status.as_deref().map(|status| format!("{status}, ")).unwrap_or_default(),
        place = about.place,
    );
    match show {
        Some(show) if write => out.push_str(&format!(
            " {show} reads the whole topic, and `{cli} topic reply {id} --to <reply> <words>` says something \
             elsewhere in it.\n",
            id = about.id,
        )),
        Some(show) => out.push_str(&format!(" {show} reads the whole topic.\n")),
        None => out.push_str(
            " What is here of the topic is all you have of it: you may not read topics or say anything in them \
             yourself, so answer from what is here.\n",
        ),
    }
    if !about.path.is_empty() {
        out.push_str(&format!(
            "It is a page of the wiki \"{wiki}\", under {path}.\n",
            wiki = about.path[0],
            path = about.path.join(" › ")
        ));
    }
    if !about.pages.is_empty() {
        let named =
            about.pages.iter().take(MOST_PAGES).map(|title| format!("\"{title}\"")).collect::<Vec<_>>();
        let more = about.pages.len().saturating_sub(MOST_PAGES);
        let more = if more == 0 { String::new() } else { format!(" and {more} more") };
        let reads =
            if read { format!("; `{cli} topic show <page>` reads any of them") } else { String::new() };
        let makes = if write {
            format!(", and `{cli} topic make <title> --under {}` makes one", about.id)
        } else {
            String::new()
        };
        out.push_str(&format!("The pages under it: {}{more}{reads}{makes}.\n", named.join(", ")));
    }
    out.push_str(&cite(about, read));
    if !about.summary.trim().is_empty() {
        out.push_str(&format!("\nWhat is known now:\n{}\n", bounded(&about.summary, MOST_SUMMARY, show)));
    }
    if !about.body.trim().is_empty() {
        out.push_str(&format!("\nThe topic:\n{}\n", body_of(&about.body, show)));
    }
    if !chain.is_empty() {
        out.push_str("\nThe replies this answers, from the top:\n");
        let line = |said: &Said| format!("[{}] {}\n", said.who, said.body);
        if chain.len() <= MOST_CHAIN {
            chain.iter().for_each(|said| out.push_str(&line(said)));
        } else {
            out.push_str(&line(&chain[0]));
            let left = chain.len() - 1 - LAST;
            out.push_str(&format!("[{left} more replies, {}]\n", rest(show)));
            chain[chain.len() - LAST..].iter().for_each(|said| out.push_str(&line(said)));
        }
    }
    out.push_str(&format!("\nThe newest reply, from {}, is for you:\n{}", newest.who, newest.body));
    out
}
