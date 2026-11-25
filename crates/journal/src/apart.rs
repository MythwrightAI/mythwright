//! How a row kept beside the events says whose it is.
//!
//! A project is to have a journal of its own, and what is kept of a
//! project beside its events (its records, what was said of them) is to
//! be kept in that journal's file. Moving a row there, and asking the
//! right file for it afterwards, is never to be a matter of working out
//! whose the row is: every table a crate keeps here has a line that says
//! how a row of it is told apart by project ([`Told`]), in a list of the
//! crate's own, and a test of that crate holds every row to its line.
//! Whatever moves a project's rows goes by the lists and knows no table.
//!
//! What runs keeps a list of its own in its own words (`runs::apart`),
//! since its rows are reached through an instance or a run.

/// How a row of a table is told apart by project.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Told {
    /// It is the factory's own, whatever it names: nobody's, everybody's,
    /// or the list that something of a project is found in.
    Factory,
    /// It says, in the column named. Nothing there, or the empty text,
    /// is nobody's.
    Says(&'static str),
    /// It is of a row of another table kept beside it: the row of
    /// `table` whose `key` is what this one holds in `by`, which says
    /// in `says`. Where there is no such row it is nobody's.
    Through { by: &'static str, table: &'static str, key: &'static str, says: &'static str },
    /// It is read a way of its own: what reads the project of a row
    /// called `kept`, as the database is asked it.
    Reads(&'static str),
}

impl Told {
    /// What reads the project of a row called `kept`, as the database is
    /// asked it: nothing for a row that is nobody's or the factory's own.
    pub fn reads(self) -> String {
        match self {
            Told::Factory => "NULL".to_string(),
            Told::Says(column) => format!("nullif(kept.{column}, '')"),
            Told::Through { by, table, key, says } => {
                format!("(SELECT nullif(of_it.{says}, '') FROM {table} of_it WHERE of_it.{key} = kept.{by})")
            }
            Told::Reads(how) => format!("({how})"),
        }
    }

    /// Whether a row of such a table can be a project's at all.
    pub fn of_a_project(self) -> bool {
        self != Told::Factory
    }
}

/// How `table` is told apart, by a crate's list.
pub fn told(tables: &[(&'static str, Told)], table: &str) -> Option<Told> {
    tables.iter().find(|(name, _)| *name == table).map(|(_, told)| *told)
}
