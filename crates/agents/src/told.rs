//! What an agent is told about itself and the fleet: written as its
//! standing instructions, where its package names the file its program
//! reads them from and that file is its home's own, and otherwise ahead
//! of everything it is asked; and, asked in a channel that keeps
//! documents to hand, those it may read (`crate::docs`).

use std::path::Path;

use crate::{Agents, Slot};

/// Where a turn's brief goes, as [`Agents::briefed`] is handed it.
pub(crate) struct Briefing<'a> {
    /// The home of the agent's own for the turn, the ordinary one or a
    /// kept channel's, whether or not its program is shown it.
    pub home: &'a Path,
    /// The file its package says its program reads as its standing
    /// instructions, written from the home, where it says one.
    pub named: Option<String>,
    /// That file in `home`, where it is the home's own for this turn and
    /// the brief is written there (`Behind::instructions`).
    pub at: Option<&'a Path>,
}

/// How a brief the factory wrote begins, whoever's it is: what tells it
/// from a file of the same name that somebody else wrote, which is never
/// taken away.
fn written_by_the_factory() -> String {
    format!("<!-- Written by {} before each turn of `", brand::DISPLAY)
}

/// The file beside a home that says where in it the factory last wrote a
/// brief, written from the home.
const WRITTEN_AT: &str = "briefed";

impl Agents {
    /// What `slot`'s program is handed ahead of what it was asked, for a
    /// turn whose brief is `brief`: where the file its program reads as
    /// its standing instructions is the home's own (`briefing.at`), the
    /// brief is written there and what is handed is one line, which tells
    /// what the factory asked from what somebody said; otherwise the
    /// brief itself, as it always was. Nothing of a key is in either: what
    /// it is handed is said by the variables it is under, and whatever it
    /// was handed that the words came to hold is taken out (`unsaid`).
    ///
    /// A brief the factory wrote that is not this turn's (its package no
    /// longer names that file, or names it and it could not be written
    /// for this turn) is taken away, so that the program does not read
    /// it beside what it is told; only a file that begins as the factory
    /// writes one.
    pub(crate) fn briefed(
        &self,
        slot: &Slot,
        brief: String,
        briefing: Briefing<'_>,
        unsaid: impl Fn(&str) -> String,
    ) -> String {
        let home = briefing.home;
        let record = home.with_file_name(WRITTEN_AT);
        let last = std::fs::read_to_string(&record).ok().map(|last| last.trim().to_string());
        let written = briefing.at.and_then(|at| {
            let text = format!(
                "{began}{name}`, from what the factory says of it and the standing instructions it was given. \
                 A change made here is written over at its next turn: its standing instructions are changed \
                 where the agent is (its row, the Tools panel). -->\n\n{brief}\n",
                began = written_by_the_factory(),
                name = slot.name,
                brief = unsaid(&brief),
            );
            let within = at.strip_prefix(home).ok()?.to_string_lossy().into_owned();
            write_when_other(at, &text).ok()?;
            if last.as_deref() != Some(within.as_str()) {
                let _ = std::fs::write(&record, &within);
            }
            Some(within)
        });
        // What was written before and is not this turn's.
        let stale = last.iter().chain(&briefing.named).filter(|was| Some(*was) != written.as_ref());
        for was in stale {
            take_away_written(home, was);
        }
        if written.is_none() && last.is_some() {
            let _ = std::fs::remove_file(&record);
        }
        match written {
            Some(_) => format!(
                "You are `{name}`, briefed by the {product} factory in your standing instructions. What follows \
                 was asked of you through the factory.",
                name = slot.name,
                product = brand::DISPLAY,
            ),
            // Not written, it is said as it always was: an agent is never
            // left without it.
            None => brief,
        }
    }
}

/// Takes away the file at `within` in `home` where it is a brief the
/// factory wrote, and nothing else: not one that leads out of the home,
/// and not one somebody else wrote there.
fn take_away_written(home: &Path, within: &str) {
    let down = Path::new(within).components().all(|part| matches!(part, std::path::Component::Normal(_)));
    let file = home.join(within);
    let ours = || {
        let mut began = vec![0; written_by_the_factory().len()];
        let read =
            std::fs::File::open(&file).and_then(|mut file| std::io::Read::read_exact(&mut file, &mut began));
        read.is_ok() && began == written_by_the_factory().as_bytes()
    };
    if down && !within.is_empty() && file.symlink_metadata().is_ok_and(|meta| meta.is_file()) && ours() {
        let _ = std::fs::remove_file(&file);
    }
}

/// Writes `text` at `at` unless that is what is there already, making
/// the folders it is in; by a file beside it that is put in its place,
/// so that nothing reads it half written.
fn write_when_other(at: &Path, text: &str) -> std::io::Result<()> {
    if std::fs::read_to_string(at).is_ok_and(|there| there == text) {
        return Ok(());
    }
    if let Some(folder) = at.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let name = at.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let beside = at.with_file_name(format!(".{name}.{}", kernel::id::secret().get(..8).unwrap_or("new")));
    std::fs::write(&beside, text)?;
    std::fs::rename(&beside, at).inspect_err(|_| {
        let _ = std::fs::remove_file(&beside);
    })
}

impl Agents {
    /// What an agent is told about itself and the fleet: its brief.
    pub(crate) fn preamble(
        &self,
        slot: &Slot,
        cwd: Option<&str>,
        git: Option<crate::repository::Git>,
        keys: &[String],
        docs: &str,
    ) -> String {
        let (cli, product) = (brand::CLI, brand::DISPLAY);
        let place = match (&slot.project, cwd) {
            (Some(project), Some(cwd)) => {
                format!(" Your working directory is `{cwd}`, in the project `{project}`.")
            }
            (Some(project), None) => format!(" Your working directory is the project `{project}`."),
            (None, _) => String::new(),
        };
        let standing =
            if slot.instructions.is_empty() { String::new() } else { format!("\n\n{}", slot.instructions) };
        // One that is handed keys is told which, by the variables they are
        // under, so that it uses them and does not go looking for others.
        let handed = match keys {
            [] => String::new(),
            keys => format!(
                "\n\nIn your environment you are handed a key for what you may do outside this factory, under: {}. \
                 Use it as it is; do not print it or write it into a file.",
                keys.iter().map(|variable| format!("`{variable}`")).collect::<Vec<_>>().join(", ")
            ),
        };
        // One behind a wall is told what it will find, so that it does not
        // take a folder that is not there, or a commit that is refused,
        // for something gone wrong.
        let wall = match git {
            Some(git) => format!(
                "\n\nYour program works behind a wall: of this device it is shown the folder you work in, a home \
                 of its own, and what you were told you may read or change, and nothing else. {}",
                git.told()
            ),
            None => String::new(),
        };
        format!(
            "You are `{name}`, an agent in a {product} software factory.{place}\n\n\
             You act on the factory with the `{cli}` command, which is on your PATH and already signed in as you. \
             Run `{cli} help` for everything it does. The common things:\n\
             \x20 {cli} project create \"<name>\"          create a project, with starting docs\n\
             \x20 {cli} fleet init <project>             write the project's fleet blueprint\n\
             \x20 {cli} check <file>                     check a blueprint before running it\n\
             \x20 {cli} apply <file> [--param k=v]       run a blueprint\n\
             \x20 {cli} board add <project> \"<title>\"    add a card to the board\n\
             \x20 {cli} board history <card>           what has happened to a card\n\
             \x20 {cli} post <channel> \"<message>\"       post to a channel\n\
             \x20 {cli} post <channel> \"...\" --attach <file>   with a file; an .html file is shown as a page\n\
             \x20 {cli} help blueprints                  how blueprints are written\n\n\
             To show something drawn (a chart, a diagram, an animation), write it as one .html file and attach \
             it: it is shown in the channel as a page. It must carry all it needs, for it reaches nothing over the \
             network; `/lib/chart.js` (Chart.js) is there to draw charts with, and `{cli} help views` says the rest.\n\n\
             A mockup (a page to try a design with) is not attached: it is a folder of its own in the project, \
             `mockups/<name>/`, holding an `index.html` and whatever that page uses beside it. A variant of one is \
             another such folder whose file `mockup.yaml` has the line `of: <the other's folder>`.\n\n\
             Files you create belong in the project directory. Say what you did, briefly: your final message is \
             posted for the people you are working with.{wall}{handed}{docs}{allowed}{standing}",
            name = slot.name,
            allowed = crate::allowed::told(
                slot,
                git.is_some(),
                &self.inner.studio.identity.grouped().held_by(&format!("agent:{}", slot.name))
            ),
        )
    }
}
