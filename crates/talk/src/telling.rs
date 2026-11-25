//! What a blueprint tells somebody: a line in their box, in the
//! blueprint's own words.
//!
//! A step says whom (`inbox.tell`): whoever applied its instance, an
//! actor, a handle, or a list of those. Who a handle names is asked
//! where who a word after an `@` names is ([`crate::inboxed::Naming`]),
//! and of an agent by whether one of that name holds anything; each is
//! read in the write that tells, so that everybody is told or nobody
//! is. A handle that names nobody, and an actor that is a bot or is
//! nobody's, are refused before anything is written.
//!
//! **Said as its applier.** The words are the blueprint's, and telling
//! takes what changing a box takes ([`crate::inbox::WRITE`]) of whoever
//! applied it, as what a blueprint says in a channel takes writing
//! there. Nothing is read for whoever is told: not what the line is
//! about, and not what opens it. Those are ids, opened through the
//! routes there are, which ask whether whoever opens one may read it.
//!
//! **From the factory's bot**, as what a blueprint says in a channel
//! is, with which step said it kept on the line ([`Tell::said_by`]): the
//! same step done again, the daemon having stopped in the middle of it,
//! finds its line and tells nothing twice. A step may say which telling
//! it is in words of its own (`once`), which are kept in that place: any
//! run that tells with the same words, as one begun again after a stop
//! does, tells nothing twice. Another telling of the same thing for the
//! same reason raises the line there is; one about nothing is a line of
//! its own each time ([`crate::inbox`]).
//!
//! **The cards a part names.** A part of a board's cards in one column
//! keeps which cards that column held as it is told (`kernel::parts::keep`),
//! so that the line goes on listing what it was told of once they have
//! moved on. This crate knows no board: what keeps it says how a column
//! is read ([`Channels::columns_with`]), and it is read just before the
//! write that tells, as whoever keeps the board reads it, by the column's
//! own index. It cannot be read in that write: a project's cards are kept
//! in its own journal's file, and the factory's thread, which writes the
//! line, never waits on a project's. A telling done again reads the
//! column again and tells nothing; one that raises the line keeps the
//! column as it is then. Nothing of it is read for whoever is told: the
//! line hands out none of the cards (`kernel::parts::shown`), which are
//! read for its owner only as they may read the board.
//!
//! **Passed over.** Somebody who holds no reading of a box has none: an
//! agent the factory starts, so far. Such a one is told nothing, nothing
//! is written, and the step is told so.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use kernel::principal::covers;
use kernel::{Error, ErrorKind, Principal, Result};

use crate::Channels;
use crate::channels::{bot_actor, kind_of};
use crate::inbox::{Kind, READ, Tell};

/// The word that names whoever applied the instance that asks.
pub const APPLIER: &str = "applier";
/// Why a line is there, where the step that tells says no word.
pub const TOLD: &str = "told";

/// What a step asks to have told.
#[derive(Clone, Debug)]
pub struct Telling {
    /// Whom: each [`APPLIER`], an actor, or a handle with or without `@`.
    pub to: Vec<String>,
    pub says: String,
    /// Why, in a word of the blueprint's own.
    pub why: String,
    /// The id of what it is about, or nothing.
    pub about: String,
    /// What opens it, where that is not what it is about.
    pub place: String,
    /// Which thing done under which step of which run says it; or the
    /// words that say which telling it is, whoever says it (`once ...`).
    pub said_by: String,
    /// What it tells with beside its words, as they are kept
    /// (`kernel::parts::read`): nothing where it was told with none.
    pub parts: Vec<serde_json::Value>,
}

/// What reads the cards a column of a project's board holds, by their ids
/// in the column's order: handed the project and the column. A board that
/// is not there answers as not found, and its part keeps nothing.
pub type Columns =
    dyn Fn(String, String) -> Pin<Box<dyn Future<Output = Result<Vec<String>>> + Send>> + Send + Sync;

/// Who a telling came to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Told {
    /// Each actor the line is in the box of, in the order they were named.
    pub told: Vec<String>,
    /// Each that has no box, and was told nothing.
    pub passed: Vec<String>,
}

/// Why a word of `to` could not be told, as a sentence.
fn nobody(word: &str) -> String {
    format!(
        "`{word}` names nobody to tell: say `{APPLIER}`, an actor (`user:<id>`, `agent:<name>`), or what \
         somebody with an account, or an agent, is called after `@`"
    )
}

impl Channels {
    /// Puts a line in the box of each of those `asked.to` names, for
    /// `by`, whose instance asks, and says each, as one write: who was
    /// told, and who was passed over.
    pub async fn tell(&self, by: &Principal, asked: Telling) -> Result<Told> {
        by.require(crate::inbox::WRITE)?;
        if asked.says.trim().is_empty() {
            return Err(Error::invalid("a telling says something: `says` is empty"));
        }
        if asked.why.is_empty() || asked.why.chars().any(char::is_whitespace) {
            return Err(Error::invalid(format!(
                "`why` is one word of the blueprint's own, such as `asked`; it was given `{}`",
                asked.why
            )));
        }
        let mut asked = asked;
        self.keep_cards(&mut asked.parts).await?;
        let applier = by.actor();
        let from = bot_actor(&self.speaker.name());
        let (channels, now) = (self.clone(), self.journal.now_ms());
        let wrote = self.journal.write(move |writing| {
            // Who each word names, as this write finds them; then what
            // each holds, which says whether they have a box at all.
            let mut owners: Vec<String> = Vec::new();
            for word in &asked.to {
                let named = match channels.named_by(writing.transaction(), word, &applier)? {
                    Ok(named) => named,
                    Err(why) => return Ok(Err(why)),
                };
                for owner in named {
                    if !owners.contains(&owner) {
                        owners.push(owner);
                    }
                }
            }
            let mut came = Told::default();
            for owner in owners {
                let held = channels.inbox.holds_now(writing.transaction(), &owner)?;
                if held.is_some_and(|held| !covers(&held, READ)) {
                    came.passed.push(owner);
                    continue;
                }
                let tell = Tell {
                    owner: owner.clone(),
                    kind: Kind::Told,
                    about: asked.about.clone(),
                    place: if asked.place.is_empty() { asked.about.clone() } else { asked.place.clone() },
                    from: from.clone(),
                    why: asked.why.clone(),
                    says: Some(asked.says.clone()),
                    said_by: Some(asked.said_by.clone()),
                    parts: (!asked.parts.is_empty()).then(|| serde_json::Value::from(asked.parts.clone())),
                };
                channels.inbox.tell_in(writing, &tell, now)?;
                came.told.push(owner);
            }
            Ok(Ok(came))
        });
        wrote.await?.0.map_err(Error::invalid)
    }

    /// Whom `to` names, for `by`, whose instance asks, as a telling would
    /// find them: each that has a box (`told`), and each that has none
    /// (`passed`). Nothing is written, and nobody is told: it is for what
    /// names them in words somebody reads (a question said on a record,
    /// whose naming is what tells them). A word that names nobody is
    /// refused, as a telling refuses it.
    pub async fn whom(&self, by: &Principal, to: &[String]) -> Result<Told> {
        let (channels, to, applier) = (self.clone(), to.to_vec(), by.actor());
        let found = self
            .journal
            .db()
            .call(move |connection| {
                let mut owners: Vec<String> = Vec::new();
                for word in &to {
                    match channels.named_by(connection, word, &applier)? {
                        Ok(named) => {
                            for one in named {
                                if !owners.contains(&one) {
                                    owners.push(one);
                                }
                            }
                        }
                        Err(why) => return Ok(Err(why)),
                    }
                }
                let mut came = Told::default();
                for owner in owners {
                    match channels.inbox.holds_now(connection, &owner)? {
                        Some(held) if !covers(&held, READ) => came.passed.push(owner),
                        _ => came.told.push(owner),
                    }
                }
                Ok(Ok(came))
            })
            .await?;
        found.map_err(Error::invalid)
    }

    /// Says what reads which cards a column of a board holds, for every
    /// keeper of these channels. Said once; a second is not taken.
    pub fn columns_with(&self, columns: Arc<Columns>) {
        let _ = self.columns.set(columns);
    }

    /// Each part of `parts` that lists a column's cards, with the cards
    /// that column holds now kept on it. With nothing said of how a
    /// column is read, and for a board that is not there, none is kept,
    /// and the part lists what is there when it is read.
    async fn keep_cards(&self, parts: &mut [serde_json::Value]) -> Result<()> {
        let Some(columns) = self.columns.get() else { return Ok(()) };
        for part in parts.iter_mut() {
            let Some((project, column)) = kernel::parts::keeps(part) else { continue };
            match columns(project, column).await {
                Ok(cards) => kernel::parts::keep(part, cards),
                Err(err) if err.kind == ErrorKind::NotFound => {}
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }

    /// The actors `word` names, or why it names nobody that can be told.
    fn named_by(
        &self,
        connection: &rusqlite::Connection,
        word: &str,
        applier: &str,
    ) -> rusqlite::Result<std::result::Result<Vec<String>, String>> {
        let word = word.trim();
        if word == APPLIER {
            return Ok(match kind_of(applier) {
                "user" | "agent" => Ok(vec![applier.to_string()]),
                _ => Err(format!(
                    "whoever applied this is `{applier}`, which has no inbox: a box is a person's or an agent's"
                )),
            });
        }
        if word.contains(':') {
            // An actor: somebody's, where they hold anything at all.
            let there = match self.inbox.holds_now(connection, word)? {
                Some(held) => !held.is_empty(),
                None => true,
            };
            return Ok(match kind_of(word) {
                "user" | "agent" if there => Ok(vec![word.to_string()]),
                "bot" => {
                    Err(format!("`{word}` is a bot, which has no inbox: a box is a person's or an agent's"))
                }
                _ => Err(nobody(word)),
            });
        }
        let handle = word.strip_prefix('@').unwrap_or(word).to_string();
        if handle.is_empty() {
            return Ok(Err(nobody(word)));
        }
        let mut named: Vec<String> = match self.naming.get() {
            Some(naming) => {
                naming(connection, std::slice::from_ref(&handle))?.iter().map(Principal::actor).collect()
            }
            None => Vec::new(),
        };
        // An agent is called what it is: one of that name, where one
        // holds anything.
        let agent = format!("agent:{handle}");
        if self.inbox.holds_now(connection, &agent)?.is_some_and(|held| !held.is_empty()) {
            named.push(agent);
        }
        Ok(if named.is_empty() { Err(nobody(word)) } else { Ok(named) })
    }
}
