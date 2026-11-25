//! `for:` on a handler: which events are addressed to it.

use blueprint::address::{self, Address};
use blueprint::expr;
use serde_json::{Value, json};

use crate::{check, error_codes, first_error};

/// A handler on an event, with `address` as its `for:`.
fn handler(address: &str) -> String {
    format!(
        "blueprint: card\npermissions: []\nparams:\n  item:\nstate: {{ stage: \"\" }}\non:\n  moved:\n    \
         event: board.item.moved\n    for: {address}\n    do: noop\n"
    )
}

#[test]
fn for_is_read_as_fields_of_the_events_data_and_the_value_each_must_have() {
    let checked = check(&handler("{ id: \"${params.item}\", to: done, card: \"${instance.key}\" }"));
    assert_eq!(checked.diagnostics, []);
    let blueprint = checked.into_result().unwrap();
    assert_eq!(
        Value::Object(blueprint.triggers[0].address.clone()),
        json!({ "id": "${params.item}", "to": "done", "card": "${instance.key}" })
    );

    let plain = check("blueprint: b\npermissions: []\non:\n  h: { event: a.b, do: noop }\n");
    assert!(plain.into_result().unwrap().triggers[0].address.is_empty(), "everything on the topic is its");
}

#[test]
fn a_value_in_for_may_read_only_what_is_fixed_while_the_handler_lives() {
    for fixed in ["${params.item}", "${instance.key}", "${target.id}", "card-${params.item}", "done", "7"] {
        assert_eq!(error_codes(&handler(&format!("{{ id: \"{fixed}\" }}"))), [] as [&str; 0], "{fixed}");
    }
    assert_eq!(error_codes(&handler("{ id: \"${params.card}\" }")), ["unknown-param"]);

    // Each is refused with why, and with what to do instead.
    for (name, why, instead) in [
        ("event.data.other", "before any event arrives", "put any other condition on the event in `when`"),
        ("state.stage", "`state` can change under the handler", "Compare with `state` in `when`"),
        ("steps.card.out.id", "outside any run", "compare with that in `when`"),
        ("self.out", "when no step is running", "`when` for anything else"),
        ("item", "one element of a `for_each`", "`when` for anything else"),
        ("index", "one turn of a `for_each`", "`when` for anything else"),
    ] {
        let source = handler(&format!("{{ id: \"${{ {name} }}\" }}"));
        assert_eq!(error_codes(&source), ["misplaced-name"], "{name}");
        let message = first_error(&source);
        let root = name.split('.').next().unwrap();
        assert!(message.contains(&format!("a value in `for` cannot read `{root}`")), "{name}: {message}");
        assert!(message.contains(why), "{name}: {message}");
        assert!(message.contains(instead), "{name}: {message}");
    }
}

#[test]
fn for_is_refused_on_a_handler_that_no_event_fires() {
    let timer = "blueprint: b\npermissions: []\non:\n  h: { every: 5m, for: { id: x }, do: noop }\n";
    assert_eq!(error_codes(timer), ["conflict"]);
    let message = first_error(timer);
    assert!(message.contains("a handler with `every` is not fired by an event"), "{message}");
    assert!(message.contains("take `for` away, and put any condition in `when`"), "{message}");

    let poll = "blueprint: b\npermissions: []\non:\n  h:\n    poll: { every: 30s, do: noop }\n    for: { id: x }\n    do: noop\n";
    assert_eq!(error_codes(poll), ["conflict"]);
    assert!(first_error(poll).contains("a handler with `poll` is not fired by an event"));
}

#[test]
fn for_must_name_fields_and_name_them_as_the_event_does() {
    let message = first_error(&handler("\"${params.item}\""));
    assert!(message.contains("`for` must be a mapping from a field of the event's data"), "{message}");

    let message = first_error(&handler("{}"));
    assert!(message.contains("`for` names no field"), "{message}");

    let message = first_error(&handler("{ event.data.id: \"${params.item}\" }"));
    assert!(message.contains("`event.data.id` is not a field of the event's data"), "{message}");
    assert!(message.contains("as in `id: ...`"), "{message}");

    assert_eq!(error_codes(&handler("{ id: \"${ params.item + }\" }")), ["bad-expression"]);
}

#[test]
fn being_addressed_is_the_test_that_equality_in_when_would_be() {
    let values = [
        json!(null),
        json!(true),
        json!(false),
        json!(0),
        json!(-0.0),
        json!(1),
        json!(1.0),
        json!(1.5),
        json!("1"),
        json!("card-12"),
        json!(""),
        json!([1, "a"]),
        json!([1.0, "a"]),
        json!({ "a": 1, "b": [true] }),
        json!({ "b": [true], "a": 1 }),
        json!({ "a": 1.0, "b": [true] }),
    ];
    let equal = expr::parse("wanted == found").unwrap();
    for wanted in &values {
        let scope = json!({ "params": { "item": wanted } }).as_object().cloned().unwrap();
        let asked = json!({ "id": "${params.item}" }).as_object().cloned().unwrap();
        let address = Address::of(&asked, &scope).unwrap();
        for found in &values {
            let in_when =
                equal.eval(json!({ "wanted": wanted, "found": found }).as_object().unwrap()).unwrap();
            assert_eq!(
                json!(address.carried_by(&json!({ "id": found }))),
                in_when,
                "asked for {wanted}, the event has {found}"
            );
            assert_eq!(json!(address::key(wanted) == address::key(found)), in_when, "{wanted} and {found}");
        }
        assert!(!address.carried_by(&json!({ "other": wanted })), "a field the event does not have");
        assert!(!address.carried_by(wanted), "an event whose data is not a mapping");
    }
}

#[test]
fn an_address_is_found_by_one_field_and_holds_only_if_every_field_does() {
    let scope = json!({ "params": { "item": "shop-12" } }).as_object().cloned().unwrap();
    let asked = json!({ "id": "${params.item}", "to": "done" }).as_object().cloned().unwrap();
    let address = Address::of(&asked, &scope).unwrap();

    let (field, value) = address.first().unwrap();
    assert_eq!(value, address::key(&json!({ "id": "shop-12", "to": "done" })[field]));
    assert!(address.carried_by(&json!({ "id": "shop-12", "to": "done", "from": "review" })));
    assert!(!address.carried_by(&json!({ "id": "shop-12", "to": "doing" })));
    assert!(!address.carried_by(&json!({ "to": "done" })));

    let nothing = Address::of(&serde_json::Map::new(), &scope).unwrap();
    assert_eq!(nothing.first(), None);
    assert!(nothing.carried_by(&json!({ "anything": 1 })), "with no `for`, everything on the topic");

    let broken = json!({ "id": "${ int(params.item) }" }).as_object().cloned().unwrap();
    assert!(Address::of(&broken, &scope).unwrap_err().contains("is not a whole number"));
}

#[test]
fn a_field_inside_one_of_the_events_data_is_named_by_its_path_and_a_path_from_the_event_is_refused() {
    // What a record's table says is carried by its events is under `fields`.
    let checked = check(&handler("{ id: \"${params.item}\", fields.sprint: autumn }"));
    assert_eq!(checked.diagnostics, []);
    for from_the_event in ["event.data.sprint", "data.fields.sprint"] {
        let message = first_error(&handler(&format!("{{ {from_the_event}: autumn }}")));
        assert!(message.contains("is not a field of the event's data"), "{from_the_event}: {message}");
    }

    let scope = json!({ "params": { "item": "shop-12" } }).as_object().cloned().unwrap();
    let asked = json!({ "fields.sprint": "autumn" }).as_object().cloned().unwrap();
    let address = Address::of(&asked, &scope).unwrap();
    assert!(address.carried_by(&json!({ "id": "shop-12", "fields": { "sprint": "autumn", "notes": null } })));
    assert!(!address.carried_by(&json!({ "fields": { "sprint": "spring" } })));
    assert!(!address.carried_by(&json!({ "fields": { "notes": "autumn" } })), "a field it does not carry");
    assert!(!address.carried_by(&json!({ "fields": "autumn" })), "a field that is no mapping");
    assert_eq!(address::at(&json!({ "a": { "b": { "c": 3 } } }), "a.b.c"), Some(&json!(3)));
    assert_eq!(address::at(&json!({ "a.b": 1, "a": { "b": 2 } }), "a.b"), Some(&json!(1)), "a name as it is");
}
