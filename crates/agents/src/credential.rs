//! An agent's credential: what it acts on the factory with, as itself.
//!
//! It is this factory's, made for the slot and holding what the slot
//! holds, and is kept in a file of the slot's own folder which only
//! whoever runs the daemon can read. Its program is pointed at the file
//! and reads it each time it asks the factory anything, so a credential
//! that is replaced is the one it asks with from then on.

use std::path::PathBuf;

use kernel::Result;
use studio::CredentialKind;

use crate::{Agents, Slot};

impl Agents {
    /// Makes sure the slot has a credential that works: the one in its
    /// file if that is still good, and a new one if not. Asked when a slot
    /// is set up and again each time it is about to work, because a slot
    /// can be here without one: a factory started from a copy of another
    /// has that factory's agents and none of its credentials.
    pub(crate) async fn credential(&self, slot: &Slot) -> Result<()> {
        if self.credential_is_current(slot).await {
            return Ok(());
        }
        let issued = self
            .inner
            .studio
            .identity
            .issue(CredentialKind::Agent, &slot.name, &slot.name, Some(slot.scopes.clone()))
            .await?;
        self.write_token(&slot.name, &issued)
    }

    /// Whether the token in the slot's file is still accepted and still
    /// grants exactly what the slot is meant to have.
    async fn credential_is_current(&self, slot: &Slot) -> bool {
        let Ok(token) = std::fs::read_to_string(self.token_path(&slot.name)) else { return false };
        match self.inner.studio.identity.authenticate(token.trim()).await {
            Ok(principal) => principal.actor() == slot.principal().actor() && principal.scopes == slot.scopes,
            Err(_) => false,
        }
    }

    pub(crate) fn token_path(&self, slot: &str) -> PathBuf {
        self.inner.config.home.join(slot).join("token")
    }

    /// The slot's credential, in a file only whoever runs the daemon can
    /// read from the moment it is there: written first and closed off
    /// afterwards, it was anybody's on the machine in between.
    fn write_token(&self, slot: &str, token: &str) -> Result<()> {
        let path = self.token_path(slot);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        private_token(&path, token)?;
        Ok(())
    }
}

/// Writes a credential to a file that is private as it is made. The folder
/// it is in is the slot's own, where its agent works when it has no
/// project, and is left as it is.
fn private_token(path: &std::path::Path, token: &str) -> std::io::Result<()> {
    use std::io::Write;
    let _ = std::fs::remove_file(path);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)?.write_all(token.as_bytes())
}
