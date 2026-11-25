//! Every word of the language says what it is for, and is shown by a
//! whole blueprint written with it: what the Reference and the editor's
//! card show of it. Each example reads, is laid out as the formatter lays
//! it out, and is written with the word it is an example of. That each
//! checks with no warning against every action there is, is held where
//! there are all of them (the API suite's `examples.rs`).

use std::collections::BTreeMap;

use blueprint::vocabulary::{CALLED, SHAPES, written};
use blueprint::{NoCatalog, ROOTS, Settings, format};

/// Every word, as what it is called on the page, its name and its sentence.
fn words() -> Vec<(String, &'static str, &'static str)> {
    let mut all = Vec::new();
    for shape in SHAPES {
        all.push((format!("the mapping `{}`", shape.name), shape.name, shape.help));
        for field in shape.fields {
            all.push((format!("`{}` of {}", field.name, shape.what), field.name, field.help));
        }
    }
    for root in ROOTS {
        all.push((format!("the name `{}`", root.name), root.name, root.meaning));
    }
    for (name, usage) in blueprint::expr::FUNCTIONS {
        all.push((format!("the function `{name}`"), name, usage));
    }
    all
}

/// Every example, with what it is the example of.
fn examples() -> Vec<(String, &'static str)> {
    let mut all = Vec::new();
    for shape in SHAPES {
        all.extend(shape.example.map(|example| (format!("the mapping `{}`", shape.name), example)));
        for field in shape.fields {
            all.extend(field.example.map(|example| (format!("`{}` of {}", field.name, shape.what), example)));
        }
    }
    all.extend(ROOTS.iter().map(|root| (format!("the name `{}`", root.name), root.example)));
    all.extend(CALLED.iter().map(|(name, example)| (format!("the function `{name}`"), *example)));
    all
}

#[test]
fn every_word_says_in_a_sentence_of_its_own_what_it_is_for() {
    let mut said: BTreeMap<&str, (String, &str)> = BTreeMap::new();
    for (what, name, sentence) in words() {
        let bare = sentence.trim_matches(|c: char| c == '`' || c == '.' || c.is_whitespace());
        let short = sentence.split_whitespace().count() < 5;
        assert!(
            !short && bare != name && !bare.ends_with(&format!(" {name}")),
            "{what} says only {sentence:?}: it says in a sentence what it is for and when somebody reaches for it"
        );
        // A word of the same name in another mapping may mean the same there.
        if let Some((other, other_name)) = said.insert(sentence, (what.clone(), name)) {
            assert_eq!(
                other_name, name,
                "{what} and {other} say the same sentence: each says what it is for"
            );
        }
    }
}

#[test]
fn every_word_and_every_mapping_has_an_example_and_every_function_and_name() {
    for shape in SHAPES {
        assert!(shape.example.is_some(), "the mapping `{}` has no example", shape.name);
        for field in shape.fields {
            assert!(field.example.is_some(), "`{}` of {} has no example", field.name, shape.what);
        }
    }
    for (name, _) in blueprint::expr::FUNCTIONS {
        assert!(CALLED.iter().any(|(called, _)| called == name), "the function `{name}` has no example");
    }
    for (name, _) in CALLED {
        assert!(blueprint::expr::FUNCTIONS.iter().any(|(known, _)| known == name), "`{name}` is no function");
    }
}

#[test]
fn every_example_is_a_whole_blueprint_that_reads_and_is_laid_out_as_the_formatter_lays_it_out() {
    for (what, example) in examples() {
        let checked = blueprint::check(example, &NoCatalog);
        let errors: Vec<String> = checked.errors().map(|problem| problem.message.clone()).collect();
        assert!(
            checked.blueprint.is_some() && errors.is_empty(),
            "the example of {what} does not read: {errors:?}\n{example}"
        );
        let laid =
            format(example, &Settings::default()).unwrap_or_else(|_| panic!("{what}: it does not lay out"));
        assert_eq!(laid, example, "the example of {what} is not laid out as the formatter lays it out");
        let lines = example.lines().count();
        assert!(lines <= 24, "the example of {what} is {lines} lines: a few lines are read at a glance");
    }
}

#[test]
fn every_example_is_written_with_the_word_it_is_the_example_of() {
    for shape in SHAPES {
        let has = |example: &str, field: Option<&str>| {
            let words = written(example);
            match field {
                Some(field) => words.contains(&(shape.name, field)),
                None => words.iter().any(|(of, _)| *of == shape.name),
            }
        };
        if let Some(example) = shape.example {
            assert!(has(example, None), "the example of the mapping `{}` has none", shape.name);
        }
        for field in shape.fields {
            if let Some(example) = field.example {
                assert!(
                    has(example, Some(field.name)),
                    "the example of `{}` of {} does not say it",
                    field.name,
                    shape.what
                );
            }
        }
    }
    for root in ROOTS {
        let used = blueprint::used::used(&[("example".to_string(), root.example.to_string())]);
        assert!(
            used.names.contains_key(root.name),
            "the example of `{}` reads no `{}`",
            root.name,
            root.name
        );
    }
    for (name, example) in CALLED {
        assert!(example.contains(&format!("{name}(")), "the example of `{name}` does not call it");
    }
}
