//! Links between topics, as plain values: how a page names another (or
//! one of its replies) in what it says, and which pages a text links to.
//!
//! A link is a Markdown link whose address is the page's id, so that it
//! reads well as text, an agent can write it, and neither a new title nor
//! a move changes where it leads:
//!
//! ```text
//! [Chytrid fungus](topic:@-topics-49)          the page
//! [what Ada said](topic:@-topics-49#817039)    one of its replies
//! ```
//!
//! The words are the writer's, as any link's are. What is read here is
//! read the same by the client (`ui/src/pagelinks.ts`, through
//! `prose.ts`), case for case with the table both read
//! (`ui/src/linkcases.json`): nothing in a block of code is a link, and a
//! picture's marks (`![…](…)`) are not one either. The blocks an article
//! draws as its boxes ([`BOXES`]) are read as words all the same, since
//! their links are drawn as links and lead where they say.

/// How the address of a link to a topic begins.
pub const LINKED: &str = "topic:";

/// The languages of the blocks of code an article draws as boxes (an
/// info box, a series, a note at its head: `ui/src/boxes.ts`), whose
/// lines hold links as words do.
pub const BOXES: [&str; 3] = ["infobox", "series", "note"];

/// Where a link leads: a page, by its id, and one of its replies, by the
/// number it was said at, where it names one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub topic: String,
    pub reply: Option<u64>,
}

/// Where an address leads, where it is a link to a topic: `topic:` and an
/// id of letters, digits and `@_.-`, and `#` and a number for a reply.
pub fn link_of(to: &str) -> Option<Link> {
    let rest = to.strip_prefix(LINKED)?;
    let (topic, reply) = match rest.split_once('#') {
        Some((topic, reply)) => (topic, Some(reply.parse::<u64>().ok()?)),
        None => (rest, None),
    };
    let fits = |c: char| c.is_ascii_alphanumeric() || "@_.-".contains(c);
    if topic.is_empty() || !topic.chars().all(fits) {
        return None;
    }
    Some(Link { topic: topic.to_string(), reply })
}

/// A line with each piece of code in it (between two backticks) blanked
/// out, as long as it was: nothing written as code is a link.
fn uncoded(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        match after.find('`').filter(|close| *close > 0) {
            Some(close) => {
                out.push_str(&rest[..open]);
                out.push_str(&" ".repeat(close + 2));
                rest = &after[close + 1..];
            }
            None => {
                out.push_str(&rest[..=open]);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The links of one line outside any block of code, in the order written.
fn links_of_line(line: &str, found: &mut Vec<Link>) {
    let line = uncoded(line);
    let mut from = 0;
    while let Some(at) = line[from..].find("](topic:") {
        let close = from + at;
        let address = &line[close + 2..];
        let Some(end) = address.find(|c: char| c == ')' || c.is_whitespace()) else { break };
        from = close + 2 + end;
        if !address[end..].starts_with(')') {
            continue;
        }
        // The words: after the last `]` before them, from the first `[`; never none, and never a picture's.
        let before = &line[..close];
        let words_from = before.rfind(']').map_or(0, |at| at + 1);
        let Some(open) = before[words_from..].find('[').map(|at| words_from + at) else { continue };
        if open + 1 == close || before[..open].ends_with('!') {
            continue;
        }
        if let Some(link) = link_of(&address[..end]) {
            found.push(link);
        }
    }
}

/// Each line of a text a link may be in, and the box it is in where it is
/// one's (which box of the text, counted from 0, and its language as
/// written): none in a block of code (between two lines of ``` or ~~~) but
/// for a box's ([`BOXES`], written right after its ```).
fn readable_lines<'a>(text: &'a str, mut each: impl FnMut(Option<(usize, &'a str)>, &'a str)) {
    // Inside a block of code, the box it is, where its lines are read: only a box's are.
    let mut fenced: Option<Option<(usize, &str)>> = None;
    let mut boxes = 0;
    for line in text.lines() {
        let lead = line.trim_start_matches(' ');
        if line.len() - lead.len() <= 3 && (lead.starts_with("```") || lead.starts_with("~~~")) {
            let boxed = lead
                .strip_prefix("```")
                .filter(|lang| BOXES.iter().any(|one| lang.eq_ignore_ascii_case(one)));
            fenced = if fenced.is_some() {
                None
            } else {
                boxes += usize::from(boxed.is_some());
                Some(boxed.map(|lang| (boxes - 1, lang)))
            };
            continue;
        }
        match fenced {
            None => each(None, line),
            Some(Some(boxed)) => each(Some(boxed), line),
            Some(None) => {}
        }
    }
}

/// The links a text holds, each once, in the order first written: none in
/// a block of code (between two lines of ``` or ~~~) or in code in a line,
/// but for a box's ([`BOXES`], written right after its ```).
pub fn links_in(text: &str) -> Vec<Link> {
    let mut found = Vec::new();
    readable_lines(text, |_, line| links_of_line(line, &mut found));
    let mut once: Vec<Link> = Vec::with_capacity(found.len());
    for link in found {
        if !once.contains(&link) {
            once.push(link);
        }
    }
    once
}

/// Whether a text links to the page `topic`, or to one of its replies.
pub fn links_to(text: &str, topic: &str) -> bool {
    text.contains(LINKED) && links_in(text).iter().any(|link| link.topic == topic)
}

/// Where a text links to a page: from anywhere but a series box (its
/// words, a list, another box), and the title of the first series box that
/// links to it, as its `title:` line says it (nothing written there is
/// empty words).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Linking {
    pub elsewhere: bool,
    pub series: Option<String>,
}

impl Linking {
    /// Where two texts of one page (its post and its summary) link to it, together.
    pub fn and(self, other: Linking) -> Linking {
        Linking { elsewhere: self.elsewhere || other.elsewhere, series: self.series.or(other.series) }
    }

    /// The title of the series box through which alone the page links, where
    /// that is the only way it does: so many pages of a series each carry
    /// the same box that, listed one by one, they would hide the pages that
    /// link from what they say.
    pub fn series_only(&self) -> Option<&str> {
        if self.elsewhere { None } else { self.series.as_deref() }
    }
}

/// Where a text links to the page `topic` ([`Linking`]): nothing where it does not.
pub fn linking_of(text: &str, topic: &str) -> Linking {
    let mut elsewhere = false;
    // Each series box, by its number: its title where it says one, and whether it links to the page.
    let mut series: Vec<(usize, Option<String>, bool)> = Vec::new();
    if text.contains(LINKED) {
        readable_lines(text, |boxed, line| {
            let mut found = Vec::new();
            links_of_line(line, &mut found);
            let links = found.iter().any(|link| link.topic == topic);
            let Some((nth, _)) = boxed.filter(|(_, lang)| lang.eq_ignore_ascii_case("series")) else {
                elsewhere |= links;
                return;
            };
            if series.last().is_none_or(|(last, ..)| *last != nth) {
                series.push((nth, None, false));
            }
            let Some((_, title, linked)) = series.last_mut() else { return };
            match line.trim_start().strip_prefix("title:") {
                Some(said) if title.is_none() => *title = Some(said.trim().to_string()),
                _ => *linked |= links,
            }
        });
    }
    let series =
        series.into_iter().find(|(_, _, linked)| *linked).map(|(_, title, _)| title.unwrap_or_default());
    Linking { elsewhere, series }
}
