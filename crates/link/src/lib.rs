//! What a link between two installations is made of: each one's secret and
//! the keys made from it ([`own`]), and the handshake and the sealed frames
//! that follow it ([`sealed`]).
//!
//! Nothing here knows who the two are to each other, what they say, or what
//! carries it. That is for whoever makes the link: a device and its hub
//! today. It stands on nothing else of the application's, so that anything
//! that has to prove a key to an installation, or hold one to its own, is
//! built on the same handshake and never on a second writing of it.

pub mod kept;
pub mod own;
pub mod sealed;

use std::fmt;

/// Why something of an installation's own could not be had: a sentence for
/// whoever runs it, with what to do where there is something to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Error {
        Error { message: message.into() }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
