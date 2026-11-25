//! Whose each row is, and what is asked of the rows by project.
//!
//! A project is to have a journal of its own, and a conductor of its own
//! that runs what was applied for it. What is kept here is then to be
//! moved, project by project, and taken up by each project's conductor,
//! and neither is to be a matter of working out whose a row is. So every
//! row says its project or is reached, by its own key, from one that
//! does ([`TABLES`]), and what a project's conductor will ask as it
//! starts is asked of what holds its own rows together and reads nobody
//! else's ([`Store::alive_of`], [`Store::under_way_of`],
//! [`Store::places_of`]): what is alive and what is under way are kept
//! by project in small tables of their own, by the database itself, as
//! an index would be (an index over the tables that hold everything
//! that ever ran could not be made without reading all of them).
//!
//! It rests on two things that are held where the rows are written. A
//! tree of instances never spans two projects: an instance is of its
//! parent's project, or is not written ([`Store::insert_instance`]). And
//! what says whose a row is never changes: an instance's project, a
//! run's instance, and what a run or a place copied from its instance's
//! row are written once, and the database refuses a write that would
//! make any of them another (the rules made with the tables).
//!
//! A row that is nobody's is the factory's own: applied for no project.
//! An instance says so by saying nothing (`project` is `NULL`, as it
//! always was). A run and a place say so by the empty text, so that one
//! that says nothing is told from them: a run a program from before
//! wrote, which is reached through its instance.

use journal::optional;
use kernel::Result;

use crate::places::Place;
use crate::store::{
    INSTANCE_COLUMNS, InstanceInfo, RUN_COLUMNS, RunInfo, Store, collect, read_instance, read_run,
};

/// How a row of a table kept here is told apart by project.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Told {
    /// It says, in `project`: nothing where it is nobody's.
    Says,
    /// It is reached through the instance that the column names, which
    /// is the first of its key: one indexed step.
    Instance(&'static str),
    /// It says, in `project` (the empty text where it is nobody's),
    /// copied from its instance's row in the write that made it; and one
    /// that says nothing is reached through the instance that the column
    /// names.
    SaysOrInstance(&'static str),
    /// It is reached through the run that the column names, which is the
    /// first of its key, and so through what the run says or reaches.
    Run(&'static str),
}

/// Every table kept here, each with how its rows are told apart by
/// project. A table added to what the runner keeps is a line here (the
/// suite fails on one that is in a copy and not here), and what it says
/// is held by a test that reads every row of each.
///
/// What is kept in a column of one of these rows (how far what is done
/// around a step has come, what an instance owes, what a run is to carry
/// its instance over to, the labels an instance carries) goes with its
/// row.
pub const TABLES: &[(&str, Told)] = &[
    // The project it was applied for, which it always said.
    ("instances", Told::Says),
    // The definitions an instance was given: its instance's.
    ("instance_versions", Told::Instance("instance")),
    // A run under way is found by project, so it says; one that had
    // ended before it could is its instance's, as every run is.
    ("runs", Told::SaysOrInstance("instance")),
    // A step is found by its run and never by project: it goes where
    // its run goes.
    ("steps", Told::Run("run")),
    // And so does how far a loop of one has come, which is there only
    // while the run is under way.
    ("step_elements", Told::Run("run")),
    // Found by project as a project's conductor starts: it says.
    ("places", Told::SaysOrInstance("instance")),
    // What is alive and what is under way, each kept by project by the
    // database itself, from the row it is of.
    ("alive", Told::SaysOrInstance("instance")),
    ("under_way", Told::SaysOrInstance("instance")),
];

/// The project of the instance bound as parameter `n`, as a run or a
/// place says it: for the write that makes one. Read in that write from
/// what the database keeps of the instance, so it is what the rows say
/// and nothing handed in: from what is alive, which is a small row
/// found at once, and for an instance that is over from its own row.
/// The empty text where the instance is nobody's.
pub(crate) fn of_instance(n: usize) -> String {
    format!(
        "coalesce((SELECT project FROM alive WHERE instance = ?{n}),
                  (SELECT project FROM instances WHERE id = ?{n}), '')"
    )
}

impl Told {
    /// What reads the project of a row called `kept`, as the database is
    /// asked it: nothing for one that is nobody's.
    pub fn reads(self) -> String {
        let through = |row: &str, column: &str| {
            format!(
                "CASE WHEN {row}.project IS NULL
                      THEN (SELECT project FROM instances WHERE id = {row}.{column})
                      ELSE nullif({row}.project, '') END"
            )
        };
        match self {
            Told::Says => "kept.project".to_string(),
            Told::Instance(column) => format!("(SELECT project FROM instances WHERE id = kept.{column})"),
            Told::SaysOrInstance(column) => through("kept", column),
            Told::Run(column) => {
                format!(
                    "(SELECT {} FROM runs of_run WHERE of_run.id = kept.{column})",
                    through("of_run", "instance")
                )
            }
        }
    }
}

/// What a run or a place says for `project`: the empty text for nobody's.
pub(crate) fn said(project: Option<&str>) -> String {
    project.unwrap_or_default().to_string()
}

/// A list of columns, each said to be `table`'s: for what is read
/// beside another table that has a column of the same name.
pub(crate) fn of_table(table: &str, columns: &str) -> String {
    // A column that is read by an expression says its table itself.
    let named = |column: &str| match column.contains(' ') {
        true => column.to_string(),
        false => format!("{table}.{column}"),
    };
    columns.split(',').map(|column| named(column.trim())).collect::<Vec<_>>().join(", ")
}

impl Store {
    /// The project an instance is of: nothing inside for one that is
    /// nobody's, and nothing at all where there is no such instance.
    pub async fn project_of(&self, instance: &str) -> Result<Option<Option<String>>> {
        let instance = instance.to_string();
        let began = std::time::Instant::now();
        let read = self.db.call(move |connection| {
            optional(connection.query_row(
                "SELECT project FROM instances WHERE id = ?1",
                [&instance],
                |row| row.get(0),
            ))
        });
        let read = read.await;
        self.looked_up("project", began, read.is_ok());
        read
    }

    /// The project a run is of, the same way: what it says, or its
    /// instance's where a program from before wrote it.
    pub async fn project_of_run(&self, run: &str) -> Result<Option<Option<String>>> {
        let run = run.to_string();
        let reads = Told::SaysOrInstance("instance").reads();
        let began = std::time::Instant::now();
        let read = self.db.call(move |connection| {
            optional(connection.query_row(
                &format!("SELECT {reads} FROM runs kept WHERE kept.id = ?1"),
                [&run],
                |row| row.get(0),
            ))
        });
        let read = read.await;
        self.looked_up("run", began, read.is_ok());
        read
    }

    /// Every instance of `project` that is alive, in the order they were
    /// made; with no project, those that are nobody's. Found by what is
    /// kept of what is alive, by project: no row of another project's is
    /// read, however much the others have.
    pub async fn alive_of(&self, project: Option<&str>) -> Result<Vec<InstanceInfo>> {
        let project = said(project);
        let columns = of_table("instances", INSTANCE_COLUMNS);
        self.db
            .call(move |connection| {
                collect(
                    connection,
                    &format!(
                        "SELECT {columns} FROM alive INDEXED BY alive_project
                         CROSS JOIN instances ON instances.id = alive.instance
                         WHERE alive.project = ?1 ORDER BY instances.rowid"
                    ),
                    [&project],
                    read_instance,
                )
            })
            .await
    }

    /// The runs of `project` that are under way, in the order they were
    /// made, without their steps; with no project, those of what is
    /// nobody's. Found by what is kept of what is under way, by project.
    pub async fn under_way_of(&self, project: Option<&str>) -> Result<Vec<RunInfo>> {
        let project = said(project);
        let columns = of_table("runs", RUN_COLUMNS);
        self.db
            .call(move |connection| {
                collect(
                    connection,
                    &format!(
                        "SELECT {columns} FROM under_way CROSS JOIN runs ON runs.id = under_way.run
                         WHERE under_way.project = ?1 ORDER BY runs.rowid"
                    ),
                    [&project],
                    read_run,
                )
            })
            .await
    }

    /// Where each handler of `project` has got to, in each journal it has
    /// a place in; with no project, those of what is nobody's. Read from
    /// the index of places by project.
    pub async fn places_of(&self, project: Option<&str>) -> Result<Vec<Place>> {
        let project = said(project);
        self.db
            .call(move |connection| {
                let mut statement = connection.prepare(
                    "SELECT instance, handler, journal, seq FROM places INDEXED BY places_project
                     WHERE project = ?1 ORDER BY instance, handler, journal",
                )?;
                let places = statement.query_map([&project], |row| {
                    Ok(Place {
                        instance: row.get(0)?,
                        handler: row.get(1)?,
                        journal: row.get(2)?,
                        seq: row.get::<_, i64>(3)? as u64,
                    })
                })?;
                places.collect()
            })
            .await
    }
}
