//! One test binary for the crate; add modules here.
//!
//! The window itself is not tested here: these cover how the app decides
//! where the daemon is and whether it needs starting, and (in `window`) what
//! it and the client say to each other about the window.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::Duration;

/// Answers one request with `body`, as the daemon answers `GET /api/meta`.
fn answering(body: String) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let mut request = [0u8; 1024];
            let _ = stream.read(&mut request);
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    url
}

fn scratch(name: &str) -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("desktop-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_daemon_is_looked_for_where_asked_then_where_the_user_last_signed_in() {
    assert_eq!(desktop::address(None, None, None), brand::DEFAULT_URL);
    assert_eq!(desktop::address(None, None, Some("http://10.0.0.5:7700/")), "http://10.0.0.5:7700");
    assert_eq!(desktop::address(None, Some("http://env:1"), Some("http://saved:2")), "http://env:1");
    assert_eq!(
        desktop::address(Some("http://flag:3"), Some("http://env:1"), Some("http://saved:2")),
        "http://flag:3"
    );
    assert_eq!(
        desktop::address(Some("  "), None, Some("http://saved:2")),
        "http://saved:2",
        "an empty answer is no answer"
    );

    let dir = scratch("session");
    assert_eq!(desktop::saved_address(&dir), None);
    std::fs::write(dir.join("session.json"), r#"{"url":"http://10.0.0.9:7700","token":"ses_x"}"#).unwrap();
    assert_eq!(desktop::saved_address(&dir).as_deref(), Some("http://10.0.0.9:7700"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn only_this_products_daemon_counts_as_answering() {
    let ours = answering(format!(r#"{{"name":"x","id":"{}","needs_setup":true}}"#, brand::ID));
    assert!(desktop::answers(&ours));
    let other = answering(r#"{"id":"something-else"}"#.to_string());
    assert!(!desktop::answers(&other), "another service on the port is not the daemon");
    assert!(!desktop::answers("http://127.0.0.1:9"), "nothing listening");
    assert!(!desktop::answers("https://127.0.0.1:7700") && !desktop::answers("nonsense"));
}

#[test]
fn a_daemon_is_only_started_on_this_machine_and_failing_to_says_what_to_do() {
    assert!(desktop::is_local("http://127.0.0.1:7700") && desktop::is_local("http://localhost:7700/"));
    assert!(!desktop::is_local("http://10.0.0.5:7700") && !desktop::is_local("http://example.com"));

    let away = desktop::ensure_running("http://192.0.2.1:7700", "none".as_ref(), Duration::ZERO).unwrap_err();
    assert!(away.contains("not on this machine"), "{away}");

    let missing = scratch("missing").join("no-such-program");
    let failed = desktop::ensure_running("http://127.0.0.1:9", &missing, Duration::ZERO).unwrap_err();
    assert!(
        failed.contains("could not be started") && failed.contains(&format!("`{} up`", brand::CLI)),
        "{failed}"
    );

    // Already running: nothing is started, whatever the program is.
    let running = answering(format!(r#"{{"id":"{}"}}"#, brand::ID));
    assert_eq!(
        desktop::ensure_running(&running, &missing, Duration::ZERO),
        Ok(false),
        "it was already running, so nothing was started and nobody needs telling"
    );
}

#[test]
fn the_daemons_program_is_the_one_beside_the_app_when_there_is_one() {
    let dir = scratch("beside");
    assert_eq!(
        desktop::daemon_program(Some(&dir)),
        PathBuf::from(brand::CLI),
        "none beside it: the one on the PATH"
    );
    let built = dir.join(format!("factory{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&built, "").unwrap();
    assert_eq!(desktop::daemon_program(Some(&dir)), built, "as built");
    let installed = dir.join(format!("{}{}", brand::CLI, std::env::consts::EXE_SUFFIX));
    std::fs::write(&installed, "").unwrap();
    assert_eq!(desktop::daemon_program(Some(&dir)), installed, "as installed, which wins");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn trouble_is_shown_as_a_page_with_the_message_made_safe() {
    let page = desktop::trouble_page("Nothing is answering at <http://x> & so on");
    assert!(page.contains(&format!("{} is not running", brand::DISPLAY)));
    assert!(page.contains("Nothing is answering at &lt;http://x&gt; &amp; so on"));
}

mod asked;
mod window;

#[test]
fn the_windows_icon_is_a_square_of_pixels_with_the_mark_drawn_on_it() {
    let side = desktop::ICON_SIDE as usize;
    assert_eq!(desktop::ICON.len(), side * side * 4, "four bytes a pixel, a row at a time");
    let pixel = |x: usize, y: usize| &desktop::ICON[(y * side + x) * 4..(y * side + x) * 4 + 4];
    // It is the drawing as made: the mark on a ground that
    // fills the whole square, corners and all.
    assert_eq!(pixel(0, 0)[3], 255, "the corner is the ground's");
    assert_eq!(pixel(side - 1, side - 1)[3], 255);
    assert_eq!(pixel(side / 2, side / 2)[3], 255, "the middle is solid");
    // And it is not one colour all over: something is drawn on its ground.
    let solid: Vec<&[u8]> = desktop::ICON.chunks(4).filter(|pixel| pixel[3] == 255).collect();
    assert!(solid.iter().any(|pixel| pixel[..3] != solid[0][..3]), "a ground with nothing on it");
}
