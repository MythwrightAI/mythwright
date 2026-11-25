//! A project called something else: its display name (`Project::name`),
//! by whoever may write its files, which is what arranging it takes too.
//! Its id is what everything knows it by (an address, a table's records,
//! a channel's project, a terminal's word) and never changes: only what
//! people read it as does.
//!
//! The row and what is said of it (`project.renamed`) are one write of the
//! factory's journal, where a project's row is whatever journal keeps the
//! rest of what is said of it (`journal::HERE`), and the name is looked at
//! for another project's inside that write, so that two renamed at once to
//! one name are not both kept.

use super::*;

/// The most letters a project's name has.
pub const LONGEST_NAME: usize = 80;

/// What a project is to be called, trimmed, or why `name` is not that.
pub fn named(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::invalid("a project is called something: say what it is to be called"));
    }
    if name.chars().any(char::is_control) {
        return Err(Error::invalid("a project's name is one line of words"));
    }
    let letters = name.chars().count();
    if letters > LONGEST_NAME {
        return Err(Error::invalid(format!(
            "a project's name is {LONGEST_NAME} letters at most, and this one is {letters}"
        )));
    }
    Ok(name.to_string())
}

impl Projects {
    /// Calls a project something else, for everybody, as `by`: refused
    /// where the name is empty or another project is called it, whatever
    /// its capitals. One that comes to what it is called is no write and
    /// says nothing.
    pub async fn rename(&self, id: &str, name: &str, by: &Principal) -> Result<Project> {
        by.require("projects:write")?;
        let name = named(name)?;
        // Its id, and so its address, stays as it is; but a project is never called what the factory's own
        // addresses begin with, by its name either (`kernel::paths`), as it is refused where one is made.
        let word = blueprint::expr::slug(&name);
        if kernel::paths::refused(&word).is_some() {
            return Err(Error::invalid(format!(
                "a project cannot be called {name}: `/{word}` is where this factory's own pages and files are \
                 found, and a project is never called what they are. Call it something else"
            )));
        }
        self.get(id).await?;
        let (key, called, actor) = (id.to_string(), name.clone(), by.actor());
        let (came, _) = self
            .journal
            .write(move |writing| {
                let transaction = writing.transaction();
                let was: Option<String> = optional(transaction.query_row(
                    "SELECT name FROM projects WHERE id = ?1",
                    [&key],
                    |row| row.get(0),
                ))?;
                let Some(was) = was else {
                    return Ok(Err(Error::not_found(format!("there is no project `{key}`"))));
                };
                if was == called {
                    return Ok(Ok(()));
                }
                let other: Option<String> = optional(transaction.query_row(
                    "SELECT id FROM projects WHERE id != ?1 AND lower(trim(name)) = lower(?2)",
                    [&key, &called],
                    |row| row.get(0),
                ))?;
                if let Some(other) = other {
                    return Ok(Err(Error::conflict(format!(
                        "another project is called {called} (`{other}`): call this one something else"
                    ))));
                }
                transaction.execute("UPDATE projects SET name = ?2 WHERE id = ?1", params![key, called])?;
                let said = json!({ "project": key, "name": called, "was": was, "by": actor });
                writing.say_here("project.renamed", Source::actor(actor.clone()), said)?;
                Ok(Ok(()))
            })
            .await?;
        came?;
        self.get(id).await
    }
}
