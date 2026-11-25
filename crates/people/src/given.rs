//! What a person is given beyond what their role holds.
//!
//! A member holds what members hold (`MEMBER_SCOPES`), and the owner
//! holds besides what nobody is given without saying so (`admin`,
//! `private`, `environments:secrets`). Somebody may be given more, a
//! scope at a time, by whoever holds that scope and `admin`: nobody gives
//! what they do not hold. It is kept on the account (`accounts.given`),
//! and a session's scopes are worked out from the account as it is used,
//! so what is given or taken away reaches them at once.
//!
//! Taken away, it is taken from the keys that person made too, since a
//! key holds no more than whoever made it: a key that held it holds what
//! is left of it. Whoever was told something because they held it is
//! asked again (`Identity::withdrawn`). What somebody did with it while
//! they held it stays done: a table's row they wrote, an agent they gave
//! it to, an instance they applied.
//!
//! Each change is one write with what is said of it (`identity.given`,
//! `identity.taken`), heard by whoever manages the factory.

use journal::optional;
use kernel::principal::{covers, named};
use kernel::{Error, Principal, Result, Source};
use rusqlite::params;
use serde_json::json;

use crate::identity::{ACCOUNT, Account, Identity, read_account};

/// What is kept of what an account was given, read back: nothing for
/// none, and for what cannot be read as a list.
pub(crate) fn read(kept: Option<String>) -> Vec<String> {
    kept.and_then(|kept| serde_json::from_str(&kept).ok()).unwrap_or_default()
}

/// What giving or taking away came to: the account as it is now, and
/// whether anything changed. A taking away says how many of the person's
/// keys held what was taken, and hold it no longer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Given {
    pub account: Account,
    pub changed: bool,
    pub keys: usize,
}

/// Who did it, as what is said names them: a key by its id.
pub(crate) fn actor_of(by: &Principal) -> String {
    by.key.as_ref().map(|key| format!("key:{key}")).unwrap_or_else(|| by.actor())
}

fn no_account(id: &str) -> Error {
    Error::not_found(format!("there is no account `{id}`: whoever manages the factory sees each in Settings"))
}

impl Identity {
    /// Gives the account `id` the scope `scope`, as `by`, who holds it and
    /// `admin`. One the account holds already, by its role or given, is
    /// left as it is and nothing is said.
    pub async fn give(&self, id: &str, scope: &str, by: &Principal) -> Result<Given> {
        by.require("admin")?;
        if !named(scope) {
            return Err(Error::invalid(format!(
                "`{scope}` is not a scope this factory checks: a scope is an area, such as `board`, or one thing \
                 done in it, such as `board:read`"
            )));
        }
        if !by.can(scope) {
            return Err(Error::denied(format!(
                "nobody gives what they do not hold, and you do not hold `{scope}`: it is given by somebody who \
                 holds it"
            )));
        }
        let (id, scope, actor) = (id.to_string(), scope.to_string(), actor_of(by));
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let Some(mut account) = optional(transaction.query_row(
                &format!("SELECT {ACCOUNT} FROM accounts WHERE id = ?1"),
                [&id],
                read_account,
            ))?
            else {
                return Ok(Err(no_account(&id)));
            };
            if account.role == "owner" {
                return Ok(Err(Error::conflict(format!(
                    "{} owns this factory and holds every scope there is: nothing is given to them",
                    account.name
                ))));
            }
            let grouped = crate::groups::held_through(transaction, &format!("user:{id}"))?;
            if account.principal_with(&grouped).can(&scope) {
                return Ok(Ok(Given { account, changed: false, keys: 0 }));
            }
            account.given.push(scope.clone());
            account.given.sort();
            let kept = serde_json::to_string(&account.given).unwrap_or_default();
            transaction.execute("UPDATE accounts SET given = ?1 WHERE id = ?2", params![kept, id])?;
            let data = json!({ "account": id, "scope": scope, "given": account.given });
            writing.say("identity.given", Source::actor(actor), data)?;
            Ok(Ok(Given { account, changed: true, keys: 0 }))
        });
        wrote.await?.0
    }

    /// Takes the scope `scope` away from what the account `id` was given,
    /// as `by`, who manages the factory, and from the keys it made. What
    /// its role holds is not taken away here, and one it was not given is
    /// refused.
    pub async fn take(&self, id: &str, scope: &str, by: &Principal) -> Result<Given> {
        by.require("admin")?;
        let (id, scope, actor) = (id.to_string(), scope.to_string(), actor_of(by));
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let Some(mut account) = optional(transaction.query_row(
                &format!("SELECT {ACCOUNT} FROM accounts WHERE id = ?1"),
                [&id],
                read_account,
            ))?
            else {
                return Ok(Err(no_account(&id)));
            };
            if !account.given.contains(&scope) {
                let without = Account { given: Vec::new(), ..account.clone() };
                let why = match without.principal().can(&scope) {
                    true => format!(
                        "{} holds `{scope}` by being {}, not by being given it, and a role's scopes are not taken \
                         away here",
                        account.name,
                        if account.role == "owner" { "the owner" } else { "a member" }
                    ),
                    false => format!(
                        "{} was not given `{scope}`; what they were given is {}",
                        account.name,
                        match account.given.is_empty() {
                            true => "nothing".to_string(),
                            false => account.given.iter().map(|one| format!("`{one}`")).collect::<Vec<_>>().join(", "),
                        }
                    ),
                };
                return Ok(Err(Error::conflict(why)));
            }
            account.given.retain(|one| *one != scope);
            let kept = match account.given.is_empty() {
                true => None,
                false => Some(serde_json::to_string(&account.given).unwrap_or_default()),
            };
            transaction.execute("UPDATE accounts SET given = ?1 WHERE id = ?2", params![kept, id])?;
            // Their keys hold no more than they do now, groups and all.
            let whose = format!("user:{id}");
            let holds = account.principal_with(&crate::groups::held_through(transaction, &whose)?).scopes;
            let narrowed = narrow_keys(transaction, &id, &holds)?;
            let data = json!({ "account": id, "scope": scope, "given": account.given, "keys": narrowed });
            writing.say("identity.taken", Source::actor(actor), data)?;
            Ok(Ok(Given { account, changed: true, keys: narrowed }))
        });
        let taken = wrote.await?.0?;
        self.say_withdrawn();
        Ok(taken)
    }
}

/// Narrows each key `account` made to what `holds` covers, in a write that
/// is under way: how many were narrowed. A key holds no more than whoever
/// made it, as things stand.
pub(crate) fn narrow_keys(
    transaction: &rusqlite::Connection,
    account: &str,
    holds: &[String],
) -> rusqlite::Result<usize> {
    let keys: Vec<(String, String)> = {
        let mut keys = transaction.prepare(
            "SELECT id, scopes FROM credentials WHERE kind = 'key' AND subject = ?1 AND revoked IS NULL",
        )?;
        keys.query_map([account], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<rusqlite::Result<_>>()?
    };
    let mut narrowed = 0;
    for (key, scopes) in keys {
        let scopes: Vec<String> = serde_json::from_str(&scopes).unwrap_or_default();
        let left: Vec<&String> = scopes.iter().filter(|one| covers(holds, one)).collect();
        if left.len() != scopes.len() {
            let left = serde_json::to_string(&left).unwrap_or_default();
            transaction.execute("UPDATE credentials SET scopes = ?1 WHERE id = ?2", params![left, key])?;
            narrowed += 1;
        }
    }
    Ok(narrowed)
}
