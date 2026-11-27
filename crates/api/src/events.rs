//! The events the node itself emits: its own starting and stopping, and
//! who is let in from outside. What is said of its data folder (a copy,
//! an arrival, a move, a repack), and that a step asked for it to be
//! started again, is `folders`'; that the client it serves was replaced,
//! and the names it answers to, `served`'s.

use blueprint::events::ANYONE;
use blueprint::{EventSpec, ValueType::*};

pub fn events() -> Vec<EventSpec> {
    let all = vec![
        EventSpec::new(
            "system.started",
            "The daemon started, and took up what it was doing: it answered what was read from before, and what \
             changes anything from here.",
        )
            .field("address", String, "where it answers")
            .field("version", String, "its version")
            .field("resumed", Int, "how many instances it took up again")
            .maybe("note", String, "why it was started, when it was said")
            .maybe("profile", String, "the profile this factory is")
            .field("runs_from", String, "`installed`, or `checkout` for one run from where it was built"),
        EventSpec::new("system.stopping", "The daemon was asked to stop.")
            .maybe(
                "at",
                String,
                "the step it is to stop at as that step begins, where it was asked to stop at one",
            )
            .maybe("of", String, "the blueprint that step is a step of, where one was named")
            .maybe("drained", Bool, "where it drained first: whether everything in hand had ended")
            .maybe("cut", List, "where it drained first: what was still in hand at the bound, with what becomes of each"),
        EventSpec::new(
            "system.draining",
            "The daemon was asked to stop once what is in hand is done: it begins nothing new from now, and stops \
             when the turns in hand and the runs under way have ended, or at the bound.",
        )
        .field("since", Int, "when it began draining, in milliseconds")
        .field("until", Int, "the latest it stops, in milliseconds")
        .field("turns", Int, "how many turns of agents were in hand as this was said")
        .field("runs", Int, "how many runs were under way as this was said"),
        EventSpec::new(
            "system.across",
            "What every page says across its top changed: whoever manages the factory said it may stop, or what it is to say.",
        )
        .maybe("across", Object, "what it says now: its `kind`, its `words` and what they are made of; nothing where it says nothing")
        .field("quiet", Bool, "whether it was told it may stop"),
        EventSpec::new("system.stopped", "The daemon stopped."),
        EventSpec::new(
            "system.experiment",
            "Something of the program's that is not yet whole was turned on or off for this factory, in Settings, \
             Experiments.",
        )
        .field("name", String, "the experiment")
        .field("on", Bool, "whether it is on now")
        .field("by", String, "who turned it, as an actor")
        .field("by_name", String, "who turned it, by name"),
        EventSpec::new(
            "caller.set",
            "A caller was described, or described again: a service outside that signs what it sends.",
        )
        .field("name", String, "the caller's name, which its address ends in")
        .field("topic", String, "the topic its calls are said as")
        .field("by_name", String, "who described it"),
        EventSpec::new("caller.removed", "A caller was taken away, and its address answers nobody.")
            .field("name", String, "the caller's name")
            .field("topic", String, "the topic its calls were said as")
            .field("by_name", String, "who took it away"),
        EventSpec::new(
            "project.branch.moved",
            "A branch of a project's repository moved, was made or was taken away, as its repository says: said \
             only for a project something asked to have its branches followed (`branches.follow`), and a move \
             made while the daemon was away is said once it is back.",
        )
        .field("project", String, "the project")
        .field("branch", String, "the branch, by its short name (`main`, `lane/shop-4`)")
        .field("from", String, "the commit it was at; empty where it was made")
        .field("to", String, "the commit it is at now; empty where it was taken away"),
        EventSpec::new(
            "project.blueprints",
            "Where a project keeps blueprints of its own was said: in the factory's place for them, in a folder \
             of its repository read before that, or nowhere. Kept with the setting, in the factory's own journal, \
             and heard as the project's.",
        )
        .field("project", String, "the project")
        .field("said", String, "`usual`, `folder` or `none`")
        .maybe("folder", String, "the folder of its repository read for them now, from its top, where it said one")
        .maybe("was", String, "the folder of its repository read for them before, where it said one")
        .maybe("was_said", String, "what it said before: `usual`, `folder` or `none`")
        .field("usual", String, "where a project keeps them where it says nothing: the factory's place, as a path of the project's"),
        EventSpec::new(
            "project.git.changed",
            "Something of a project's repository may have changed: a commit, a branch made, moved or switched to, \
             a file staged, a fetch. Said as the system says its files changed.",
        )
        .field("project", String, "the project")
        .passing(),
        EventSpec::new(
            "card.approved",
            "Somebody approved a card: a review that passed it, or a person who passed it where a told line lists it.",
        )
        .field("card", String, "the card")
        .field("project", String, "its project")
        .field("by", String, "who approved it, by name")
        .field("words", String, "what was said with it; nothing where nothing was"),
        EventSpec::new("device.sample", "How loaded this device is.")
            .field("host", String, "its name")
            .field("os", String, "its system")
            .field("arch", String, "its processor")
            .field("cpus", Int, "how many processors")
            .field("total_mem_mb", Int, "its memory")
            .field("free_mem_mb", Int, "how much of that is free")
            // Not every system can say either: one that keeps no load
            // average says none (Windows), and a folder on no volume the
            // system lists has no room to say.
            .maybe("free_disk_mb", Int, "how much room is left on disk, where that could be determined")
            .maybe("load1", Number, "its load over the last minute, where the system keeps one")
            .passing(),
    ];
    // Who may be told of each. That the daemon started or stopped is for
    // anyone who is signed in: every page goes by it.
    blueprint::events::heard(
        all,
        &[
            ("system.started", ANYONE),
            ("system.stopping", ANYONE),
            ("system.draining", ANYONE),
            ("system.across", ANYONE),
            ("system.stopped", ANYONE),
            ("system.experiment", ANYONE),
            ("device.sample", "fleet:read"),
            // Who is let in from outside is the owner's to say and to see.
            ("caller", "admin"),
            ("project.blueprints", "projects:read"),
            ("project.git", "projects:read"),
            ("project.branch.moved", "projects:read"),
            ("card", "board:read"),
        ],
    )
}
