//! How a running tree comes by new texts: what the runner does about it.
//!
//! The setting itself, what its two parts mean and what `follow` gives
//! up are `texts::taking`, which keeps it. What is here is what is done
//! to what is running: brought to the setting when it changes
//! ([`Engine::set_taking`]: followed, everything takes the texts as they
//! now are; held, every tree is held to what it reaches now), and brought
//! to a followed text when it changes ([`Engine::follow`]), a project's
//! own as whoever turned `follow` on vouched for it.
//!
//! A text that does not check is not taken. What runs it goes on with the
//! last that did, and says so once, beside itself and as an event
//! (`instance.text_left`), until the text checks or is what it runs.

use std::collections::{BTreeMap, HashSet};
use std::sync::Mutex;

use kernel::{Principal, PrincipalKind, Result, Source};
use runs::{InstanceFilter, InstanceInfo};
use serde::Serialize;
use serde_json::json;
pub use texts::taking::{Come, GIVES_UP, Land, NAMESPACE, Take, Taking};

use crate::current::Takes;
use crate::engine::Engine;

/// A text that changed and was not taken by something that follows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NotTaken {
    pub instance: String,
    pub key: String,
    pub blueprint: String,
    pub project: Option<String>,
    /// What is wrong with the text.
    pub error: String,
    /// The revision it goes on with, and the one it did not take.
    pub kept: String,
    pub left: String,
}

/// Which texts have been said not to be taken: the instance, and the
/// revision it was told of. One for every conductor of the factory.
#[derive(Default)]
pub(crate) struct Kept {
    unsaid: Mutex<HashSet<(String, String)>>,
}

/// What an instance says beside itself of a text it did not take begins so.
fn begins(name: &str) -> String {
    format!("`{name}` was changed and does not check")
}

impl Engine {
    /// The setting as it stands.
    pub fn taking(&self) -> Taking {
        self.inner.texts.taking()
    }

    /// Whether texts are followed here because somebody said so: a
    /// project's own among them.
    pub fn follows(&self) -> bool {
        self.inner.texts.follows()
    }

    /// Whether anything is followed here: said so, or nothing said, where
    /// what is held to nothing (a checkout's blueprints) is followed.
    pub fn brings(&self) -> bool {
        self.inner.texts.brings()
    }

    /// Whether a text that is taken has its instance's steps run again.
    pub(crate) fn lands_again(&self) -> bool {
        self.inner.texts.lands_again()
    }

    /// Whether the text `load` gives for `name` is one a running tree goes
    /// on with as it reached it ([`crate::pinned`]). Never a project's own,
    /// which is held by who vouched for it.
    pub(crate) fn holds(&self, name: &str, project: Option<&str>) -> bool {
        self.inner.texts.holds(name, project)
    }

    /// Changes the setting, as `by`: the factory's owner's to do, and
    /// having texts followed takes `shell` as well. What is running is
    /// brought to what was said: followed, everything takes the texts as
    /// they now are; held, every tree is held to what it reaches now.
    /// Given back with any text that was not taken because it does not
    /// check.
    pub async fn set_taking(&self, asked: Take, by: &Principal) -> Result<(Taking, Vec<NotTaken>)> {
        let (was, now) = self.inner.texts.set_taking(asked, by).await?;
        let (come, land) = (now.come, now.land);
        let word = |come: Option<Come>| match come {
            Some(Come::Held) => "held",
            Some(Come::Follow) => "follow",
            None => "usual",
        };
        let gives_up = (come == Some(Come::Follow) && was.come != come).then_some(GIVES_UP);
        let said = json!({
            "come": word(come), "land": land, "was": word(was.come), "landed": was.land,
            "by": by.actor(), "by_name": by.name, "gives_up": gives_up,
        });
        self.emit("system.texts", Source::default(), said).await;
        let mut left = Vec::new();
        if come != was.come {
            match come {
                // Everything there is, as it stands, vouched for by whoever said so.
                Some(Come::Follow) => left = self.follow(None, None).await,
                // From now on a tree goes on with what it reaches now,
                // in whichever conductor it is.
                _ => {
                    for conductor in self.every() {
                        conductor.hold_now().await;
                    }
                }
            }
        }
        Ok((now, left))
    }

    /// Holds every tree to the texts it reaches as they now are: what was
    /// kept of what each reached before is from another way of coming by
    /// texts, and is put away.
    async fn hold_now(&self) {
        let roots =
            InstanceFilter { live_only: true, roots_only: true, limit: 100_000, ..Default::default() };
        for root in self.inner.store.instances(roots).await.unwrap_or_default() {
            let project = root.project.as_deref();
            let (_, reached) = self.check_reaching(&root.source, project, &BTreeMap::new());
            self.put_pins(&root.id, None).await;
            self.pin(&root.id, project, &reached, None).await;
        }
    }

    /// Where texts are followed: has everything running that is behind a
    /// text take it, the project's own as whoever turned `follow` on
    /// vouches for them; with nothing said, only what is held to nothing
    /// and is nobody's own. `project` and `name` say what changed, where that
    /// is known; with neither, everything is looked at. Given back is each
    /// text that was not taken because it does not check, and has not
    /// been said yet.
    pub async fn follow(&self, project: Option<&str>, name: Option<&str>) -> Vec<NotTaken> {
        // A project's, by the conductor that runs it; with none named,
        // by every conductor for what it runs.
        let conductors = match project {
            Some(named) => self.of(Some(named)).await.map_or_else(|_| vec![self.clone()], |one| vec![one]),
            None => self.every(),
        };
        let mut left = Vec::new();
        for conductor in conductors {
            left.extend(conductor.follow_own(project, name).await);
        }
        left
    }

    /// `project` keeps its own blueprints in another folder now, or none
    /// (`library::folders`), and `were` are the names its own were read
    /// by before. Where texts are followed it is as though each file
    /// there was taken away and each there now saved: what was vouched
    /// for under those names is what is there now, or nothing, as
    /// whoever turned following on, and what is running is brought to it
    /// ([`Engine::follow`]). Where they are not, nothing is taken.
    pub async fn follow_folder(&self, project: &str, were: &[String]) -> Vec<NotTaken> {
        let setting = self.taking();
        if setting.come != Some(Come::Follow) {
            return Vec::new();
        }
        let owner = followed_by(&setting);
        for name in were {
            self.vouch_saved(project, name, &owner).await;
        }
        self.follow(Some(project), None).await
    }

    /// [`Engine::follow`], for what this conductor runs.
    async fn follow_own(&self, project: Option<&str>, name: Option<&str>) -> Vec<NotTaken> {
        let setting = self.taking();
        if setting.come == Some(Come::Held) {
            return Vec::new();
        }
        let said = setting.come == Some(Come::Follow);
        let filter = InstanceFilter {
            live_only: true,
            project: project.map(str::to_string),
            limit: 100_000,
            ..Default::default()
        };
        let live = self.inner.store.instances(filter).await.unwrap_or_default();
        // A project's own files are taken as vouched for by whoever said
        // texts are followed here, holding what they held then.
        let owner = followed_by(&setting);
        let mut projects: Vec<&str> = live.iter().filter_map(|one| one.project.as_deref()).collect();
        projects.sort_unstable();
        projects.dedup();
        // Only where somebody said so: with nothing said, a project's own
        // go on as they were vouched for.
        for project in projects.into_iter().filter(|_| said) {
            let own = self.inner.library.own(Some(project));
            for (own_name, _) in own.iter().filter(|(own_name, _)| name.is_none_or(|name| name == own_name)) {
                self.vouch_saved(project, own_name, &owner).await;
            }
        }
        let mut left = Vec::new();
        for instance in live {
            if name.is_some_and(|name| name != instance.blueprint) {
                continue;
            }
            // With nothing said, what is followed is what is held to
            // nothing and is nobody's own: a checkout's blueprints.
            let (its, project) = (instance.blueprint.as_str(), instance.project.as_deref());
            if !said && (self.inner.library.is_own(its, project) || self.holds(its, project)) {
                continue;
            }
            if let Some(not) = self.follow_one(instance).await {
                left.push(not);
            }
        }
        left
    }

    /// Brings one instance to the text now there for it, as itself: what
    /// it runs with is whoever applied it, as before.
    async fn follow_one(&self, instance: InstanceInfo) -> Option<NotTaken> {
        let (project, name) = (instance.project.as_deref(), instance.blueprint.as_str());
        // The text as it is where it is kept, and the one this instance may
        // be given: a project's own is given as it was vouched for, and one
        // that does not check is vouched for by nobody.
        let file = self.inner.library.load(name, project)?;
        let unfit = self.shown_behind(&instance, &file).filter(|behind| behind.takes == Takes::Unfit);
        let Some(behind) = unfit else {
            self.taken_now(&instance).await;
            let by = instance.principal.clone();
            self.bring_to(instance, crate::current::Look::Followed, &by).await;
            return None;
        };
        let key = (instance.id.clone(), behind.revision.clone());
        if !self.inner.taking.unsaid.lock().expect("taking lock").insert(key) {
            return None;
        }
        let error = behind.why.clone();
        let note = format!("{}, so this goes on with the last text that did.\n{error}", begins(name));
        let now = self.inner.own.now_ms();
        let _ = self.inner.store.set_instance_note(&instance.id, Some(note), begins(name), now).await;
        let said =
            json!({ "blueprint": name, "error": error, "kept": instance.revision, "left": behind.revision });
        self.emit("instance.text_left", Source::instance(&instance.id), said).await;
        Some(NotTaken {
            instance: instance.id.clone(),
            key: instance.key.clone(),
            blueprint: instance.blueprint.clone(),
            project: instance.project.clone(),
            error,
            kept: instance.revision.clone(),
            left: behind.revision,
        })
    }

    /// Takes back what an instance said of a text it did not take: the
    /// text there now is one it runs, or can.
    async fn taken_now(&self, instance: &InstanceInfo) {
        let had = {
            let mut unsaid = self.inner.taking.unsaid.lock().expect("taking lock");
            let before = unsaid.len();
            unsaid.retain(|(id, _)| id != &instance.id);
            unsaid.len() != before
        };
        if had {
            let now = self.inner.own.now_ms();
            let _ = self
                .inner
                .store
                .set_instance_note(&instance.id, None, begins(&instance.blueprint), now)
                .await;
        }
    }
}

/// Whoever turned following on, holding what they held then: what a
/// project's own files are taken as vouched for by.
fn followed_by(setting: &Taking) -> Principal {
    let by = setting.by.clone().unwrap_or_default();
    Principal { kind: PrincipalKind::User, id: by.clone(), name: by, scopes: setting.held.clone(), key: None }
}
