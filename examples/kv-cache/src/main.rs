//! A small key-value cache, used as the running example of something a
//! blueprint builds, hosts and rolls out.
//!
//! It speaks one line per command over TCP:
//!
//! ```text
//! SET greeting hello   ->  OK
//! GET greeting         ->  VALUE hello
//! GET missing          ->  NIL
//! DEL greeting         ->  OK
//! VERSION              ->  VERSION v1
//! PING                 ->  PONG
//! ```
//!
//! and answers `GET /health` as HTTP, so either kind of health check works.
//!
//! Configuration is by environment, the way a hosted service gets it:
//! `PORT` (default 7379), `KV_VERSION` (default `v1`), and `KV_FAULT` to make
//! it misbehave on purpose so that rollbacks can be seen:
//! `refuse` exits at once, and `after:N` exits after serving N connections.
//! It serves N and not one more: it stops listening as it takes the Nth, so
//! whoever connects from then on is refused, as by a program that is gone.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

type Store = Arc<Mutex<HashMap<String, String>>>;

fn main() {
    let port: u16 = std::env::var("PORT").ok().and_then(|port| port.parse().ok()).unwrap_or(7379);
    let version = std::env::var("KV_VERSION").unwrap_or_else(|_| "v1".to_string());
    let fault = std::env::var("KV_FAULT").unwrap_or_default();

    if fault == "refuse" {
        eprintln!("kv-cache {version}: refusing to start (KV_FAULT=refuse)");
        std::process::exit(1);
    }
    let serve_limit: Option<usize> = fault.strip_prefix("after:").and_then(|count| count.parse().ok());

    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("kv-cache {version}: cannot listen on port {port}: {err}");
            std::process::exit(1);
        }
    };
    println!("kv-cache {version} listening on 127.0.0.1:{port}");

    let store: Store = Arc::default();
    let mut served = 0;
    loop {
        // A failed accept (out of descriptors, say) is not a reason to stop
        // listening.
        let Ok((stream, _)) = listener.accept() else { continue };
        if let Some(limit) = serve_limit
            && says_something(&stream)
        {
            served += 1;
            if served >= limit {
                // The last it serves. It stops listening before it answers,
                // and not as it exits: between this one's answer and the
                // process being gone there is a moment, long on a busy
                // machine, and a connection taken in it would be one more
                // served than was asked for, or one cut off unanswered.
                drop(listener);
                serve(stream, &store, &version);
                eprintln!("kv-cache {version}: crashing after {limit} connections (KV_FAULT)");
                std::process::exit(2);
            }
        }
        let (store, version) = (store.clone(), version.clone());
        std::thread::spawn(move || serve(stream, &store, &version));
    }
}

/// Whether a connection says anything, waiting until it does or is closed.
/// A health probe that only connects sends nothing and is not counted, so a
/// fault trips on real traffic.
fn says_something(stream: &TcpStream) -> bool {
    stream.peek(&mut [0]).is_ok_and(|read| read > 0)
}

/// Serves one connection.
fn serve(stream: TcpStream, store: &Store, version: &str) {
    let Ok(mut writer) = stream.try_clone() else { return };
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else { break };
        let mut words = line.split_whitespace();
        let reply = match (words.next().map(str::to_ascii_uppercase).as_deref(), words.next()) {
            (None, _) => continue,
            (Some("GET"), Some(path)) if path.starts_with('/') => {
                let _ = writer.write_all(b"HTTP/1.0 200 OK\r\nContent-Length: 2\r\n\r\nok");
                return;
            }
            (Some("PING"), _) => "PONG".to_string(),
            (Some("VERSION"), _) => format!("VERSION {version}"),
            (Some("QUIT"), _) => break,
            (Some("GET"), Some(key)) => match store.lock().expect("store lock").get(key) {
                Some(value) => format!("VALUE {value}"),
                None => "NIL".to_string(),
            },
            (Some("SET"), Some(key)) => {
                let value = words.collect::<Vec<_>>().join(" ");
                store.lock().expect("store lock").insert(key.to_string(), value);
                "OK".to_string()
            }
            (Some("DEL"), Some(key)) => {
                store.lock().expect("store lock").remove(key);
                "OK".to_string()
            }
            (Some(other), _) => format!("ERR unknown command {other}"),
        };
        if writeln!(writer, "{reply}").is_err() {
            break;
        }
    }
}
