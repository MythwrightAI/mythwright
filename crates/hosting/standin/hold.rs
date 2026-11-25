//! Where stand-ins wait to be told (`held ADDRESS WHO` in `main.rs`,
//! beside this): shared by the suites that hold one, each of which names
//! this file (`#[path]`).
//!
//! A stand-in comes here by a connection of its own, says who it is, and
//! goes on when the test lets it. Nothing waits on a clock, and nothing
//! is looked for again and again; however the test ends, whoever was
//! held finds its connection closed and goes.

use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

/// How long a stand-in is waited for before the test has failed.
const WITHIN: Duration = Duration::from_secs(30);

pub struct Hold(TcpListener);

/// One stand-in that came, and waits.
pub struct Held {
    /// Who it said it is.
    pub who: String,
    stream: TcpStream,
}

impl Hold {
    pub async fn new() -> Hold {
        Hold(TcpListener::bind("127.0.0.1:0").await.unwrap())
    }

    /// What a stand-in is told to be held at.
    pub fn address(&self) -> String {
        self.0.local_addr().unwrap().to_string()
    }

    /// Waits for the next stand-in to come and say who it is. One that
    /// came before this was asked is waiting already.
    pub async fn came(&self) -> Held {
        let coming = async {
            let (stream, _) = self.0.accept().await.unwrap();
            let mut said = BufReader::new(stream);
            let mut who = String::new();
            said.read_line(&mut who).await.unwrap();
            Held { who: who.trim_end().to_string(), stream: said.into_inner() }
        };
        match tokio::time::timeout(WITHIN, coming).await {
            Ok(held) => held,
            Err(_) => panic!("timed out waiting for: a stand-in to come to be held"),
        }
    }
}

impl Held {
    /// Lets it go on, telling it `word`. (Let go of without a word, it
    /// goes on too: it is held by the connection.)
    pub async fn told(mut self, word: &str) {
        let _ = self.stream.write_all(format!("{word}\n").as_bytes()).await;
    }

    pub async fn go(self) {
        self.told("go").await;
    }
}
