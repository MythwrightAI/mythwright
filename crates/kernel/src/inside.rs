//! What is written inside the arguments that declare a table and read its
//! rows, said once: each word a field, a view or a form may say, with its
//! sentence.
//!
//! They are plain tables of rows, each where something is written, what
//! kind of thing it is and a sentence, as an action says what is written
//! inside one of its arguments (`ActionSpec::within`, which reads them;
//! the language's notes on that say how a row is written: `*` is a thing
//! by a name of its writer's choosing, `each` says what each such thing
//! is, and `word` one of the few words the key before it may be). Whoever
//! shows a blueprint explains and offers by them: an editor, the notes,
//! the command line, a schema. Nothing is checked by them; what reads the
//! words is what takes or refuses them, and tests hold each table to that
//! reading, so that a word that is read and not said here, or said here
//! and not read, fails.
//!
//! They are here, and not beside the actions that say them, because that
//! crate is near its ceiling and these are sentences and nothing else.
//! Nothing here knows a table.

/// One row: where, what kind, and the sentence.
pub type Row = (&'static str, &'static str, &'static str);

/// What a table's `fields` hold: each field by its name.
pub const FIELDS: &[Row] = &[
    ("*", "each", "a field of the collection, by its name"),
    ("*.type", "string", "what it holds: `text` where it is not said"),
    ("*.type.text", "word", "words"),
    ("*.type.choice", "word", "one of the words `of` says can be chosen"),
    ("*.type.actor", "word", "somebody: a person or an agent, as in `agent:builder`"),
    ("*.type.list", "word", "several words"),
    ("*.type.number", "word", "a number"),
    ("*.type.flag", "word", "true or false"),
    ("*.type.date", "word", "a day, written `2026-11-30`"),
    ("*.type.record", "word", "a record of the collection `of` names, by its id"),
    ("*.type.records", "word", "several records of the one collection `of` names, by their ids"),
    ("*.type.document", "word", "a document: its Markdown, kept in the row, as a topic's body is"),
    ("*.required", "bool", "a record cannot be without it"),
    (
        "*.of",
        "any",
        "for a choice what can be chosen, in order; for a record or records the collection whose records it names",
    ),
    (
        "*.quiet",
        "bool",
        "its changes are not news: no line of a record's history is written for a change to it alone",
    ),
    ("*.hidden", "bool", "it is left out of a view of the collection until somebody shows it there"),
    (
        "*.said",
        "bool",
        "every event about a record of the collection carries what it holds, under `fields` of the event's data, so \
         that a handler is chosen by it with nothing run: a text longer than a kilobyte cut there, and anything \
         else that long left out. Not of a field that is quiet or says what giving it a value takes",
    ),
    (
        "*.indexed",
        "bool",
        "its column is kept in order by an index from the first row, so that a table of many rows is sorted and \
         held by it at once: any column of a large table is indexed when it is first sorted by, and this says it of \
         one before anybody asks. Not of a field that holds several things",
    ),
    (
        "*.suggests",
        "list",
        "for a text that takes any word, the usual ones, which are offered to whoever fills it",
    ),
    (
        "*.zoned",
        "bool",
        "for words that are a time of day with the zone it is in after it (`02:00 Europe/Zagreb`): whoever writes one \
         with no zone is given their own",
    ),
    (
        "*.calls",
        "number",
        "where it is among the fields a record is called by, from 1, wherever the record is named: an attempt called \
         by its card and its contender, where its title is what something else knows it by",
    ),
    (
        "*.writer_of",
        "string",
        "another field of the table, whose writer it keeps: the factory writes into it who wrote that field each \
         time it is written, and nobody writes it themselves",
    ),
    (
        "*.icon",
        "string",
        "the name of the icon its column is drawn with, until somebody who looks at the table chooses another",
    ),
    (
        "*.takes",
        "any",
        "what giving it a value takes beside what changing the record does: a scope, or a list of scopes every one \
         of which is asked, or for a choice or a flag the scopes of each value that takes any, or `each` for a list \
         of scopes that the record gives somebody, where adding the record or changing anything of it takes holding \
         each scope it then says",
    ),
    (
        "*.unsaid",
        "any",
        "with `takes: each`, the scopes such a list stands for where a record leaves it empty; of a choice, the one \
         word of `of` nothing chosen stands for, which is where nothing ranks and what a number written before the \
         field was a choice is counted from",
    ),
    (
        "*.stands",
        "bool",
        "with `takes`, what giving it its value took is asked again of whoever adds the record or changes any \
         cell of it, for as long as it says that value: for a cell that lets something with more authority do \
         more, beside cells that say what that does",
    ),
    (
        "*.takes.*",
        "each",
        "a value of the field as it is written, with the scope, or the list of scopes, giving the field that value \
         takes",
    ),
];

/// What a table's `views` hold: each way of looking at it by its name.
pub const VIEWS: &[Row] = &[
    ("*", "each", "a way of looking at the collection, by its name"),
    ("*.kind", "string", "how its records are laid out: `table` where it is not said"),
    ("*.kind.table", "word", "rows and columns"),
    ("*.kind.board", "word", "a column for each thing the choice `by` names can be"),
    ("*.kind.list", "word", "a line for each record"),
    ("*.kind.gallery", "word", "a tile for each record"),
    (
        "*.kind.calendar",
        "word",
        "each record on its day: the date field `by` names, or `created` or `updated`, and when it was made where none is said",
    ),
    (
        "*.kind.order",
        "word",
        "its records by what comes first: what can be begun now, what waits only on those, and so on",
    ),
    (
        "*.by",
        "string",
        "what it goes by: for a board the choice field its columns come from, for a calendar the date its records \
         are put on the days of, for an order the field in which a record says which others come before it",
    ),
    ("*.hides", "list", "the fields it leaves out"),
    (
        "*.done",
        "string",
        "for an order, the choice whose last value says a record is done: the collection's first choice where none is said",
    ),
    ("*.group", "string", "for a board, an order or a table, the field its records are gathered by"),
    (
        "*.only",
        "object",
        "what it is held to: a map from a field to the things a record has to hold one of there to be shown",
    ),
    ("*.columns", "list", "its columns in the order they stand"),
    ("*.freezes", "string", "for a table, the column it is frozen up to"),
    ("*.figures", "object", "for a table, a map from a column to the figure at its foot"),
    ("*.widths", "object", "for a table, a map from a column to how wide it is"),
    ("*.only.*", "each", "a field, with the things a record has to hold one of there: a list, or one alone"),
    ("*.figures.*", "each", "a column, with the figure at its foot"),
    ("*.figures.*.filled", "word", "how many rows say something there"),
    ("*.figures.*.empty", "word", "how many rows say nothing there"),
    ("*.figures.*.sum", "word", "what a number's rows add up to"),
    ("*.figures.*.mean", "word", "what a number's rows come to on average"),
    ("*.figures.*.least", "word", "the least a number's rows say"),
    ("*.figures.*.greatest", "word", "the greatest a number's rows say"),
    ("*.figures.*.ticked", "word", "how many rows a flag is ticked in"),
    (
        "*.figures.*.different",
        "word",
        "how many different things are said, of what is neither a number nor a flag",
    ),
    (
        "*.widths.*",
        "each",
        "a column, with how wide it is: whole pixels at the usual size of text, from 60 to 2000",
    ),
];

/// What a cell is given where rows are read: by cell, a value or a form.
/// `fill` and `giving` take the same forms.
pub const GIVEN: &[Row] = &[
    ("*", "each", "a cell, with what it is given: a value, or a form that says how the value is come by"),
    ("*.like", "string", "another cell of the row: what that one says"),
    (
        "*.else",
        "any",
        "with `like`, what is given where that cell says nothing, or a number not above nothing: a value or a form",
    ),
    ("*.slug_of", "string", "another cell of the row: what that one says, as a name is written"),
    (
        "*.by",
        "string",
        "another cell of the row: the form written beside what that one says is taken, as in `{ by: does, review: 3, otherwise: 1 }`",
    ),
    ("*.otherwise", "any", "with `by`, the form taken where nothing is written beside what the cell says"),
    (
        "*.share",
        "any",
        "with `each`, how many times the one goes into the other, never fewer than one: what is shared out, a value or a form",
    ),
    ("*.each", "any", "with `share`, what each is given of it: a value or a form"),
    (
        "*.least",
        "list",
        "forms, the least of whose numbers is given: one that says nothing, or a number not above nothing, is passed over",
    ),
];

/// What `counting` holds where rows are read: figures over other rows.
pub const COUNTING: &[Row] = &[
    ("*", "each", "a figure over the rows of another list, by the name each row is given it under"),
    ("*.of", "list", "the rows it is worked out over"),
    (
        "*.by",
        "string",
        "the cell in which those rows say what the row says: it is worked out over those that say the same there",
    ),
    (
        "*.sum",
        "string",
        "the cell of those rows whose numbers are added up; how many of them there are, where it is not said",
    ),
    ("*.unless", "string", "a cell of those rows: one in which it is so is left out"),
];

/// What `gathering` holds where rows are read: lists of what a cell says.
pub const GATHERING: &[Row] = &[
    ("*", "each", "a list of what one cell says, by the name it is given out under"),
    ("*.of", "string", "the cell"),
    (
        "*.saying",
        "object",
        "what a row is to say to be in it, by cell, as `saying` is written; every row, where it is not said",
    ),
];

/// What `saying` holds where rows are read: what a row is to say, by cell.
pub const SAYING: &[Row] = &[
    ("*", "each", "a cell, with what a row is to say in it: a value for exactly that, or a form"),
    ("*.not", "any", "anything but this value"),
    ("*.in", "list", "any one of these values"),
    ("*.said", "bool", "`true` for something, `false` for nothing"),
];
