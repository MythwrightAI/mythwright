//! What a package says it can show of a repository, beside what git says
//! of it, as the agents answer for it: which parts the installed packages
//! offer for a repository's remotes, and each part read by the package's
//! own action, as whoever asks. What a package says, and how a row is
//! handed on, are the package's (`extensions::shown`), offered here under
//! the names they always had.

use std::path::Path;

use kernel::Fields;
use serde_json::{Map, Value, json};

pub use extensions::shown::{Column, MOST_ROWS, Offered, Part, Read, Shows};
use extensions::shown::{READING, address, cell};

use crate::Agents;
use crate::performed::{At, once, user};

impl Agents {
    /// What each installed package shows of a repository with these
    /// remotes (each a name and its address as it may be handed on), for
    /// `who`: a package that serves none of them is left out, and one
    /// whose actions they may not use says why.
    pub fn shown_for(&self, remotes: &[(String, String)], who: &kernel::Principal) -> Vec<Offered> {
        self.extensions()
            .iter()
            .filter_map(|extension| {
                let shows = extension.shows.as_ref()?;
                remotes.iter().any(|(name, url)| shows.serves(name, url)).then(|| {
                    let needs = format!("extensions:{}", extension.name);
                    Offered {
                        package: extension.name.clone(),
                        title: if extension.title.is_empty() {
                            extension.name.clone()
                        } else {
                            extension.title.clone()
                        },
                        parts: shows.parts.clone(),
                        refused: (!who.can(&needs)).then(|| {
                            format!("{} may not use the package `{}` ({needs})", who.name, extension.name)
                        }),
                    }
                })
            })
            .collect()
    }

    /// One part a package shows, read in `dir` as `who`, who must hold
    /// what using the package's actions takes.
    pub async fn shown(
        &self,
        package: &str,
        part: &str,
        dir: &Path,
        who: &kernel::Principal,
    ) -> Result<Read, kernel::Error> {
        let extension = self.extension(package).ok_or_else(|| {
            kernel::Error::not_found(format!(
                "the extension package `{package}` is not installed on this device"
            ))
        })?;
        let shows = extension.shows.as_ref().ok_or_else(|| {
            kernel::Error::not_found(format!(
                "the extension package `{package}` shows nothing of a repository"
            ))
        })?;
        let found = shows.parts.iter().find(|one| one.name == part).ok_or_else(|| {
            let has: Vec<&str> = shows.parts.iter().map(|one| one.name.as_str()).collect();
            kernel::Error::not_found(format!(
                "the extension package `{package}` shows no `{part}` of a repository; it shows: {}",
                has.join(", ")
            ))
        })?;
        who.require(&format!("extensions:{package}"))?;
        let declared = extension.actions.get(&found.action).cloned().ok_or_else(|| {
            kernel::Error::internal(format!(
                "`{package}` says `{part}` is read by an action it does not have"
            ))
        })?;
        let daemon = json!({ "url": self.inner.hosting.address().unwrap_or_default(), "user": user() });
        let scope = declared.before(&Fields::new(), &daemon).map_err(kernel::Error::invalid)?;
        let (none, cancel) = (std::collections::BTreeMap::new(), kernel::Cancel::new());
        let at = At {
            workdir: dir.to_path_buf(),
            env: &none,
            secrets: &none,
            asker: &who.name,
            may_give: false,
            cancel: &cancel,
            log: &|_, _| {},
        };
        let asked = once(&extension, &declared, &self.inner.hosting, &at, scope);
        let gave = match tokio::time::timeout(READING, asked).await {
            Ok(Ok(gave)) => gave,
            Ok(Err(failed)) => return Err(kernel::Error::unavailable(failed.message)),
            Err(_) => {
                cancel.cancel();
                return Err(kernel::Error::unavailable(format!(
                    "`{}` did not say what `{}` has in {} seconds",
                    extension.program,
                    found.title,
                    READING.as_secs()
                )));
            }
        };
        let given = match &gave["rows"] {
            Value::Array(rows) => rows.clone(),
            // A program that answered nothing has said nothing of what is there: that is not "none", and the
            // part says so, with how the package says its program is got and signed in.
            Value::Null => {
                let how = if extension.install.is_empty() {
                    String::new()
                } else {
                    format!(" {}", extension.install)
                };
                return Err(kernel::Error::unavailable(format!(
                    "`{}` answered nothing for {}, so there is nothing of it to show.{how}",
                    extension.program, found.title
                )));
            }
            _ => {
                return Err(kernel::Error::unavailable(format!(
                    "`{package}` gave out `rows` for `{part}` that are not a list"
                )));
            }
        };
        let more = given.len().saturating_sub(MOST_ROWS);
        let rows = given
            .iter()
            .take(MOST_ROWS)
            .filter_map(Value::as_object)
            .map(|row| {
                let mut shown = Map::new();
                for column in &found.columns {
                    shown.insert(column.field.clone(), cell(row.get(&column.field).unwrap_or(&Value::Null)));
                }
                if let Some(link) = &found.link
                    && let Some(at) = row.get(link).and_then(address)
                {
                    shown.insert(link.clone(), at);
                }
                shown
            })
            .collect();
        Ok(Read { part: found.clone(), rows, more })
    }
}
