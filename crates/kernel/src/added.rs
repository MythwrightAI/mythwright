//! A column somebody added to a table, and what somebody chose of any
//! column: the plain values of it, with no table and no database.
//!
//! A table's columns are said where it is declared, and whoever looks at
//! the table may add more, call any of them something else and put what
//! a choice offers in another order. What they add and choose is the
//! table's, kept with it for everybody, and stands beside what is
//! declared. This is what can be worked out of that from names and words
//! alone: what a column somebody adds may be called and may hold, which
//! of the added ones still stand once the table is declared again, and
//! what order a choice's options are in when somebody chose one and the
//! declaration then says others. (`kernel::columns` is another thing:
//! what a *view* says of the columns there are.)
//!
//! A column's name is what everything else knows it by: the key in every
//! record, in a file, in a view, in a blueprint that reads it. So a name
//! is made once, from what the column was first called, and is never
//! changed; what somebody calls a column afterwards is a label that
//! stands over the name.

use crate::{Error, Result};

/// The scope adding a column to a table takes, and changing what kind of
/// thing one holds, what can be chosen in it, and taking it away.
pub const SCOPE: &str = "columns";

/// What a column somebody adds can hold: the kinds of field with no
/// authority in them. An `actor` is left out: it names somebody, and the
/// one there is that anything acts on (who a card is given to) is its
/// table's to declare.
pub const KINDS: &[&str] = &["text", "number", "date", "flag", "choice", "list", "record", "records"];

/// What no column somebody adds is called: what every record has without
/// declaring it, how a record kept as a file says where it stands, and
/// what a record is known by and its text, which are its table's to say.
pub const RESERVED: &[&str] = &[
    "id",
    "number",
    "project",
    "collection",
    "position",
    "created",
    "updated",
    "created_by",
    "title",
    "body",
];

/// The most letters a column is called by, and the most a name has.
pub const LONGEST: usize = 64;

/// The most things a choice somebody made offers.
pub const MOST: usize = 200;

/// What a column is to be called, as it is kept: one line, without the
/// space round it; or why what was written is not that.
pub fn label(written: &str) -> Result<String> {
    let label = written.trim();
    let count = label.chars().count();
    if count > 0 && count <= LONGEST && !label.chars().any(char::is_control) {
        return Ok(label.to_string());
    }
    let shown: String = label.chars().filter(|letter| !letter.is_control()).take(LONGEST).collect();
    Err(Error::invalid(format!(
        "a column is called by a word or a few, on one line and {LONGEST} letters at most; `{shown}` is not that"
    )))
}

/// The name a column first called `label` is given: its letters and
/// digits in lowercase, with `_` where anything else was, beginning with
/// a letter. It is what a record's file, a view and a blueprint call the
/// column, so it is written one way and reads in an expression as a
/// word. A label with no such letter in it (one in another alphabet)
/// comes to `column`: the label is still what the column is called.
pub fn name(label: &str) -> String {
    let mut name = String::new();
    for letter in label.trim().chars() {
        if letter.is_ascii_alphanumeric() {
            name.push(letter.to_ascii_lowercase());
        } else if !name.is_empty() && !name.ends_with('_') {
            name.push('_');
        }
    }
    let name = name.trim_end_matches('_');
    let name = match name.chars().next() {
        None => "column".to_string(),
        Some(first) if first.is_ascii_digit() => format!("column_{name}"),
        Some(_) => name.to_string(),
    };
    name.chars().take(LONGEST).collect::<String>().trim_end_matches('_').to_string()
}

/// How a name reads where a column is headed by it and nobody called it
/// anything: its first letter large, and a space where a dash or `_` is.
pub fn reads(name: &str) -> String {
    let mut letters = name.chars();
    match letters.next() {
        None => String::new(),
        Some(first) => first
            .to_uppercase()
            .chain(letters.map(|one| if one == '-' || one == '_' { ' ' } else { one }))
            .collect(),
    }
}

/// `name`, or the first of `name_2`, `name_3` and so on that nothing is
/// called: no column there is (`taken`), and nothing a column may not be
/// called.
pub fn free(name: &str, taken: &[&str]) -> String {
    let is_free = |one: &str| !taken.contains(&one) && !RESERVED.contains(&one);
    if is_free(name) {
        return name.to_string();
    }
    (2..).map(|nth| format!("{name}_{nth}")).find(|one| is_free(one)).unwrap_or_else(|| name.to_string())
}

/// What a copy of the column that reads `reads` is called to begin with:
/// that and `copy`, numbered where columns already read so.
pub fn copy(reads: &str, read: &[&str]) -> String {
    let alike = |one: &str| read.iter().any(|kept| kept.eq_ignore_ascii_case(one));
    let first = format!("{reads} copy");
    if !alike(&first) {
        return first;
    }
    (2..).map(|nth| format!("{reads} copy {nth}")).find(|one| !alike(one)).unwrap_or(first)
}

/// Which of the columns somebody added still stand once a table is
/// declared with the fields `declared`: those whose names the
/// declaration does not take. One it takes is the declaration's from
/// then on.
pub fn standing<'a>(declared: &[&str], added: &[&'a str]) -> Vec<&'a str> {
    added.iter().copied().filter(|name| !declared.contains(name)).collect()
}

/// The order a choice's options are in where somebody chose one
/// (`chosen`) and the declaration says `declared`: those of the chosen
/// order that are still declared, as chosen; and each the declaration
/// has that nobody placed, after the one it follows in the declaration
/// (first, where it follows none). With nothing chosen it is the
/// declaration's order.
pub fn ordered(declared: &[String], chosen: &[String]) -> Vec<String> {
    in_order(declared, chosen, |option| declared.contains(option))
}

/// What a choice offers, in order, once it is declared again as
/// `declared` over what somebody chose of it (`chosen`): as [`ordered`],
/// and what they added to it beyond any declaration (`added`: a board's
/// column, arranged in) stands too, where they put it. That is theirs
/// and no declaration's, and a declaration said again, or a new text of
/// the blueprint, takes away only what a declaration offered. Where what
/// they added is not known (nothing said: a table kept before it was),
/// everything chosen stands.
pub fn offered(declared: &[String], added: Option<&[String]>, chosen: &[String]) -> Vec<String> {
    in_order(declared, chosen, |option| {
        declared.contains(option) || added.is_none_or(|added| added.contains(option))
    })
}

/// What of a choice arranged as `chosen` somebody has added to it: what
/// no declaration offers (`declared`), and what they had added before
/// (`added`) that is still chosen, whatever a declaration has come to
/// say of it since. Each once, in the order chosen.
pub fn added(chosen: &[String], declared: &[String], added: &[String]) -> Vec<String> {
    let mut theirs: Vec<String> = Vec::new();
    for option in chosen {
        if (!declared.contains(option) || added.contains(option)) && !theirs.contains(option) {
            theirs.push(option.clone());
        }
    }
    theirs
}

fn in_order(declared: &[String], chosen: &[String], stands: impl Fn(&String) -> bool) -> Vec<String> {
    let mut order: Vec<String> = Vec::new();
    for option in chosen {
        if stands(option) && !order.contains(option) {
            order.push(option.clone());
        }
    }
    if order.is_empty() {
        return declared.to_vec();
    }
    for (at, option) in declared.iter().enumerate() {
        if order.contains(option) {
            continue;
        }
        // After the nearest one before it in the declaration: by now every one before it is placed.
        let after = declared[..at].iter().rev().find_map(|before| order.iter().position(|one| one == before));
        order.insert(after.map_or(0, |after| after + 1), option.clone());
    }
    order
}

/// Whether two lists offer the same things, in whatever order.
pub fn same(one: &[String], other: &[String]) -> bool {
    one.len() == other.len() && one.iter().all(|option| other.contains(option))
}

/// The order a view's columns stand in once the column `name` stands
/// before the column `before` among those that stood as `order`, or
/// after the last of them where `before` is nothing or is not there.
pub fn placed(order: &[String], name: &str, before: Option<&str>) -> Vec<String> {
    let mut placed: Vec<String> = Vec::new();
    for one in order.iter().filter(|one| one.as_str() != name) {
        if !placed.contains(one) {
            placed.push(one.clone());
        }
    }
    let at = before.and_then(|before| placed.iter().position(|one| one == before)).unwrap_or(placed.len());
    placed.insert(at, name.to_string());
    placed
}
