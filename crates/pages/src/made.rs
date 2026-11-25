//! Pages: a document as the web page it would be published as.
//!
//! A document is a Markdown file. This turns one into the HTML of a page,
//! standing alone: its own style sheet inside it, nothing fetched from
//! anywhere. It is what "Preview as page" shows in the client, and it is
//! what publishing will write out, so the preview is not an approximation
//! of the published page but the same thing.
//!
//! The style sheet is the client's own file for how a document looks while
//! it is being written. One file, used in both places, is what keeps what is
//! typed and what is published from drifting apart.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd, html};

/// How a document looks as a page; shared with the client's editor.
pub const STYLE: &str = include_str!("../../../ui/src/page.css");

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        // A block of settings at the head of a file is about the document,
        // not part of it.
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
}

/// Text as it is written inside a page.
pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// The blocks of a document, as HTML.
pub fn body(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len() * 2);
    html::push_html(&mut out, Parser::new_ext(markdown, options()));
    out
}

/// Where a link or a picture of a document leads on the page being made.
pub enum Leads {
    /// There, written this way.
    To(String),
    /// Nowhere a reader of the page can go. Its words stay, as words.
    Nowhere,
}

/// The blocks of a document, as HTML, with whoever is making the page
/// asked of each link and each picture where it is to lead (`picture` says
/// which it is). It is [`body`] in every other way: a page that is served
/// and one that is written out are the same document made the same way,
/// and differ only in how what they link to is reached.
pub fn body_with(markdown: &str, leads: &mut dyn FnMut(bool, &str) -> Leads) -> String {
    // How many links and pictures that lead nowhere are open: their ends
    // are left out with their beginnings, and what is between is kept.
    let mut dropped = Vec::new();
    let events = Parser::new_ext(markdown, options()).filter_map(|event| match event {
        Event::Start(Tag::Link { link_type, dest_url, title, id }) => match leads(false, &dest_url) {
            Leads::To(dest) => {
                dropped.push(false);
                Some(Event::Start(Tag::Link { link_type, dest_url: dest.into(), title, id }))
            }
            Leads::Nowhere => {
                dropped.push(true);
                None
            }
        },
        Event::Start(Tag::Image { link_type, dest_url, title, id }) => match leads(true, &dest_url) {
            Leads::To(dest) => {
                dropped.push(false);
                Some(Event::Start(Tag::Image { link_type, dest_url: dest.into(), title, id }))
            }
            Leads::Nowhere => {
                dropped.push(true);
                None
            }
        },
        Event::End(TagEnd::Link | TagEnd::Image) => match dropped.pop() {
            Some(true) => None,
            _ => Some(event),
        },
        other => Some(other),
    });
    let mut out = String::with_capacity(markdown.len() * 2);
    html::push_html(&mut out, events);
    out
}

/// What a document calls itself: its first top-level heading, if it has one.
pub fn title(markdown: &str) -> Option<String> {
    let mut inside = false;
    let mut found = String::new();
    for event in Parser::new_ext(markdown, options()) {
        match event {
            Event::Start(Tag::Heading { level: HeadingLevel::H1, .. }) => inside = true,
            Event::End(TagEnd::Heading(HeadingLevel::H1)) if inside => break,
            Event::Text(text) | Event::Code(text) if inside => found.push_str(&text),
            _ => {}
        }
    }
    let found = found.trim();
    (!found.is_empty()).then(|| found.to_string())
}

/// A whole page for a document. `fallback` names it when it has no heading.
pub fn page(markdown: &str, fallback: &str) -> String {
    let title = title(markdown).unwrap_or_else(|| fallback.to_string());
    shell(&title, &body(markdown))
}

/// A whole page around what is already HTML: the same page a document
/// gets, for what stands beside documents (the front page of a library).
pub fn shell(title: &str, inside: &str) -> String {
    shell_beside(title, inside, "")
}

/// [`shell`] with something standing beside what is in the page (a
/// library's contents), after it in the page and laid out by the style
/// sheet.
pub fn shell_beside(title: &str, inside: &str, beside: &str) -> String {
    framed(title, &format!("<style>\n{STYLE}</style>\n"), inside, beside)
}

/// The same page with its style sheet in a file beside it, at `sheet` from
/// where the page is, and with what it may do said in the page itself:
/// for a page that is written out to be served by something that will say
/// nothing of the kind. `may` is a content security policy, and `beside`
/// is what stands beside what is in the page, as [`shell_beside`] has it.
pub fn standing(title: &str, sheet: &str, may: &str, inside: &str, beside: &str) -> String {
    let head = format!(
        "<meta http-equiv=\"Content-Security-Policy\" content=\"{}\">\n<link rel=\"stylesheet\" href=\"{}\">\n",
        escape(may),
        escape(sheet)
    );
    framed(title, &head, inside, beside)
}

fn framed(title: &str, head: &str, inside: &str, beside: &str) -> String {
    let class = if beside.is_empty() { "page" } else { "page navigated" };
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta name=\"color-scheme\" content=\"light dark\">\n<title>{}</title>\n{head}</head>\n\
         <body class=\"{class}\">\n<main>\n{inside}</main>\n{beside}</body>\n</html>\n",
        escape(title)
    )
}
