//! Turns `brand.toml` into constants.
//!
//! Two rules keep this script off the rebuild path: it watches exactly one
//! file that always exists, and it emits nothing derived from the clock or the
//! environment. A no-op build therefore never reruns it.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::{env, fs};

const REQUIRED: &[&str] = &["display", "id", "cli", "env_prefix", "app_id", "blueprint_ext"];

/// What may be left empty, and is the empty text where the file says
/// nothing of it: whoever builds the product fills it in or does not.
const OPTIONAL: &[&str] = &["support"];

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets this"));
    let source = manifest.join("../../brand.toml");
    println!("cargo:rerun-if-changed={}", source.display());

    let text =
        fs::read_to_string(&source).unwrap_or_else(|err| panic!("cannot read {}: {err}", source.display()));
    let pairs = parse(&text);

    let mut out = String::new();
    for key in REQUIRED {
        let value = pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .unwrap_or_else(|| panic!("brand.toml is missing `{key}`"));
        assert!(!value.is_empty(), "brand.toml: `{key}` is empty");
        writeln!(out, "pub const {}: &str = {value:?};", key.to_uppercase()).unwrap();
    }
    for key in OPTIONAL {
        let value = pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str()).unwrap_or_default();
        writeln!(out, "pub const {}: &str = {value:?};", key.to_uppercase()).unwrap();
    }

    let dest = PathBuf::from(env::var("OUT_DIR").expect("cargo sets this")).join("brand.rs");
    fs::write(dest, out).expect("write generated constants");
}

/// Flat `key = "value"` lines, comments and blank lines. Anything else is a
/// build error, which is the point: this file must stay trivially parseable by
/// every tool that reads it.
fn parse(text: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parsed = line.split_once('=').and_then(|(key, value)| {
            let value = value.trim().strip_prefix('"')?.strip_suffix('"')?;
            Some((key.trim().to_string(), value.to_string()))
        });
        match parsed {
            Some(pair) => pairs.push(pair),
            None => panic!("brand.toml:{}: expected `key = \"value\"`", index + 1),
        }
    }
    pairs
}
