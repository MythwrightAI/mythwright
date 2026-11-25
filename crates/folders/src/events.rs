//! The events said of a factory's data folder: a copy of it written, a
//! factory started from one, and what was done to its files while it was
//! stopped (written out again, a project moved apart), said by the daemon
//! that next starts on it; and that a step asked for the factory to be
//! started again (`restarting`).

use blueprint::EventSpec;
use blueprint::ValueType::*;
use blueprint::events::ANYONE;

pub fn events() -> Vec<EventSpec> {
    let all = vec![
        EventSpec::new("system.snapshot", "A snapshot of the factory was written.")
            .field("snapshot", String, "its name")
            .field("to", String, "the folder it was written to")
            .maybe("profile", String, "the profile it was taken of")
            .field("note", String, "what was said of it")
            .field("made", Int, "when, in milliseconds since 1970")
            .field("files", Int, "how many files it holds")
            .field("bytes", Int, "how large it is")
            .field("holds", Object, "what was taken, by kind")
            .field("left_out", Object, "what was not, by kind")
            .field("took_ms", Number, "how long it took"),
        EventSpec::new("system.copied", "This factory was started from a snapshot of another.")
            .field("snapshot", String, "the snapshot's name")
            .field("from", Object, "the factory it was taken of")
            .field("profile", String, "the profile this factory is")
            .field("note", String, "what was said of the snapshot")
            .field("made", Int, "when the snapshot was made, in milliseconds since 1970")
            .field("arrived", Int, "when it arrived here, in milliseconds since 1970")
            .field("files", Int, "how many files came")
            .field("projects", Int, "how many projects came")
            .field("left_out", Object, "what the snapshot did not hold, by kind")
            .field("removed", Object, "what was taken out on arrival, by kind")
            .field("without_files", List, "the projects that arrived without their files")
            .field("announced", Bool, "whether this was said in a channel"),
        EventSpec::new(
            "system.reindexed",
            "The journal of a factory from before had its index of when events were said made anew, as \
             this program first started on it. Said once.",
        )
        .maybe("took_ms", Int, "how long it took, where that is known"),
        EventSpec::new(
            "system.repack_cleared",
            "A copy of the database that a `repack` was cut off writing was found beside it and taken away, \
             as the daemon started.",
        )
        .field("bytes", Int, "how big it was"),
        EventSpec::new(
            "system.repacked",
            "The factory's database was written out again as one is made now, while the daemon was stopped. \
             Said by the daemon that next starts.",
        )
        .field("before", Int, "how big it was, in bytes")
        .field("after", Int, "how big it is, in bytes")
        .field("events", Int, "how many events it holds, as it did")
        .field("took_ms", Int, "how long it took"),
        EventSpec::new(
            "system.moved_apart",
            "A project of a factory from before was moved to a journal of its own, while the daemon was \
             stopped: what is run for it is kept there from now on, by a conductor of its own, with its \
             tables, their records and what its agents were asked. Said by the daemon that next starts, once \
             for each project.",
        )
        .field("moved", String, "the project")
        .field("journal", String, "what its journal is called")
        .field("rows", Object, "how many rows went, by table")
        .field(
            "beside",
            Object,
            "how much of what is kept beside the rows went, by the name it is kept under",
        )
        .field("at", Int, "where the giver of numbers stood: what was said of it before is at or below")
        .field("took_ms", Int, "how long it took"),
        EventSpec::new(
            "system.move_cleared",
            "What a move of a project to a journal of its own left when it was cut off (a file that no \
             project's journal is kept in) was found and taken away, as the daemon started.",
        )
        .field("files", Int, "how many files")
        .field("bytes", Int, "how big they were together"),
        EventSpec::new(
            "system.restarting",
            "A step asked for the daemon to be stopped and started again by what keeps it running.",
        )
        .field("at_rest", Bool, "whether it waited first until nothing else was under way"),
    ];
    // Where a copy of the factory was written, and what was done to its
    // files, is its owner's to know.
    blueprint::events::heard(
        all,
        &[
            ("system.snapshot", "admin"),
            ("system.copied", "admin"),
            ("system.reindexed", "admin"),
            ("system.repack_cleared", "admin"),
            ("system.repacked", "admin"),
            ("system.moved_apart", "admin"),
            ("system.move_cleared", "admin"),
            // That the daemon is to come back is for anyone who is signed
            // in, as its starting and stopping are: every page goes by them.
            ("system.restarting", ANYONE),
        ],
    )
}
