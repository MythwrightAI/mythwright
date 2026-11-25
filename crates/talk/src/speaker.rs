//! The speaker: the one bot that says what the factory's blueprints say.
//!
//! A blueprint's step that posts to a channel, and names no bot to speak
//! as, is not a member of anything: it is one of a dozen texts the
//! conductor is running. So what it says is said by one bot, a member of
//! the channel like any other, and which blueprint said it, in which run,
//! is kept on the message for whoever opens it ([`SaidBy`]). What the
//! factory's own code has to say in a channel (that an agent could not
//! answer, say) is said by the same bot: there is one voice for the
//! factory, and an agent's name is on what its program said and nothing
//! else.
//!
//! What the bot is called is a setting of the whole factory, with a
//! default. It is kept in the journal's small durable state, because it is
//! true of the factory and of nothing smaller: no project, blueprint or
//! environment owns it, and a copy of a factory takes it along, so that a
//! channel goes on under the name it had. Changing it changes nothing that
//! was said: a message keeps the name it was said under.
//!
//! The bot has no model. What is said to it is handed, with the key of
//! whoever said it, to whatever the daemon gave it to hear with
//! ([`Speaker::hear_with`]); nothing here reads a message for meaning.
//!
//! What it may ask is a ceiling, kept with its name ([`Speaker::holds`]):
//! it holds nothing of its own, and does what it is told as whoever told
//! it, held to no more than the ceiling (`kernel::Principal::within`).
//! Nothing said is the usual: everything its commands reach, so nothing
//! changes until somebody narrows it. What a blueprint says as the bot is
//! whoever applied it, and is not narrowed by it.

use std::sync::{Arc, Mutex, OnceLock};

use journal::Journal;
use kernel::{BoxFut, Error, Principal, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::channels::{Message, mentions};

/// The namespace of the journal's small durable state the name is kept in.
pub const NAMESPACE: &str = "speaker";
/// What the bot is called until somebody says otherwise.
pub const DEFAULT: &str = "conductor";

/// Which step of which blueprint said a message, for one a blueprint said.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaidBy {
    pub blueprint: String,
    pub instance: String,
    pub run: String,
    pub step: String,
    /// Which element of the list it was said for, where the step is done
    /// once for each of one: what tells one such message of a run's step
    /// from the next.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<usize>,
    /// Which of the things done under the step's name in its run said
    /// it: the step's own action or an entry of what is done around it,
    /// at which try, for which of a list, inside which steps done as one
    /// (`conductor::ActionCx::doing`). What tells what one of them said
    /// from what another did. Nothing, on a message a program from before
    /// this kept, which is told by its element alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doing: Option<String>,
    /// More than the message says, for whoever opens it: the whole of an
    /// error whose reason the message gives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The run the message is about, where it is about one: a run that
    /// failed, whose page is opened from the message at the step that
    /// failed. Not the run that said it, which is `run`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about_run: Option<String>,
}

/// What is done with a message that names the bot: handed whoever said it,
/// as they were let in, and the message.
pub type Hearing = Arc<dyn Fn(Principal, Message) -> BoxFut<'static, ()> + Send + Sync>;

#[derive(Clone)]
pub struct Speaker {
    journal: Journal,
    name: Arc<Mutex<String>>,
    /// What it may ask, at most; nothing for the usual.
    holds: Arc<Mutex<Option<Vec<String>>>>,
    hearing: Arc<OnceLock<Hearing>>,
}

impl Speaker {
    pub(crate) async fn open(journal: Journal) -> Result<Speaker> {
        let kept = journal.kv_get(NAMESPACE, "name").await?;
        let name = kept.as_ref().and_then(|name| name.as_str()).unwrap_or(DEFAULT).to_string();
        let holds =
            journal.kv_get(NAMESPACE, "holds").await?.and_then(|kept| serde_json::from_value(kept).ok());
        let (name, holds) = (Arc::new(Mutex::new(name)), Arc::new(Mutex::new(holds)));
        Ok(Speaker { journal, name, holds, hearing: Arc::default() })
    }

    /// What it may ask at most, as scopes; nothing where nobody narrowed
    /// it, which is everything its commands reach.
    pub fn holds(&self) -> Option<Vec<String>> {
        self.holds.lock().expect("speaker lock").clone()
    }

    /// Narrows what it may ask to `holds`, or gives it the usual again
    /// with nothing. The owner's to say, as its name is, and nobody gives
    /// it what they do not hold themselves. Kept where its name is, in
    /// one write with what is said of it; the same again writes nothing.
    pub(crate) async fn hold(
        &self,
        holds: Option<Vec<String>>,
        by: &Principal,
    ) -> Result<Option<Vec<String>>> {
        by.require("admin")?;
        let mut holds = holds;
        if let Some(scopes) = holds.as_mut() {
            scopes.sort();
            scopes.dedup();
            if let Some(scope) = scopes.iter().find(|scope| !kernel::principal::named(scope)) {
                return Err(Error::invalid(format!("`{scope}` is no scope the factory checks")));
            }
            if let Some(scope) = scopes.iter().find(|scope| !by.can(scope)) {
                return Err(Error::denied(format!(
                    "{} does not hold `{scope}`, so it is not theirs to let the bot ask",
                    by.name
                )));
            }
        }
        if holds == self.holds() {
            return Ok(holds);
        }
        let (kept, name, actor) = (holds.clone(), self.name(), by.actor());
        self.journal
            .write(move |writing| {
                let transaction = writing.transaction();
                match &kept {
                    Some(scopes) => transaction.execute(
                        "INSERT INTO kv (namespace, key, value) VALUES (?1, 'holds', ?2)
                         ON CONFLICT (namespace, key) DO UPDATE SET value = excluded.value",
                        [NAMESPACE, &json!(scopes).to_string()],
                    )?,
                    None => transaction
                        .execute("DELETE FROM kv WHERE namespace = ?1 AND key = 'holds'", [NAMESPACE])?,
                };
                writing.say_here(
                    "channel.speaker_held",
                    kernel::Source::actor(&actor),
                    json!({ "name": name, "holds": kept }),
                )?;
                Ok(())
            })
            .await?;
        *self.holds.lock().expect("speaker lock") = holds.clone();
        Ok(holds)
    }

    /// What the bot is called now.
    pub fn name(&self) -> String {
        self.name.lock().expect("speaker lock").clone()
    }

    /// Calls it something else from here on. The owner's to say: it is the
    /// name on everything the factory says of itself.
    pub(crate) async fn call(&self, name: &str, by: &Principal) -> Result<(String, String)> {
        by.require("admin")?;
        let name = blueprint::expr::slug(name);
        if name.is_empty() {
            return Err(Error::invalid(
                "what the factory speaks as needs a name with at least one letter or digit",
            ));
        }
        self.journal.kv_set(NAMESPACE, "name", &json!(name)).await?;
        let was = std::mem::replace(&mut *self.name.lock().expect("speaker lock"), name.clone());
        Ok((was, name))
    }

    /// Says what is done with a message that names the bot. Said once, by
    /// the daemon as it starts; until then the bot hears nothing.
    pub fn hear_with(&self, hearing: Hearing) {
        let _ = self.hearing.set(hearing);
    }

    /// Hands on a message that names the bot, with who said it as they
    /// were let in: what comes of it is done as them and with what their
    /// key holds, which is why it is handed on here, where the key is, and
    /// not worked out later from the name on the message.
    pub(crate) fn heard(&self, by: &Principal, message: &Message) {
        let Some(hearing) = self.hearing.get() else { return };
        if mentions(&message.body).contains(&self.name()) {
            tokio::spawn(hearing(by.clone(), message.clone()));
        }
    }
}
