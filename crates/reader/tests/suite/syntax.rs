use reader::syntax::{SyntaxError, parse};
use serde_json::{Value, json};

fn data(source: &str) -> Value {
    parse(source).unwrap_or_else(|err| panic!("{}: {}", err.span.start.line, err.message)).to_json()
}

fn error(source: &str) -> SyntaxError {
    parse(source).expect_err("expected a syntax error")
}

#[test]
fn block_mappings_and_sequences_nest() {
    let source = "\
name: demo
steps:
  build:
    needs: [a, b]
    args:
      - one
      - two: 2
        three: 3
  empty:
";
    assert_eq!(
        data(source),
        json!({
            "name": "demo",
            "steps": {
                "build": { "needs": ["a", "b"], "args": ["one", { "two": 2, "three": 3 }] },
                "empty": null,
            }
        })
    );
}

#[test]
fn a_list_may_sit_at_its_keys_indentation() {
    assert_eq!(data("items:\n- a\n- b\nafter: 1\n"), json!({ "items": ["a", "b"], "after": 1 }));
}

#[test]
fn only_true_false_and_null_are_special_words() {
    let parsed = data("a: yes\nb: no\nc: on\nd: true\ne: null\nf: ~\ng: False\n");
    assert_eq!(
        parsed,
        json!({ "a": "yes", "b": "no", "c": "on", "d": true, "e": null, "f": null, "g": "False" })
    );
}

#[test]
fn numbers_are_typed_and_near_numbers_stay_text() {
    let parsed = data("a: 42\nb: -7\nc: 1.5\nd: 1.2.3\ne: 5m\nf: \"42\"\ng: 0x10\n");
    assert_eq!(
        parsed,
        json!({ "a": 42, "b": -7, "c": 1.5, "d": "1.2.3", "e": "5m", "f": "42", "g": "0x10" })
    );
}

#[test]
fn comments_are_dropped_but_a_hash_inside_a_word_is_kept() {
    let parsed = data("# heading\na: one # trailing\nb: issue#12\n\nc: \"quoted # kept\"\n");
    assert_eq!(parsed, json!({ "a": "one", "b": "issue#12", "c": "quoted # kept" }));
}

#[test]
fn an_expression_is_one_token_wherever_it_appears() {
    let source = "\
when: ${ a.b == \"x: y\" ? 1 : 2 } # why
with: { name: ${ {\"k\": 1}.k }, port: ${p.port}, list: [${a}, ${b ? c : d}] }
text: before ${ x } after
";
    assert_eq!(
        data(source),
        json!({
            "when": "${ a.b == \"x: y\" ? 1 : 2 }",
            "with": { "name": "${ {\"k\": 1}.k }", "port": "${p.port}", "list": ["${a}", "${b ? c : d}"] },
            "text": "before ${ x } after",
        })
    );
}

#[test]
fn a_value_may_contain_a_colon() {
    let parsed = data("run: echo \"a: b\"\nurl: http://localhost:8080/x\n");
    assert_eq!(parsed, json!({ "run": "echo \"a: b\"", "url": "http://localhost:8080/x" }));
}

#[test]
fn a_backslash_in_double_quotes_that_begins_no_escape_is_refused_saying_how_such_a_path_is_written() {
    let refused = error("home: \"C:\\Users\\me\"\n");
    assert!(refused.message.starts_with("unknown escape `\\U`"), "{}", refused.message);
    assert!(refused.message.contains("single quotes"), "it says what to write: {}", refused.message);
    // In single quotes a backslash is itself, and written twice in double quotes it is one.
    for written in ["'C:\\Users\\me'", "\"C:\\\\Users\\\\me\""] {
        assert_eq!(data(&format!("home: {written}\n")), json!({ "home": "C:\\Users\\me" }), "{written}");
    }
}

#[test]
fn quoted_strings_unescape() {
    let parsed = data("a: \"line\\nnext \\\"q\\\" \\u00e9\"\nb: 'it''s'\n'k: 1': v\n");
    assert_eq!(parsed, json!({ "a": "line\nnext \"q\" é", "b": "it's", "k: 1": "v" }));
}

#[test]
fn literal_blocks_keep_lines_and_folded_blocks_join_them() {
    let source = "\
script: |
  set -e
  cargo build

  echo done
stripped: |-
  no newline
folded: >
  one
  two

  three
after: 1
";
    assert_eq!(
        data(source),
        json!({
            "script": "set -e\ncargo build\n\necho done\n",
            "stripped": "no newline",
            "folded": "one two\nthree\n",
            "after": 1,
        })
    );
}

#[test]
fn a_block_scalar_keeps_comment_looking_lines() {
    assert_eq!(data("s: |\n  # not a comment\n  x\n"), json!({ "s": "# not a comment\nx\n" }));
}

#[test]
fn flow_collections_may_span_lines() {
    let source = "with: {\n  a: 1,   # first\n  b: [x,\n      y],\n}\nnext: 2\n";
    assert_eq!(data(source), json!({ "with": { "a": 1, "b": ["x", "y"] }, "next": 2 }));
}

#[test]
fn spans_point_at_what_was_read() {
    let tree = parse("a: 1\nsteps:\n  build:\n    do: shell\n").unwrap();
    let steps = &tree.as_map().unwrap()[1];
    assert_eq!((steps.key_span.start.line, steps.key_span.start.col), (2, 1));
    let build = &steps.value.as_map().unwrap()[0];
    assert_eq!((build.key_span.start.line, build.key_span.start.col), (3, 3));
    let action = &build.value.as_map().unwrap()[0].value;
    assert_eq!((action.span.start.line, action.span.start.col, action.span.end.col), (4, 9, 14));
}

#[test]
fn mistakes_are_refused_by_name() {
    let cases: &[(&str, &str)] = &[
        ("a: 1\na: 2\n", "duplicate key `a`"),
        ("a:\n\tb: 1\n", "tabs cannot be used"),
        ("a: &x 1\n", "anchors"),
        ("a: *x\n", "aliases"),
        ("a: !!str 1\n", "tags"),
        ("<<: {a: 1}\n", "merge keys"),
        ("---\na: 1\n", "multiple documents"),
        ("- a\n- b\n", "a mapping, not a list"),
        ("a: [1, 2\n", "never closed"),
        ("a: \"open\n", "never closed"),
        ("a: ${ open\n", "never closed"),
        ("a: 1\n  b: 2\n", "unexpected indentation"),
        ("just text\n", "expected `key: value`"),
        ("a: \"x\" y\n", "unexpected text after the value"),
        ("", "empty"),
    ];
    for (source, expected) in cases {
        let message = error(source).message;
        assert!(message.contains(expected), "{source:?} gave {message:?}, wanted {expected:?}");
    }
}

#[test]
fn an_error_says_where() {
    let err = error("ok: 1\nbad: [1,\n");
    assert_eq!(err.span.start.line, 3, "the unclosed bracket is reported at the end of input");
    let err = error("a: 1\nb: 2\na: 3\n");
    assert_eq!((err.span.start.line, err.span.start.col), (3, 1));
}

#[test]
fn windows_line_endings_are_accepted() {
    assert_eq!(data("a: 1\r\nb:\r\n  - x\r\n"), json!({ "a": 1, "b": ["x"] }));
}
