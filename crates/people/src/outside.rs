//! Somebody a service outside this installation vouches for.
//!
//! A hosted identity service is no part of this application, and nothing
//! here reaches one or says which to believe: no route asks this yet.
//! This is the seam for it, and says what a statement from outside can
//! come to. A statement is who says it (the issuer), whom they say it of
//! (their subject), and what they say of them: an address, a name, a
//! plan.
//!
//! The account is the issuer's subject's, and is found by those two and
//! by nothing else. The address a statement carries finds nobody: an
//! address is what the issuer says, and a service that named an address
//! the installation already knew, its owner's included, would have been
//! handed that account and have set its plan. So where the address is an
//! account's that is not already this issuer's subject, the sign-in is
//! refused. Joining the two would take that account's own say, and
//! nothing asks for it yet.

use journal::optional;
use kernel::{Error, Result};
use rusqlite::params;

use crate::identity::{ACCOUNT, Account, Identity, SignedIn, Tier, normalise_email, read_account};

/// How an account says whose it is outside: the issuer and its subject,
/// written so that the two are told apart whatever either holds. Joined
/// by a mark, as they were, `a:b` of `c` and `a` of `b:c` were one, and
/// an issuer is as often an address, which has the mark in it.
fn theirs(issuer: &str, subject: &str) -> String {
    serde_json::json!([issuer, subject]).to_string()
}

impl Identity {
    /// Signs in the subject `issuer` vouches for, making their account
    /// here the first time, and gives that account the plan the
    /// statement carries.
    ///
    /// Refused where the address is one an account here has that is not
    /// this issuer's subject already: nobody is handed an account by its
    /// address. Two issuers that say the same subject are two accounts.
    pub async fn sign_in_external(
        &self,
        issuer: &str,
        subject: &str,
        email: &str,
        name: &str,
        tier: Tier,
    ) -> Result<SignedIn> {
        if issuer.trim().is_empty() || subject.trim().is_empty() {
            return Err(Error::invalid(
                "a sign-in from outside says who vouches and for whom, and one of the two is empty: \
                 nobody is signed in by it",
            ));
        }
        let email = normalise_email(email)?;
        let external = theirs(issuer, subject);
        let made = Account {
            id: self.journal.new_id("acct"),
            email,
            name: name.trim().to_string(),
            // Settled where the account is written, among the accounts
            // there are then.
            role: String::new(),
            tier,
            created: self.journal.now_ms(),
            given: Vec::new(),
        };
        let said = (issuer.to_string(), subject.to_string());
        // One piece of work, so that whose the account is, is decided
        // and written among the accounts as they are at one moment.
        let account = self
            .journal
            .db()
            .call(move |connection| {
                let known = optional(connection.query_row(
                    &format!("SELECT {ACCOUNT} FROM accounts WHERE external = ?1"),
                    [&external],
                    read_account,
                ))?;
                if let Some(mut account) = known {
                    connection.execute(
                        "UPDATE accounts SET tier = ?2 WHERE id = ?1",
                        params![account.id, tier.as_str()],
                    )?;
                    account.tier = tier;
                    return Ok(Ok(account));
                }
                let taken: i64 = connection.query_row(
                    "SELECT count(*) FROM accounts WHERE email = ?1",
                    [&made.email],
                    |row| row.get(0),
                )?;
                if taken > 0 {
                    return Ok(Err(Error::conflict(format!(
                        "an account here already has the address {}, and it is not the one `{}` vouches for as \
                         `{}`: nobody is signed in to an account by its address, and nothing of that account \
                         was changed. Joining the two takes that account's own say, which nothing asks for \
                         yet: sign in as that account the way it always has, or have `{}` vouch for an \
                         address no account here has",
                        made.email, said.0, said.1, said.0
                    ))));
                }
                let first: i64 = connection.query_row("SELECT count(*) FROM accounts", [], |row| row.get(0))?;
                let role = if first == 0 { "owner" } else { "member" };
                connection.execute(
                    "INSERT INTO accounts (id, email, name, role, tier, external, created) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![made.id, made.email, made.name, role, made.tier.as_str(), external, made.created],
                )?;
                Ok(Ok(Account { role: role.to_string(), ..made }))
            })
            .await??;
        self.start_session(account).await
    }
}
