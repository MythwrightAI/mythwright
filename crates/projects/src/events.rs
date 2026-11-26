//! The events said of a project, its files and the Trash's rows, where
//! they are said. The studio hands them out among its own where they
//! always were (`studio::events`).

use blueprint::{EventSpec, ValueType::*};

pub fn events() -> Vec<EventSpec> {
    let trashed = |topic: &str, summary: &str| {
        EventSpec::new(topic, summary)
            .field("id", String, "what it is in the Trash")
            .field(
                "kind",
                String,
                "`record`, a row of a table or a card; `file`, a file or a folder of a project; or `project`",
            )
            .field(
                "thing",
                String,
                "what it is known by: a record's id, a file's path in its project, a project's id",
            )
            .maybe("project", String, "the project it is of; not said of a project, which is the factory's")
    };
    let all = vec![
        EventSpec::new("project.created", "A project was made, or a folder was taken up as one.")
            .field("project", String, "the project's id")
            .field("name", String, "what it is called")
            .field("path", String, "where its files are")
            .field("brownfield", Bool, "the folder was there before, with files in it")
            .maybe("committed", String, "its first commit, short: the files the factory started it with, committed as it was made")
            .maybe("restored", Bool, "that it was given back from the Trash, where it was forgotten"),
        EventSpec::new("project.removed", "A project was taken away.").field("project", String, "its id"),
        EventSpec::new(
            "project.renamed",
            "A project was called something else: what people read it as. Its id, which everything knows it by, is \
             as it was. Kept with its row, in the factory's own journal.",
        )
        .field("project", String, "its id")
        .field("name", String, "what it is called now")
        .field("was", String, "what it was called")
        .field("by", String, "who called it so, as an actor"),
        EventSpec::new(
            "project.trusted",
            "A project's own files may stand in for the blueprints that ship: its factory's owner said so.",
        )
        .field("project", String, "its id"),
        EventSpec::new("project.untrusted", "That was taken back: the blueprints that ship are used there.")
            .field("project", String, "its id"),
        EventSpec::new("project.file.changed", "A file of a project was added, changed or removed.")
            .field("project", String, "the project")
            .field("path", String, "the file, from the top of the project")
            .field("folders", List, "every folder it is in, the outermost first, each from the top of the project")
            .field("kind", String, "what kind of file it is by its name: what follows the last dot of it, and nothing where there is none")
            .field("change", String, "`added`, `changed` or `removed`"),
        EventSpec::new("project.folder.made", "A folder was made in a project, empty, through the factory.")
            .field("project", String, "the project")
            .field("path", String, "the folder, from the top of the project")
            .field("folders", List, "every folder it is in, the outermost first, each from the top of the project"),
        EventSpec::new(
            "project.rescanned",
            "So many files of a project changed that they are not each said.",
        )
        .field("project", String, "the project")
        .field("changed", Int, "how many"),
        trashed("trash.put", "Something was deleted, and is kept in the Trash until it is given back or goes for good."),
        trashed("trash.restored", "Something was given back from the Trash, where it was.")
            .maybe("to", String, "where a file is now, which is where it was unless it was given back beside what is there")
            .maybe("with", Int, "how many records came back with it, which went into the Trash with it: the pages under a page")
            .maybe("said", String, "where it came to be, where that is not where it was: a page whose page above it is gone, given back as a wiki of its own"),
        trashed("trash.gone", "Something in the Trash was deleted for good.")
            .field("why", String, "`asked`, where somebody deleted it for good; `days`, where it had been there as long as the factory keeps what is deleted; `settled`, where a daemon finished what one stopped in the middle of; `project`, where its project was deleted for good"),
    ];
    // Who may be told of each: whoever may read what it is about.
    blueprint::events::heard(all, &[("project", "projects:read"), ("trash", "projects:read")])
}
