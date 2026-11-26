//! Links as a page that is served writes them (`kernel::addressed`).

use kernel::addressed::saying;

#[test]
fn a_link_that_leads_somewhere_on_the_same_host_says_what_the_page_was_asked_with_and_no_other_does() {
    let cases: &[(&str, &str, &str)] = &[
        ("guide.md", "by=ab12", "guide.md?by=ab12"),
        ("./", "by=ab12", "./?by=ab12"),
        ("../../", "by=ab12", "../../?by=ab12"),
        ("more/usage.md#steps", "by=ab12", "more/usage.md?by=ab12#steps"),
        ("art/mark.png", "by=ab12", "art/mark.png?by=ab12"),
        ("/library/shop/docs/", "by=ab12", "/library/shop/docs/?by=ab12"),
        ("page.md?size=2", "by=ab12", "page.md?size=2&by=ab12"),
        ("page.md?", "by=ab12", "page.md?by=ab12"),
        ("page.md?#top", "by=ab12", "page.md?by=ab12#top"),
        // Elsewhere, within the page, to nothing: as they were.
        ("https://example.com/a", "by=ab12", "https://example.com/a"),
        ("mailto:ada@example.com", "by=ab12", "mailto:ada@example.com"),
        ("//example.com/a", "by=ab12", "//example.com/a"),
        ("#contents", "by=ab12", "#contents"),
        ("", "by=ab12", ""),
        // Where nothing is said, nothing is added.
        ("guide.md", "", "guide.md"),
    ];
    for (dest, said, wanted) in cases {
        assert_eq!(saying(dest, said), *wanted, "{dest} with {said}");
    }
}
