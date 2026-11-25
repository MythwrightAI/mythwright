//! What giving a field a value takes may be several scopes, every one of
//! which is asked (`takes: [shell, secrets:use]`), of any value or of one
//! (`takes: { on: [shell, secrets:use] }`): a value that may be a command
//! and may be the name of a secret is guarded as both. One scope is
//! written, kept and handed out as it always was.

use kernel::{ErrorKind, Principal};
use serde_json::{Map, Value, json};
use studio::collections::{Spec, Takes};

use crate::{Rig, ada, agent};

fn data(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

/// A table of deploys: what each hands on, which may be run or name a
/// secret, and whether it is loud, which is the same of one value.
fn deploys(hands: Value, loud: Value) -> Spec {
    let fields = json!({
        "title": { "type": "text", "required": true },
        "hands": { "type": "list", "takes": hands },
        "loud": if loud.is_null() { json!({ "type": "flag" }) } else { json!({ "type": "flag", "takes": { "true": loud } }) },
        "note": { "type": "text", "takes": "shell" },
    });
    Spec::read(&fields, &json!(null), None).unwrap()
}

fn both() -> Spec {
    deploys(json!(["shell", "secrets:use"]), json!(["shell", "secrets:use"]))
}

fn holding(scopes: &[&str]) -> Principal {
    let all: Vec<&str> = ["projects", "records"].into_iter().chain(scopes.iter().copied()).collect();
    agent("narrow", &all)
}

#[tokio::test]
async fn a_field_that_takes_several_scopes_is_given_a_value_only_by_whoever_holds_every_one_and_is_told_which_it_lacks()
 {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let (collections, records) = (&rig.studio.collections, &rig.studio.records);
    let made = collections.ensure("shop", "deploys", both(), None, &ada()).await.unwrap();
    // Kept as it was written: several as a list, one as a word.
    let fields = serde_json::to_value(&made).unwrap()["fields"].clone();
    let field =
        |name: &str| fields.as_array().unwrap().iter().find(|one| one["name"] == name).unwrap().clone();
    assert_eq!(field("hands")["takes"], json!(["shell", "secrets:use"]));
    assert_eq!(field("loud")["takes_of"], json!({ "true": ["shell", "secrets:use"] }));
    assert_eq!(field("note")["takes"], "shell");
    assert_eq!(made.field("hands").unwrap().takes.scopes(), ["shell", "secrets:use"]);

    // A row that says nothing of them takes nothing more than a row does.
    let row = records.add("shop", "deploys", data(json!({ "title": "Nightly" })), &holding(&[]), None).await;
    let row = row.unwrap().id;
    for (holds, why) in [
        (
            &[][..],
            "changing `hands` of `deploys` takes `shell` and `secrets:use`, which narrow does not hold",
        ),
        (
            &["shell"][..],
            "changing `hands` of `deploys` takes `shell` and `secrets:use`, and narrow does not hold `secrets:use`",
        ),
        (
            &["secrets"][..],
            "changing `hands` of `deploys` takes `shell` and `secrets:use`, and narrow does not hold `shell`",
        ),
    ] {
        let by = holding(holds);
        let refused = records.update(&row, data(json!({ "hands": ["token=deploy"] })), &by, None).await;
        let refused = refused.unwrap_err();
        assert_eq!((refused.kind, refused.message.as_str()), (ErrorKind::Denied, why), "{holds:?}");
        let added = json!({ "title": "Weekly", "hands": ["token=deploy"] });
        let refused = records.add("shop", "deploys", data(added), &by, None).await.unwrap_err();
        assert_eq!(refused.message, why, "{holds:?}");
        // The same of the one value that takes them, and of no other.
        let refused = records.update(&row, data(json!({ "loud": true })), &by, None).await.unwrap_err();
        assert!(
            refused
                .message
                .starts_with("giving `loud` of `deploys` the value `true` takes `shell` and `secrets:use`"),
            "{}",
            refused.message
        );
        records.update(&row, data(json!({ "loud": false })), &by, None).await.unwrap();
    }
    let kept = collections.record(&row).await.unwrap().1;
    assert!(kept.data.get("hands").is_none_or(Value::is_null), "{:?}", kept.data);
    // Whoever holds every one writes it; the area holds what is in it.
    for by in [holding(&["shell", "secrets:use"]), holding(&["shell", "secrets"])] {
        let changed = data(json!({ "hands": ["token=deploy"], "loud": true }));
        records.update(&row, changed, &by, None).await.unwrap();
    }
    // One scope is asked and said as it always was.
    let refused = records.update(&row, data(json!({ "note": "hi" })), &holding(&[]), None).await.unwrap_err();
    assert_eq!(refused.message, "changing `note` of `deploys` takes `shell`, which narrow does not hold");
}

#[tokio::test]
async fn a_scope_taken_off_what_a_field_takes_is_declared_away_only_by_whoever_holds_that_one() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let collections = &rig.studio.collections;
    collections.ensure("shop", "deploys", both(), None, &ada()).await.unwrap();
    let shell = holding(&["shell"]);
    // Somebody who holds `shell` alone may not take `secrets:use` off,
    // of any value or of the one, nor take the word off the field.
    for (spec, why) in [
        (
            deploys(json!("shell"), json!(["shell", "secrets:use"])),
            "giving `hands` of `deploys` a value takes `shell` and `secrets:use`, and declaring `deploys` again so \
             that it does not takes holding `secrets:use`, which narrow does not hold",
        ),
        (
            deploys(json!(["shell", "secrets:use"]), json!("shell")),
            "giving `loud` of `deploys` the value `true` takes `shell` and `secrets:use`, and declaring `deploys` \
             again so that it does not takes holding `secrets:use`, which narrow does not hold",
        ),
        (
            deploys(json!(null), json!(["shell", "secrets:use"])),
            "giving `hands` of `deploys` a value takes `shell` and `secrets:use`, and declaring `deploys` again so \
             that it does not takes holding `secrets:use`, which narrow does not hold",
        ),
    ] {
        let refused = collections.ensure("shop", "deploys", spec, None, &shell).await.unwrap_err();
        assert_eq!((refused.kind, refused.message.as_str()), (ErrorKind::Denied, why));
    }
    let hands = collections.get("shop", "deploys").await.unwrap();
    assert_eq!(hands.field("hands").unwrap().takes.scopes(), ["shell", "secrets:use"]);
    // What they hold is theirs to take off; what is asked of every value
    // still holds what one value asked; said again as it is, or of more,
    // it is whoever's may declare it.
    let off = deploys(json!(["secrets:use"]), json!(["shell", "secrets:use"]));
    collections.ensure("shop", "deploys", off, None, &shell).await.unwrap();
    let wider = deploys(json!(["shell", "secrets:use"]), json!(["shell", "secrets:use", "admin"]));
    collections.ensure("shop", "deploys", wider, None, &holding(&[])).await.unwrap();
    let all = json!({
        "title": { "type": "text", "required": true },
        "hands": { "type": "list", "takes": ["shell", "secrets:use"] },
        "loud": { "type": "flag", "takes": ["shell", "secrets:use"] },
        "note": { "type": "text", "takes": "shell" },
    });
    let all = Spec::read(&all, &json!(null), None).unwrap();
    collections.ensure("shop", "deploys", all, None, &holding(&["admin"])).await.unwrap();
    // Whoever holds both takes both off.
    let bare = deploys(json!(null), json!(null));
    collections.ensure("shop", "deploys", bare, None, &holding(&["shell", "secrets"])).await.unwrap();
    let bare = collections.get("shop", "deploys").await.unwrap();
    assert!(bare.field("hands").unwrap().takes.is_empty());
}

#[test]
fn what_a_field_takes_is_a_scope_or_a_list_of_them_and_anything_else_is_refused_saying_so() {
    let read = |takes: Value| {
        Spec::read(&json!({ "hands": { "type": "list", "takes": takes } }), &json!(null), None)
            .map(|spec| spec.fields[0].takes.clone())
    };
    let scopes = |takes: &Takes| takes.scopes().to_vec();
    assert_eq!(scopes(&read(json!("shell")).unwrap()), ["shell"]);
    assert_eq!(scopes(&read(json!(" shell ")).unwrap()), ["shell"]);
    // Each once, in the order written; a list of one is one.
    assert_eq!(scopes(&read(json!(["shell", "secrets:use", "shell"])).unwrap()), ["shell", "secrets:use"]);
    assert_eq!(serde_json::to_value(read(json!(["shell"])).unwrap()).unwrap(), "shell");
    for nothing in [json!(null), json!(""), json!([]), json!(["", " "])] {
        assert!(read(nothing.clone()).unwrap().is_empty(), "{nothing}");
    }
    // A scope nobody could hold, and `each` among others, are refused.
    let refused = read(json!(["shell", "everything"])).unwrap_err();
    assert!(
        refused.message.starts_with("the field `hands` takes `everything` is not something"),
        "{}",
        refused.message
    );
    let refused = read(json!(["each", "shell"])).unwrap_err();
    assert!(refused.message.contains("`takes: each` is said alone"), "{}", refused.message);
    // What is neither a word nor a list of words is refused, saying what
    // `takes` may be: taken as saying nothing, a guard somebody meant was
    // not there.
    for (neither, said) in [
        (json!(true), "that giving it a value takes `true`"),
        (json!(3), "that giving it a value takes `3`"),
        (json!(["shell", false]), "`false` among what giving it a value takes"),
        (json!([2]), "`2` among what giving it a value takes"),
        (json!([["shell"]]), "`[\"shell\"]` among what giving it a value takes"),
    ] {
        let refused = read(neither.clone()).unwrap_err();
        assert_eq!(
            (refused.kind, refused.message),
            (
                ErrorKind::Invalid,
                format!(
                    "the field `hands` says {said}, which is no scope: what giving it a value takes is a scope or \
                     a list of them, as in `takes: shell` or `takes: [shell, secrets:use]`, or for a choice or a \
                     flag those of each value, as in `takes: {{ on: shell }}`"
                )
            ),
            "{neither}"
        );
    }
    // Of one value the same, and a value that says no scope is refused.
    let of = |takes: Value| {
        let fields = json!({ "loud": { "type": "flag", "takes": { "true": takes } } });
        Spec::read(&fields, &json!(null), None).map(|spec| spec.fields[0].takes_of["true"].clone())
    };
    assert_eq!(scopes(&of(json!(["shell", "secrets:use"])).unwrap()), ["shell", "secrets:use"]);
    assert_eq!(scopes(&of(json!("shell")).unwrap()), ["shell"]);
    for nothing in [json!([]), json!("")] {
        let refused = of(nothing.clone()).unwrap_err();
        assert!(
            refused.message.contains("that is a scope or a list of them"),
            "{nothing}: {}",
            refused.message
        );
    }
    for (neither, said) in [
        (json!(true), "that giving it `true` takes `true`"),
        (json!(3), "that giving it `true` takes `3`"),
        (json!(["shell", 1]), "`1` among what giving it `true` takes"),
    ] {
        let refused = of(neither.clone()).unwrap_err();
        assert_eq!(
            refused.message,
            format!(
                "the field `loud` says {said}, which is no scope: what giving it `true` takes is a scope or a list \
                 of them, as in `takes: {{ true: shell }}` or `takes: {{ true: [shell, secrets:use] }}`"
            ),
            "{neither}"
        );
    }
    // What a table from before kept reads as it did.
    let kept: studio::collections::Field = serde_json::from_value(
        json!({ "name": "wall", "kind": "flag", "takes": "shell", "takes_of": { "false": "shell" } }),
    )
    .unwrap();
    assert_eq!(
        (scopes(&kept.takes), scopes(&kept.takes_of["false"])),
        (vec!["shell".to_string()], vec!["shell".to_string()])
    );
}
