//! A library's contents in the order its front pages say (`kernel::contents`).

use std::collections::BTreeMap;

use kernel::contents::{Document, Place, contents, here_of, in_order, order_in, ordered};

fn lines(of: &[&str]) -> Vec<String> {
    of.iter().map(|one| one.to_string()).collect()
}

fn place(path: &str, title: &str) -> Place {
    Place { path: path.to_string(), title: title.to_string(), under: Vec::new() }
}

fn paths(places: &[Place]) -> Vec<&str> {
    places.iter().map(|place| place.path.as_str()).collect()
}

#[test]
fn an_order_is_read_from_a_front_pages_head_as_a_list_the_words_with_commas_or_a_line_for_each() {
    let cases: &[(&[&str], &[&str])] = &[
        (&["public: true", "order: [install, usage.md, more/]"], &["install", "usage.md", "more/"]),
        (&["order: install, usage"], &["install", "usage"]),
        (&["order: [\"install\", 'faq']"], &["install", "faq"]),
        (&["order:", "  - install", "  - more/", "public: true"], &["install", "more/"]),
        (&["order:", "- install", "-   faq  "], &["install", "faq"]),
        (&["order: []"], &[]),
        (&["order:"], &[]),
        (&["order: [install, , faq]"], &["install", "faq"]),
        // Another setting that only begins the same says nothing of it.
        (&["ordered: [install]"], &[]),
        (&["public: true"], &[]),
        (&[], &[]),
    ];
    for (head, said) in cases {
        assert_eq!(order_in(&lines(head)), lines(said), "{head:?}");
    }
}

#[test]
fn what_an_order_says_comes_first_as_it_says_it_and_the_rest_after_by_title_documents_before_folders() {
    let places = || {
        vec![
            place("zebra.md", "Zebra"),
            place("install.md", "Installing it"),
            place("about.md", "about us"),
            place("faq.md", "Questions"),
            place("guides/", "Guides"),
            place("api/", "The API"),
            place("same.md", "Twice"),
            place("again.md", "Twice"),
        ]
    };
    let cases: &[(&[&str], &[&str])] = &[
        // Nothing said: documents by title whatever their letters are written in, then folders by title.
        (&[], &["about.md", "install.md", "faq.md", "again.md", "same.md", "zebra.md", "guides/", "api/"]),
        // Said: as said, folders among them, then the rest as before.
        (
            &["zebra", "guides/", "install.md"],
            &["zebra.md", "guides/", "install.md", "about.md", "faq.md", "again.md", "same.md", "api/"],
        ),
        // A folder by its name alone, and with `./` before it.
        (
            &["./api", "guides"],
            &["api/", "guides/", "about.md", "install.md", "faq.md", "again.md", "same.md", "zebra.md"],
        ),
        // What is not there, and what is said again, is passed over.
        (
            &["gone", "faq", "faq.md", "nothing/"],
            &["faq.md", "about.md", "install.md", "again.md", "same.md", "zebra.md", "guides/", "api/"],
        ),
        // A name with `/` after it is no document's, and one that ends as a document no folder's.
        (
            &["zebra/", "guides.md"],
            &["about.md", "install.md", "faq.md", "again.md", "same.md", "zebra.md", "guides/", "api/"],
        ),
    ];
    for (order, wanted) in cases {
        assert_eq!(paths(&ordered(&lines(order), places())), wanted.to_vec(), "{order:?}");
    }
    // A word that names a document and a folder is the document.
    let both = vec![place("more/", "More"), place("more.md", "More, a page")];
    assert_eq!(paths(&ordered(&lines(&["more"]), both.clone())), ["more.md", "more/"]);
    assert_eq!(paths(&ordered(&lines(&["more/"]), both)), ["more/", "more.md"]);
}

#[test]
fn the_contents_are_every_folder_a_document_is_in_each_in_the_order_its_front_page_says() {
    let documents: Vec<Document> = [
        ("README.md", "The shop"),
        ("welcome.md", "Welcome"),
        ("plan.md", ""),
        ("guides/README.md", "All the guides"),
        ("guides/install.md", "Installing it"),
        ("guides/usage.md", "Using it"),
        ("guides/more/deep one.md", "Deep"),
        ("notes/a.md", "B"),
        ("notes/b.md", "A"),
    ]
    .iter()
    .map(|(path, title)| Document { path: path.to_string(), title: title.to_string() })
    .collect();
    let fronts: BTreeMap<String, String> = [("", "README.md"), ("guides/", "guides/README.md")]
        .iter()
        .map(|(folder, front)| (folder.to_string(), front.to_string()))
        .collect();
    let orders: BTreeMap<String, Vec<String>> =
        [("", lines(&["guides/", "welcome"])), ("guides/", lines(&["usage", "more/", "install"]))]
            .into_iter()
            .map(|(folder, order)| (folder.to_string(), order))
            .collect();
    let made = contents(&documents, &fronts, &orders);

    // The front page stands for its folder and is not listed in it.
    assert_eq!(paths(&made), ["guides/", "welcome.md", "plan.md", "notes/"]);
    // A folder is called what its front page is, else its name; a document with no title its file's name.
    let titles: Vec<&str> = made.iter().map(|place| place.title.as_str()).collect();
    assert_eq!(titles, ["All the guides", "Welcome", "plan", "notes"]);
    assert_eq!(paths(&made[0].under), ["guides/usage.md", "guides/more/", "guides/install.md"]);
    assert_eq!(paths(&made[0].under[1].under), ["guides/more/deep one.md"]);
    // A folder whose front page says nothing is by title.
    assert_eq!(paths(&made[3].under), ["notes/b.md", "notes/a.md"]);
    assert!(made[1].under.is_empty());

    // The documents in that order, each front page where its folder is.
    assert_eq!(
        in_order(&made, &fronts),
        [
            "README.md",
            "guides/README.md",
            "guides/usage.md",
            "guides/more/deep one.md",
            "guides/install.md",
            "welcome.md",
            "plan.md",
            "notes/b.md",
            "notes/a.md",
        ]
    );
    // A front page's page is its folder's place; any other is its own.
    assert_eq!(here_of("guides/README.md", &fronts), "guides/");
    assert_eq!(here_of("README.md", &fronts), "");
    assert_eq!(here_of("guides/install.md", &fronts), "guides/install.md");

    // A library with no documents has no contents.
    assert!(contents(&[], &BTreeMap::new(), &BTreeMap::new()).is_empty());
}
