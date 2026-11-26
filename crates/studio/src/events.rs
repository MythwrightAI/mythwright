//! The events the studio emits: what happens to a project, its files, its
//! channels, its board and whatever else it keeps.

use blueprint::{EventSpec, ValueType::*};

pub fn events() -> Vec<EventSpec> {
    let carried = "what the record holds of each field its table says is `said` with it, by the field's name: a text \
                   longer than a kilobyte cut there, and anything else that long left out; left out where its \
                   table carries none";
    let card = |topic: &str, summary: &str| {
        EventSpec::new(topic, summary)
            .field("id", String, "the card")
            .field("project", String, "the project whose board it is on")
            .maybe(crate::fieldsaid::KEY, Object, carried)
    };
    let record = |topic: &str, summary: &str| {
        EventSpec::new(topic, summary)
            .field("id", String, "the record")
            .field("project", String, "the project it is kept for")
            .field("collection", String, "the collection it is a record of")
            .maybe("through", String, "the blueprint that did it, when one did")
            .maybe(crate::fieldsaid::KEY, Object, carried)
    };
    let doing = "which of the things done under the name of the blueprint's step that did it this was \
                 (its action, at which try, for which of a list), which the step looks for where it is done again \
                 after the daemon stopped in the middle of it; left out where no step did it";
    let wrote = "the agent whose work the blueprint that did it did it from, as an actor (`agent:<name>`), where the \
                 blueprint was told that";
    let through = "the blueprint that did it, when one did";
    let all = vec![
        EventSpec::new(
            "collection.exported",
            "A library was written out as pages. It is said when something was written, not when what was there was already so.",
        )
        .field("project", String, "the project")
        .field("name", String, "the library")
        .field("to", String, "the folder it was written to, from the top of the project")
        .field("pages", Int, "how many pages")
        .field("files", Int, "how many other files")
        .field("left_out", Int, "how many links and pictures were written as their words"),
        EventSpec::new("workspace.created", "A workspace was made: a name for the projects that belong together.")
            .field("workspace", String, "the workspace's id")
            .field("name", String, "what it is called"),
        EventSpec::new("workspace.renamed", "A workspace is called something else. Its id is what it was.")
            .field("workspace", String, "the workspace's id")
            .field("name", String, "what it is called now")
            .field("was", String, "what it was called"),
        EventSpec::new(
            "workspace.removed",
            "A workspace was taken away. What was in it is in another, and is otherwise as it was.",
        )
        .field("workspace", String, "the workspace's id")
        .field("name", String, "what it was called")
        .field("projects", List, "the projects that were in it")
        .field("channels", List, "the channels that were its own")
        .maybe("to", String, "the workspace what was in it is in now, where it was said where that goes"),
        EventSpec::new(
            "project.placed",
            "A project was put in a workspace: as it was made, out of another, or out of none by a factory \
             that an earlier program made. It is always in one. Kept with its place, in the factory's own \
             journal, and heard as the project's.",
        )
        .field("project", String, "the project")
        .maybe("workspace", String, "the workspace it is in now")
        .maybe("was", String, "the workspace it was in, where it was in one"),
        EventSpec::new(
            "channel.placed",
            "What a channel belongs to was said: a project or a workspace, and never both. Said as one that \
             is no project's is made, as it is said to be another's, and as what it was of is taken away.",
        )
        .field("channel", String, "the channel's id")
        .field("name", String, "what it is called")
        .maybe("project", String, "the project it belongs to now")
        .maybe("workspace", String, "the workspace it belongs to now"),
        EventSpec::new("board.columns", "A board's columns were set.")
            .field("project", String, "the project")
            .field("columns", List, "the columns, left to right"),
        card("board.item.created", "A card was added to a board.")
            .field("title", String, "its title")
            .field("status", String, "the column it was put in")
            .maybe(
                "by",
                String,
                "who added it, as an actor: the key, where it was added with a key somebody made; not said of one \
                 said late, of which it is not known",
            )
            .maybe(
                "late",
                Bool,
                "that it is said now of a card added earlier: the daemon stopped between writing it down and \
                 saying it (`board.settle`)",
            )
            .maybe("restored", Bool, "that it was given back from the Trash, where it was in its column before")
            .maybe("through", String, through)
            .maybe("wrote", String, wrote)
            .maybe("doing", String, doing),
        card("board.item.moved", "A card was moved to another column.")
            .field("title", String, "its title")
            .field("from", String, "the column it was in")
            .field("to", String, "the column it is in now")
            .field("by", String, "who moved it, as an actor")
            .maybe("note", String, "what was said as it was moved")
            .maybe("decided", String, "the agent whose decision the move was (`agent:<name>`), where a blueprint moved it as an agent decided")
            .maybe("uncounted", Bool, "that a blueprint sent it back for something other than a verdict on its work (to be brought up to date), which is not counted among the times it was sent back")
            .maybe(
                "late",
                Bool,
                "that it is said now of a move made earlier: the daemon stopped between writing it down and \
                 saying it, and who made it is not known (`board.settle`)",
            )
            .maybe("through", String, through),
        card("board.item.updated", "Something on a card was changed, or its place in its column.")
            .maybe("title", String, "its title")
            .field("status", String, "the column it is in")
            .maybe("assignee", String, "who it is assigned to")
            .field("changed", List, "the names of what changed")
            .field("reworded", Bool, "whether its title or its body is among them: what the card asks")
            .field(
                "by",
                String,
                "who changed it, as an actor: the key, where it was changed with a key somebody made",
            )
            .maybe("through", String, through)
            .maybe("decided", String, "the agent whose decision it was (`agent:<name>`), where a blueprint moved it where it was as an agent decided")
            .maybe("approved_by", String, "who approved it, as an actor, where it was changed as whoever asked for it (`record.asked`)")
            .maybe("asked", String, "the ask it was approved by"),
        card("board.item.removed", "A card was taken off its board."),
        card("board.item.pressed", "Something a card's column offers was pressed on it, or a press was taken back.")
            .field("status", String, "the column it is in, whose row was pressed")
            .field("action", String, "what was pressed, by its name")
            .field("title", String, "what it is called")
            .field("by", String, "who pressed it, as an actor: the key, where it was pressed with a key somebody made")
            .field("name", String, "what whoever pressed it is called")
            .field("coming", Int, "which time the card came to the column it is in: the count of its moves")
            .field("back", Bool, "the press was taken back")
            .field("did", String, "what its history says was done")
            .field("released", Bool, "the card was held, and this was the last of what held it: it is due what its column does")
            .field("waits", List, "what still holds it, by what each is called")
            .maybe("through", String, through),
        card("board.item.held", "A card waits where it is for something its column offers to be pressed: said once each time it comes there.")
            .field("card", String, "the card, as what waits for a person says it")
            .field("status", String, "the column it waits in")
            .field("coming", Int, "which time it came there: the count of its moves")
            .field("waits", List, "what is to be pressed, by what each is called")
            .field("who", List, "who may press each, in words")
            .field("says", String, "the sentence that says so")
            .maybe("through", String, through),
        record("record.created", "A record was added to a collection.")
            .maybe("restored", Bool, "that it was given back from the Trash")
            .maybe("with", String, "the record it was given back with, which it went into the Trash with: the page it is under, at any depth, of a page given back with the pages under it")
            .maybe("wrote", String, wrote)
            .maybe("doing", String, doing),
        record("record.updated", "A record's fields were changed.")
            .field("changed", List, "the names of the fields that changed")
            .maybe("approved_by", String, "who approved it, as an actor, where it was changed as whoever asked for it once somebody who could make it approved (`record.asked`)")
            .maybe("asked", String, "the ask it was approved from, by its id")
            .maybe("chose", Object, "what each changed field that is a choice now says, by its name; left out where none of them is one")
            .maybe("flags", Object, "what each changed field that is a flag now says, by its name: `true`, `false`, or nothing where it was emptied; left out where none of them is one"),
        record("record.removed", "A record was taken away.")
            .maybe("with", String, "the record it was taken away with, in the same write: the page it is under, at any depth, of a page deleted with the pages under it")
            .maybe("trash", String, "what the record it went with is in the Trash, which gives it back with it; left out where it went at once")
            .maybe("into", String, "where it was moved, `the Trash`, for its history to say; left out where it went at once"),
        record("record.reworded", "A number a record held came to a word, as its field was declared a choice that says what nothing stands for: once, as the table is declared so.")
            .field("field", String, "the field, which was a number and is a choice")
            .field("was", Number, "the number it held")
            .field("now", String, "the word it holds now: as many places above the one nothing stands for as the number was above nought, below it for less, and the first or the last where it went past them"),
        record("record.commented", "Something was said on a record.")
            .field("body", String, "what was said")
            .field("by", String, "who said it, as an actor: the key, where it was said with a key somebody made")
            .maybe(
                "public",
                Bool,
                "that whoever said it marked it public; left out, it is private, and goes nowhere outside the factory",
            )
            .maybe(
                "anchor",
                Object,
                "the words of a document it is about (`quote`, with `before`, `after` and `under`, the headings over them); left out, it is about the whole record",
            )
            .maybe(
                "decided",
                String,
                "the agent whose words a blueprint said (`agent:<name>`): a question it asked, which the record's history says is that agent's",
            )
            .maybe("once", String, "which saying a blueprint's step says it is, which the step done again looks for")
            .maybe("doing", String, doing)
            .maybe("asker", String, "who asked a question a blueprint said: an agent's name, or a record that asks, which the bot speaks for")
            .maybe(
                "parent",
                Int,
                "what it answers: the number something said on the same record was said at, a reply being put beside the one it answers where replies would go deeper than eight; left out, it answers nothing",
            ),
        record("record.attached", "Something was attached to a record: a file kept beside it, read by whoever may read the record.")
            .field("attachment", String, "what is attached, by its id")
            .field("name", String, "what it is called")
            .field("media", String, "what kind of thing it is, as a media type: `image/png`")
            .field("size", Int, "how big it is, in bytes")
            .field("by", String, "who attached it, as an actor: the key, where it was attached with a key somebody made")
            .maybe("restored", Bool, "that it was given back from the Trash, and not attached anew"),
        record("record.detached", "Something attached to a record was taken away, and its file with it.")
            .field("attachment", String, "what was attached, by its id")
            .field("name", String, "what it was called")
            .field("media", String, "what kind of thing it was")
            .field("size", Int, "how big it was, in bytes")
            .field("by", String, "who attached it, as an actor")
            .maybe("trash", String, "what it is in the Trash, where it was moved to be given back from; left out where the factory keeps nothing that is deleted, and it went at once")
            .maybe("into", String, "where it was moved, `the Trash`, for its record's history to say; left out where it went at once"),
        EventSpec::new("collection.ensured", "A collection was declared, or declared again.")
            .field("project", String, "the project")
            .field("name", String, "the collection")
            .field("store", String, "where its records are kept: `local`, `files` or `service`")
            .maybe("at", String, "the address of its store service")
            .maybe("owner", String, "the instance whose blueprint declared it")
            .field("made", Bool, "it was not there before"),
        EventSpec::new("collection.views", "The views kept of a collection changed.")
            .field("project", String, "the project")
            .field("name", String, "the collection")
            .field("view", String, "the view that was saved or taken away"),
        EventSpec::new("collection.icon", "Somebody chose the icon a field's column is drawn with, or took their choice away.")
            .field("project", String, "the project")
            .field("name", String, "the collection")
            .field("field", String, "the field")
            .maybe("icon", String, "the icon's name; left out where the choice was taken away"),
        EventSpec::new(
            "collection.titled",
            "Somebody called a table something else for people, or took that away: its name, which everything else \
             knows it by, is as it was.",
        )
        .field("project", String, "the project")
        .field("name", String, "the collection, by its name")
        .maybe("title", String, "what it is called now; left out where it is called by its name again")
        .field("by", String, "who called it so, as an actor"),
        EventSpec::new(
            "collection.fields",
            "Somebody who looks at a collection changed its columns: added one, called one something else, changed what one holds or offers, or took one away.",
        )
        .field("project", String, "the project")
        .field("name", String, "the collection")
        .field("field", String, "the field, by the name records keep it under")
        .field("did", String, "what was done: `added`, `called`, `retyped`, `options` or `removed`"),
        EventSpec::new(
            "collection.indexed",
            "A column of a table was kept in order by an index from here on, so that a table of many rows is read a \
             window at a time by it: as it was first sorted or held by it, or as the table was declared.",
        )
        .field("project", String, "the project")
        .field("name", String, "the collection")
        .field("field", String, "the field, by its name"),
        EventSpec::new("collection.history", "A line was written in a record's history.")
            .field("project", String, "the project")
            .field("name", String, "the collection")
            .field("record", String, "the record")
            .field("seq", Int, "which line of its history this is")
            .field("topic", String, "the event the line is about")
            .field("who", String, "who the line says did it")
            .field("said", String, "the line"),
        EventSpec::new(
            "collection.forgotten",
            "A project's table was taken away, with its records and what was said of each, as the project was \
             deleted for good.",
        )
        .field("project", String, "the project")
        .field("name", String, "the collection")
        .field("store", String, "where its records were kept: `local`, `files` or `service`; files and a service keep theirs"),
        EventSpec::new("search.indexed", "What can be searched was brought up to date.")
            .field("docs", Int, "how many things can be searched")
            .field("written", Int, "how many were read again")
            .field("took_ms", Int, "how long it took"),
        EventSpec::new("search.progress", "How far reading for the search has got.")
            .field("phase", String, "`idle`, `scanning` or `indexing`")
            .field("done", Int, "how many are read")
            .field("total", Int, "how many there are to read")
            .field("docs", Int, "how many things can be searched")
            .passing(),
    ];
    // Who may be told of each: whoever may read what it is about.
    let mut all = blueprint::events::heard(
        all,
        &[
            ("project", "projects:read"),
            ("workspace", "projects:read"),
            ("channel", "channels:read"),
            ("board", "board:read"),
            ("record", "records:read"),
            ("collection", "records:read"),
            ("search", "search"),
        ],
    );
    // What is said of accounts and keys, and of channels, is described
    // where each is said (`people`, `talk`), and is handed out with these
    // where it always was.
    put_before(&mut all, "search.indexed", people::events::events());
    let (made, rest): (Vec<_>, Vec<_>) =
        talk::events::events().into_iter().partition(|spec| spec.topic == "channel.created");
    put_before(&mut all, "workspace.created", made);
    put_before(&mut all, "board.columns", rest);
    // And what is said of projects, their files and the Trash's rows, by
    // what came out of here (`projects`).
    let (trash, project): (Vec<_>, Vec<_>) =
        projects::events::events().into_iter().partition(|spec| spec.topic.starts_with("trash."));
    put_before(&mut all, "collection.exported", project);
    put_before(&mut all, "collection.ensured", trash);
    all
}

/// Puts `events` among `all` just before the one called `topic`, so that
/// what another crate describes is listed where it was when it was
/// described here.
fn put_before(all: &mut Vec<EventSpec>, topic: &str, events: Vec<EventSpec>) {
    let at = all.iter().position(|spec| spec.topic == topic).unwrap_or(all.len());
    all.splice(at..at, events);
}
