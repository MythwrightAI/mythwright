//! More sent than a route takes (`api::sent`): it is read to its end
//! before it is refused, so that whoever sent it reads the refusal.
//!
//! The client here is written out by hand, on a connection of its own,
//! so that the test says when each part is sent: the refusal used to be
//! answered, and the connection closed, while the rest was still to
//! come, and a client told so in the middle of sending never read it.

use reqwest::StatusCode;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::{Rig, soon};

/// A client that says each thing it sends, on one connection.
struct ByHand {
    stream: TcpStream,
    host: String,
    token: String,
}

impl ByHand {
    async fn to(rig: &Rig) -> ByHand {
        let host = rig.url.trim_start_matches("http://").to_string();
        ByHand { stream: TcpStream::connect(&host).await.unwrap(), host, token: rig.token.clone() }
    }

    /// The head of a request that says it sends `length` bytes.
    async fn head(&mut self, method: &str, path: &str, length: usize) {
        let head = format!(
            "{method} {path} HTTP/1.1\r\nhost: {}\r\nauthorization: Bearer {}\r\ncontent-length: {length}\r\n\r\n",
            self.host, self.token
        );
        self.stream.write_all(head.as_bytes()).await.expect("the head of the request is sent");
    }

    /// So many bytes of what the request sends.
    async fn sends(&mut self, mut count: usize) {
        let piece = vec![0u8; 64 * 1024];
        while count > 0 {
            let now = count.min(piece.len());
            self.stream.write_all(&piece[..now]).await.expect("what the request sends is taken to its end");
            count -= now;
        }
    }

    /// One answer, whole: what it says first, and what it carries.
    async fn answer(&mut self) -> (u16, String) {
        let mut read = Vec::new();
        let ends = |read: &[u8]| read.windows(4).position(|four| four == b"\r\n\r\n").map(|at| at + 4);
        let begins = loop {
            if let Some(begins) = ends(&read) {
                break begins;
            }
            let mut piece = [0u8; 4096];
            let got = self.stream.read(&mut piece).await.expect("the answer is read");
            assert!(got > 0, "the connection ended before an answer: {:?}", String::from_utf8_lossy(&read));
            read.extend_from_slice(&piece[..got]);
        };
        let head = String::from_utf8_lossy(&read[..begins]).to_lowercase();
        let status: u16 = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        let length: usize = head
            .lines()
            .find_map(|line| line.strip_prefix("content-length:"))
            .map_or(0, |length| length.trim().parse().unwrap());
        while read.len() < begins + length {
            let mut piece = [0u8; 4096];
            let got = self.stream.read(&mut piece).await.expect("the answer is read");
            assert!(got > 0, "the connection ended in the middle of an answer");
            read.extend_from_slice(&piece[..got]);
        }
        (status, String::from_utf8_lossy(&read[begins..begins + length]).into_owned())
    }
}

/// How many requests the route that attaches a file has refused.
async fn refused(rig: &Rig) -> u64 {
    refused_at(rig, "POST", "/attachments").await
}

/// How many requests asked by `method` of a route whose address ends in `ends` were refused.
async fn refused_at(rig: &Rig, method: &str, ends: &str) -> u64 {
    let measured = rig.read(&format!("/api/metrics?name=http&by=route,status&labels=method:{method}")).await;
    let of =
        |one: &serde_json::Value, label: &str| one["labels"][label].as_str().unwrap_or_default().to_string();
    measured["series"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|one| of(one, "route").ends_with(ends) && of(one, "status") == "4xx")
        .map(|one| one["total"]["count"].as_u64().unwrap())
        .sum()
}

#[tokio::test]
async fn more_than_may_be_attached_is_read_to_its_end_before_it_is_refused_and_the_connection_goes_on() {
    let rig = Rig::new().await;
    let channel = rig.send("/api/channels", json!({ "name": "general" })).await;
    let path = format!("/api/channels/{}/attachments?name=huge.bin", channel["id"].as_str().unwrap());
    // One byte more than the route reads, and then a good deal still to
    // come: more than is read of a connection in one go.
    let over = studio::attachments::MAX_ATTACHMENT_BYTES + 4096 + 1;
    let rest = 8 * 1024 * 1024;

    let mut client = ByHand::to(&rig).await;
    client.head("POST", &path, over + rest).await;
    client.sends(over).await;
    // The route has what it takes and one byte more, and refuses: with
    // the rest not sent yet, which is what the test is there for.
    soon("the route to refuse what it was sent", async {
        while refused(&rig).await == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await;

    // The client goes on sending, to the end, and then reads why it was
    // refused: it was not told so in the middle.
    client.sends(rest).await;
    let (status, _) = client.answer().await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE.as_u16());

    // Everything it sent was read, so the connection is where the next
    // request begins, and is answered on.
    client.head("GET", "/api/meta", 0).await;
    let (status, meta) = client.answer().await;
    assert_eq!(status, StatusCode::OK.as_u16(), "{meta}");
    assert!(meta.contains("\"needs_setup\""), "{meta}");
    assert_eq!(refused(&rig).await, 1);
}

#[tokio::test]
async fn a_request_that_says_it_is_longer_than_is_read_through_is_refused_with_the_rest_unsent() {
    let rig = Rig::new().await;
    let channel = rig.send("/api/channels", json!({ "name": "general" })).await;
    let path = format!("/api/channels/{}/attachments?name=huge.bin", channel["id"].as_str().unwrap());
    let over = studio::attachments::MAX_ATTACHMENT_BYTES + 4096 + 1;
    // Longer than anybody reads on for the sake of an answer: the most a
    // route takes, and that many times again, and more.
    let said = over * (api::sent::READ_THROUGH as usize + 2);

    let mut client = ByHand::to(&rig).await;
    client.head("POST", &path, said).await;
    client.sends(over).await;
    // Nothing more is sent, and the answer comes all the same: the rest
    // is not waited for. (The connection is then closed under what was
    // still to be sent, and a client in the middle of sending it may not
    // read this: that is what is not held past the bound.)
    let (status, _) = client.answer().await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE.as_u16());
}

#[tokio::test]
async fn a_file_larger_than_is_written_as_it_is_is_refused_as_too_large_with_nothing_written() {
    let rig = Rig::new().await;
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = std::path::PathBuf::from(project["path"].as_str().unwrap());
    let path = "/api/projects/shop/raw?path=art/huge.png";
    // One byte more than the route reads, and a little still to come. It
    // is sent from one small piece over and over: nothing of it is held
    // whole on this side.
    let over = studio::projects::MAX_RAW_BYTES as usize + 4096 + 1;
    let rest = 1024 * 1024;

    let mut client = ByHand::to(&rig).await;
    client.head("PUT", path, over + rest).await;
    client.sends(over).await;
    soon("the route to refuse what it was sent", async {
        while refused_at(&rig, "PUT", "/raw").await == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await;
    client.sends(rest).await;
    let (status, why) = client.answer().await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE.as_u16(), "{why}");
    assert!(!dir.join("art/huge.png").exists(), "nothing of it is written");

    // The connection is where the next request begins, and one as large as is taken is written.
    client.head("PUT", "/api/projects/shop/raw?path=art/small.png", 3).await;
    client.sends(3).await;
    let (status, written) = client.answer().await;
    assert_eq!(status, StatusCode::OK.as_u16(), "{written}");
    assert_eq!(std::fs::read(dir.join("art/small.png")).unwrap(), [0, 0, 0]);
}
