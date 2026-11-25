//! Beginning with no registration: the owner made as a factory first
//! starts, and a link that signs somebody in once.
//!
//! A factory started from the command line has an owner before it answers
//! anybody ([`Identity::make_owner`]): called `Unnamed` ([`kernel::UNNAMED`])
//! until they say what they are called, with no email and no password, and
//! signed in on the command line that started it. Nothing of the machine or
//! of whoever runs it is taken for a name. A browser is signed in by a link the command line
//! asks for (`open`): a secret that is good once and for a minute
//! ([`LINK_MS`]), kept in this process's memory by its hash and nowhere
//! else, and traded for a session by a route that takes none.
//!
//! Who may ask for a link is a person, for themselves, with something that
//! holds all they hold: their own session, or a key of theirs that holds
//! every scope they do ([`Identity::link_for`]). A key that holds less would
//! otherwise become a session that holds more; an agent and a device are
//! signed in by nothing of the kind. And the owner's link is given to
//! whoever proves they can read the factory's data folder, which is how a
//! command line on this machine that holds no session comes by one
//! ([`Identity::nonce`], `link::own::Own::proves`): what is in that folder
//! is the factory whole, so being able to read it is being its owner
//! already.
//!
//! A password is something an account may come to have, to sign in another
//! way (from another device), and never needs to begin
//! ([`Identity::set_password`]).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use journal::optional;
use kernel::{Error, Principal, PrincipalKind, Result, Source};
use rusqlite::params;
use serde::Serialize;
use serde_json::json;

use crate::identity::{ACCOUNT, Account, Identity, MIN_PASSWORD, SignedIn, Tier, digest, read_account};

/// How long a link, and what is asked to be proved for one, is good for.
pub const LINK_MS: i64 = 60 * 1000;

/// The most links, and the most things to prove, held at once: past it
/// the oldest goes. Nobody signs in sixty-four times in a minute.
const MOST: usize = 64;

/// A link made: the secret it carries, shown once, and when it stops working.
#[derive(Clone, Debug, Serialize)]
pub struct Link {
    pub once: String,
    pub expires: i64,
}

/// The links not yet spent, and what was given to be proved, each by its
/// hash and with when it stops working. In memory alone: a daemon that
/// stops forgets them, and a minute is shorter than anything that keeps.
#[derive(Clone, Default)]
pub struct Links(Arc<Mutex<Held>>);

#[derive(Default)]
struct Held {
    /// A link's hash: whose it is and until when.
    once: HashMap<String, (String, i64)>,
    /// What was given to be proved, as it was given: until when.
    nonces: HashMap<String, i64>,
}

/// Keeps `key` in `kept` until `until`, with what has stopped working
/// dropped first and the oldest after it where there is no room.
fn keep<V: Clone>(kept: &mut HashMap<String, V>, key: String, value: V, now: i64, until: impl Fn(&V) -> i64) {
    kept.retain(|_, value| until(value) > now);
    while kept.len() >= MOST {
        let oldest = kept.iter().min_by_key(|(_, value)| until(value)).map(|(key, _)| key.clone());
        match oldest {
            Some(oldest) => kept.remove(&oldest),
            None => break,
        };
    }
    kept.insert(key, value);
}

impl Identity {
    /// Makes the owner where nobody has an account yet, called `Unnamed`, with
    /// no email and no password, and starts a session of theirs for
    /// whoever started the factory; nothing where there is an account
    /// already. Asked as a factory starts, before it answers anybody: so
    /// that nobody who can reach it ever finds it with no owner.
    pub async fn make_owner(&self) -> Result<Option<SignedIn>> {
        let name = kernel::UNNAMED;
        let account = Account {
            id: self.journal.new_id("acct"),
            email: String::new(),
            name: name.to_string(),
            role: "owner".to_string(),
            tier: Tier::Free,
            created: self.journal.now_ms(),
            given: Vec::new(),
        };
        let row = account.clone();
        // Written only while there is no account at all, in the one
        // statement, as the first account always was, with what is said of
        // it.
        let written = self.journal.write(move |writing| {
            let made = writing.transaction().execute(
                "INSERT INTO accounts (id, email, name, password, role, tier, created) \
                 SELECT ?1, '', ?2, NULL, 'owner', ?3, ?4 WHERE NOT EXISTS (SELECT 1 FROM accounts)",
                params![row.id, row.name, row.tier.as_str(), row.created],
            )?;
            if made > 0 {
                let data = json!({ "account": row.id, "role": "owner", "made": true });
                writing.say("identity.registered", Source::actor("system"), data)?;
            }
            Ok(made)
        });
        if written.await?.0 == 0 {
            return Ok(None);
        }
        Ok(Some(self.start_session(account).await?))
    }

    /// The account `who` is, where what they asked with holds all it holds:
    /// a session, or a key that holds every scope they do as things stand.
    async fn whole(&self, who: &Principal, what: &str) -> Result<Account> {
        if who.kind != PrincipalKind::User {
            return Err(Error::denied(format!("{what} is a person's, and is asked as one")));
        }
        let account = self.account(&who.id).await?;
        if who.key.is_some() {
            let theirs = self.principal_of(&account).scopes;
            let lacks: Vec<&String> =
                theirs.iter().filter(|scope| !kernel::principal::covers(&who.scopes, scope)).collect();
            if !lacks.is_empty() {
                let lacks: Vec<&str> = lacks.iter().map(|scope| scope.as_str()).collect();
                return Err(Error::denied(format!(
                    "{what} is asked with something that holds all its account holds, and this key holds less \
                     (not {}): ask signed in, or with a key that holds everything its maker does",
                    lacks.join(", ")
                )));
            }
        }
        Ok(account)
    }

    /// A link that signs `who` in once, as themselves: asked with their
    /// session, or a key of theirs that holds all they hold.
    pub async fn link_for(&self, who: &Principal) -> Result<Link> {
        let account = self.whole(who, "a link that signs somebody in").await?;
        Ok(self.link_of(&account.id))
    }

    /// A link that signs the owner in once, for whoever proved they can
    /// read the factory's data folder.
    pub async fn link_for_owner(&self) -> Result<Link> {
        let owner = self.accounts().await?.into_iter().find(|account| account.role == "owner");
        let owner = owner.ok_or_else(|| Error::not_found("this factory has no owner yet"))?;
        Ok(self.link_of(&owner.id))
    }

    fn link_of(&self, account: &str) -> Link {
        let (once, now) = (format!("once_{}", kernel::id::secret()), self.journal.now_ms());
        let expires = now + LINK_MS;
        let mut held = self.links.0.lock().expect("links lock");
        keep(&mut held.once, digest(&once), (account.to_string(), expires), now, |(_, until)| *until);
        Link { once, expires }
    }

    /// Something to be proved, given to whoever asks and taken back once.
    pub fn nonce(&self) -> String {
        let (nonce, now) = (kernel::id::secret(), self.journal.now_ms());
        let mut held = self.links.0.lock().expect("links lock");
        keep(&mut held.nonces, nonce.clone(), now + LINK_MS, now, |until| *until);
        nonce
    }

    /// Takes back what was given to be proved, where it was given and is
    /// still good: once, whatever the proof comes to.
    pub fn take_nonce(&self, nonce: &str) -> Result<()> {
        let now = self.journal.now_ms();
        let until = self.links.0.lock().expect("links lock").nonces.remove(nonce);
        match until {
            Some(until) if until > now => Ok(()),
            _ => Err(Error::unauthenticated(
                "that was not given here to be proved, or is more than a minute old",
            )),
        }
    }

    /// Spends a link: a session of whoever it was made for, the first time
    /// it is asked within its minute, and nothing ever after.
    pub async fn spend(&self, once: &str) -> Result<SignedIn> {
        let now = self.journal.now_ms();
        let found = self.links.0.lock().expect("links lock").once.remove(&digest(once));
        let refused = || Error::unauthenticated("that link was used already, or is more than a minute old");
        let (account, _) = found.filter(|(_, until)| *until > now).ok_or_else(refused)?;
        let account = self.account(&account).await.map_err(|_| refused())?;
        self.start_session(account).await
    }

    /// Whether `account` has a password to sign in with.
    pub async fn has_password(&self, account: &str) -> Result<bool> {
        let id = account.to_string();
        let found: Option<Option<String>> = self
            .journal
            .db()
            .call(move |c| {
                optional(c.query_row("SELECT password FROM accounts WHERE id = ?1", [&id], |row| row.get(0)))
            })
            .await?;
        Ok(matches!(found, Some(Some(_))))
    }

    /// Gives the account `who` is a password to sign in with another way
    /// (from another device), with an email to sign in by where it has
    /// none: for an account that has no password, asked by the person
    /// themselves with all they hold. One that has a password keeps it.
    pub async fn set_password(
        &self,
        who: &Principal,
        email: Option<&str>,
        password: &str,
    ) -> Result<Account> {
        let account = self.whole(who, "a password").await?;
        if self.has_password(&account.id).await? {
            return Err(Error::conflict("this account has a password already, and it is kept"));
        }
        if password.chars().count() < MIN_PASSWORD {
            return Err(Error::invalid(format!("the password needs at least {MIN_PASSWORD} characters")));
        }
        let email = match (account.email.is_empty(), email.filter(|email| !email.trim().is_empty())) {
            (true, None) => return Err(Error::invalid("an email to sign in by is needed too")),
            (true, Some(email)) => crate::identity::normalise_email(email)?,
            (false, _) => account.email.clone(),
        };
        let hash = crate::identity::hash_password(password.to_string()).await?;
        let (id, written_email, actor) = (account.id.clone(), email.clone(), who.actor());
        let written = self.journal.write(move |writing| {
            writing.transaction().execute(
                "UPDATE accounts SET email = ?2, password = ?3 WHERE id = ?1 AND password IS NULL",
                params![id, written_email, hash],
            )?;
            writing.say("identity.password", Source::actor(actor), json!({ "account": id }))?;
            Ok(())
        });
        written.await.map_err(|err| match err.kind {
            kernel::ErrorKind::Conflict => {
                Error::conflict(format!("there is already an account for {email}"))
            }
            _ => err,
        })?;
        self.account(&account.id).await
    }

    /// Calls the account `who` is something else.
    pub async fn rename(&self, who: &Principal, name: &str) -> Result<(Account, String)> {
        self.whole(who, "a name of one's own").await?;
        let name = name.trim().to_string();
        if name.is_empty() || name.chars().count() > 60 || name.contains('\n') {
            return Err(Error::invalid("a name is one line of at most 60 letters, and is something"));
        }
        let (id, actor) = (who.id.clone(), who.actor());
        let written = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let read = |transaction: &rusqlite::Connection| {
                optional(transaction.query_row(
                    &format!("SELECT {ACCOUNT} FROM accounts WHERE id = ?1"),
                    [&id],
                    read_account,
                ))
            };
            let Some(was) = read(transaction)? else { return Ok(None) };
            transaction.execute("UPDATE accounts SET name = ?2 WHERE id = ?1", params![id, name])?;
            let account = read(transaction)?;
            let data = json!({ "account": id, "name": name, "was": was.name });
            writing.say("identity.renamed", Source::actor(actor), data)?;
            Ok(account.map(|account| (account, was.name)))
        });
        written.await?.0.ok_or_else(|| Error::not_found("there is no such account"))
    }
}
