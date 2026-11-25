//! Invitations to register: how somebody other than the first person comes
//! to have an account here.
//!
//! One is a code, made by whoever manages the installation, for the email
//! it names where it names one. It works once, and for a while: when it
//! stops is read from the clock as it is used, so nothing looks for the
//! ones that have run out.

use kernel::{Error, Principal, Result};
use rusqlite::params;

use crate::identity::{Identity, normalise_email};

/// How long an invitation to register works unless whoever makes it says,
/// and the longest it may: somebody asked by letter takes days to answer,
/// and a code that works for ever is one that is found in a year.
pub const INVITED_FOR: std::time::Duration = std::time::Duration::from_secs(7 * 24 * 3600);
pub const INVITED_AT_MOST: std::time::Duration = std::time::Duration::from_secs(30 * 24 * 3600);

impl Identity {
    /// Makes an invitation code for someone to register with. Working with
    /// other people is what the paid tiers are for.
    pub async fn invite(&self, by: &Principal, email: Option<&str>) -> Result<String> {
        Ok(self.invite_for(by, email, None).await?.0)
    }

    /// The same, working for `lasts` from now ([`INVITED_FOR`] unless
    /// said, [`INVITED_AT_MOST`] at the longest): the code, and when it
    /// stops working. Read from the clock when the code is used; nothing
    /// looks for the ones that have run out.
    pub async fn invite_for(
        &self,
        by: &Principal,
        email: Option<&str>,
        lasts: Option<std::time::Duration>,
    ) -> Result<(String, i64)> {
        by.require("admin")?;
        if !self.tier().await?.allows_team() {
            return Err(Error::denied("inviting people needs the premium or enterprise tier"));
        }
        let lasts = lasts.unwrap_or(INVITED_FOR);
        if lasts.is_zero() || lasts > INVITED_AT_MOST {
            return Err(Error::invalid(
                "an invitation works for a week unless said, and for no more than thirty days: say how long as \
                 something like 2d or 12h",
            ));
        }
        let email = email.map(normalise_email).transpose()?;
        let code = format!("inv_{}", &kernel::id::secret()[..24]);
        let now = self.journal.now_ms();
        let expires = now + lasts.as_millis() as i64;
        let row = (code.clone(), email, by.id.clone(), now);
        self.journal
            .db()
            .call(move |connection| {
                connection.execute(
                    "INSERT INTO invites (code, email, created_by, created, expires) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![row.0, row.1, row.2, row.3, expires],
                )
            })
            .await?;
        Ok((code, expires))
    }

    pub(crate) async fn use_invite(&self, code: &str, email: &str) -> Result<()> {
        let (code, email) = (code.to_string(), email.to_string());
        let (now, usual) = (self.journal.now_ms(), INVITED_FOR.as_millis() as i64);
        let used = self
            .journal
            .db()
            .call(move |connection| {
                connection.execute(
                    "UPDATE invites SET used_by = ?2
                     WHERE code = ?1 AND used_by IS NULL AND (email IS NULL OR email = ?2)
                       AND COALESCE(expires, created + ?4) > ?3",
                    params![code, email, now, usual],
                )
            })
            .await?;
        if used == 1 {
            Ok(())
        } else {
            Err(Error::denied(
                "that invitation is not valid for this email, has been used, or has stopped working: one is good \
                 for a while and no longer. Ask whoever invited you for another",
            ))
        }
    }
}
