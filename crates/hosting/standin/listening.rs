//! A file stand-ins say a line at (`tell AT TEXT` in `main.rs`, beside
//! this): for the suites whose stand-ins say that they began, each of
//! which names this file (`#[path]`) beside `holds.rs`.

use std::path::Path;

use tokio::io::{AsyncBufReadExt, BufReader};

use super::holds::{WITHIN, listening_at};

/// A file stand-ins say lines at (`tell AT TEXT`), read here a line at a
/// time, in the order they came.
pub struct Listening(tokio::sync::mpsc::UnboundedReceiver<String>);

impl Listening {
    /// Listens, and makes the file at `path` that says where.
    pub fn at(path: &Path) -> Listening {
        let listener = listening_at(path);
        let (said, lines) = tokio::sync::mpsc::unbounded_channel();
        // Read by a task of its own, so that whoever waits for a line
        // beside other things loses none by turning to one of them.
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let mut line = String::new();
                let read = tokio::time::timeout(WITHIN, BufReader::new(stream).read_line(&mut line)).await;
                if matches!(read, Ok(Ok(_))) && said.send(line.trim_end().to_string()).is_err() {
                    return;
                }
            }
        });
        Listening(lines)
    }

    /// The next line said there, waited for.
    pub async fn next_line(&mut self) -> std::io::Result<Option<String>> {
        Ok(self.0.recv().await)
    }
}
