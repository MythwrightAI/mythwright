//! What the app and the client say to each other about the window.

use desktop::{Ask, Edge, Framing, dark_from_registry, page_setup};

#[test]
fn the_client_asks_for_what_a_title_bar_would_do() {
    for (message, ask) in [
        (r#"{"window":"drag"}"#, Ask::Drag),
        (r#"{"window":"minimize"}"#, Ask::Minimize),
        (r#"{"window":"maximize"}"#, Ask::Maximize),
        (r#"{"window":"close"}"#, Ask::Close),
        (r#"{"window":{"resize":"n"}}"#, Ask::Resize(Edge::North)),
        (r#"{"window":{"resize":"se"}}"#, Ask::Resize(Edge::SouthEast)),
        (r#"{"window":{"resize":"w"}}"#, Ask::Resize(Edge::West)),
    ] {
        assert_eq!(Ask::from_message(message), Some(ask), "{message}");
    }
}

#[test]
fn anything_else_a_page_sends_is_ignored() {
    // The page is whatever is at the address the app was pointed at. It can
    // ask for these few things and nothing more.
    for message in [
        "",
        "close",
        r#"{"window":"quit"}"#,
        r#"{"window":{"resize":"up"}}"#,
        r#"{"window":{"run":"rm -rf /"}}"#,
        r#"{"other":"close"}"#,
        r#"{"window":["close"]}"#,
    ] {
        assert_eq!(Ask::from_message(message), None, "{message}");
    }
}

#[test]
fn the_page_is_told_how_it_is_framed() {
    assert_eq!(page_setup(Framing::Page), r#"window.desktop = Object.freeze({"frame":"page"});"#);
    assert_eq!(page_setup(Framing::System), r#"window.desktop = Object.freeze({"frame":"system"});"#);
}

#[test]
fn the_systems_own_title_bar_is_only_used_when_asked_for() {
    assert_eq!(Framing::choose(true), Framing::System);
    assert_ne!(Framing::choose(false), Framing::System);
}

#[test]
fn windows_says_whether_it_is_set_to_dark() {
    let asked = |value: &str| {
        format!(
            "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\r\n    AppsUseLightTheme    REG_DWORD    {value}\r\n\r\n"
        )
    };
    assert_eq!(dark_from_registry(&asked("0x0")), Some(true));
    assert_eq!(dark_from_registry(&asked("0x1")), Some(false));
    // No answer is not an answer of "light".
    assert_eq!(
        dark_from_registry("ERROR: The system was unable to find the specified registry key or value."),
        None
    );
    assert_eq!(dark_from_registry(""), None);
}

#[test]
fn the_window_goes_only_to_the_daemon_it_was_opened_for() {
    use desktop::Going::{Here, Nowhere, Outside};
    let daemon = "http://127.0.0.1:7700";
    for (to, expected) in [
        ("http://127.0.0.1:7700/", Here),
        ("http://127.0.0.1:7700/library/shop/docs/", Here),
        ("about:blank", Here),
        ("about:srcdoc", Here),
        ("blob:http://127.0.0.1:7700/6c1f", Here),
        // Another port of this machine is another program's.
        ("http://127.0.0.1:3000/", Outside),
        ("http://localhost:7700/", Outside),
        ("https://example.com/a/page", Outside),
        // A name that only looks like the daemon's.
        ("http://127.0.0.1:7700.example.com/", Outside),
        ("http://127.0.0.1:7700@example.com/", Outside),
        ("blob:https://example.com/6c1f", Nowhere),
        ("file:///etc/passwd", Nowhere),
        ("javascript:alert(1)", Nowhere),
        ("", Nowhere),
    ] {
        assert_eq!(desktop::going(daemon, to), expected, "{to}");
    }
}
