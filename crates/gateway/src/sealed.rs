//! What a device and its hub say to each other, sealed. The handshake and
//! the frames are the `link` crate's ([`link::sealed`]), where the layout of
//! a frame and the rules for taking one are written.

pub use link::sealed::*;
