//! Cooperative cancellation.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::Notify;

/// A flag that can be raised once and awaited by many.
///
/// Cancelling a parent cancels every child made from it; cancelling a child
/// leaves the parent alone. A step's token is a child of its run's, which is
/// a child of its instance's.
#[derive(Clone, Default)]
pub struct Cancel {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    cancelled: AtomicBool,
    notify: Notify,
    parent: Option<Cancel>,
}

impl Cancel {
    pub fn new() -> Self {
        Self::default()
    }

    /// A token that is cancelled when either it or `self` is.
    pub fn child(&self) -> Cancel {
        Cancel { inner: Arc::new(Inner { parent: Some(self.clone()), ..Inner::default() }) }
    }

    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::SeqCst);
        self.inner.notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
            || self.inner.parent.as_ref().is_some_and(Cancel::is_cancelled)
    }

    /// Completes once the token, or any ancestor, is cancelled.
    pub async fn cancelled(&self) {
        let mut chain = Vec::new();
        let mut next = Some(self);
        while let Some(token) = next {
            chain.push(token);
            next = token.inner.parent.as_ref();
        }
        loop {
            // Register interest before checking, so a cancel that lands
            // between the check and the await is not missed.
            let waits: Vec<_> = chain.iter().map(|token| token.inner.notify.notified()).collect();
            if self.is_cancelled() {
                return;
            }
            let mut waits: Vec<_> = waits.into_iter().map(Box::pin).collect();
            std::future::poll_fn(|cx| {
                for wait in &mut waits {
                    if wait.as_mut().poll(cx).is_ready() {
                        return std::task::Poll::Ready(());
                    }
                }
                std::task::Poll::Pending
            })
            .await;
        }
    }
}
