//! An agent behind a wall: what its program is shown of the device.
//!
//! [`hosting::wall`] holds a wall and knows nothing of agents. This says
//! what an agent's is made of. Its program is started as whoever runs the
//! daemon, and with no wall could read what they can: the session of the
//! person signed in, the factory's own secret, every other agent's
//! credential, the keys in their home. Its slot's scopes say what it may
//! ask of the factory; the wall is what makes asking the only way.
//!
//! Behind its wall an agent's program is shown:
//!
//! - the system, and the folders on the path, to read;
//! - where it works, to change: its project, or the folder of its own
//!   there that a turn names (a card's worktree), with the rest of the
//!   project to read. One held to `read` may change neither;
//! - a home of its own, where the home of whoever runs the daemon is:
//!   one for what it is asked in the open, and one for each channel
//!   kept to its members that it is asked in (`crate::kept`), so that
//!   what its program keeps of such a channel is there for that
//!   channel's turns and for no other;
//! - its own credential and the command-line program, to read;
//! - what it signs in with: the folders it was handed, where it draws a
//!   sign-in the factory holds, and otherwise, in whichever of its homes,
//!   only what its package says is its sign-in, of what its program keeps
//!   in the home of whoever runs it, where it is; the rest of what its
//!   program keeps is that home's own. One whose sign-in its package
//!   cannot tell from the rest is shown all of what it keeps there, as
//!   one place, and has no home of its own (`crate::kept::Theirs`);
//! - and what its slot adds, to read (`reads`) and to change (`changes`).
//!
//! What a table of its project keeps private (`reads`, `studio::closed`)
//! is hidden from an agent that does not hold what it is kept under: the
//! folder its records are files in, the copies of it in cards' worktrees,
//! and the repository, whose history has them too.
//!
//! A repository's own folder (`.git`) is shown to read and never to
//! change: what is written there (a hook, a setting) is run by whoever
//! next asks git for something, and that is the daemon, with no wall.
//! What an agent changed is committed for it by the blueprint that gave
//! it the work.
//!
//! What a package says its program needs there, and what a turn behind
//! one is started with, are the package's (`extensions::walled`), offered
//! here under the names they always had.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use hosting::{Shown, Walls};
use kernel::{Error, Principal, Result};

pub use extensions::walled::{Behind, Needs, said};
pub(crate) use extensions::walled::{folders, hint, place};

use crate::repository::Git;
use crate::signins::Handed;
use crate::{Agents, Slot, SlotSpec};

impl Agents {
    /// What is said besides, of a program that did not do behind a wall
    /// what it was started for, when its package is not the one of its
    /// name that came with this installation: that one may say what the
    /// program needs there, and this one was installed before it did.
    pub(crate) fn package_note(&self, adapter: &str) -> Option<String> {
        self.behind_what_came().iter().any(|name| name == adapter).then(|| {
            format!(
                "Its package `{adapter}` is not the one that came with this installation, which may say more of \
                 what its program needs behind a wall: `{cli} extension update {adapter}` takes that one, and keeps \
                 this one beside it.",
                cli = brand::CLI
            )
        })
    }

    /// A failed turn's sentence, with what [`hint`] and [`Self::package_note`] add.
    pub(crate) fn hinted(&self, adapter: &str, message: String, walled: bool, code: Option<i32>) -> String {
        let note = self.package_note(adapter).filter(|_| walled && code.is_some());
        let message = hint(message, walled, code);
        match note {
            Some(note) => format!("{message} {note}"),
            None => message,
        }
    }

    /// What `spec` makes of a slot's wall: whether it has one, and what it
    /// is shown there. Said, each is as said; unsaid, as the slot had it;
    /// and a new slot has a wall and is shown nothing more.
    ///
    /// With no wall its program sees this device as whoever runs the daemon
    /// does, and a folder added is a folder of theirs. Either gives what
    /// running a command here gives, so making such an agent, or saying
    /// anything of one that is, takes being allowed to run them.
    pub(crate) fn wall_of(
        &self,
        spec: &SlotSpec,
        existing: Option<&Slot>,
        by: &Principal,
    ) -> Result<(bool, Vec<String>, Vec<String>)> {
        let wall = spec.wall.or(existing.map(|slot| slot.wall)).unwrap_or(true);
        let reads = match &spec.reads {
            Some(said) => folders(said, "may read")?,
            None => existing.map(|slot| slot.reads.clone()).unwrap_or_default(),
        };
        let changes = match &spec.changes {
            Some(said) => folders(said, "may change")?,
            None => existing.map(|slot| slot.changes.clone()).unwrap_or_default(),
        };
        if by.can("shell") {
            return Ok((wall, reads, changes));
        }
        if !wall {
            return Err(Error::denied(format!(
                "an agent with no wall has a program that sees this device as whoever runs the daemon does, and \
                 saying anything of one takes `shell`, which {} does not hold. Give it a wall (`wall: on`), or ask \
                 someone who holds `shell`",
                by.name
            )));
        }
        if !reads.is_empty() || !changes.is_empty() {
            return Err(Error::denied(format!(
                "showing an agent more of this device than its work ({}) takes `shell`, which {} does not hold",
                reads.iter().chain(&changes).cloned().collect::<Vec<_>>().join(", "),
                by.name
            )));
        }
        Ok((wall, reads, changes))
    }

    /// Whether `spec` has a slot's program go on without the daemon: as
    /// said, and unsaid as the slot had it; a new slot's does not.
    ///
    /// One that goes on works while no daemon is there, with nobody
    /// watching and nobody able to stop it through the daemon. Turning
    /// that on is letting a program at this device as taking a wall away
    /// is, and takes being allowed to run commands here. Turning it off
    /// again takes nothing more, and neither does saying anything else of
    /// one that already goes on.
    pub(crate) fn outlives_of(
        &self,
        spec: &SlotSpec,
        existing: Option<&Slot>,
        by: &Principal,
    ) -> Result<bool> {
        let was = existing.is_some_and(|slot| slot.outlives);
        let outlives = spec.outlives.unwrap_or(was);
        if outlives && !was && !by.can("shell") {
            return Err(Error::denied(format!(
                "an agent whose program goes on without the daemon works with nobody watching while the daemon is \
                 down, and turning that on takes `shell`, which {} does not hold. Leave it off (`outlives: off`), \
                 or ask someone who holds `shell`",
                by.name
            )));
        }
        Ok(outlives)
    }

    /// Whether `spec` lets a slot's program use the web unasked: as said,
    /// and unsaid as the slot had it; a new slot's may not.
    ///
    /// A wall does not hold the network, which is the device's, so what
    /// keeps such a program from the web is only its own asking leave of
    /// somebody who is not there. Letting it look things up and fetch
    /// pages by itself lets it bring in whatever a page says and send out
    /// whatever it puts in a search or an address: of a kind with showing
    /// it more of the device, and it takes the same, being allowed to run
    /// commands here. Turning it off again takes nothing more, and neither
    /// does saying anything else of one that already may.
    pub(crate) fn web_of(&self, spec: &SlotSpec, existing: Option<&Slot>, by: &Principal) -> Result<bool> {
        let was = existing.is_some_and(|slot| slot.web);
        let web = spec.web.unwrap_or(was);
        if web && !was && !by.can("shell") {
            return Err(Error::denied(format!(
                "an agent whose program may use the web unasked brings in whatever a page says and sends out                  whatever it looks for, and turning that on takes `shell`, which {} does not hold. Leave it off                  (`web: off`), or ask someone who holds `shell`",
                by.name
            )));
        }
        Ok(web)
    }

    /// Whether `turn` was told folders to hide and nothing hid them: its
    /// agent works with no wall, or the factory holds none.
    pub(crate) async fn unhidden(&self, turn: &crate::TurnInfo) -> bool {
        if turn.hidden.is_empty() {
            return false;
        }
        let walled = self.slot(&turn.slot).await.is_ok_and(|slot| slot.wall);
        !walled || self.inner.hosting.walls() == Walls::Open
    }

    /// What `slot`'s program is started behind for a turn taken in
    /// `workdir`, holding `handed` if it draws a sign-in; nothing, where
    /// it has no wall or the factory holds none. `kept` is the channel
    /// kept to its members that the turn is of, where it is of one: its
    /// home is then that channel's. `hidden` are folders of its project,
    /// from the project's top, it is not shown, wherever it works there.
    pub(crate) fn behind(
        &self,
        slot: &Slot,
        workdir: &Path,
        handed: Option<&Handed>,
        env: &BTreeMap<String, String>,
        (holds, kept, hidden): (bool, Option<&str>, &[String]),
    ) -> Option<Behind> {
        let walls = self.inner.hosting.walls();
        if !slot.wall || walls == Walls::Open {
            return None;
        }
        let config = &self.inner.config;
        // Where a system has no `HOME`, it says the same another way.
        let said = || std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
        let home = config.user_home.clone().or_else(said).filter(|home| home.is_absolute());
        let mut shown = Shown::starting_in(workdir);
        shown.never(&config.data);
        shown.path(env.get("PATH").map(String::as_str).unwrap_or_default());

        // A home of its own, where the home of whoever runs the daemon is:
        // what its program looks for there it finds, or makes, in its own.
        // For a turn of a kept channel, the one that is that channel's.
        let own = self.home_for(&slot.name, kept);
        let _ = std::fs::create_dir_all(&own);
        if let Some(home) = &home {
            shown.never(home);
            shown.put(&own, home);
        }
        shown.change(&own);

        // Who it is to the factory, and the program it asks the factory with.
        shown.read(&self.token_path(&slot.name));
        shown.read(&config.bin);
        if let Ok(program) = std::env::current_exe() {
            shown.read(&program);
        }

        // Where it works.
        let reading = slot.permission_mode == "read";
        let work = |shown: &mut Shown, folder: &Path| match reading {
            true => shown.read(folder),
            false => shown.change(folder),
        };
        let project = slot.project.as_ref().and_then(|project| self.inner.studio.projects.dir_of(project));
        let project = project.map(|project| kernel::whole::whole(&project).unwrap_or(project));
        // What it may do with the repository: as its slot says, and
        // nothing where the repository holds what is kept from it.
        let git = Git::of(&slot.git, slot.permission_mode == "read", holds);
        match project.clone() {
            Some(project) => {
                let folder = shown.work().to_path_buf();
                let own_folder = folder != project;
                if own_folder {
                    // A folder of its own within the project: a card's worktree.
                    shown.read(&project);
                    work(&mut shown, &folder);
                } else {
                    work(&mut shown, &project);
                    // The folders other cards are worked in are theirs.
                    shown.read(&project.join(".lanes"));
                }
                // What another piece of work on the same item is done
                // in, where several are done at once: not its to read.
                for folder in hidden {
                    shown.hide(&project.join(folder));
                }
                let repository = project.join(".git");
                match repository.exists() {
                    true => {
                        crate::repository::show(&mut shown, git, &project, own_folder.then_some(&*folder))
                    }
                    // And none can be made where there is none: a
                    // repository of an agent's own design would be one
                    // the daemon then asks git about.
                    false if !reading && !own_folder => shown.hide(&repository),
                    false => {}
                }
            }
            None => {
                let folder = shown.work().to_path_buf();
                work(&mut shown, &folder);
                // It works in the folder that is its own, and what it
                // kept of each kept channel is in there: not for a turn
                // of anything else.
                let of_kept = config.home.join(&slot.name).join(crate::kept::PLACES);
                if kept.is_none() && of_kept.exists() {
                    shown.hide(&of_kept);
                }
            }
        }

        // What a table of its project keeps private from it is not there,
        // wherever there is a copy of it: in the project and in each
        // card's worktree. (And in the repository, where its history has
        // any of it: that is why it may then do nothing with git, above.)
        if let (Some(id), Some(project)) = (&slot.project, &project) {
            let kept = self.inner.studio.projects.kept_from(id, &slot.principal());
            let lanes = std::fs::read_dir(project.join(".lanes")).into_iter().flatten().flatten();
            let places: Vec<PathBuf> =
                std::iter::once(project.clone()).chain(lanes.map(|lane| lane.path())).collect();
            for folder in &kept {
                for hidden in places.iter().map(|place| place.join(folder)).filter(|hidden| hidden.exists()) {
                    shown.hide(&hidden);
                }
            }
        }

        // What it signs in with.
        let package = self.extension(&slot.adapter);
        let needs = package.as_ref().map(|package| package.wall.clone()).unwrap_or_default();
        let theirs = package.as_deref().and_then(crate::kept::theirs).filter(|_| handed.is_none());
        // Whether the home it is shown is its own, whatever its program
        // keeps of the conversations it is asked is kept there.
        let own_home = theirs.is_none() || kept.is_some();
        let mut inherits = Vec::new();
        match handed {
            Some(handed) => {
                for folder in &handed.folders {
                    shown.change(folder);
                }
            }
            None => {
                // In its home of its own, of what it keeps only its
                // sign-in, which its package says: the rest is that
                // home's own, the ordinary one's or a kept channel's, and
                // what it is asked in one is found in no other. One
                // whose sign-in cannot be told from the rest is shown it
                // whole, as one place (`crate::kept::Theirs`), and is in
                // no kept channel.
                let shared = match theirs.is_some() && kept.is_none() {
                    true => &needs.keeps,
                    false => &needs.signin,
                };
                for kept in shared {
                    if let Some(home) = &home {
                        shown.change(&home.join(kept));
                    }
                }
                // And what whoever runs the daemon told its program by.
                inherits = package
                    .iter()
                    .flat_map(|package| package.signin.iter())
                    .flat_map(|by| by.variables())
                    .collect();
                // A folder they named for it is where it keeps
                // everything, and is no kept channel's turn's to be shown.
                let named = inherits.iter().filter_map(std::env::var_os).filter(|_| kept.is_none());
                for folder in named {
                    let folder = PathBuf::from(folder);
                    if folder.is_absolute() && folder.is_dir() {
                        shown.change(&folder);
                    }
                }
            }
        }

        // What its slot adds; and of that, nothing that is a credential
        // by what it is called, unless it was itself named.
        let added = |written: &Vec<String>| -> Vec<PathBuf> {
            written.iter().filter_map(|written| place(written, home.as_deref())).collect()
        };
        let (reads, changes) = (added(&slot.reads), added(&slot.changes));

        // The file it reads as its standing instructions, where that is
        // its home's own for this turn and nothing of anybody else's is
        // shown over it: never in what is shown from the home of whoever
        // runs the daemon, which is theirs, nor where its program is told
        // a folder to read in place of its home.
        let instructions = needs.instructions.as_ref().filter(|_| home.is_some()).and_then(|file| {
            let file = Path::new(file);
            let from_theirs = match handed {
                Some(handed) => handed.moves_its_folder(),
                None => {
                    let shared = if own_home { &needs.signin } else { &needs.keeps };
                    let named = inherits.iter().any(|variable| std::env::var_os(variable).is_some());
                    named
                        || shared
                            .iter()
                            .any(|shown| file.starts_with(shown) || Path::new(shown).starts_with(file))
                }
            };
            let at = home.as_ref().map(|home| home.join(file))?;
            let added_over =
                reads.iter().chain(&changes).any(|shown| at.starts_with(shown) || shown.starts_with(&at));
            (!from_theirs && !added_over).then(|| own.join(file))
        });
        reads.iter().for_each(|path| shown.read(path));
        changes.iter().for_each(|path| shown.change(path));
        reads.iter().chain(&changes).for_each(|path| shown.without_credentials(path));

        // Who its commits are by, where it may make any: itself, at an
        // address that is nobody's, as what is committed for it says.
        let by = format!("{}@agents.invalid", slot.name);
        let sets = match git >= Git::Commit {
            true => ["GIT_AUTHOR_NAME", "GIT_COMMITTER_NAME"]
                .map(|name| (name.to_string(), slot.name.clone()))
                .into_iter()
                .chain(["GIT_AUTHOR_EMAIL", "GIT_COMMITTER_EMAIL"].map(|name| (name.to_string(), by.clone())))
                .collect(),
            false => Vec::new(),
        };
        Some(Behind { walls, shown, beside: needs.beside, inherits, home, git, sets, instructions, own_home })
    }
}
