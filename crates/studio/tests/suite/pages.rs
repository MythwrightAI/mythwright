//! A document as the page it would be published as.

use studio::pages::{STYLE, body, page, title};

const DOCUMENT: &str = "---\npublish: true\n---\n# Riverside Shop\n\nA shop for *well made* things.\n\n\
- one\n- two\n\n> Quoted.\n\n```sh\ncargo build\n```\n\n| Item | Price |\n|---|---|\n| Mug | 9 |\n\n~~gone~~ and `code`.\n";

#[test]
fn a_document_becomes_the_blocks_it_was_written_as() {
    let html = body(DOCUMENT);
    for expected in [
        "<h1>Riverside Shop</h1>",
        "<p>A shop for <em>well made</em> things.</p>",
        "<li>one</li>",
        "<blockquote>",
        "<pre><code class=\"language-sh\">cargo build\n</code></pre>",
        "<th>Item</th>",
        "<td>Mug</td>",
        "<del>gone</del>",
        "<code>code</code>",
    ] {
        assert!(html.contains(expected), "missing {expected} in:\n{html}");
    }
}

#[test]
fn settings_at_the_head_of_a_file_are_about_the_document_and_not_part_of_the_page() {
    let html = body(DOCUMENT);
    assert!(!html.contains("publish"), "{html}");
    assert_eq!(title(DOCUMENT).as_deref(), Some("Riverside Shop"));
}

#[test]
fn a_page_stands_alone_and_is_named_after_its_first_heading() {
    let whole = page(DOCUMENT, "notes.md");
    assert!(whole.starts_with("<!doctype html>"));
    assert!(whole.contains("<title>Riverside Shop</title>"));
    assert!(whole.contains("<body class=\"page\">"));
    // Its style is inside it, and is the very file the editor draws with.
    assert!(whole.contains(STYLE) && STYLE.contains(".page h1"));
    assert!(!whole.contains("<link") && !whole.contains("<script") && !whole.contains("src=\"http"));

    let untitled = page("Just a paragraph with <angle> & \"quotes\".", "a <b>file</b>.md");
    assert!(untitled.contains("<title>a &lt;b&gt;file&lt;/b&gt;.md</title>"), "a name is text, not markup");
    assert_eq!(title("no heading here"), None);
}
