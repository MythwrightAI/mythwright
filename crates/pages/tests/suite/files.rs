use pages::document::{ENDING, head, is_document, parts, setting, stem};
use pages::made::{escape, title};
use pages::mockup::{is_name, titled};
use pages::site::way_up;

#[test]
fn a_document_is_its_block_of_settings_its_heading_and_the_rest() {
    let text = "---\npublic: true\ntags: [a, b]\n---\n\n# Install\n\nRun it.\n";
    let read = parts(text);
    assert_eq!(read.head, vec!["public: true".to_string(), "tags: [a, b]".to_string()]);
    assert_eq!(read.heading.as_deref(), Some("Install"));
    assert_eq!(read.body, "Run it.");
    assert_eq!(setting(text, "public").as_deref(), Some("true"));
    assert_eq!(setting(text, "order"), None);
    assert_eq!(head("# Only a heading\n"), Vec::<String>::new());
}

#[test]
fn a_document_is_a_markdown_file_whose_name_can_be_written_in_an_id() {
    assert_eq!(ENDING, ".md");
    assert!(is_document("guides/install.md"));
    assert!(!is_document("guides/install.txt"));
    assert!(!is_document(".md"));
    assert!(!is_document("a:b.md"));
    assert_eq!(stem("guides/install.md"), "install");
}

#[test]
fn a_mockup_is_called_what_its_page_says_and_is_a_folder_with_a_plain_name() {
    assert_eq!(
        titled("<html><head><TITLE> Pricing\n  page </title></head></html>").as_deref(),
        Some("Pricing page")
    );
    assert_eq!(titled("<html><title></title></html>"), None);
    assert!(is_name("pricing"));
    assert!(!is_name(".hidden"));
    assert!(!is_name("a/b"));
    assert!(!is_name(""));
}

#[test]
fn a_page_says_what_it_is_called_and_escapes_what_it_writes() {
    assert_eq!(title("# Hello *there*\n\nmore").as_deref(), Some("Hello there"));
    assert_eq!(escape("<a & \"b\">"), "&lt;a &amp; &quot;b&quot;&gt;");
    assert!(way_up("Shop docs", "../index.html").contains("href=\"../index.html\""));
}
