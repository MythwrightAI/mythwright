//! A key withdrawn, and a key rolled.
//!
//! A key is a credential somebody makes for a script ([`Identity::issue`]).
//! Withdrawn, it is nobody's credential from then, and so is every key
//! that was made with it, and with those, all the way down: a key holds no
//! more than whatever made it, and a key made with a withdrawn key would
//! otherwise be a way for whoever held the first to go on. Which key a key
//! was made with is kept on it (`credentials.made_with`); a key made
//! before that was kept says nothing, and is withdrawn by itself.
//!
//! What a withdrawn key applied is stopped by whoever withdraws it, which
//! is the runner's to do (`conductor::Engine::applied_with`): this keeps
//! and says the withdrawing, and says too whether what it applied was left
//! running (`leave`).
//!
//! Rolled, a key is given another secret: the one it had is refused from
//! then, and the key is the same key, by its id, holding what it held, so
//! what it applied, which runs as that key, goes on.
//!
//! Each is one write with what is said of it (`identity.key.revoked`, one
//! for each key withdrawn; `identity.key.rolled`), and whatever goes on
//! answering somebody it let in once is told to ask again
//! ([`Identity::withdrawn`]).

use journal::optional;
use kernel::{Error, Principal, PrincipalKind, Result, Source};
use rusqlite::params;
use serde_json::json;

use crate::identity::{Credential, CredentialKind, Identity, digest, read_credential};

/// A key and the keys made with it, each still in use: what withdrawing
/// the first withdraws, the first first.
#[derive(Clone, Debug)]
pub struct Withdrawing {
    pub key: Credential,
    /// The keys made with it, and with those, all the way down.
    pub made_with: Vec<Credential>,
}

impl Withdrawing {
    /// Every key it withdraws, the first first.
    pub fn all(&self) -> impl Iterator<Item = &Credential> {
        std::iter::once(&self.key).chain(&self.made_with)
    }
}

/// The columns a credential is read by.
const COLUMNS: &str = "id, kind, subject, name, scopes, created, revoked, made_with";

/// The key `id`, in use, if `by` may withdraw or roll it: its maker,
/// signed in, and whoever manages the installation. Asked with a key, it
/// takes what managing the installation takes, but for a key rolling
/// itself (`own`): a key given to a script is not a way to take its
/// maker's others away.
fn theirs(found: Option<Credential>, id: &str, by: &Principal, own: bool) -> Result<Credential> {
    let Some(key) = found.filter(|one| one.kind == CredentialKind::Key && one.revoked.is_none()) else {
        return Err(Error::not_found(format!("there is no key `{id}` of yours that is in use")));
    };
    if by.can("admin") || (own && by.key.as_deref() == Some(id)) {
        return Ok(key);
    }
    if by.key.is_some() {
        return Err(Error::denied(
            "a key may not withdraw or roll another key, and this one does not hold `admin`: sign in and ask \
             again",
        ));
    }
    match by.kind == PrincipalKind::User && key.subject == by.id {
        true => Ok(key),
        false => Err(Error::not_found(format!("there is no key `{id}` of yours that is in use"))),
    }
}

/// The key `id` as it is kept, in use or not.
fn one(connection: &rusqlite::Connection, id: &str) -> rusqlite::Result<Option<Credential>> {
    optional(connection.query_row(
        &format!("SELECT {COLUMNS} FROM credentials WHERE id = ?1"),
        [id],
        read_credential,
    ))
}

/// The keys in use that were made with `id`, and with those, all the way
/// down, the nearest first.
fn made_with(connection: &rusqlite::Connection, id: &str) -> rusqlite::Result<Vec<Credential>> {
    let mut statement = connection.prepare(&format!(
        "WITH RECURSIVE under(id, depth) AS (
             SELECT id, 1 FROM credentials WHERE made_with = ?1 AND kind = 'key' AND revoked IS NULL
             UNION
             SELECT credentials.id, under.depth + 1 FROM credentials JOIN under ON credentials.made_with = under.id
             WHERE credentials.kind = 'key' AND credentials.revoked IS NULL
         )
         SELECT {COLUMNS} FROM credentials WHERE id IN (SELECT id FROM under)
         ORDER BY (SELECT min(depth) FROM under WHERE under.id = credentials.id), created"
    ))?;
    let rows = statement.query_map([id], read_credential)?;
    rows.collect()
}

impl Identity {
    /// What withdrawing the key `id` would withdraw, as `by` would ask it:
    /// the key and the keys made with it. Nothing is changed.
    pub async fn withdrawing(&self, id: &str, by: &Principal) -> Result<Withdrawing> {
        let key = id.to_string();
        let (found, under) = self
            .journal
            .db()
            .call(move |connection| Ok((one(connection, &key)?, made_with(connection, &key)?)))
            .await?;
        let key = theirs(found, id, by, false)?;
        Ok(Withdrawing { key, made_with: under })
    }

    /// Withdraws the key `id`, as `by`, and every key made with it: from
    /// then none of them is anybody's credential. Each is said
    /// (`identity.key.revoked`), with `leave` where whoever withdrew them
    /// left what they applied running. What they applied is the runner's
    /// to stop, and is not touched here.
    pub async fn withdraw_key(&self, id: &str, by: &Principal, leave: bool) -> Result<Withdrawing> {
        let (id, by, now) = (id.to_string(), by.clone(), self.journal.now_ms());
        let actor = crate::given::actor_of(&by);
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let found = one(transaction, &id)?;
            let key = match theirs(found, &id, &by, false) {
                Ok(key) => key,
                Err(refused) => return Ok(Err(refused)),
            };
            let under = made_with(transaction, &id)?;
            let withdrawing = Withdrawing { key, made_with: under };
            for each in withdrawing.all() {
                transaction.execute(
                    "UPDATE credentials SET revoked = ?2 WHERE id = ?1 AND revoked IS NULL",
                    params![each.id, now],
                )?;
                let mut said = json!({ "key": each.id, "name": each.name, "account": each.subject });
                if each.id != id {
                    said["with"] = json!(id);
                }
                if leave {
                    said["leave"] = json!(true);
                }
                writing.say("identity.key.revoked", Source::actor(actor.clone()), said)?;
            }
            Ok(Ok(withdrawing))
        });
        let withdrawn = wrote.await?.0?;
        self.say_withdrawn();
        Ok(withdrawn)
    }

    /// Gives the key `id` another secret, as `by`: the token it had is
    /// refused from then. It is the same key by its id, holding what it
    /// held, so what it applied goes on. Its maker may, whoever manages the
    /// installation, and the key itself. The new token, which is shown
    /// only now.
    pub async fn roll_key(&self, id: &str, by: &Principal) -> Result<(Credential, String)> {
        let (id, by) = (id.to_string(), by.clone());
        let actor = crate::given::actor_of(&by);
        let token = format!("key_{}", kernel::id::secret());
        let hash = digest(&token);
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let found = one(transaction, &id)?;
            let key = match theirs(found, &id, &by, true) {
                Ok(key) => key,
                Err(refused) => return Ok(Err(refused)),
            };
            transaction.execute(
                "UPDATE credentials SET token = ?2 WHERE id = ?1 AND revoked IS NULL",
                params![key.id, hash],
            )?;
            let said = json!({ "key": key.id, "name": key.name, "account": key.subject });
            writing.say("identity.key.rolled", Source::actor(actor), said)?;
            Ok(Ok(key))
        });
        let key = wrote.await?.0?;
        self.say_withdrawn();
        Ok((key, token))
    }
}
