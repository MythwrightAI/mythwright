//! The statements a window of a table is read by, and those that make
//! and take away the index of a column it is read in order by: functions
//! of plain values, so that what is asked of the database is said where
//! the order it keeps is (`kernel::window`), and the studio only asks it.
//!
//! Each names the index it reads by (`INDEXED BY`) and says the project
//! and the table as literals: an index of one table's column holds that
//! table's rows alone, and is used only where a statement says the same
//! (asked with parameters for them, the database answered that it had no
//! way to read it). So nothing a window costs rests on what the database
//! guesses.

use super::{Gather, Key, Order, Place, Stretch, key_sql, literal};

/// Below every number and every word, as a bound a walk begins at: what
/// is past it says something. Said as a range and not as `IS NOT NULL`,
/// which the database walks to by every row that says nothing.
const SAID: &str = "-9e999";

/// What every one of these is called, before what it is of.
pub const NAMED: &str = "records_by ";

/// What the index of `field` of `project`'s `table` is called.
pub fn name(project: &str, table: &str, field: &str) -> String {
    format!("{NAMED}{project}/{table}/{field}")
}

/// That, as a statement names it.
pub fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// The statement that takes the index of a column away, where there is one.
pub fn dropped(project: &str, table: &str, field: &str) -> String {
    format!("DROP INDEX IF EXISTS {};", quoted(&name(project, table, field)))
}

/// The statement that makes the index of `column` of `project`'s `table`.
pub fn made(project: &str, table: &str, column: &super::Column) -> String {
    format!(
        "CREATE INDEX IF NOT EXISTS {} ON records (project, collection, {}, number) WHERE project = {} AND collection = {}",
        quoted(&name(project, table, &column.name)),
        key_sql(column),
        literal(project),
        literal(table)
    )
}

/// What every statement of one table's window says first.
#[derive(Clone, Debug)]
pub struct Shape {
    /// The columns of a record each statement reads first, before what it reads besides.
    pub record: String,
    pub project: String,
    pub table: String,
    pub order: Order,
    /// What the view checks of each row ([`super::shows_sql`]).
    pub shows: String,
}

impl Shape {
    fn of(&self) -> String {
        format!("project = {} AND collection = {}", literal(&self.project), literal(&self.table))
    }

    fn key(&self) -> String {
        self.order.column.as_ref().map_or("NULL".into(), key_sql)
    }

    /// The index the order is walked by.
    pub fn index(&self) -> String {
        match &self.order.column {
            None => "records_numbered".into(),
            Some(column) => quoted(&name(&self.project, &self.table, &column.name)),
        }
    }

    /// What of one stretch is walked from a place (or from that place,
    /// where `inclusive`), one way: the range and where it begins, and
    /// whether it is ordered by the key before the number.
    fn walked(&self, stretch: usize, from: Option<(&Place, bool)>, ascending: bool) -> (String, bool) {
        let key = self.key();
        let (gt, ge) = if ascending { (">", ">=") } else { ("<", "<=") };
        let (range, by_key) = match &self.order.stretches[stretch] {
            Stretch::Numbered => (String::new(), false),
            // Said as a range, so that the walk begins past what says nothing.
            Stretch::Said => (format!(" AND {key} > {SAID}"), true),
            Stretch::Option(option) => (format!(" AND {key} = {}", literal(option)), false),
            Stretch::Others(offered) => {
                let offered: Vec<String> = offered.iter().map(|one| literal(one)).collect();
                (format!(" AND {key} > {SAID} AND {key} NOT IN ({})", offered.join(", ")), true)
            }
            Stretch::Empty => (format!(" AND {key} IS NULL"), false),
        };
        let from = match from {
            None => String::new(),
            Some((place, inclusive)) => {
                let after = if inclusive { ge } else { gt };
                let number = place.number;
                if by_key {
                    let at = place.key.sql();
                    format!(" AND {key} {ge} {at} AND ({key} {gt} {at} OR number {after} {number})")
                } else {
                    format!(" AND number {after} {number}")
                }
            }
        };
        (format!("{range}{from}"), by_key)
    }

    /// The rows of one stretch, from a place (or from that place, where
    /// `inclusive`), walking its index one way.
    pub fn walk(&self, stretch: usize, from: Option<(&Place, bool)>, ascending: bool) -> String {
        let key = self.key();
        let dir = if ascending { "ASC" } else { "DESC" };
        let (range, by_key) = self.walked(stretch, from, ascending);
        let order = if by_key { format!("{key} {dir}, number {dir}") } else { format!("number {dir}") };
        format!(
            "SELECT {}, {key} AS k, ({}) AS ok FROM records INDEXED BY {} WHERE {}{range} ORDER BY {order}",
            self.record,
            self.shows,
            self.index(),
            self.of()
        )
    }

    /// How many rows of one stretch the view shows that come before
    /// `place` (all of them, where none is said): read off the index
    /// alone where the view checks nothing of a row.
    pub fn before(&self, stretch: usize, place: Option<&Place>) -> String {
        // Before a place in the order is after it walking the other way.
        let (range, _) = self.walked(stretch, place.map(|place| (place, false)), self.order.down);
        let shows = if self.shows == "1" { String::new() } else { format!(" AND ({})", self.shows) };
        format!("SELECT count(*) FROM records INDEXED BY {} WHERE {}{range}{shows}", self.index(), self.of())
    }

    /// Which stretch a row is in, as the statement says it.
    fn stretch_sql(&self) -> String {
        let key = self.key();
        let whens: Vec<String> = self
            .order
            .stretches
            .iter()
            .enumerate()
            .map(|(at, stretch)| match stretch {
                Stretch::Numbered => format!("WHEN 1 THEN {at}"),
                Stretch::Said => format!("WHEN {key} IS NOT NULL THEN {at}"),
                Stretch::Option(option) => format!("WHEN {key} = {} THEN {at}", literal(option)),
                Stretch::Others(_) => format!("WHEN {key} IS NOT NULL THEN {at}"),
                Stretch::Empty => format!("WHEN {key} IS NULL THEN {at}"),
            })
            .collect();
        format!("CASE {} END", whens.join(" "))
    }

    /// What rows sorted in the statement are read from: the index they
    /// are gathered by and what they hold there, and where they begin.
    fn sorted_where(
        &self,
        gather: Option<&Gather>,
        from: Option<(&Place, bool)>,
        forward: bool,
    ) -> (String, String) {
        let (key, stretch) = (self.key(), self.stretch_sql());
        let ascending = forward != self.order.down;
        let (gt, ge) = if ascending { (">", ">=") } else { ("<", "<=") };
        let sgt = if forward { ">" } else { "<" };
        let (index, gathered) = match gather {
            None => (String::new(), String::new()),
            Some(gather) => {
                let keys: Vec<String> = gather.keys.iter().map(Key::sql).collect();
                (
                    format!(" INDEXED BY {}", quoted(&name(&self.project, &self.table, &gather.column.name))),
                    format!(" AND {} IN ({})", key_sql(&gather.column), keys.join(", ")),
                )
            }
        };
        let from = match from {
            None => String::new(),
            Some((place, inclusive)) => {
                let (at, s, n) = (place.key.sql(), place.stretch, place.number);
                let after = if inclusive { ge } else { gt };
                format!(
                    " AND ({stretch} {sgt} {s} OR ({stretch} = {s} AND ({key} {gt} {at} OR ({key} IS {at} AND number {after} {n}))))"
                )
            }
        };
        (index, format!("{}{gathered} AND ({}){from}", self.of(), self.shows))
    }

    /// The rows in order sorted in the statement: of a small table, or
    /// gathered by an indexed column a view is held to.
    pub fn sorted(
        &self,
        gather: Option<&Gather>,
        from: Option<(&Place, bool)>,
        forward: bool,
        take: usize,
        skip: u64,
    ) -> String {
        let (key, stretch) = (self.key(), self.stretch_sql());
        let ascending = forward != self.order.down;
        let dir = if ascending { "ASC" } else { "DESC" };
        let sdir = if forward { "ASC" } else { "DESC" };
        let (index, at) = self.sorted_where(gather, from, forward);
        format!(
            "SELECT {}, {key} AS k, {stretch} AS s FROM records{index} WHERE {at} \
             ORDER BY s {sdir}, k {dir}, number {dir} LIMIT {take} OFFSET {skip}",
            self.record
        )
    }

    /// How many of the rows sorted in the statement come before `place`.
    pub fn sorted_before(&self, gather: Option<&Gather>, place: &Place) -> String {
        let (index, at) = self.sorted_where(gather, Some((place, false)), false);
        format!("SELECT count(*) FROM records{index} WHERE {at}")
    }

    /// How many rows a gathering comes to, read off its index, as far as
    /// one more than [`super::GATHER`].
    pub fn gathered(&self, gather: &Gather) -> String {
        let keys: Vec<String> = gather.keys.iter().map(Key::sql).collect();
        format!(
            "SELECT count(*) FROM (SELECT 1 FROM records INDEXED BY {} WHERE {} AND {} IN ({}) LIMIT {})",
            quoted(&name(&self.project, &self.table, &gather.column.name)),
            self.of(),
            key_sql(&gather.column),
            keys.join(", "),
            super::GATHER + 1
        )
    }
}

/// What every statement read off one column's index says first.
fn off_index(project: &str, table: &str, column: &super::Column) -> String {
    format!(
        "FROM records INDEXED BY {} WHERE project = {} AND collection = {}",
        quoted(&name(project, table, &column.name)),
        literal(project),
        literal(table)
    )
}

/// The statement a figure of a column is read off its index by, where it
/// can be: how many rows say something there, but in words (those that
/// say nothing are what is left), how many are ticked, and the least and the greatest
/// number. What adds up, and how many different things, read the rows.
pub fn figure(project: &str, table: &str, column: &super::Column, which: &str) -> Option<String> {
    let (from, key) = (off_index(project, table, column), key_sql(column));
    match which {
        // Words may be nothing but spaces, which say nothing: whether they
        // do is no part of the index, so a text's are counted by its rows.
        "filled" | "empty" if column.how() != Some(super::How::Words) => {
            Some(format!("SELECT count(*) {from} AND {key} > {SAID}"))
        }
        "ticked" if column.kind == "flag" => Some(format!("SELECT count(*) {from} AND {key} = 1")),
        "least" | "greatest" if column.kind == "number" => {
            let way = if which == "least" { "ASC" } else { "DESC" };
            // Numbers are below every word, so these are the numbers.
            Some(format!(
                "SELECT {key} {from} AND {key} > {SAID} AND {key} < '' ORDER BY {key} {way} LIMIT 1"
            ))
        }
        _ => None,
    }
}

/// The next value of a column after `after` (the first, after nothing),
/// read off its index: one step of going through what it holds.
pub fn next_value(project: &str, table: &str, column: &super::Column, after: &Key) -> String {
    let (from, key) = (off_index(project, table, column), key_sql(column));
    let past = match after {
        Key::Null => format!("{key} > {SAID}"),
        after => format!("{key} > {}", after.sql()),
    };
    format!("SELECT {key} {from} AND {past} ORDER BY {key} LIMIT 1")
}

/// How many rows hold `value` in a column (nothing, for those that hold
/// nothing), read off its index.
pub fn holding(project: &str, table: &str, column: &super::Column, value: &Key) -> String {
    let (from, key) = (off_index(project, table, column), key_sql(column));
    match value {
        Key::Null => format!("SELECT count(*) {from} AND {key} IS NULL"),
        value => format!("SELECT count(*) {from} AND {key} = {}", value.sql()),
    }
}
