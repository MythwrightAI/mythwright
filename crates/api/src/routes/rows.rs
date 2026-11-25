//! A table's rows a window at a time (`studio::window`), and what is
//! counted of a view of it: how a page reads a table of a million rows.
//!
//! `GET /collections/{project}/{table}/rows` answers one window: the rows
//! of a view (`view`) as the reader sorts it (`sort`, `down`), of one band
//! (`band`), from where they got to (`after` or `before`, a place handed
//! out and handed back unread; `at`, a rank; or `around` a row, by its id
//! or its number, which says whether the view shows it and where it
//! stands, `around`), searched for words (`find`), so many (`limit`, a
//! hundred where nobody says, two hundred at most), with the fields asked
//! (`fields`, by name and commas). With `ids` it answers instead each of
//! those rows as the view shows it, whether it shows it and where it
//! stands. `at` beside the answer is where the factory's events had got
//! to as it was read, so that what is said meanwhile is not missed.
//!
//! `GET …/rows/counted` answers how many rows the view shows and the
//! table has, the bands of the column it is gathered by and the figures
//! asked (`figures`, `points:sum,due:filled`), or the view's own; `whole`
//! is false where that is so far, and each count at least so many, with
//! how many of the table's rows were `looked` through.

use std::collections::BTreeMap;

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use kernel::Error;
use kernel::window::{Sort, USUAL};
use serde::Deserialize;
use serde_json::Value;
use studio::window::{Asked, Reading};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/collections/{project}/{name}/rows", get(rows))
        .route("/collections/{project}/{name}/rows/counted", get(counted))
}

/// What a window is asked, as an address says it.
#[derive(Deserialize, Default)]
struct Window {
    view: Option<String>,
    sort: Option<String>,
    down: Option<String>,
    band: Option<String>,
    after: Option<String>,
    before: Option<String>,
    at: Option<u64>,
    around: Option<String>,
    find: Option<String>,
    limit: Option<usize>,
    fields: Option<String>,
    ids: Option<String>,
    figures: Option<String>,
}

/// A list said by commas, each part trimmed and none empty.
fn listed(said: Option<&str>) -> Option<Vec<String>> {
    said.map(|said| {
        said.split(',').map(str::trim).filter(|one| !one.is_empty()).map(str::to_string).collect()
    })
}

/// What a part of an address says, where it says anything.
fn said(value: &Option<String>) -> Option<&str> {
    value.as_deref().map(str::trim).filter(|value| !value.is_empty())
}

fn asked(query: &Window) -> Result<Asked, Error> {
    let reading = match (said(&query.after), said(&query.before), query.at, said(&query.around)) {
        (None, None, None, None) => Reading::Start,
        (Some(after), None, None, None) => Reading::After(after.to_string()),
        (None, Some(before), None, None) => Reading::Before(before.to_string()),
        (None, None, Some(at), None) => Reading::At(at),
        (None, None, None, Some(around)) => Reading::Around(around.to_string()),
        _ => {
            return Err(Error::invalid(
                "a window is read from one place: say one of `after`, `before`, `at` and `around`",
            ));
        }
    };
    let down = matches!(said(&query.down), Some("1" | "true" | "yes"));
    Ok(Asked {
        view: said(&query.view).map(str::to_string),
        sort: said(&query.sort).map(|field| Sort { field: field.to_string(), down }),
        // A band of rows that name nothing is said as nothing: kept as it is.
        band: query.band.clone(),
        find: said(&query.find).map(str::to_string),
        reading,
        limit: query.limit.unwrap_or(USUAL),
        fields: listed(said(&query.fields)),
    })
}

async fn rows(
    State(app): State<App>,
    Auth(who): Auth,
    Path((project, name)): Path<(String, String)>,
    Query(query): Query<Window>,
) -> ApiResult<Json<Value>> {
    let asked = asked(&query)?;
    // Where the events had got to before the rows were read: what is said
    // after it may not be in them.
    let at = crate::cursor::at(&app.journal.now());
    let records = &app.studio.records;
    if let Some(ids) = listed(query.ids.as_deref()) {
        let rows = records.window_ids(&project, &name, &asked, &ids, &who).await?;
        return Ok(Json(serde_json::json!({ "rows": rows, "at": at })));
    }
    let mut answer = records.window(&project, &name, &asked, &who).await?.json();
    answer["at"] = Value::String(at);
    Ok(Json(answer))
}

async fn counted(
    State(app): State<App>,
    Auth(who): Auth,
    Path((project, name)): Path<(String, String)>,
    Query(query): Query<Window>,
) -> ApiResult<Json<Value>> {
    let asked = asked(&query)?;
    let figures = match listed(query.figures.as_deref()) {
        None => None,
        Some(said) => {
            let mut figures = BTreeMap::new();
            for one in said {
                let Some((field, which)) = one.split_once(':') else {
                    return Err(Error::invalid(format!(
                        "`{one}` is no figure: say each as a column and what is counted of it, as in `points:sum`"
                    ))
                    .into());
                };
                figures.insert(field.trim().to_string(), which.trim().to_string());
            }
            Some(figures)
        }
    };
    Ok(Json(app.studio.records.counted_rows(&project, &name, &asked, figures, &who).await?))
}
