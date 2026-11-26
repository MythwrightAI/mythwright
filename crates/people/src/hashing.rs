//! What is kept of a password in place of it, and how hard that is made
//! to guess from.

use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use argon2::{Algorithm, Argon2, Params, Version};
use kernel::{Error, Result};

/// How hard what is kept of a password is made to guess from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Hashing {
    /// As hard as the hash's own defaults make it, which is slow on
    /// purpose: every factory that is run.
    #[default]
    Hard,
    /// With the least the hash takes, for a factory that is its own test,
    /// whose passwords are written in the test and stand against nobody.
    /// A test's factory registers its owner, and the API's suite makes
    /// some fifteen hundred: hashed as hard, that was a twentieth of what
    /// the suite asked of the processors (2026-11-26, docs/build-times.md).
    /// What is kept says how it was hashed, so either signs in either way.
    Least,
}

/// Hashing is deliberately slow, so it is kept off the async threads.
pub(crate) async fn hash_password(password: String, hashing: Hashing) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        let hasher = match hashing {
            Hashing::Hard => Argon2::default(),
            Hashing::Least => {
                let least = Params::new(Params::MIN_M_COST, Params::MIN_T_COST, Params::MIN_P_COST, None)
                    .map_err(|err| Error::internal(format!("could not hash the password: {err}")))?;
                Argon2::new(Algorithm::default(), Version::default(), least)
            }
        };
        hasher
            .hash_password(password.as_bytes())
            .map(|hash| hash.to_string())
            .map_err(|err| Error::internal(format!("could not hash the password: {err}")))
    })
    .await
    .map_err(|err| Error::internal(err.to_string()))?
}

pub(crate) async fn verify_password(password: String, hash: String) -> bool {
    tokio::task::spawn_blocking(move || {
        Argon2::default().verify_password(password.as_bytes(), hash.as_str()).is_ok()
    })
    .await
    .unwrap_or(false)
}
