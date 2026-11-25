//! The sandbox tests a daemon has under way.
//!
//! A sandbox is a factory of its own, made of nothing of this one (the
//! rest of this crate). What is the daemon's is the keeping: it makes
//! one from what it knows as plain values (the text, a project's own
//! texts as they are read now, what each action and event here says of
//! itself) and the blueprints that ship, which a sandbox only reads; it
//! finds one again for whoever made it; and it ends it. The daemon hands a
//! sandbox nothing else, and takes nothing from one but what it answers.
//!
//! A sandbox lives in memory, for as long as somebody asks it anything
//! and no longer than a while unasked ([`UNASKED`]): each has a task
//! that waits that long on the daemon's clock, begun again by whatever
//! is asked of the sandbox, and ends it. There are a handful at most
//! ([`MOST`]). A daemon that stops ends them all, and nothing of one is
//! written where a later daemon would find it: its folder, under
//! [`FOLDER`] in the data folder, is removed as it ends, and the whole
//! of that folder as a daemon starts, since one that was killed in the
//! middle of a sandbox test leaves it. It is no part of a copy of the
//! factory, and nothing follows it.
//!
//! It came out of `api`, which offers it under the name it always had
//! (`api::sandboxes`): it asks nothing of the daemon but what it is
//! handed, the blueprints the packages bring among it as a `Library`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use crate::{Answers, Limits, Made, Sandbox};
use conductor::{Layers, Library, MemoryLibrary, Registry};
use kernel::{Clock, Error, Fields, Principal, Result};
use tokio::sync::Notify;
use tokio::task::AbortHandle;

/// The folder of the data folder that sandboxes' own folders are in.
pub const FOLDER: &str = "sandboxes";

/// How many sandboxes a daemon has at once.
pub const MOST: usize = 8;

/// How long a sandbox that nobody asks anything is kept.
pub const UNASKED: Duration = Duration::from_secs(15 * 60);

/// One sandbox, and whose it is.
pub struct Held {
    pub sandbox: Sandbox,
    /// Who made it: only they find it.
    maker: String,
    /// Told when something is asked of it: its while unasked begins again.
    asked: Notify,
}

struct Inner {
    folder: PathBuf,
    clock: Arc<dyn Clock>,
    /// The blueprints that ship here, each read without anything of
    /// this factory's: a checkout's own folder of them, where the program
    /// is run from one, and the standard ones, which are compiled in.
    checkout: Option<Arc<dyn Library>>,
    standard: Arc<dyn Library>,
    /// And what the installed packages bring, which is kept by the
    /// agents: read as texts as each sandbox is made, and never handed.
    brought: Arc<dyn Library>,
    held: Mutex<HashMap<String, (Arc<Held>, AbortHandle)>>,
    /// Kept while one is being made, so that no more than [`MOST`] are.
    making: tokio::sync::Mutex<()>,
}

/// The sandboxes of one daemon.
#[derive(Clone)]
pub struct Sandboxes {
    inner: Arc<Inner>,
}

/// What a sandbox is asked to be made from.
pub struct Asked {
    pub name: String,
    pub source: String,
    /// A project's own texts it may call, as they are now.
    pub own: Vec<(String, String)>,
    pub answers: Answers,
    pub told: Fields,
}

/// Takes away what sandboxes left under `data`: a daemon that was
/// killed in the middle of one leaves its folder.
pub fn clear(data: &Path) {
    let _ = std::fs::remove_dir_all(data.join(FOLDER));
}

/// Who a sandbox is its maker's by: the account, and the key it was
/// made with where it was made with one.
fn maker(who: &Principal) -> String {
    format!("{}/{}", who.actor(), who.key.as_deref().unwrap_or_default())
}

impl Sandboxes {
    /// The sandboxes of a daemon whose data is in `data`, with the
    /// blueprints a checkout has in its folder (`checkout`, where the
    /// program is run from one) and those the installed packages bring
    /// (`brought`).
    pub fn new(
        data: &Path,
        clock: Arc<dyn Clock>,
        checkout: Option<Arc<dyn Library>>,
        brought: Arc<dyn Library>,
    ) -> Sandboxes {
        let inner = Inner {
            folder: data.join(FOLDER),
            clock,
            checkout,
            standard: Arc::new(conductor::standard()),
            brought,
            held: Mutex::default(),
            making: tokio::sync::Mutex::default(),
        };
        Sandboxes { inner: Arc::new(inner) }
    }

    /// How many there are.
    pub fn count(&self) -> usize {
        self.inner.held.lock().expect("sandboxes lock").len()
    }

    /// Makes a sandbox for `who` from `asked`, on a device whose actions
    /// and events are those of `registry`: what each says of itself is
    /// handed over, and no action.
    pub async fn make(&self, who: &Principal, asked: Asked, registry: &Registry) -> Result<Arc<Held>> {
        let _one_at_a_time = self.inner.making.lock().await;
        if self.count() >= MOST {
            return Err(Error::unavailable(format!(
                "{MOST} sandbox tests are under way here, which is as many as there are at once: a sandbox is to \
                 be light. End one, or wait: one that nobody asks anything is ended after {} minutes",
                UNASKED.as_secs() / 60
            )));
        }
        let id = kernel::id::new("sandbox", self.inner.clock.now_ms());
        let made = Made {
            id: id.clone(),
            name: asked.name,
            source: asked.source,
            own: asked.own,
            ships: self.inner.ships(),
            actions: registry.all_specs(),
            events: registry.events().specs().to_vec(),
            knows_events: registry.knows_events(),
            folder: self.inner.folder.join(&id),
            now_ms: self.inner.clock.now_ms(),
            answers: asked.answers,
            told: asked.told,
            limits: Limits::default(),
        };
        let sandbox = Sandbox::make(made).await?;
        let held = Arc::new(Held { sandbox, maker: maker(who), asked: Notify::new() });
        let keeper = tokio::spawn(unasked(Arc::downgrade(&self.inner), held.clone())).abort_handle();
        self.inner.held.lock().expect("sandboxes lock").insert(id, (held.clone(), keeper));
        Ok(held)
    }

    /// The sandbox called `id`, for `who`, who made it: anybody else is
    /// answered as for one that is not there. Asking for it is asking it
    /// something: its while unasked begins again.
    pub fn find(&self, id: &str, who: &Principal) -> Result<Arc<Held>> {
        let held = self.inner.held.lock().expect("sandboxes lock").get(id).map(|(held, _)| held.clone());
        match held.filter(|held| held.maker == maker(who)) {
            Some(held) => {
                held.asked.notify_one();
                Ok(held)
            }
            None => Err(Error::not_found(format!(
                "there is no sandbox test `{id}`: it was ended, by whoever made it or for having been asked \
                 nothing for {} minutes, or the daemon was started since. Run it again",
                UNASKED.as_secs() / 60
            ))),
        }
    }

    /// Ends the sandbox called `id`, for `who`, who made it.
    pub async fn end(&self, id: &str, who: &Principal) -> Result<()> {
        self.find(id, who)?;
        self.inner.end(id).await;
        Ok(())
    }

    /// Ends every sandbox: for a daemon that is stopping.
    pub async fn end_all(&self) {
        let ids: Vec<String> = self.inner.held.lock().expect("sandboxes lock").keys().cloned().collect();
        for id in ids {
            self.inner.end(&id).await;
        }
    }

    /// Lets go of every sandbox without waiting for any to have ended:
    /// each ends by itself once nothing holds it, and takes its folder
    /// away. For a node that is let go of.
    pub fn let_go(&self) {
        for (_, (_, keeper)) in self.inner.held.lock().expect("sandboxes lock").drain() {
            keeper.abort();
        }
    }
}

impl Inner {
    /// The blueprints that ship, for one sandbox to read: in the order
    /// the daemon looks for one, with what the packages bring as it is
    /// read now. A package's blueprints and the files each links by a
    /// name written out are texts in memory from here; one linked by a
    /// name that is worked out as a step runs is not among them.
    fn ships(&self) -> Arc<dyn Library> {
        let brought = MemoryLibrary::new();
        for name in self.brought.names(None) {
            let Some(text) = self.brought.load(&name, None) else { continue };
            if let Some(read) = blueprint::check(&text, &blueprint::NoCatalog).blueprint {
                for link in blueprint::parts::linked(&read) {
                    let file = blueprint::parts::linked_as(&name, link.from);
                    if let Some(words) = self.brought.load(&file, None) {
                        brought.put(&file, &words);
                    }
                }
            }
            brought.put(&name, &text);
        }
        brought.ship();
        let layers = match &self.checkout {
            Some(checkout) => Layers::new().then(checkout.clone()),
            None => Layers::new(),
        };
        Arc::new(layers.then(Arc::new(brought)).then(self.standard.clone()))
    }

    async fn end(&self, id: &str) {
        let held = self.held.lock().expect("sandboxes lock").remove(id);
        if let Some((held, keeper)) = held {
            keeper.abort();
            held.sandbox.end().await;
            // A sandbox takes its own folder away as it ends; one whose
            // thread ended some other way did not.
            let _ = std::fs::remove_dir_all(self.folder.join(id));
            self.tidy();
        }
    }

    /// Takes the folder sandboxes' own folders are in away, where none
    /// is left in it: a factory with no sandbox has no such folder.
    fn tidy(&self) {
        let _ = std::fs::remove_dir(&self.folder);
    }
}

/// Ends `held` once nothing was asked of it for [`UNASKED`].
async fn unasked(sandboxes: Weak<Inner>, held: Arc<Held>) {
    let Some(clock) = sandboxes.upgrade().map(|inner| inner.clock.clone()) else { return };
    loop {
        // Asked first: one that was asked something in the moment its
        // while ran out was asked, and is kept.
        tokio::select! {
            biased;
            () = held.asked.notified() => {}
            () = clock.sleep(UNASKED) => break,
        }
    }
    let Some(inner) = sandboxes.upgrade() else { return };
    // Taken out here and ended here: `end` would abort this very task.
    let was = inner.held.lock().expect("sandboxes lock").remove(held.sandbox.id());
    if was.is_some() {
        held.sandbox.end().await;
        inner.tidy();
    }
}
