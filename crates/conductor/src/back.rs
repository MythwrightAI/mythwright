//! Putting a tree back on the texts it had.
//!
//! Somebody who applied saying to take what ships as it now is
//! ([`crate::pinned`]) may find the new texts are not what they wanted.
//! What the tree went on with before was kept beside what it was moved to,
//! and the texts themselves are kept by revision, so it can be put back
//! without the program it had.
//!
//! - **It is said first what would be put back** ([`Engine::put_back`]
//!   asked only to say): each text by the version it is on and the one it
//!   would go back to, and what moved between them.
//! - **It is refused where what is running would not fit them.** A text
//!   of the instance's own that was written for the new ones (a project's
//!   file changed along with the move) is checked against the old ones
//!   as any apply is, and what does not fit is said; nothing is changed.
//!   An instance whose own text ships moved with the rest, and goes back
//!   with them.
//! - **Putting back is an apply**, by whoever asks, on the texts it had:
//!   the instance comes to its text by the one rule ([`crate::update`]),
//!   and what is below it is brought to what it then reaches. What it was
//!   on is kept in turn, so putting back twice is where it started.

use std::collections::BTreeMap;

use kernel::{Error, Principal, Result};

use crate::asked::{Applied, Apply};
use crate::engine::Engine;
use crate::pinned::{Before, Moved, Pins, Reached};

impl Engine {
    /// Puts an instance that is running back, as `by`, applied as it is:
    /// the text there for its blueprint (or the one it had, where its own
    /// ships), with the params, key and place it was started with.
    pub async fn put_back_instance(&self, id: &str, by: Principal, say: bool) -> Result<Applied> {
        let info = self.instance(id).await?;
        let request = Apply {
            blueprint: info.blueprint,
            source: None,
            params: info.params,
            key: Some(info.key),
            parent: info.parent,
            target: info.target,
            project: info.project,
            principal: by,
            force: false,
            take: false,
            labels: info.labels,
            environment: (!info.environment.is_empty()).then_some(info.environment),
        };
        self.put_back(request, say).await
    }

    /// Applies `request` over the instance it names, on the texts that
    /// instance reached before it was last moved; or, asked only to `say`,
    /// gives what that would put back and changes nothing. Refused, saying
    /// why, where nothing was moved or where the text does not fit them.
    pub async fn put_back(&self, request: Apply, say: bool) -> Result<Applied> {
        request.principal.require("blueprints:run")?;
        self.not_while_held(&request.blueprint)?;
        // By the conductor that runs what was applied for the project.
        let conductor = self.of(request.project.as_deref()).await?;
        if !std::sync::Arc::ptr_eq(&conductor.inner, &self.inner) {
            return Box::pin(conductor.put_back(request, say)).await;
        }
        if self.follows() {
            return Err(Error::conflict(
                "texts are followed here: what is running takes each as it changes, and is held to none that \
                 it could be put back on",
            ));
        }
        let project = request.project.clone();
        let library = &self.inner.library;
        let there = request.source.clone().or_else(|| library.load(&request.blueprint, project.as_deref()));
        let named = || {
            let read = |text: &String| blueprint::check(text, &blueprint::NoCatalog).blueprint;
            there.as_ref().and_then(read).map(|blueprint| blueprint.name)
        };
        let key = request.key.clone().or_else(named).unwrap_or_else(|| request.blueprint.clone());
        let store = &self.inner.store;
        let Some(info) =
            store.live_instance_by_key(request.parent.as_deref(), &key, project.as_deref()).await?
        else {
            return Err(Error::conflict(format!("`{key}` is not running, so there is nothing to put back")));
        };
        let nothing = || {
            Error::conflict(format!(
                "`{key}` has not been moved to other texts since it was applied (`--take`), so there is \
                 nothing to put it back on"
            ))
        };
        let pins = self.pins(&info.id).await.ok_or_else(nothing)?;
        let before = pins.before.clone().ok_or_else(nothing)?;
        let gone = |what: &str| Error::conflict(format!("{what} `{key}` had is no longer kept"));
        // The text it runs once it is back: the one handed, the one it
        // had where its own ships and moved too, or the one there.
        let ships = request.source.is_none() && self.holds(&request.blueprint, project.as_deref());
        let source = match (&before.own, ships) {
            (Some(revision), true) => self.text_of(revision).await.ok_or_else(|| gone("the text"))?,
            _ => there.ok_or_else(|| {
                Error::not_found(format!("there is no blueprint called `{}`", request.blueprint))
            })?,
        };
        let mut over = BTreeMap::new();
        let mut back = Vec::new();
        for (name, revision) in &before.texts {
            let was = self.text_of(revision).await.ok_or_else(|| gone("a text"))?;
            // Said the way a move is: from the one it is on to the one it goes back to.
            match pins.texts.get(name) {
                Some(on) if on == revision => {}
                Some(on) => back.push(Moved::between(name, self.text_of(on).await.as_deref(), &was)),
                // One it does not reach now: nothing is on it, and nothing moves.
                None => {}
            }
            over.insert(name.clone(), was);
        }
        let (checked, _) = self.check_reaching(&source, project.as_deref(), &over);
        if let Err(wrong) = checked.into_result() {
            let file = self.file_of(&info.blueprint, project.as_deref());
            let said = crate::reach::unfit(&info.blueprint, &file, &wrong, true);
            return Err(Error::invalid(format!(
                "`{key}` cannot be put back on the texts it had, because this text does not fit them. \
                 Nothing was changed.\n{}",
                said.message
            )));
        }
        let reached = Reached { taken: back, left: Vec::new() };
        if say {
            return Ok(Applied {
                instance: info.id,
                run: None,
                created: false,
                changed: false,
                took: None,
                vouched: Default::default(),
                reached,
            });
        }
        // What it is on now is what it can be put back on in turn.
        let own = match before.own {
            Some(_) => {
                self.keep_text(&info.revision, &info.source).await;
                Some(info.revision.clone())
            }
            None => None,
        };
        let on = Before { at: pins.at, texts: pins.texts.clone(), own };
        let put = Pins { at: self.inner.own.now_ms(), texts: before.texts, before: Some(on) };
        self.put_pins(&info.id, Some(put)).await;
        let request = Apply { source: Some(source), key: Some(key), take: false, ..request };
        match self.apply(request).await {
            Ok(applied) => Ok(Applied { reached, ..applied }),
            Err(refused) => {
                // Refused, it is on what it was on.
                self.put_pins(&info.id, Some(pins)).await;
                Err(refused)
            }
        }
    }
}
