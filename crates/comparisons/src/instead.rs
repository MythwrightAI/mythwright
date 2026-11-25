//! A way of a comparison that reads other words where a file is linked.
//!
//! What an agent is told is a file a blueprint links ([`conductor::words`]),
//! and the question somebody running a fleet asks first of it is whether
//! other words would do better. That is a comparison whose ways differ in
//! nothing else: the same blueprint, the same agents, and under one way
//! the step that links `std/prompts/review-seat.md` is given another file
//! of the library in its place.
//!
//! - **A way says which** ([`Reads`], by the linked file's name in the
//!   library). It is checked when the comparison is started: the file
//!   read instead is there, something the comparison reaches links the
//!   file it stands in for, and what links it hands every blank the other
//!   has.
//! - **Which words is settled when a thing is started under the way**,
//!   and written down for what is started (`Comparisons::keep_reads`): the
//!   words kept when the comparison began where texts are held, and the
//!   file as it then is where they are followed ([`conductor::taking`]), the
//!   last that fitted where that one does not. So what a column of the
//!   comparison says it ran is what was read, and a file that changes
//!   half way is counted apart, as a blueprint's text is.
//! - **The step asks before it reads** (`Comparisons::read_instead`): of
//!   what started it, and of what started that, whether a way gave it
//!   other words for this file. Nothing is asked where no way of any
//!   comparison ever did.

use std::collections::{BTreeMap, HashMap, HashSet};

use kernel::{Error, Result};
use serde_json::json;

pub use kernel::compared::Reads;

use crate::Comparisons;
use crate::compared::Way;

/// Where what each thing started under a way reads instead is kept: by
/// the instance that started it and the key it was started under. It is
/// that instance's, and is kept beside its rows, in the journal of the
/// conductor that runs it (`conductor::Engine::kept_beside`): a project's
/// in the project's own file.
pub(crate) const NAMESPACE: &str = "compared.reads";

/// How far up from a step what started it is looked for.
const DEEPEST: usize = 64;

/// What is kept for one thing started under a way: for each file linked,
/// the revision of the words read in its place.
pub(crate) type Kept = BTreeMap<String, String>;

fn place(parent: &str, key: &str) -> String {
    format!("{parent}\n{key}")
}

impl Comparisons {
    /// Holds what the way `way` reads instead to being something that can
    /// be: each file it names is one a step links, among `reached` (every
    /// text the comparison's blueprints reach, by name); the file read in
    /// its place is there; and it has no blank that what links the first
    /// does not hand. What was found is kept on the way.
    pub(crate) async fn reads_checked(
        &self,
        way: &mut Way,
        reached: &[(String, String)],
        project: Option<&str>,
    ) -> Result<()> {
        let folder = conductor::words::FOLDER;
        for (linked, reads) in way.prompts.iter_mut() {
            for name in [linked, &reads.name] {
                if !conductor::library::is_linked(name) || !conductor::library::is_safe_name(name) {
                    return Err(Error::invalid(format!(
                        "the way `{}` reads another file where one is linked, and `{name}` is not the name of \
                         such a file: it is one of a `{folder}/`, as the library calls it, such as \
                         `std/{folder}/review-seat.md`",
                        way.name
                    )));
                }
            }
            // Every step that links it, among what the comparison reaches.
            let mut handed: Option<Vec<String>> = None;
            let mut links = Vec::new();
            for (name, text) in reached.iter().filter(|(name, _)| !conductor::library::is_linked(name)) {
                let Ok(blueprint) = self.engine.compile(name, text, project) else { continue };
                for link in conductor::words::linked(&blueprint) {
                    let file = conductor::words::linked_as(&blueprint.name, link.from);
                    if &file != linked {
                        links.push(file);
                        continue;
                    }
                    let hands: Vec<String> =
                        link.with.as_object().map(|with| with.keys().cloned().collect()).unwrap_or_default();
                    // What they all hand: the other file is given to each of them.
                    handed = Some(match handed {
                        Some(before) => before.into_iter().filter(|name| hands.contains(name)).collect(),
                        None => hands,
                    });
                }
            }
            let Some(handed) = handed else {
                links.sort();
                links.dedup();
                let there = match links.is_empty() {
                    true => "They link no file".to_string(),
                    false => format!("What they link: {}", links.join(", ")),
                };
                return Err(Error::invalid(format!(
                    "the way `{}` reads `{}` in place of `{linked}`, and nothing this comparison starts links \
                     that file. {there}",
                    way.name, reads.name
                )));
            };
            let Some(words) = self.engine.library().load(&reads.name, project) else {
                let folder = conductor::library::folder_of(self.engine.library().as_ref(), project);
                let to_do = match folder.path() {
                    Some(path) => format!(": write it as `{path}/{}` in the project", reads.name),
                    None => {
                        format!(". {}", conductor::library::folders::write_as(&folder, &reads.name, project))
                    }
                };
                return Err(Error::not_found(format!(
                    "the way `{}` reads `{}` in place of `{linked}`, and there is no such file{to_do}",
                    way.name, reads.name
                )));
            };
            let names: Vec<&str> = handed.iter().map(String::as_str).collect();
            if let Some(why) = conductor::words::unfit(&words, &names) {
                return Err(Error::invalid(format!(
                    "the way `{}` reads `{}` in place of `{linked}`, and it cannot be filled as that file is: \
                     {why}. What is handed: {}",
                    way.name,
                    reads.name,
                    names.join(", ")
                )));
            }
            let revision = blueprint::revision(&words);
            self.engine.keep_text(&revision, &words).await;
            (reads.revision, reads.handed) = (Some(revision), handed);
        }
        Ok(())
    }

    /// The words `way` reads instead as a thing is started under it by
    /// `instance` acting as `whom`: for each file linked, their revision.
    /// Held, they are the ones from when the comparison began; followed,
    /// the file as it now is, where what links the other still fills it.
    pub(crate) async fn reads_now(
        &self,
        way: &Way,
        instance: &str,
        whom: &kernel::Principal,
        project: Option<&str>,
    ) -> Kept {
        let mut now = Kept::new();
        for (linked, reads) in &way.prompts {
            let there = match self.engine.follows() {
                true => self.engine.text_within(instance, whom, &reads.name, project).await,
                false => None,
            };
            let handed: Vec<&str> = reads.handed.iter().map(String::as_str).collect();
            let fits = there.filter(|there| conductor::words::unfit(there, &handed).is_none());
            match (fits, &reads.revision) {
                (Some(there), _) => {
                    let revision = blueprint::revision(&there);
                    self.engine.keep_text(&revision, &there).await;
                    now.insert(linked.clone(), revision);
                }
                (None, Some(kept)) => drop(now.insert(linked.clone(), kept.clone())),
                (None, None) => {}
            }
        }
        now
    }

    async fn reads_kept(&self) {
        let kept = &self.kept.reads;
        if kept.lock().expect("reads lock").is_some() {
            return;
        }
        // From every journal a conductor keeps its rows in, the factory's
        // first: where a program from before kept one there for an
        // instance of a project's, what its own journal says stands.
        let mut all: HashMap<String, Kept> = HashMap::new();
        for journal in self.engine.own_journals() {
            for (place, kept) in journal.kv_list(NAMESPACE).await.unwrap_or_default() {
                if let Ok(reads) = serde_json::from_value(kept) {
                    all.insert(place, reads);
                }
            }
        }
        kept.lock().expect("reads lock").get_or_insert(all);
    }

    /// Writes down what the child `parent` starts under `key` reads
    /// instead; with nothing, what was kept for an earlier one of that key
    /// is taken away.
    pub(crate) async fn keep_reads(&self, parent: &str, key: &str, reads: Kept) {
        self.reads_kept().await;
        let place = place(parent, key);
        let had = {
            let mut kept = self.kept.reads.lock().expect("reads lock");
            let all = kept.get_or_insert_with(HashMap::new);
            match reads.is_empty() {
                true => all.remove(&place).is_some(),
                false => all.insert(place.clone(), reads.clone()).as_ref() != Some(&reads),
            }
        };
        if had {
            let value = if reads.is_empty() { serde_json::Value::Null } else { json!(reads) };
            let beside = self.engine.kept_beside(parent).await;
            let _ = beside.kv_set(NAMESPACE, &place, &value).await;
            // Taken away, it is taken out of the factory's journal too,
            // where a program from before kept every one of them.
            let factory = self.engine.journal();
            if reads.is_empty() && beside.name() != factory.name() {
                let _ = factory.kv_set(NAMESPACE, &place, &serde_json::Value::Null).await;
            }
        }
    }

    /// The words `instance` is to read where it links `name`, where a way
    /// of a comparison gave what started it, or anything above that,
    /// other words for that file.
    pub(crate) async fn read_instead(&self, instance: &str, name: &str) -> Option<String> {
        self.reads_kept().await;
        if self.kept.reads.lock().expect("reads lock").as_ref().is_none_or(HashMap::is_empty) {
            return None;
        }
        let mut at = instance.to_string();
        for _ in 0..DEEPEST {
            let (key, parent) = self.engine.above(&at).await?;
            let parent = parent?;
            let revision = {
                let kept = self.kept.reads.lock().expect("reads lock");
                kept.as_ref()
                    .and_then(|all| all.get(&place(&parent, &key)))
                    .and_then(|reads| reads.get(name).cloned())
            };
            if let Some(revision) = revision {
                return self.engine.text_of(&revision).await;
            }
            at = parent;
        }
        None
    }

    /// When the conductor starts: what was kept for things started by
    /// instances that are gone is put away, and the revisions the rest
    /// read are given, to be kept with the texts trees are held to.
    pub(crate) async fn reads_resumed(&self, alive: &HashSet<&str>) -> Vec<String> {
        self.reads_kept().await;
        let all: Vec<(String, Kept)> = {
            let kept = self.kept.reads.lock().expect("reads lock");
            kept.as_ref().map(|all| all.iter().map(|(place, reads)| (place.clone(), reads.clone())).collect())
        }
        .unwrap_or_default();
        let mut wanted = Vec::new();
        for (place, reads) in all {
            match place.split_once('\n') {
                Some((parent, _)) if alive.contains(parent) => wanted.extend(reads.into_values()),
                Some((parent, key)) => self.keep_reads(parent, key, Kept::new()).await,
                None => {}
            }
        }
        wanted
    }
}
