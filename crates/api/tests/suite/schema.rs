//! The language as a schema an editor reads.
//!
//! It is worked out from the language's own word list, so it cannot say a
//! word the reader does not take. What it must not do is the other thing:
//! mark as wrong a blueprint that is right. So every blueprint that ships
//! is held against it here, by a reading of the schema that knows just
//! the words the schema is written in.

use std::path::{Path, PathBuf};

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

fn top() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

/// Every blueprint under a folder, by its file.
fn blueprints_under(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            blueprints_under(&path, found);
        } else if brand::is_blueprint_path(&path.to_string_lossy()) {
            found.push(path);
        }
    }
}

/// What is wrong with `value` by `schema`, each thing with where it is.
/// It reads the words `blueprint::schema` writes and no others, and says
/// so of one it does not know: a schema that came to be written with a
/// new word is not taken to hold for want of anybody reading it.
fn wrong(schema: &Value, value: &Value, at: &str, root: &Value, found: &mut Vec<String>) {
    let Some(said) = schema.as_object() else { return };
    for (word, rule) in said {
        match word.as_str() {
            "$schema" | "$id" | "title" | "description" | "definitions" | "then" => {}
            "$ref" => {
                let name = rule.as_str().unwrap().strip_prefix("#/definitions/").unwrap();
                wrong(&root["definitions"][name], value, at, root, found);
            }
            "type" => {
                let is = |kind: &str| match kind {
                    "object" => value.is_object(),
                    "array" => value.is_array(),
                    "string" => value.is_string(),
                    "boolean" => value.is_boolean(),
                    "null" => value.is_null(),
                    "integer" => value.is_i64() || value.is_u64(),
                    "number" => value.is_number(),
                    other => panic!("the schema says a type this does not know: {other}"),
                };
                let kinds: Vec<&str> = match rule {
                    Value::String(one) => vec![one.as_str()],
                    many => many.as_array().unwrap().iter().map(|kind| kind.as_str().unwrap()).collect(),
                };
                if !kinds.iter().any(|kind| is(kind)) {
                    found.push(format!("{at}: is not {}: {value}", kinds.join(" or ")));
                }
            }
            "const" => {
                if value != rule {
                    found.push(format!("{at}: is not {rule}"));
                }
            }
            "required" => {
                for name in rule.as_array().unwrap() {
                    if value.is_object() && value.get(name.as_str().unwrap()).is_none() {
                        found.push(format!("{at}: has no {name}"));
                    }
                }
            }
            "properties" => {
                for (name, of) in rule.as_object().unwrap() {
                    if let Some(inner) = value.get(name) {
                        wrong(of, inner, &format!("{at}.{name}"), root, found);
                    }
                }
            }
            "patternProperties" => {}
            "additionalProperties" => {
                let named = |name: &str| said.get("properties").is_some_and(|all| all.get(name).is_some());
                // The one pattern there is: a name written out, dot by dot.
                let handed_on = |name: &str| {
                    said.get("patternProperties").and_then(Value::as_object).is_some_and(|patterns| {
                        patterns.keys().any(|pattern| pattern.replace(['^', '$', '\\'], "") == name)
                    })
                };
                for (name, inner) in value.as_object().into_iter().flatten() {
                    if named(name) || handed_on(name) {
                        continue;
                    }
                    match rule {
                        Value::Bool(false) => found.push(format!("{at}: `{name}` is not a word there")),
                        Value::Bool(true) => {}
                        of => wrong(of, inner, &format!("{at}.{name}"), root, found),
                    }
                }
            }
            "items" => {
                for (index, inner) in value.as_array().into_iter().flatten().enumerate() {
                    wrong(rule, inner, &format!("{at}[{index}]"), root, found);
                }
            }
            "anyOf" => {
                let fits = rule.as_array().unwrap().iter().any(|one| {
                    let mut inner = Vec::new();
                    wrong(one, value, at, root, &mut inner);
                    inner.is_empty()
                });
                if !fits {
                    found.push(format!("{at}: is none of what it may be: {value}"));
                }
            }
            "allOf" => {
                for one in rule.as_array().unwrap() {
                    wrong(one, value, at, root, found);
                }
            }
            "not" => {
                let mut inner = Vec::new();
                wrong(rule, value, at, root, &mut inner);
                // The one `not` there is says two fields that are never both there.
                if inner.is_empty() && value.is_object() {
                    found.push(format!("{at}: has both of {}", rule["required"]));
                }
            }
            "if" => {
                let mut inner = Vec::new();
                wrong(rule, value, at, root, &mut inner);
                if inner.is_empty() {
                    wrong(&said["then"], value, at, root, found);
                }
            }
            other => panic!("the schema says a word this does not know: {other}"),
        }
    }
}

/// What a reader of YAML that is not this program's makes of a text: the
/// one thing written in it, as the data it is. It reads YAML 1.2, as an
/// editor's does, and says itself which plain word is a number or a flag.
fn as_yaml_reads(text: &str) -> Result<Value, String> {
    use yaml_rust2::Yaml;
    fn data(read: &Yaml) -> Result<Value, String> {
        Ok(match read {
            Yaml::Null => Value::Null,
            Yaml::Boolean(flag) => json!(flag),
            Yaml::Integer(number) => json!(number),
            Yaml::Real(written) => json!(written.parse::<f64>().map_err(|err| format!("{written}: {err}"))?),
            Yaml::String(text) => json!(text),
            Yaml::Array(items) => Value::Array(items.iter().map(data).collect::<Result<_, _>>()?),
            Yaml::Hash(entries) => {
                let mut map = serde_json::Map::new();
                for (key, value) in entries {
                    // A key is the word that was written, whatever a
                    // reader would take that word for as a value.
                    let key = match key {
                        Yaml::String(text) | Yaml::Real(text) => text.clone(),
                        Yaml::Integer(number) => number.to_string(),
                        Yaml::Boolean(flag) => flag.to_string(),
                        other => return Err(format!("a key that is no word: {other:?}")),
                    };
                    map.insert(key, data(value)?);
                }
                Value::Object(map)
            }
            other => return Err(format!("something no blueprint has: {other:?}")),
        })
    }
    let read = yaml_rust2::YamlLoader::load_from_str(text).map_err(|why| why.to_string())?;
    match read.as_slice() {
        [one] => data(one),
        many => Err(format!("{} documents, where a blueprint is one", many.len())),
    }
}

fn held_to(schema: &Value, source: &str) -> Vec<String> {
    let read = blueprint::syntax::parse(source).unwrap().to_json();
    let mut found = Vec::new();
    wrong(schema, &read, "blueprint", schema, &mut found);
    found
}

#[tokio::test]
async fn the_schema_kept_in_the_repository_is_the_one_this_program_writes() {
    let rig = Rig::new().await;
    // With the actions this program has by itself: what a package brings
    // is on a factory that has the package, and is in what that factory
    // says of itself.
    let mut own = blueprint::intrinsics();
    own.extend(rig.node().app.engine.registry().specs().specs().iter().cloned());
    let written = format!("{}\n", serde_json::to_string_pretty(&blueprint::schema::schema(&own)).unwrap());

    let path = top().join("contract/blueprint.schema.json");
    if std::env::var_os("WRITE_SCHEMA").is_some() {
        std::fs::write(&path, &written).unwrap();
    }
    let kept = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        kept == written,
        "contract/blueprint.schema.json is not the schema this program writes: a word of the language or an \
         action has changed. Write it again with `WRITE_SCHEMA=1 cargo test -p api the_schema_kept`"
    );
    // What it calls itself is a name, which nothing fetches: no address of
    // anybody's is in it but the name of the dialect it is written in. And
    // the product's name is nowhere in it, so the kept one is not a second
    // place that is written.
    assert!(written.contains("\"$id\": \"urn:blueprint:language\""), "{written}");
    assert!(!written.to_lowercase().contains(&brand::ID.to_lowercase()), "the schema names the product");
    assert_eq!(written.matches("://").count(), 1, "one address-like name, the dialect's");
}

#[tokio::test]
async fn the_daemon_gives_the_schema_with_what_this_device_can_do_to_whoever_may_read_blueprints() {
    let rig = Rig::new().await;
    let schema = rig.read("/api/blueprints/schema").await;
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["required"], json!(["blueprint"]));
    // A word of the language, with the sentence that says what it is for.
    let steps = &schema["properties"]["steps"];
    assert!(steps["description"].as_str().unwrap().starts_with("The work, as a graph of steps."), "{steps}");
    // An action this device has is a name `do` is offered, with what it does.
    // It is said once and pointed at from where a step names what it does.
    assert_eq!(
        schema["definitions"]["step"]["properties"]["do"]["allOf"][0]["$ref"],
        "#/definitions/an-action"
    );
    let offered = schema["definitions"]["an-action"]["anyOf"].as_array().unwrap();
    let shell = offered.iter().find(|one| one["const"] == "shell").expect("`shell` is offered");
    assert!(shell["description"].as_str().is_some_and(|said| !said.is_empty()), "{shell}");

    // It is read by whoever may read blueprints, and by nobody else.
    let narrow = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": ["channels"] })).await;
    let token = narrow["token"].as_str().unwrap();
    let (status, answer) = rig.call(Method::GET, "/api/blueprints/schema", Some(token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{answer}");
}

#[tokio::test]
async fn every_blueprint_that_ships_fits_the_schema_and_one_that_is_wrong_is_marked_where_it_is() {
    let rig = Rig::new().await;
    let schema = rig.read("/api/blueprints/schema").await;

    let mut files = Vec::new();
    blueprints_under(&top().join("blueprints"), &mut files);
    blueprints_under(&top().join("examples"), &mut files);
    assert!(files.len() >= 30, "the blueprints that ship were found: {}", files.len());
    for file in &files {
        let found = held_to(&schema, &std::fs::read_to_string(file).unwrap());
        assert!(
            found.is_empty(),
            "{} is right, and the schema marks it:\n{}",
            file.display(),
            found.join("\n")
        );
    }

    // And what is wrong is marked: a word that is not one, two of what a
    // step has one of, something an action does not take, a flag that is
    // not true or false, and a blueprint that does not say what it is called.
    let marked = |source: &str, wanted: &str| {
        let found = held_to(&schema, source);
        assert!(found.iter().any(|one| one.contains(wanted)), "`{wanted}` of:\n{source}\n{found:?}");
    };
    marked("blueprint: x\nstpes: {}\n", "`stpes` is not a word there");
    marked("blueprint: x\nsteps:\n  a: { do: emit, call: other }\n", "has both of");
    marked("blueprint: x\nsteps:\n  a: { do: shell, with: { rnu: ls } }\n", "`rnu` is not a word there");
    marked("blueprint: x\nsteps:\n  a: { do: emit, allow_failure: sometimes }\n", "is not boolean");
    marked("steps: {}\n", "has no \"blueprint\"");
    // All it was handed may be handed on under the one name for that.
    let handed_on = "blueprint: x\nsteps:\n  a: { do: shell, with: { ...: \"${params}\", run: ls } }\n";
    assert_eq!(held_to(&schema, handed_on), Vec::<String>::new());
}

/// What an editor sees of a blueprint that ships, with nobody having told
/// it what a blueprint is: it reads the file as YAML, with a reader that
/// is not ours, and holds what it read to the schema kept in the
/// repository. So each is read here by such a reader, is to it what it is
/// to this program, value for value, and fits that schema.
#[test]
fn every_blueprint_that_ships_is_to_a_reader_of_yaml_what_it_is_to_this_one_and_fits_the_kept_schema() {
    let kept = std::fs::read_to_string(top().join("contract/blueprint.schema.json")).unwrap();
    let schema: Value = serde_json::from_str(&kept).unwrap();
    let mut files = Vec::new();
    blueprints_under(&top().join("blueprints"), &mut files);
    blueprints_under(&top().join("examples"), &mut files);
    assert!(files.len() >= 30, "the blueprints that ship were found: {}", files.len());
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap();
        let theirs = as_yaml_reads(&text).unwrap_or_else(|why| {
            panic!(
                "{} is not YAML to a reader that is not ours, so an editor marks it: {why}\n\
                 `{} fmt` writes what such a reader takes; lay it out again",
                file.display(),
                brand::CLI
            )
        });
        let ours = blueprint::syntax::parse(&text).unwrap().to_json();
        assert!(
            theirs == ours,
            "{} is one thing to this program and another to a reader of YAML that is not ours:\n{}",
            file.display(),
            differing(&ours, &theirs, "blueprint").join("\n")
        );
        let mut found = Vec::new();
        wrong(&schema, &theirs, "blueprint", &schema, &mut found);
        assert!(found.is_empty(), "{} is marked by the kept schema:\n{}", file.display(), found.join("\n"));
    }

    // And it is a reader that parts what this one takes whole, or this
    // would hold of anything: the two things written plain, each read here
    // and not there, and each the same to both once it is laid out.
    for plain in [
        "blueprint: x\ndescription: A board: its cards\n",
        "blueprint: x\nsteps:\n  a: { do: emit, with: { topic: shop.a, n: ${ params.n + 1 } } }\n",
    ] {
        let ours = blueprint::syntax::parse(plain).unwrap().to_json();
        assert_ne!(as_yaml_reads(plain).ok(), Some(ours.clone()), "{plain}");
        let laid = blueprint::format(plain, &blueprint::Settings::default()).unwrap();
        assert_eq!(as_yaml_reads(&laid), Ok(ours), "{laid}");
    }
}

/// Where two readings of one text differ, each place with what each read.
fn differing(ours: &Value, theirs: &Value, at: &str) -> Vec<String> {
    match (ours, theirs) {
        (Value::Object(ours), Value::Object(theirs)) => {
            let mut keys: Vec<&String> = ours.keys().chain(theirs.keys()).collect();
            keys.sort();
            keys.dedup();
            let null = Value::Null;
            keys.into_iter()
                .flat_map(|key| {
                    let (one, other) = (ours.get(key).unwrap_or(&null), theirs.get(key).unwrap_or(&null));
                    differing(one, other, &format!("{at}.{key}"))
                })
                .collect()
        }
        (Value::Array(ours), Value::Array(theirs)) if ours.len() == theirs.len() => ours
            .iter()
            .zip(theirs)
            .enumerate()
            .flat_map(|(index, (one, other))| differing(one, other, &format!("{at}[{index}]")))
            .collect(),
        (ours, theirs) if ours == theirs => Vec::new(),
        (ours, theirs) => vec![format!("{at}: here {ours}, there {theirs}")],
    }
}
