//! An engine held weakly, by what the engine holds in turn.

use std::sync::Arc;

use crate::engine::{Engine, Inner};

/// An engine held by something the engine holds in turn, which would
/// keep it for good: an engine again only while it is there.
#[derive(Clone)]
pub struct Downgraded(std::sync::Weak<Inner>);

impl Engine {
    pub fn downgrade(&self) -> Downgraded {
        Downgraded(Arc::downgrade(&self.inner))
    }
}

impl Downgraded {
    pub fn upgrade(&self) -> Option<Engine> {
        self.0.upgrade().map(|inner| Engine { inner })
    }
}
