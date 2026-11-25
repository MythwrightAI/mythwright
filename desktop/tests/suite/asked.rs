//! What the window is told on its command line, and what it says where
//! it is told to start nothing and nothing answers.

use std::net::TcpListener;

use desktop::Asked;

fn read(args: &[&str]) -> Result<Asked, String> {
    Asked::read(&args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>())
}

#[test]
fn the_window_is_told_an_address_whether_to_start_a_daemon_and_how_it_is_framed() {
    assert_eq!(
        read(&[]),
        Ok(Asked::default()),
        "nothing said: the address is looked for, a daemon may be started"
    );
    assert_eq!(
        read(&["--url", "http://127.0.0.1:7700", "--no-start"]),
        Ok(Asked { url: Some("http://127.0.0.1:7700".into()), no_start: true, ..Asked::default() })
    );
    assert_eq!(read(&["--system-frame"]), Ok(Asked { system_frame: true, ..Asked::default() }));
    assert_eq!(
        read(&["--no-start", "--url", "http://localhost:7700", "--system-frame"]),
        Ok(Asked {
            url: Some("http://localhost:7700".into()),
            no_start: true,
            system_frame: true,
            help: false
        }),
        "in any order"
    );
    for asked in [&["--help"][..], &["-h"], &["--url", "x", "--help"], &["--nonsense", "--help"]] {
        assert_eq!(read(asked), Ok(Asked { help: true, ..Asked::default() }), "{asked:?}");
    }
}

#[test]
fn an_address_left_out_or_a_word_the_window_does_not_take_is_refused_in_a_sentence() {
    for asked in [&["--url"][..], &["--url", "--no-start"], &["--url", "  "]] {
        let said = read(asked).unwrap_err();
        assert!(said.contains("`--url` needs the daemon's address after it"), "{asked:?}: {said}");
        assert!(said.contains(brand::DEFAULT_URL) && said.ends_with('.'), "{said}");
    }
    // A word misspelt is no word passed over: told `--no-strat`, the window
    // would otherwise start a daemon it was meant not to.
    for word in ["--no-strat", "http://127.0.0.1:7700", "-u"] {
        let said = read(&[word]).unwrap_err();
        assert_eq!(said, format!("`{word}` is not something the window takes; `--help` says what it does."));
    }
}

#[test]
fn told_to_start_nothing_where_nothing_answers_the_window_says_so_and_what_to_do() {
    let said = desktop::unanswered("http://127.0.0.1:9");
    assert!(said.starts_with("Nothing is answering at http://127.0.0.1:9 as "), "{said}");
    assert!(said.contains("`--no-start`") && said.contains("Start the daemon"), "{said}");
}

#[test]
fn a_name_that_comes_to_several_addresses_is_asked_at_each() {
    // Listening at 127.0.0.1 alone: where `localhost` comes to `::1` first,
    // the daemon is found at the second address.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let body = format!(r#"{{"id":"{}"}}"#, brand::ID);
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let mut request = [0u8; 1024];
            let _ = stream.read(&mut request);
            let _ = stream.write_all(
                format!("HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{body}", body.len()).as_bytes(),
            );
        }
    });
    assert!(desktop::answers(&format!("http://localhost:{port}")));
}

#[test]
fn the_window_opens_at_the_link_the_daemons_program_said_and_at_the_address_where_it_said_none() {
    let url = "http://127.0.0.1:7700";
    let said = "http://127.0.0.1:7700/#once=once_abc\n";
    assert_eq!(desktop::link_in(said, url).as_deref(), Some("http://127.0.0.1:7700/#once=once_abc"));
    // A link to another daemon is nobody's to open here, and nothing said is nothing.
    assert_eq!(desktop::link_in("http://127.0.0.1:7701/#once=once_abc\n", url), None);
    assert_eq!(desktop::link_in("", url), None);
    // A program that cannot be started opens the address.
    let nothing = std::path::Path::new("/nowhere/at/all/mw");
    assert_eq!(desktop::signed_in_address(url, nothing), url);
}
