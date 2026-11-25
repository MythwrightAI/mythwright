//! What an action says is written inside one of its arguments: a table of
//! rows read into what is handed out, and written back as the same rows.

use blueprint::inside::{Row, Within};
use blueprint::{ActionSpec, ValueType};
use serde_json::json;

/// A table in each way one is said: things by a name that each take
/// keys, one of whose keys is one of a few words and another holds things
/// by a name that are each a word; and under a key, keys taken directly.
const TABLE: &[Row] = &[
    ("*", "each", "a thing of its writer's naming"),
    ("*.kind", "string", "what kind of thing it is"),
    ("*.kind.plain", "word", "nothing more than its name"),
    ("*.kind.marked", "word", ""),
    ("*.quiet", "bool", "nothing is said of a change to it"),
    ("*.marks", "object", "the mark each of its parts has"),
    ("*.check", "object", "how it is looked at"),
    ("*.marks.*", "each", "a part, with its mark"),
    ("*.marks.*.round", "word", "a ring"),
    ("*.marks.*.square", "word", "a box"),
    ("*.check.every", "duration", "how often"),
    ("*.check.tries", "int", "how many times"),
];

/// A table as what is read gives it back: the same rows, owned.
fn owned(table: &[Row]) -> Vec<(String, String, String)> {
    table.iter().map(|(path, kind, says)| (path.to_string(), kind.to_string(), says.to_string())).collect()
}

#[test]
fn a_table_of_rows_is_read_into_what_is_handed_out_and_written_back_as_the_same_rows() {
    let within = Within::of(TABLE);
    assert_eq!(within.rows(), owned(TABLE));
    assert_eq!(within.each.as_deref(), Some("a thing of its writer's naming"));
    let names: Vec<&str> = within.keys.iter().map(|key| key.name.as_str()).collect();
    assert_eq!(names, ["kind", "quiet", "marks", "check"]);
    let kind = &within.keys[0];
    assert_eq!((kind.ty, kind.words.len(), kind.words[1].help.as_str()), (ValueType::String, 2, ""));
    let marks = within.keys[2].within.as_ref().unwrap();
    assert_eq!(
        (marks.each.as_deref(), marks.keys.len(), marks.words.len()),
        (Some("a part, with its mark"), 0, 2)
    );
    let check = within.keys[3].within.as_ref().unwrap();
    assert_eq!((check.each.as_deref(), check.keys.len()), (None, 2));
}

#[test]
fn a_row_that_fits_nowhere_is_left_out_and_the_rest_are_read() {
    let cases: &[(Row, &str)] = &[
        (("*.missing.every", "duration", "how often"), "its path goes through a key no row said"),
        (("*.quiet.missing.every", "duration", "how often"), "the same, further down"),
        (("*.quiet.*", "each", "a part"), "nothing: a key may hold things by name"),
        (("kind", "each", "a thing"), "what each thing is, said of what is no `*`"),
        (("*.quiet", "sometimes", "now and then"), "a kind that is no kind"),
        (("*.missing.round", "word", "a ring"), "a word of a key no row said"),
        (("plain", "word", "nothing more"), "a word of nothing"),
        (("", "bool", "nothing"), "nothing: a key may be called anything"),
    ];
    for (row, why) in cases {
        let mut table = TABLE.to_vec();
        table.push(*row);
        let read = Within::of(&table).rows();
        let kept = read.len() > TABLE.len();
        assert_eq!(kept, why.starts_with("nothing:"), "{row:?}: {why}; read {read:?}");
        if !kept {
            assert_eq!(read, owned(TABLE), "the rows before it are as they were");
            assert_eq!(Within::of(&table), Within::of(TABLE), "and nothing is left of it");
        }
    }
    // Things by a name are said before what each takes: a `*` nobody said is no place.
    assert_eq!(Within::of(&[("*.kind", "string", "what kind")]), Within::default());
}

#[test]
fn an_action_says_it_of_an_argument_and_it_is_handed_out_only_where_it_is_said() {
    let spec = ActionSpec::new("thing.make", "Make a thing.")
        .required("name", ValueType::String, "what it is called")
        .arg("parts", ValueType::Object, "its parts, each by its name")
        .within("parts", TABLE)
        .within("nothing", TABLE);
    assert_eq!(spec.args[1].within.as_ref(), Some(&Within::of(TABLE)));
    let handed = serde_json::to_value(&spec).unwrap();
    assert_eq!(
        handed["args"][0],
        json!({ "name": "name", "type": "string", "required": true, "help": "what it is called" })
    );
    let within = &handed["args"][1]["within"];
    assert_eq!(within["each"], "a thing of its writer's naming");
    assert_eq!(
        within["keys"][0],
        json!({
            "name": "kind", "type": "string", "help": "what kind of thing it is",
            "words": [{ "name": "plain", "help": "nothing more than its name" }, { "name": "marked", "help": "" }],
        })
    );
    assert_eq!(
        within["keys"][1],
        json!({ "name": "quiet", "type": "bool", "help": "nothing is said of a change to it" })
    );
    assert_eq!(within["keys"][2]["within"]["words"][0], json!({ "name": "round", "help": "a ring" }));
    assert_eq!(within["keys"][3]["within"]["keys"][1]["type"], "int");
}

#[test]
fn the_schema_says_what_is_written_inside_an_argument_and_holds_nothing_to_it() {
    let spec = ActionSpec::new("thing.make", "Make a thing.")
        .arg("parts", ValueType::Object, "its parts, each by its name")
        .within("parts", TABLE)
        .arg("rows", ValueType::List, "its rows")
        .within("rows", &[("title", "string", "what a row is called")]);
    let schema = blueprint::schema::schema(&[spec]);
    let taken = &schema["definitions"]["what-an-action-takes"]["allOf"][0]["then"]["properties"]["with"];
    let parts = &taken["properties"]["parts"];
    assert_eq!(parts["description"], "its parts, each by its name");
    assert_eq!(
        parts["type"],
        json!(["object", "string"]),
        "the argument is what it was: a mapping, or an expression"
    );
    let each = &parts["additionalProperties"];
    assert_eq!(each["description"], "a thing of its writer's naming");
    assert!(
        each.get("type").is_none() && each.get("additionalProperties").is_none(),
        "a thing is held to nothing"
    );
    let kind = &each["properties"]["kind"];
    assert_eq!(kind["description"], "what kind of thing it is");
    assert_eq!(
        kind["anyOf"],
        json!([{ "const": "plain", "description": "nothing more than its name" }, { "const": "marked" }, {}]),
        "each of its words with what it means, and anything else"
    );
    assert_eq!(each["properties"]["quiet"], json!({ "description": "nothing is said of a change to it" }));
    let marks = &each["properties"]["marks"]["additionalProperties"];
    assert_eq!(marks["anyOf"][1], json!({ "const": "square", "description": "a box" }));
    assert_eq!(each["properties"]["check"]["properties"]["tries"]["description"], "how many times");
    // A list of mappings is said as one of them is.
    assert_eq!(
        taken["properties"]["rows"]["items"]["properties"]["title"]["description"],
        "what a row is called"
    );
}

/// A `wait` says an event as `emit` says one, by the one function, so
/// what its `say` is said to take is what `emit` takes by name, of the
/// same kinds, with whatever else is written carried as `emit` carries it.
#[test]
fn what_a_wait_is_said_to_say_is_what_emit_takes() {
    let wait = blueprint::intrinsic("wait").unwrap();
    let say = wait.args.iter().find(|arg| arg.name == "say").unwrap().within.clone().unwrap();
    let emit = blueprint::intrinsic("emit").unwrap();
    let said: Vec<(&str, ValueType)> = say.keys.iter().map(|key| (key.name.as_str(), key.ty)).collect();
    let taken: Vec<(&str, ValueType)> = emit.args.iter().map(|arg| (arg.name.as_str(), arg.ty)).collect();
    assert_eq!(said, taken);
    assert!(say.each.is_none() && emit.open);
}
