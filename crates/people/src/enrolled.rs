//! Invitations to enrol: how something that is no person, and that this
//! factory does not start, comes by a credential of its own.
//!
//! An agent the factory starts is handed its credential in a file the
//! factory writes. One it does not start is somewhere the factory
//! writes nothing, so it is let in as a device is: whoever may bring it
//! in makes an **invitation**, one string, made for one subject, good
//! once and for a while, and whoever holds the invitation hands it over
//! and is answered the credential, once. The factory keeps the hash of
//! the invitation's secret and never the secret, as it keeps the hash of
//! a token and never the token, so nothing it keeps lets anybody in.
//!
//! Invitations are in the journal's `kv`, under [`INVITATIONS`], by the
//! hash of the secret: a copy of a factory takes none of them, since
//! that is no name a copy keeps. One that has stopped working is read
//! as not there, from the clock when it is used, and is put away when
//! the next is made; nothing looks for the ones that have run out.
//!
//! What a credential made this way holds is changed where it is, by
//! [`Identity::rescope`]: its token is kept where this factory cannot
//! write another, so it is not replaced to be changed.

use std::time::Duration;

use kernel::{Error, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::identity::{CredentialKind, Identity, digest};

/// Where each invitation to enrol is kept, by the hash of its secret.
pub const INVITATIONS: &str = "identity.enrolments";

/// What an invitation to enrol begins with, so that one is known for
/// what it is.
pub const INVITATION: &str = "enrol_";

/// How long one works unless whoever makes it says, and the longest it
/// may: what a device's does.
pub const ENROLS_FOR: Duration = Duration::from_secs(24 * 3600);
pub const ENROLS_AT_MOST: Duration = Duration::from_secs(14 * 24 * 3600);

/// An invitation, as it is kept: never its secret.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Invited {
    kind: String,
    subject: String,
    expires: i64,
    /// Who made it, as an event says who did something.
    by: String,
}

impl Identity {
    /// Makes an invitation for `subject` to enrol with, working for
    /// `lasts` from now ([`ENROLS_FOR`] unless said, [`ENROLS_AT_MOST`]
    /// at the longest): the invitation, shown only now, and when it
    /// stops working. Any earlier one for the same subject is withdrawn:
    /// a subject is let in by the last invitation made for it and no
    /// other. Whether whoever asks may bring the subject in is asked by
    /// whoever keeps what the subject is.
    pub async fn invite_to_enrol(
        &self,
        kind: CredentialKind,
        subject: &str,
        by: &str,
        lasts: Option<Duration>,
    ) -> Result<(String, i64)> {
        let lasts = lasts.unwrap_or(ENROLS_FOR);
        if lasts.is_zero() || lasts > ENROLS_AT_MOST {
            return Err(Error::invalid(
                "an invitation works for a day unless said, and for no more than fourteen days: say how long as \
                 something like 2d or 12h",
            ));
        }
        let now = self.journal.now_ms();
        // What stopped working is of no more use, and neither is an
        // earlier one for this subject.
        for (hash, kept) in self.journal.kv_list(INVITATIONS).await? {
            let gone = serde_json::from_value::<Invited>(kept).map_or(true, |invited| {
                invited.expires <= now || (invited.kind == kind.as_str() && invited.subject == subject)
            });
            if gone {
                self.journal.kv_set(INVITATIONS, &hash, &serde_json::Value::Null).await?;
            }
        }
        let secret = kernel::id::secret();
        let expires = now + lasts.as_millis() as i64;
        let kept = Invited {
            kind: kind.as_str().to_string(),
            subject: subject.to_string(),
            expires,
            by: by.to_string(),
        };
        self.journal.kv_set(INVITATIONS, &digest(&secret), &json!(kept)).await?;
        Ok((format!("{INVITATION}{secret}"), expires))
    }

    /// The subject `invitation` would let in as things stand: one this
    /// factory made for a subject of this kind, that has not been used
    /// or withdrawn, and has not stopped working. Nothing otherwise,
    /// whichever of those it is: whoever answers says one thing for all
    /// of them. It is not used by being asked about ([`Identity::enrolled`]).
    pub async fn enrolling(&self, kind: CredentialKind, invitation: &str) -> Result<Option<String>> {
        let Some(secret) = invitation.trim().strip_prefix(INVITATION) else { return Ok(None) };
        let kept = self.journal.kv_get(INVITATIONS, &digest(secret)).await?;
        let invited = kept.and_then(|kept| serde_json::from_value::<Invited>(kept).ok());
        let good =
            |invited: &Invited| invited.kind == kind.as_str() && invited.expires > self.journal.now_ms();
        Ok(invited.filter(good).map(|invited| invited.subject))
    }

    /// Uses `invitation` up: it lets nobody in from now. Whoever let its
    /// subject in says so, holding its own turn from the asking to
    /// this, so that one invitation handed over twice at once lets in
    /// once.
    pub async fn enrolled(&self, invitation: &str) -> Result<()> {
        let secret = invitation.trim().strip_prefix(INVITATION).unwrap_or_default();
        self.journal.kv_set(INVITATIONS, &digest(secret), &serde_json::Value::Null).await
    }

    /// Withdraws whatever invitation waits for `subject`: how many did.
    pub async fn uninvite(&self, kind: CredentialKind, subject: &str) -> Result<usize> {
        let mut withdrawn = 0;
        for (hash, kept) in self.journal.kv_list(INVITATIONS).await? {
            let theirs = serde_json::from_value::<Invited>(kept)
                .is_ok_and(|invited| invited.kind == kind.as_str() && invited.subject == subject);
            if theirs {
                self.journal.kv_set(INVITATIONS, &hash, &serde_json::Value::Null).await?;
                withdrawn += 1;
            }
        }
        Ok(withdrawn)
    }

    /// When the invitation that waits for `subject` stops working, where
    /// one waits that still works.
    pub async fn invited_until(&self, kind: CredentialKind, subject: &str) -> Result<Option<i64>> {
        let now = self.journal.now_ms();
        let waits = self.journal.kv_list(INVITATIONS).await?.into_iter().filter_map(|(_, kept)| {
            let invited = serde_json::from_value::<Invited>(kept).ok()?;
            (invited.kind == kind.as_str() && invited.subject == subject && invited.expires > now)
                .then_some(invited.expires)
        });
        Ok(waits.max())
    }

    /// Whether `subject` has a credential of this kind that is in use.
    pub async fn has_credential(&self, kind: CredentialKind, subject: &str) -> Result<bool> {
        let subject = subject.to_string();
        self.journal
            .db()
            .call(move |connection| {
                connection.query_row(
                    "SELECT EXISTS (SELECT 1 FROM credentials WHERE kind = ?1 AND subject = ?2 AND revoked IS NULL)",
                    params![kind.as_str(), subject],
                    |row| row.get(0),
                )
            })
            .await
    }

    /// Has the credential `subject` holds now hold `scopes` from its
    /// next use, and no others. Whether it had one.
    pub async fn rescope(&self, kind: CredentialKind, subject: &str, scopes: &[String]) -> Result<bool> {
        let (subject, scopes) = (subject.to_string(), serde_json::to_string(scopes).unwrap_or_default());
        let changed = self
            .journal
            .db()
            .call(move |connection| {
                connection.execute(
                    "UPDATE credentials SET scopes = ?3 WHERE kind = ?1 AND subject = ?2 AND revoked IS NULL",
                    params![kind.as_str(), subject, scopes],
                )
            })
            .await?;
        // What goes on answering its bearer asks again what it holds.
        if changed > 0 {
            self.say_withdrawn();
        }
        Ok(changed > 0)
    }
}
