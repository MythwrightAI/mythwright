//! Work that is done whole once it is begun, whoever was waiting for it.
//!
//! Whoever asks for something waits for it where it is done, and may go
//! away at any wait: a request is let go of when whoever made it closes,
//! a step's call is cut short. Work that is several things which only
//! make sense together (an instance's handlers told to take no more, a
//! text written as taken, the handlers started in it) was cut between
//! them by that, and left as nothing would have made it.
//!
//! So such work is the asker's to wait for and not the asker's to cut:
//! let go of once it has begun, it goes on as a task of the conductor's
//! own ([`Hands::spawn`]), holding what it held, and comes to its end.
//! One never begun is not begun for having been let go of.
//!
//! It came out of the runner, which offers it under the name it always
//! gave (`crate::whole` there), and hands it what begins a task.

use std::pin::Pin;
use std::task::{Context, Poll};

use kernel::{BoxFut, Chain};

use crate::Hands;

pub struct Whole<T: Send + 'static, H: Hands> {
    hands: H,
    /// What it is part of, which a task of its own would not be.
    chain: Option<Chain>,
    begun: bool,
    work: Option<BoxFut<'static, T>>,
}

impl<T: Send + 'static, H: Hands> Whole<T, H> {
    pub fn of(hands: &H, work: impl Future<Output = T> + Send + 'static) -> Whole<T, H> {
        let chain = kernel::chain::current();
        Whole { hands: hands.clone(), chain, begun: false, work: Some(Box::pin(work)) }
    }
}

impl<T: Send + 'static, H: Hands + Unpin> Future for Whole<T, H> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        self.begun = true;
        let Some(work) = self.work.as_mut() else { return Poll::Pending };
        let came = work.as_mut().poll(cx);
        if came.is_ready() {
            self.work = None;
        }
        came
    }
}

impl<T: Send + 'static, H: Hands> Drop for Whole<T, H> {
    fn drop(&mut self) {
        // Where nothing can run it any more, it is let go of with the rest.
        let runs = tokio::runtime::Handle::try_current().is_ok();
        if let (true, true, Some(work)) = (self.begun, runs, self.work.take()) {
            let chain = self.chain;
            self.hands.spawn(kernel::chain::within(chain, async move {
                work.await;
            }));
        }
    }
}
