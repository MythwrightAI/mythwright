use reader::template::{self, Cond, Template};
use reader::{duration, expr};
use serde_json::{Map, Value, json};

fn scope() -> Map<String, Value> {
    json!({
        "params": { "port": 7379, "name": "kv", "tags": ["a", "b"], "ratio": 0.5 },
        "state": { "builds": 2, "stable": null },
        "steps": {
            "build": { "ok": true, "failed": false, "out": { "code": 0, "lines": ["x", "y", "z"] } },
            "check": { "ok": false, "out": { "ok": false, "free": 100 } },
        },
        "event": { "topic": "board.item.moved", "data": { "to": "ready" } },
    })
    .as_object()
    .cloned()
    .unwrap()
}

fn eval(source: &str) -> Value {
    expr::parse(source)
        .unwrap_or_else(|err| panic!("{source}: {} at {}", err.message, err.offset))
        .eval(&scope())
        .unwrap_or_else(|err| panic!("{source}: {err}"))
}

#[test]
fn paths_read_the_scope_and_missing_paths_are_null() {
    assert_eq!(eval("params.port"), json!(7379));
    assert_eq!(eval("steps.build.out.lines[1]"), json!("y"));
    assert_eq!(eval("steps.build.out.lines[-1]"), json!("z"));
    assert_eq!(eval("steps[\"build\"].out.code"), json!(0));
    assert_eq!(eval("steps.nope.out.code"), Value::Null);
    assert_eq!(eval("steps.build.out.lines[9]"), Value::Null);
}

#[test]
fn arithmetic_keeps_whole_numbers_whole() {
    assert_eq!(eval("state.builds + 1"), json!(3));
    assert_eq!(eval("7 / 2"), json!(3.5));
    assert_eq!(eval("8 / 2"), json!(4));
    assert_eq!(eval("7 % 4"), json!(3));
    assert_eq!(eval("-params.port + 1"), json!(-7378));
    assert_eq!(eval("2 + 3 * 4"), json!(14));
    assert_eq!(eval("(2 + 3) * 4"), json!(20));
    assert_eq!(eval("params.ratio * 2"), json!(1.0));
}

#[test]
fn comparison_and_logic() {
    assert_eq!(eval("params.port == 7379 && params.name != \"x\""), json!(true));
    assert_eq!(eval("!steps.check.out.ok"), json!(true));
    assert_eq!(eval("steps.check.out.free < 512 || false"), json!(true));
    assert_eq!(eval("1 == 1.0"), json!(true));
    assert_eq!(eval("\"abc\" < \"abd\""), json!(true));
    assert_eq!(eval("\"b\" in params.tags"), json!(true));
    assert_eq!(eval("event.data.to == 'ready'"), json!(true));
}

#[test]
fn ternary_and_coalesce() {
    assert_eq!(eval("steps.build.ok ? \"go\" : \"stop\""), json!("go"));
    assert_eq!(eval("state.stable ?? \"none\""), json!("none"));
    assert_eq!(eval("state.builds ?? 0"), json!(2));
    assert_eq!(eval("false ? 1 : true ? 2 : 3"), json!(2));
}

#[test]
fn text_and_lists_concatenate() {
    assert_eq!(eval("params.name + \"-g\" + (state.builds + 1)"), json!("kv-g3"));
    assert_eq!(eval("params.tags + [\"c\"]"), json!(["a", "b", "c"]));
}

#[test]
fn functions() {
    assert_eq!(eval("len(params.tags)"), json!(2));
    assert_eq!(eval("len(state.stable)"), json!(0));
    assert_eq!(eval("contains(params.name, \"k\")"), json!(true));
    assert_eq!(eval("upper(params.name)"), json!("KV"));
    assert_eq!(eval("join(params.tags, \",\")"), json!("a,b"));
    assert_eq!(eval("split(\"a,b\", \",\")[1]"), json!("b"));
    assert_eq!(eval("int(\"42\") + 1"), json!(43));
    assert_eq!(eval("string(params.port)"), json!("7379"));
    assert_eq!(eval("json(params.tags)"), json!("[\"a\",\"b\"]"));
    assert_eq!(eval("slug(\"Riverside Shop!\")"), json!("riverside-shop"));
    assert_eq!(eval("max(1, state.builds, 1.5)"), json!(2.0));
    assert_eq!(eval("min(3, 2)"), json!(2));
    assert_eq!(eval("keys(params)"), json!(["name", "port", "ratio", "tags"]));
}

#[test]
fn a_list_of_mappings_is_narrowed_to_those_with_a_value_and_put_in_order_by_a_field() {
    let mut scope = scope();
    scope.insert(
        "state".to_string(),
        json!({ "pool": [
            { "name": "c", "free": true, "used": 30, "choice": 1 },
            { "name": "a", "free": false, "used": 10, "choice": 0 },
            { "name": "b", "free": true, "used": 20, "choice": 1 },
            { "name": "d", "free": true, "used": 40, "choice": 0 },
        ] }),
    );
    let eval = |source: &str| {
        expr::parse(source).unwrap().eval(&scope).unwrap_or_else(|err| panic!("{source}: {err}"))
    };
    assert_eq!(eval("len(where(state.pool, \"free\", true))"), json!(3));
    assert_eq!(eval("where(state.pool, \"name\", \"a\")[0].used"), json!(10));
    assert_eq!(
        eval("where(state.pool, \"missing\", null)[3].name"),
        json!("d"),
        "a field it has not is null"
    );
    assert_eq!(eval("sort(state.pool, \"used\")[0].name"), json!("a"));
    assert_eq!(eval("sort(state.pool, \"name\")[3].name"), json!("d"));
    // Equals keep their order, so two sorts order by the second and then the first.
    assert_eq!(
        eval("sort(sort(where(state.pool, \"free\", true), \"used\"), \"choice\")[0].name"),
        json!("d")
    );
    assert_eq!(eval("sort(sort(state.pool, \"used\"), \"choice\")[2].name"), json!("b"));
    assert_eq!(eval("sort([3, 1, 2])"), json!([1, 2, 3]));
    assert_eq!(eval("sort([\"b\", \"a\"])[0]"), json!("a"));
    // Of nothing there is nothing, and the first of nothing is nothing.
    assert_eq!(eval("where(state.nowhere, \"free\", true)"), json!([]));
    assert_eq!(eval("sort(where(state.pool, \"free\", 7), \"used\")[0].name"), json!(null));
    assert!(expr::parse("sort(1)").unwrap().eval(&scope).unwrap_err().contains("must be a list"));
}

#[test]
fn a_pattern_is_looked_for_anywhere_in_the_text_unless_it_says_where() {
    assert_eq!(eval("matches(params.name, \"v\")"), json!(true));
    assert_eq!(eval("matches(params.name, \"^v\")"), json!(false), "`^` is the start of the text");
    assert_eq!(eval("matches(\"http://127.0.0.1:7700\", \"^[A-Za-z0-9.:/+_-]*$\")"), json!(true));
    assert_eq!(eval("matches(\"http://127.0.0.1:7700/a b\", \"^[A-Za-z0-9.:/+_-]*$\")"), json!(false));
    assert_eq!(eval("matches(\"https+insecure://x\", \"^(http|https[+]insecure)://\")"), json!(true));
    // The pattern may be worked out too, and is then read when it is used.
    assert_eq!(eval("matches(\"kv-2\", \"^\" + params.name + \"-[0-9]+$\")"), json!(true));

    // One written out that does not read is a mistake where it is written.
    let err = expr::parse("1 + matches(params.name, \"(\")").unwrap_err();
    assert!(err.message.contains("matches(): `(` does not read as a pattern"), "{}", err.message);
    assert_eq!(err.offset, 4);
    let fails = |source: &str| expr::parse(source).unwrap().eval(&scope()).unwrap_err();
    assert!(fails("matches(params.name, \"(\" + \"\")").contains("does not read as a pattern"));
    assert!(fails("matches(state.stable, \"x\")").contains("argument 1 must be a string"));
    assert!(fails("matches(params.name)").contains("takes 2 argument(s), got 1"));
}

#[test]
fn parse_errors_say_what_and_where() {
    let err = expr::parse("a = b").unwrap_err();
    assert!(err.message.contains("use `==`"), "{}", err.message);
    assert_eq!(err.offset, 2);
    assert!(expr::parse("nope(1)").unwrap_err().message.contains("unknown function `nope`"));
    assert!(expr::parse("a +").unwrap_err().message.contains("ends too soon"));
    assert!(expr::parse("(a").unwrap_err().message.contains("expected `)`"));
    assert!(expr::parse("a b").unwrap_err().message.contains("unexpected text"));
}

#[test]
fn type_errors_are_errors_not_nulls() {
    let fails = |source: &str| expr::parse(source).unwrap().eval(&scope()).unwrap_err();
    assert!(fails("params.tags - 1").contains("cannot apply"));
    assert!(fails("1 / 0").contains("division by zero"));
    assert!(fails("int(\"x\")").contains("not a whole number"));
}

#[test]
fn an_expression_lists_the_paths_it_reads() {
    let paths =
        expr::parse("steps.build.out.code > 0 && len(params.tags) == state[\"builds\"]").unwrap().paths();
    let joined: Vec<String> = paths.iter().map(|path| path.join(".")).collect();
    assert_eq!(joined, ["steps.build.out.code", "params.tags", "state.builds"]);
}

#[test]
fn a_lone_hole_keeps_its_type_and_mixed_text_is_text() {
    let render = |text: &str| Template::parse(text).unwrap().render(&scope()).unwrap();
    assert_eq!(render("${params.port}"), json!(7379));
    assert_eq!(render("${ params.tags }"), json!(["a", "b"]));
    assert_eq!(render("127.0.0.1:${params.port}"), json!("127.0.0.1:7379"));
    assert_eq!(render("${params.name}-${state.builds}"), json!("kv-2"));
    assert_eq!(render("cost: $${not a hole}"), json!("cost: ${not a hole}"));
    assert_eq!(render("null is empty: [${state.stable}]"), json!("null is empty: []"));
}

#[test]
fn rendering_fills_holes_at_any_depth_and_only_in_values() {
    let value = json!({ "${params.name}": "${params.port}", "list": ["${params.name}", 1], "plain": "x" });
    let rendered = template::render(&value, &scope()).unwrap();
    assert_eq!(rendered, json!({ "${params.name}": 7379, "list": ["kv", 1], "plain": "x" }));
}

#[test]
fn a_mapping_that_says_three_dots_begins_from_the_mapping_that_comes_to_and_what_is_written_beside_it_is_said_over_it()
 {
    // Everything that was handed, handed on, with one thing said again and one added.
    let value = json!({ "...": "${params}", "port": 1, "back": true, "inner": { "...": "${state}" } });
    let rendered = template::render(&value, &scope()).unwrap();
    let mut wanted = scope()["params"].as_object().unwrap().clone();
    wanted.insert("port".into(), json!(1));
    wanted.insert("back".into(), json!(true));
    wanted.insert("inner".into(), scope()["state"].clone());
    assert_eq!(rendered, serde_json::Value::Object(wanted));
    // Nothing to hand on is nothing handed on; anything that is no mapping is refused.
    assert_eq!(
        template::render(&json!({ "...": "${ state.stable }", "a": 1 }), &scope()).unwrap(),
        json!({ "a": 1 })
    );
    let refused = template::render(&json!({ "...": "${params.name}" }), &scope()).unwrap_err();
    assert_eq!(refused, "`...` hands on a mapping, and was given kv");
}

#[test]
fn a_condition_may_be_bare_or_wrapped() {
    for text in ["steps.build.ok", "${ steps.build.ok }", "  ${steps.build.ok}  "] {
        assert!(Cond::parse(text).unwrap().test(&scope()).unwrap(), "{text}");
    }
    assert_eq!(Cond::parse("${ a } && ${ b }").unwrap_err().message, "unexpected `$`");
}

#[test]
fn durations() {
    assert_eq!(duration::parse("90s"), Ok(90_000));
    assert_eq!(duration::parse("5m"), Ok(300_000));
    assert_eq!(duration::parse("300 minutes"), Ok(18_000_000));
    assert_eq!(duration::parse("1h30m"), Ok(5_400_000));
    assert_eq!(duration::parse("1.5h"), Ok(5_400_000));
    assert_eq!(duration::parse("250ms"), Ok(250));
    assert!(duration::parse("300").unwrap_err().contains("needs a unit"));
    assert!(duration::parse("5 parsecs").unwrap_err().contains("unknown time unit"));
    assert_eq!(duration::format(5_400_000), "1h30m");
    assert_eq!(duration::format(90_000), "1m30s");
    assert_eq!(duration::format(0), "0s");
}

#[test]
fn a_list_without_a_value_or_without_the_values_of_another_is_what_is_left_in_its_order() {
    let mut scope = scope();
    scope.insert(
        "state".to_string(),
        json!({ "waiting": ["in_a", "in_b", "in_c", "in_b"], "held": ["in_c", "in_a"], "counts": [1, 2, 3] }),
    );
    let eval = |source: &str| {
        expr::parse(source).unwrap().eval(&scope).unwrap_or_else(|err| panic!("{source}: {err}"))
    };
    // One value: every place it stood is gone, and the rest keep their order.
    assert_eq!(eval("without(state.waiting, \"in_b\")"), json!(["in_a", "in_c"]));
    assert_eq!(eval("without(state.counts, 2)"), json!([1, 3]));
    assert_eq!(eval("without(state.waiting, \"in_z\")"), json!(["in_a", "in_b", "in_c", "in_b"]));
    // A list of them: whatever is in both is left out, so the first of
    // what is left is the first that waits and holds nothing.
    assert_eq!(eval("without(state.waiting, state.held)"), json!(["in_b", "in_b"]));
    assert_eq!(eval("without(state.waiting, state.held)[0]"), json!("in_b"));
    assert_eq!(eval("len(without(state.held, state.waiting))"), json!(0));
    // Nothing is the empty list, as it is for every function of a list.
    assert_eq!(eval("without(state.nobody, \"in_a\")"), json!([]));
    assert_eq!(eval("without(state.held, state.nobody)"), json!(["in_c", "in_a"]));
    let wrong = expr::parse("without(\"text\", 1)").unwrap().eval(&scope).unwrap_err();
    assert!(wrong.contains("argument 1 must be a list"), "{wrong}");
}

#[test]
fn a_field_of_each_and_the_elements_at_some_places_are_lists_and_json_text_is_read_as_what_it_says() {
    let mut scope = scope();
    scope.insert(
        "steps".to_string(),
        json!({ "added": { "out": [{ "id": "shop-4", "title": "Floor" }, { "id": "shop-5" }, { "title": "No id" }] } }),
    );
    let eval = |source: &str| {
        expr::parse(source).unwrap().eval(&scope).unwrap_or_else(|err| panic!("{source}: {err}"))
    };
    // A field of each, in their order; one that has none is left out.
    assert_eq!(eval("pluck(steps.added.out, \"id\")"), json!(["shop-4", "shop-5"]));
    assert_eq!(eval("pluck(steps.added.out, \"nothing\")"), json!([]));
    assert_eq!(eval("pluck(steps.nobody, \"id\")"), json!([]));
    // The elements at some places, counted from 1, in the order asked. A
    // place nothing is at is left out, and so is no place at all.
    assert_eq!(eval("pluck(at(steps.added.out, [2, 1]), \"id\")"), json!(["shop-5", "shop-4"]));
    assert_eq!(eval("at(steps.added.out, 2)"), json!([{ "id": "shop-5" }]));
    assert_eq!(eval("at(steps.added.out, [0, 4, -1])"), json!([]));
    assert_eq!(eval("at(steps.added.out, steps.nobody)"), json!([]));
    let wrong = expr::parse("at(steps.added.out, [\"first\"])").unwrap().eval(&scope).unwrap_err();
    assert!(wrong.contains("a place is a whole number, counted from 1"), "{wrong}");
    // What JSON text says, as a value like any other.
    assert_eq!(
        eval("from_json(\"[{ \\\"title\\\": \\\"Floor\\\", \\\"after\\\": [1] }]\")[0].after"),
        json!([1])
    );
    let wrong = expr::parse("from_json(\"[1, \")").unwrap().eval(&scope).unwrap_err();
    assert!(wrong.starts_with("from_json(): this is not JSON"), "{wrong}");
}

#[test]
fn the_whole_numbers_up_to_one_are_a_list_to_go_through() {
    assert_eq!(eval("range(3)"), json!([1, 2, 3]));
    assert_eq!(eval("range(1)"), json!([1]));
    assert_eq!(eval("range(0)"), json!([]));
    assert_eq!(eval("range(-2)"), json!([]));
    assert_eq!(eval("len(range(params.port - 7370))"), json!(9));
    let scope = scope();
    let wrong = |source: &str| expr::parse(source).unwrap().eval(&scope).unwrap_err();
    assert!(wrong("range(\"three\")").contains("takes a whole number"));
    assert!(wrong("range(2.5)").contains("takes a whole number"));
    assert!(wrong("range(100000)").contains("goes to 1000 at most"));
}

#[test]
fn numbers_are_added_up_and_a_field_within_a_field_is_read_by_its_path() {
    let mut scope = scope();
    scope.insert(
        "steps".to_string(),
        json!({ "rows": { "out": [
            { "id": "shop-agents-1", "data": { "title": "worker", "count": 2 } },
            { "id": "shop-agents-2", "data": { "title": "reviewer", "count": 1 } },
            { "id": "shop-agents-3", "data": { "title": "reviewer" } },
        ] } }),
    );
    let eval = |source: &str| {
        expr::parse(source).unwrap().eval(&scope).unwrap_or_else(|err| panic!("{source}: {err}"))
    };
    // What a record holds is within it: a list of records is asked by the path.
    assert_eq!(eval("pluck(steps.rows.out, \"data.title\")"), json!(["worker", "reviewer", "reviewer"]));
    assert_eq!(eval("where(steps.rows.out, \"data.title\", \"reviewer\")[0].id"), json!("shop-agents-2"));
    assert_eq!(eval("len(where(steps.rows.out, \"data.nothing.at.all\", 1))"), json!(0));
    assert_eq!(eval("sort(steps.rows.out, \"data.title\")[0].id"), json!("shop-agents-2"));
    // Added up; one that has none of the field is left out, and nothing at all is 0.
    assert_eq!(eval("sum(pluck(steps.rows.out, \"data.count\"))"), json!(3));
    assert_eq!(eval("sum(steps.nobody)"), json!(0));
    assert_eq!(eval("sum([1, 2.5])"), json!(3.5));
    let wrong = expr::parse("sum([1, \"two\"])").unwrap().eval(&scope).unwrap_err();
    assert!(wrong.contains("sum() adds numbers up"), "{wrong}");
}
