//! A page of a wiki that somebody follows tells them, in their box, when
//! it changes.
//!
//! **One line a page.** Whoever follows a page has one line of it
//! ([`crate::inbox`], kind `followed`, about the page and opening it),
//! which each change raises: unread again, back from where it was put
//! away, counted once more, saying who changed it last and what that
//! change was (`said_by`, its number in the journal). So a flurry of
//! changes, a person saving a page ten times in a minute, is one line
//! counted ten times, never ten lines. Following a page follows that page
//! and none under it, unless it is followed with its pages: then a change
//! of any page under it, and a page made under it, raises the line of the
//! page followed, opening the page that changed, for whoever followed it
//! before that change. Somebody who follows a page and a page above it is
//! told by the line of the nearest.
//!
//! **How often** is the follow's ([`crate::following`]): at once as
//! above; hourly or daily, the change waits for a digest; off, nothing.
//!
//! **Told of what.** A reply on it, a change of what it says (title, body,
//! summary), of how it stands (answered, closed, open again, standing,
//! its status taken away), and its being put somewhere else
//! (`kernel::topics::tells_followers`).
//! Never what the follower did themselves, a key of theirs included.
//!
//! **Told only what they may read.** Asked in the write that tells, of
//! what each follower holds then and of what reading the page takes,
//! which whoever tells hands over ([`Change::takes`]): one who may not
//! read it is told nothing, and a line they have is passed over while they
//! may not, as a line of something said on a record is
//! ([`crate::commented`]).
//!
//! **Once.** It is told by whoever keeps the page's history, as the line
//! of the change is written and before where they have read to is kept,
//! so a change heard again after a stop is told again by the same. A line
//! that is already of that change, or of one after it, is left as it is:
//! the journal's numbers say which came first, and a change heard again
//! raises nothing. A follower named in a reply is told by this line and
//! not named besides, in the same write ([`Channels::tell_commented`]).
//!
//! **Shown.** The line keeps no words. Whoever reads the box is shown the
//! change as the page's history keeps it (who, which change, and for a
//! reply its words) as they may read the page now, by what keeps records
//! ([`crate::commented::OnRecords`]).

use std::collections::HashSet;

use journal::{Writing, optional};
use kernel::Result;
use kernel::following::Often;
use rusqlite::params;

use crate::channels::Channels;
use crate::commented::reads;
use crate::following::{Pended, how, pend};
use crate::inbox::{Inbox, Kind, Tell, owns};

/// Why a line of a page somebody follows is in their box.
pub const WHY: &str = "follows";

/// A change of a page, as whoever tells of it hands it over.
#[derive(Clone, Debug)]
pub struct Change {
    /// The page, which is what the line is about and opens.
    pub record: String,
    /// The change's number in the journal: which change the line is of.
    pub seq: u64,
    /// Who the journal says made it, which the line says it came from.
    pub from: String,
    /// Whose doing that was, as an actor: for a key, its maker. Not told.
    pub by: String,
    /// What reading the page takes, every scope of it.
    pub takes: Vec<String>,
    /// When it was made.
    pub at: i64,
    /// Who follows the page, each an actor: told where `own` says.
    pub followers: Vec<String>,
    /// The pages above it, the nearest first, each with who follows it:
    /// told where they follow it with its pages.
    pub above: Vec<(String, Vec<String>)>,
    /// What was said of it, and what it did in a word or two
    /// (`kernel::following::page_change`).
    pub topic: String,
    pub noun: String,
    pub verb: String,
    /// Whether whoever follows the page itself is told of it.
    pub own: bool,
}

impl Tell {
    /// That the page `about`, which `owner` follows, was changed by `from`
    /// in the change that is `seq` in the journal: itself, or the page
    /// `place` under it.
    pub fn followed(owner: &str, about: &str, place: &str, seq: u64, from: &str) -> Tell {
        Tell {
            owner: owner.to_string(),
            kind: Kind::Followed,
            about: about.to_string(),
            place: place.to_string(),
            from: from.to_string(),
            why: WHY.to_string(),
            says: None,
            said_by: Some(seq.to_string()),
            parts: None,
        }
    }
}

/// Tells each of `change`'s followers in the write under way, where they
/// read the page as things stand, did not make the change, and have no
/// line of it, or of a change after it, already: each as they follow it.
/// Who follows it at once, whether they were told now or before, for
/// whoever names them besides; and how many were told now.
pub(crate) fn tell_in(
    writing: &mut Writing<'_>,
    inbox: &Inbox,
    change: &Change,
) -> rusqlite::Result<(HashSet<String>, usize)> {
    let (mut handled, mut at_once, mut told) = (HashSet::new(), HashSet::new(), 0);
    let own =
        change.followers.iter().filter(|_| change.own).map(|follower| (&change.record, follower, false));
    let above =
        change.above.iter().flat_map(|(page, followers)| followers.iter().map(move |one| (page, one, true)));
    for (page, follower, under) in own.chain(above) {
        if !owns(follower) || *follower == change.by || *follower == change.from || handled.contains(follower)
        {
            continue;
        }
        let follow = how(writing.transaction(), follower, "page", page)?;
        if under && !follow.as_ref().is_some_and(|follow| follow.pages && follow.since < change.seq) {
            continue;
        }
        handled.insert(follower.clone());
        if !reads(&inbox.held(writing.transaction(), follower)?, &change.takes) {
            continue;
        }
        match follow.as_ref().map_or(Often::Once, |follow| follow.often) {
            Often::Off => {}
            Often::Hourly | Often::Daily => {
                let pended = Pended {
                    seq: change.seq,
                    record: change.record.clone(),
                    noun: change.noun.clone(),
                    verb: change.verb.clone(),
                    topic: change.topic.clone(),
                    takes: None,
                    from: change.from.clone(),
                    at: change.at,
                };
                pend(writing.transaction(), follow.as_ref().expect("a digest is a follow's"), &pended)?;
            }
            Often::Once => {
                at_once.insert(follower.clone());
                let last: Option<Option<String>> = optional(writing.transaction().query_row(
                    "SELECT said_by FROM inbox WHERE owner = ?1 AND kind = ?2 AND about = ?3 AND why = ?4 AND about <> ''",
                    params![follower, Kind::Followed.as_str(), page, WHY],
                    |row| row.get(0),
                ))?;
                if last
                    .flatten()
                    .and_then(|last| last.parse::<u64>().ok())
                    .is_some_and(|last| last >= change.seq)
                {
                    continue;
                }
                let tell = Tell::followed(follower, page, &change.record, change.seq, &change.from);
                told += usize::from(inbox.tell_in(writing, &tell, change.at)?.is_some());
            }
        }
    }
    Ok((at_once, told))
}

impl Channels {
    /// Tells whoever follows the page `change` is of, or a page above it
    /// with its pages, each in their box or for a digest, in one write.
    /// How many were told.
    pub async fn tell_followed(&self, change: Change) -> Result<usize> {
        if change.followers.is_empty() && change.above.iter().all(|(_, followers)| followers.is_empty()) {
            return Ok(0);
        }
        let inbox = self.inbox.clone();
        let wrote = self.journal.write(move |writing| tell_in(writing, &inbox, &change));
        let told = wrote.await?.0.1;
        self.following.wakes().notify_one();
        Ok(told)
    }
}
