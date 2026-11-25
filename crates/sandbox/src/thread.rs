//! A sandbox as whoever holds one has it: a thread of its own, asked
//! across a channel.
//!
//! The factory inside a sandbox lives on one thread, made for it, with a
//! runtime nothing else runs on. Whoever holds the sandbox holds only the
//! sending end of a channel: an act, a reading and the ending are each
//! asked across it and answered across it, as plain values. So nothing
//! a sandbox runs is a task of the daemon's, a sandbox that goes round for
//! ever takes its own thread's time and nobody else's, and a sandbox is
//! over when its thread is: every task it had is dropped with the
//! runtime, its databases are closed and their threads ended, and its
//! folder is taken away. Letting go of the sandbox without ending it comes
//! to the same, since the channel closes.
//!
//! The runtime does no input or output of its own: it has a timer and
//! nothing else, so nothing on it can open a socket or wait on a
//! program, whatever code came to ask.

use std::path::PathBuf;
use std::sync::Arc;

use kernel::{Error, Result};
use serde_json::Value;
use tokio::sync::{Notify, mpsc, oneshot};

use crate::at::At;
use crate::inside::Inside;
use crate::rest::Parked;
use crate::{Act, Made};

enum Ask {
    Act(Act, oneshot::Sender<Result<Value>>),
    Read(oneshot::Sender<Result<Value>>),
    Stands(oneshot::Sender<Value>),
    Offers(oneshot::Sender<Value>),
    End(oneshot::Sender<()>),
}

/// What a sandbox's thread says it is at, as it is made and after:
/// read with [`Seen::said`].
#[derive(Clone, Default)]
pub struct Seen(At);

impl Seen {
    /// What the thread is at and for how long, and how each of its
    /// databases stands, in words.
    pub fn said(&self) -> String {
        self.0.said()
    }
}

/// A sandbox under way.
pub struct Sandbox {
    id: String,
    asks: mpsc::UnboundedSender<Ask>,
    /// Told when the sandbox is to end, or is let go of: whatever it is
    /// in the middle of is let go of there, and it ends.
    ending: Arc<Notify>,
    /// What its thread says it is at.
    at: At,
}

/// A sandbox that nothing holds any more ends, whatever it was doing.
impl Drop for Sandbox {
    fn drop(&mut self) {
        self.ending.notify_one();
    }
}

fn gone() -> Error {
    Error::not_found("this sandbox has ended: make another to try the blueprint again")
}

/// Takes a sandbox's folder away, whatever is in it.
fn clear(folder: &PathBuf) {
    let _ = std::fs::remove_dir_all(folder);
}

impl Sandbox {
    /// Makes a sandbox: a thread, and on it the factory `made` describes.
    /// Answered once that factory is there and at rest, with nothing
    /// applied yet. One that cannot be made leaves nothing behind.
    pub async fn make(made: Made) -> Result<Sandbox> {
        Sandbox::make_seen(made, Seen::default()).await
    }

    /// Makes a sandbox as [`Sandbox::make`] does, saying as it goes what it
    /// is at in `seen`: for whoever gives up waiting for it to be made.
    pub async fn make_seen(made: Made, seen: Seen) -> Result<Sandbox> {
        let (id, folder) = (made.id.clone(), made.folder.clone());
        let (asks, mut asked) = mpsc::unbounded_channel::<Ask>();
        let ending = Arc::new(Notify::new());
        let told_to_end = ending.clone();
        let (ready, made_ready) = oneshot::channel::<Result<()>>();
        // A thread's name is fifteen letters at most where the system
        // keeps it: the last of the id, which are the ones thrown for.
        let last: String = id.chars().rev().take(7).collect::<Vec<_>>().into_iter().rev().collect();
        let name = format!("sandbox-{last}");
        let cleared = folder.clone();
        let at = seen.0;
        let says_at = at.clone();
        let started = std::thread::Builder::new().name(name).spawn(move || {
            // A timer and nothing else: see the head of this file. One
            // thread does what is asked off this one, in the order it is
            // asked, and the runtime says each time it has nothing left
            // to run: what says when a sandbox is at rest (`rest`).
            let parked = Parked::counting(says_at.parked.clone());
            let says = parked.clone();
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .max_blocking_threads(1)
                .on_thread_park(move || says.now())
                .build();
            let runtime = match runtime {
                Ok(runtime) => runtime,
                Err(err) => {
                    let _ = ready
                        .send(Err(Error::internal(format!("a sandbox's thread could not start: {err}"))));
                    return;
                }
            };
            let ended = runtime.block_on(async move {
                says_at.now("making its factory");
                let mut inside = match Inside::make(made, parked, says_at.clone()).await {
                    Ok(inside) => inside,
                    Err(err) => {
                        let _ = ready.send(Err(err));
                        return None;
                    }
                };
                inside.settle().await;
                let _ = ready.send(Ok(()));
                let mut ended = None;
                loop {
                    says_at.now("waiting to be asked something");
                    let Some(ask) = asked.recv().await else { break };
                    match ask {
                        Ask::Act(act, answer) => {
                            says_at.now(format!("doing an act: {act:?}"));
                            // An act is over when what came of it is at
                            // rest, which a text that goes round for ever
                            // without coming to its limits never is:
                            // ending the sandbox ends the act with it.
                            let did = tokio::select! {
                                biased;
                                did = inside.act(act) => did,
                                () = told_to_end.notified() => {
                                    let _ = answer.send(Err(gone()));
                                    break;
                                }
                            };
                            let did = did.map(|did| {
                                let mut said = inside.standing();
                                said["did"] = did;
                                said
                            });
                            let _ = answer.send(did);
                        }
                        Ask::Read(answer) => {
                            says_at.now("reading everything that came of it");
                            let _ = answer.send(inside.shown().await);
                        }
                        Ask::Stands(answer) => {
                            let _ = answer.send(inside.standing());
                        }
                        Ask::Offers(answer) => {
                            let _ = answer.send(inside.offers());
                        }
                        Ask::End(answer) => {
                            ended = Some(answer);
                            break;
                        }
                    }
                }
                // Whoever asked for the ending is answered when it is over.
                while let Ok(ask) = asked.try_recv() {
                    if let Ask::End(answer) = ask {
                        ended = Some(answer);
                    }
                }
                inside.end().await;
                ended
            });
            // Every task the sandbox had goes with its runtime, and what
            // each held with it.
            drop(runtime);
            clear(&folder);
            if let Some(ended) = ended {
                let _ = ended.send(());
            }
        });
        if let Err(err) = started {
            clear(&cleared);
            return Err(Error::unavailable(format!("no thread could be started for a sandbox: {err}")));
        }
        match made_ready.await {
            Ok(Ok(())) => Ok(Sandbox { id, asks, ending, at }),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(Error::internal("a sandbox's thread ended before its factory was made")),
        }
    }

    /// What it is found by.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// What its thread is at, and how each of its databases stands, in
    /// words: for whoever waited for an answer and gave up, to say what
    /// the sandbox was in the middle of.
    pub fn at(&self) -> String {
        self.at.said()
    }

    /// Does `act` and waits until what comes of it is at rest: what the
    /// act came to, and how the sandbox then stands.
    pub async fn act(&self, act: Act) -> Result<Value> {
        let (answer, answered) = oneshot::channel();
        self.asks.send(Ask::Act(act, answer)).map_err(|_| gone())?;
        answered.await.map_err(|_| gone())?
    }

    /// Everything that came of the sandbox so far.
    pub async fn read(&self) -> Result<Value> {
        let (answer, answered) = oneshot::channel();
        self.asks.send(Ask::Read(answer)).map_err(|_| gone())?;
        answered.await.map_err(|_| gone())?
    }

    /// How the sandbox stands, in short.
    pub async fn stands(&self) -> Result<Value> {
        let (answer, answered) = oneshot::channel();
        self.asks.send(Ask::Stands(answer)).map_err(|_| gone())?;
        answered.await.map_err(|_| gone())
    }

    /// Every action the sandbox has, each as it says itself there and
    /// with which kind it is ([`crate::Kind`]).
    pub async fn offers(&self) -> Result<Value> {
        let (answer, answered) = oneshot::channel();
        self.asks.send(Ask::Offers(answer)).map_err(|_| gone())?;
        answered.await.map_err(|_| gone())
    }

    /// Ends the sandbox: answered when its thread has let go of everything
    /// and its folder is gone.
    pub async fn end(&self) {
        let (answer, answered) = oneshot::channel();
        // Asked first, so that it is there to be answered when what the
        // sandbox is in the middle of has been let go of.
        let asked = self.asks.send(Ask::End(answer)).is_ok();
        self.ending.notify_one();
        if asked {
            let _ = answered.await;
        }
    }
}
