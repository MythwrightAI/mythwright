//! The guides of `docs/` that say they are public at their head, built into
//! the program as its release notes are: whatever writes the program's
//! pages out has them from an installed copy, with no checkout beside it.
//!
//! Only those are built in. A test of the command line's suite holds this
//! list to the folder: every guide there that says it is public is here,
//! and nothing here says otherwise. The design records (how the factory is
//! built and why, the build's figures, what is planned) say nothing, and
//! are no part of what is written out.

/// Each public guide, by its file's name without `.md`, and its text.
pub const GUIDES: &[(&str, &str)] = &[
    // What the program is for: the page the client opens from the rail's mark.
    ("about", include_str!("../../../docs/about.md")),
    ("first-run", include_str!("../../../docs/first-run.md")),
    ("blueprints", include_str!("../../../docs/blueprints.md")),
    // Every button the app has for blueprints, each with an example to try.
    ("blueprint-pages", include_str!("../../../docs/blueprint-pages.md")),
    ("example-ab-service", include_str!("../../../docs/example-ab-service.md")),
    ("example-choosing-models", include_str!("../../../docs/example-choosing-models.md")),
    ("example-self-maintaining", include_str!("../../../docs/example-self-maintaining.md")),
    ("example-team-wiki", include_str!("../../../docs/example-team-wiki.md")),
    ("example-preview-pipeline", include_str!("../../../docs/example-preview-pipeline.md")),
    ("extensions", include_str!("../../../docs/extensions.md")),
    ("own-agent", include_str!("../../../docs/own-agent.md")),
    ("environments", include_str!("../../../docs/environments.md")),
    ("topics", include_str!("../../../docs/topics.md")),
    ("following", include_str!("../../../docs/following.md")),
    ("library", include_str!("../../../docs/library.md")),
    ("views", include_str!("../../../docs/views.md")),
    ("mockups", include_str!("../../../docs/mockups.md")),
    ("profiles", include_str!("../../../docs/profiles.md")),
    ("stores", include_str!("../../../docs/stores.md")),
];

/// The guide called `name`.
pub fn guide(name: &str) -> &'static str {
    GUIDES.iter().find(|(called, _)| *called == name).map_or("", |(_, text)| text)
}

/// How a guide of `docs/` says it is public on its first line: a comment,
/// which a repository's page of the file does not show, where a block of
/// settings is shown as a table above the guide.
pub const MARK: &str = "<!-- public -->";

/// The block of settings at the head of a Markdown text, line by line,
/// and what comes after it. A first line that is [`MARK`] is the block
/// `public: true`.
fn head(text: &str) -> (Vec<&str>, &str) {
    if let Some(after) = text.strip_prefix(MARK).and_then(|after| after.strip_prefix('\n')) {
        return (vec!["public: true"], after);
    }
    let Some(after) = text.strip_prefix("---\n") else { return (Vec::new(), text) };
    match after.split_once("\n---\n") {
        Some((head, rest)) => (head.lines().collect(), rest),
        None => (Vec::new(), text),
    }
}

/// Whether a Markdown text says it is public at its head: [`MARK`], or
/// `public: true` in its block of settings.
pub fn public(text: &str) -> bool {
    head(text).0.iter().any(|line| {
        line.split_once(':').is_some_and(|(name, said)| name.trim() == "public" && said.trim() == "true")
    })
}

/// A Markdown text without the block of settings at its head: what `help`
/// prints of a guide.
pub fn body(text: &str) -> &str {
    head(text).1.trim_start_matches('\n')
}
