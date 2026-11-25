//! What an installation keeps secret, sealed where it is kept.
//!
//! A value the factory holds for a program (a key of a payment service,
//! the sign-in of an agent's program) is in a file. Readable by its owner
//! alone, that file is still the value to whatever copies it: a backup of
//! the folder, a disk image, a file sent by mistake. Sealed, it is not. It
//! is opened only with a key made from this installation's secret for this
//! one purpose ([`Own::at_rest`]), and that secret is in no copy of the
//! factory and never beside what it seals in anything that is handed on.
//!
//! What this does not do is keep a value from whoever can read the whole
//! data folder as the daemon's user can: the secret the key is made from is
//! there too. That is the daemon's own position, and nothing kept on the
//! machine it runs on can be out of its reach.
//!
//! The cipher is XChaCha20-Poly1305, from the crate the links' handshake
//! already uses; nothing here is a cipher of its own. Each sealing has a
//! nonce of its own from the system's random source, long enough that two
//! never meet. What a value is sealed *as* (its name) is sealed with it, so
//! a file put under another name does not open: one secret cannot be made
//! to stand in for another by moving files about.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};

use crate::own::{KEY, Own};
use crate::{Error, Result};

/// What the key for sealing at rest is made for. Part of the key.
pub(crate) const AT_REST: &str =
    "what this installation keeps secret, sealed where it is kept (XChaCha20-Poly1305)";

/// How a sealed file begins, so that one is told from a value written as
/// it is by a build that did not seal.
const MARK: &[u8] = b"sealed 1\n";
const NONCE: usize = 24;

/// The key this installation seals what it keeps with.
pub struct Seal {
    key: [u8; KEY],
}

impl Drop for Seal {
    fn drop(&mut self) {
        self.key.fill(0);
        std::hint::black_box(&self.key);
    }
}

impl Clone for Seal {
    fn clone(&self) -> Seal {
        Seal { key: self.key }
    }
}

impl std::fmt::Debug for Seal {
    fn fmt(&self, to: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        to.write_str("Seal")
    }
}

impl Own {
    /// The key what this installation keeps secret is sealed with.
    pub fn at_rest(&self) -> Seal {
        Seal { key: self.derive(AT_REST) }
    }
}

impl Seal {
    /// Whether `bytes` is something sealed, by any installation.
    pub fn is_sealed(bytes: &[u8]) -> bool {
        bytes.starts_with(MARK)
    }

    /// Seals `plain` as what is kept under `name`.
    pub fn seal(&self, name: &str, plain: &[u8]) -> Result<Vec<u8>> {
        let mut nonce = [0u8; NONCE];
        getrandom::fill(&mut nonce)
            .map_err(|err| Error::new(format!("the system gave nothing random to seal with: {err}")))?;
        let cipher = XChaCha20Poly1305::new((&self.key).into());
        let sealed = cipher
            .encrypt(XNonce::from_slice(&nonce), Payload { msg: plain, aad: name.as_bytes() })
            .map_err(|_| Error::new("what was to be kept could not be sealed"))?;
        let mut out = Vec::with_capacity(MARK.len() + NONCE + sealed.len());
        out.extend_from_slice(MARK);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&sealed);
        Ok(out)
    }

    /// Opens what was sealed as `name` by this installation. An error for
    /// what was sealed by another, sealed as something else, or changed
    /// since: which of those it was cannot be told, and is not guessed at.
    pub fn open(&self, name: &str, sealed: &[u8]) -> Result<Vec<u8>> {
        let wrong = || {
            Error::new(format!(
                "what is kept as `{name}` does not open: it was sealed by another installation, or under another \
                 name, or has been changed since. Set it again"
            ))
        };
        let rest = sealed.strip_prefix(MARK).filter(|rest| rest.len() > NONCE).ok_or_else(wrong)?;
        let (nonce, body) = rest.split_at(NONCE);
        let cipher = XChaCha20Poly1305::new((&self.key).into());
        cipher
            .decrypt(XNonce::from_slice(nonce), Payload { msg: body, aad: name.as_bytes() })
            .map_err(|_| wrong())
    }
}
