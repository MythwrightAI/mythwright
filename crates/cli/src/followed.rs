//! One's own inbox followed as it is told (`inbox --follow`): what waits
//! unread is printed, and then each line as it is put in the box or told
//! again, a line each, for a person at a terminal or a program reading
//! what it prints.
//!
//! Nothing is looked for on a timer. The box is read once, and again
//! only when the daemon says something was told in it (`inbox.told` on
//! the event stream, which only the box's owner hears), when the stream
//! says it may have missed something (`behind`) or when the daemon is
//! back after it was lost: an event carries no words, which are read
//! through the box's own route as whoever asks may read them then. What
//! was printed is kept by each line's id with how many times it had been
//! told (`count`), so that a telling is printed once however often the
//! box is read, and one told again is printed again.
//!
//! The stream is opened before the box is first read: where the stream
//! begins is fixed as it is opened, so a line told between the two is in
//! the reading or in the stream, and printed once either way. Lost, it
//! is opened again from the place it had got to.

use std::collections::HashMap;
use std::time::Duration;

use serde_json::{Value, json};

use crate::client::{Client, Unanswered, encode};
use crate::serve::Asked;
use crate::work::{plain, text};

/// How long the stream may say nothing before it is taken for lost: the
/// daemon says it is still there every twenty seconds, and a connection
/// to a machine that went away may never say it broke.
const SILENCE: Duration = Duration::from_secs(60);

/// How long is waited before the daemon is asked again once it was lost,
/// first and at most: a daemon started again is back in a moment.
const FIRST_WAIT: Duration = Duration::from_millis(200);
const LONGEST_WAIT: Duration = Duration::from_secs(2);

/// What a following came to: the daemon gone, to be asked again until it
/// is back, or a refusal, which ends it.
enum Ended {
    Lost(String),
    Refused(String),
}

impl From<Unanswered> for Ended {
    fn from(unanswered: Unanswered) -> Ended {
        match unanswered {
            Unanswered::Gone(why) => Ended::Lost(why),
            Unanswered::Refused(why) => Ended::Refused(why),
        }
    }
}

/// Follows the box of whoever asks until the program is asked to stop,
/// which ends it well; a credential refused, or a daemon never reached,
/// ends it with why.
pub async fn follow(client: &Client) -> Result<(), String> {
    // Listened for before anything is asked, so that being asked to stop
    // at any moment ends it well.
    let mut asked = Asked::listening();
    tokio::select! {
        followed = following(client) => followed,
        () = asked.heard() => Ok(()),
    }
}

async fn following(client: &Client) -> Result<(), String> {
    let mut box_ = Followed::default();
    // The first time, a daemon that is not there is said and ends it:
    // there is nothing to go on from.
    let mut stream = client.stream(&box_.path()).await?;
    match box_.read(client, None).await {
        Ok(()) => {}
        Err(Ended::Lost(why) | Ended::Refused(why)) => return Err(why),
    }
    loop {
        let lost = match box_.frames(client, &mut stream).await {
            Ended::Lost(why) => why,
            Ended::Refused(why) => return Err(why),
        };
        eprintln!("lost the daemon at {} ({lost}); following again once it is back", client.url);
        stream = box_.again(client).await?;
        eprintln!("the daemon at {} is back; following your inbox again", client.url);
    }
}

#[derive(Default)]
struct Followed {
    /// How many times each line printed had been told when it was.
    printed: HashMap<String, u64>,
    /// Where the stream had got to, handed back as it was.
    place: Option<String>,
}

impl Followed {
    /// The stream of what is told in this box, from where it had got to.
    fn path(&self) -> String {
        let since =
            self.place.as_deref().map(|place| format!("&since={}", encode(place))).unwrap_or_default();
        format!("/api/events?topics=inbox.told{since}")
    }

    /// Opens the stream again once the daemon is back, and reads the box
    /// for what was told while it was not followed.
    async fn again(&mut self, client: &Client) -> Result<reqwest::Response, String> {
        let mut wait = FIRST_WAIT;
        loop {
            tokio::time::sleep(wait).await;
            wait = (wait * 2).min(LONGEST_WAIT);
            let stream = match client.stream_or_gone(&self.path()).await {
                Ok(stream) => stream,
                Err(Unanswered::Gone(_)) => continue,
                Err(Unanswered::Refused(why)) => return Err(why),
            };
            match self.read(client, None).await {
                Ok(()) => return Ok(stream),
                Err(Ended::Lost(_)) => continue,
                Err(Ended::Refused(why)) => return Err(why),
            }
        }
    }

    /// Reads what the stream says until it is lost or something is
    /// refused.
    async fn frames(&mut self, client: &Client, stream: &mut reqwest::Response) -> Ended {
        let mut buffer: Vec<u8> = Vec::new();
        loop {
            let chunk = match tokio::time::timeout(SILENCE, stream.chunk()).await {
                Err(_) => return Ended::Lost(format!("it said nothing for {} seconds", SILENCE.as_secs())),
                Ok(Err(_)) => return Ended::Lost("the connection was closed".into()),
                Ok(Ok(None)) => return Ended::Lost("the stream ended".into()),
                Ok(Ok(Some(chunk))) => chunk,
            };
            buffer.extend_from_slice(&chunk);
            while let Some(end) = buffer.windows(2).position(|two| two == b"\n\n") {
                let frame: Vec<u8> = buffer.drain(..end + 2).collect();
                if let Err(ended) = self.frame(client, &String::from_utf8_lossy(&frame)).await {
                    return ended;
                }
            }
        }
    }

    /// One frame of the stream: something told, or that something may
    /// have been missed. Its place is kept once what it says is printed.
    async fn frame(&mut self, client: &Client, frame: &str) -> Result<(), Ended> {
        let field = |name: &str| {
            frame.lines().find_map(|line| line.strip_prefix(name)).map(|value| value.trim_start().to_string())
        };
        if field("event:").as_deref() == Some("behind") {
            self.read(client, None).await?;
        } else if let Some(event) = field("data:").and_then(|data| serde_json::from_str::<Value>(&data).ok())
        {
            let data = &event["data"];
            let (line, count) = (text(data, "line"), data["count"].as_u64().unwrap_or(1));
            if event["topic"] == "inbox.told" && count > self.printed.get(line).copied().unwrap_or(0) {
                self.read(client, Some(line)).await?;
            }
        }
        if let Some(place) = field("id:") {
            self.place = Some(place);
        }
        Ok(())
    }

    /// Reads the box and prints, the oldest first, each line that waits
    /// unread (and `told`, which was just told whether or not it has been
    /// read since) that was not printed as told so many times.
    async fn read(&mut self, client: &Client, told: Option<&str>) -> Result<(), Ended> {
        let found = client.get_or_gone("/api/inbox").await?;
        let lines = found["lines"].as_array().map_or(&[][..], Vec::as_slice);
        for line in lines.iter().rev() {
            let (id, count) = (text(line, "id"), line["count"].as_u64().unwrap_or(1));
            let waits = line["read"].is_null() || told == Some(id);
            if !waits || count <= self.printed.get(id).copied().unwrap_or(0) {
                continue;
            }
            match client.json {
                true => println!("{}", shown(line)),
                false => println!("{}", plain(&crate::inbox::printed(line).replace(['\n', '\r'], " "))),
            }
            self.printed.insert(id.to_string(), count);
        }
        Ok(())
    }
}

/// A line as a program reads it: what it is, who said it, where, and the
/// words as whoever asks may read them now.
fn shown(line: &Value) -> Value {
    let said = &line["said"];
    let mut shown = json!({
        "id": line["id"],
        "kind": line["kind"],
        "place": line["place"],
        "count": line["count"],
        "read": !line["read"].is_null(),
        "raised": line["raised"],
    });
    let more = if said.is_object() {
        // The message: the one that named whoever asks, or the latest that
        // raised a conversation's line.
        let message = match text(line, "kind") {
            "named" => &line["about"],
            _ => &line["said_by"],
        };
        let called = said["channel_title"].as_str().unwrap_or_else(|| text(said, "channel_name"));
        json!({
            "message": message,
            "thread": said["thread"],
            "from": said["author"],
            "who": said["author_name"],
            "where": called,
            "conversation": said["conversation"] == true,
            "words": said["body"],
        })
    } else {
        // What a blueprint told: from the factory's bot, in its words.
        let from = text(line, "from");
        json!({
            "message": Value::Null,
            "from": from,
            "who": from.split_once(':').map_or(from, |(_, name)| name),
            "where": Value::Null,
            "about": line["about"],
            "words": line["says"],
        })
    };
    if let (Some(shown), Value::Object(more)) = (shown.as_object_mut(), more) {
        shown.extend(more);
    }
    shown
}
