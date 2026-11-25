//! Comparisons: one blueprint started several ways, side by side.
//!
//! What a comparison is and how it is done is a crate's of its own
//! (`comparisons`). These routes start one, end one, say its shares
//! again or the rule they change by, and show one
//! with what was measured under each of its ways: counts and times as
//! they were measured, beside each other. Nothing here says which way is
//! better; that is whoever reads them.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use journal::MetricQuery;
use kernel::Error;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/comparisons", get(comparisons).post(compare))
        .route("/comparisons/{name}", get(comparison))
        .route("/comparisons/{name}/end", post(end))
        .route("/comparisons/{name}/shares", post(share))
        .route("/comparisons/{name}/rule", put(rule))
}

#[derive(Deserialize)]
struct Of {
    project: Option<String>,
}

async fn comparisons(
    State(app): State<App>,
    Auth(who): Auth,
    Query(of): Query<Of>,
) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    Ok(Json(json!({ "comparisons": app.comparisons.comparisons(of.project.as_deref()).await })))
}

/// What is counted of a board's cards where whoever starts a comparison
/// says `cards` in place of saying each: every move by where the card was
/// and where it came to (so that done, sent back and handed on are each a
/// count), what a review came to (undecided is left for a person), an
/// agent asking in place of working, work that could not be kept or
/// merged, and a card left for a person, whatever left it (`card.left`,
/// which its stage says once for each time it is left). Each is the count's name, the event,
/// the field of it that says which card, the fields that say what it
/// came to, and what of that says the card was accepted: its review
/// passed, or it came to the last column of its board (`LAST`, which is
/// read from the board as the comparison is started: [`last_column`]).
/// And what came after its work was merged, said at another card's merge
/// (`std/merge`): its work reverted, its files changed again soon (said
/// once of every card whose files are), a card that says it fixes it;
/// each once for the card that did it (`AFTER`).
/// The conductor knows none of these: it is told them here.
type Card = (&'static str, &'static str, &'static str, &'static [&'static str], &'static [&'static str]);
const LAST: &str = "the board's last column";
const AFTER: &str = "after";
const CARDS: &[Card] = &[
    ("moved", "board.item.moved", "id", &["from", "to"], &[LAST]),
    ("verdict", "review.passed", "card", &[], &["passed"]),
    ("verdict", "review.failed", "card", &[], &[]),
    ("verdict", "review.undecided", "card", &[], &[]),
    ("asked", "card.asked", "card", &[], &[]),
    ("left", "card.unkept", "card", &[], &[]),
    ("left", "card.unmerged", "card", &[], &[]),
    ("left", "card.left", "card", &[], &[]),
    (AFTER, "card.reverted", "card", &[], &[]),
    (AFTER, "card.changed_again", "cards", &[], &[]),
    (AFTER, "card.fixed", "card", &[], &[]),
];

/// Starting one says how a blueprint is started from now on, for every
/// instance whoever asks could have applied: it takes what applying does.
/// Nothing is counted of what comes of its things unless it is asked:
/// each count said, or `cards` for the usual ones of a board's cards.
async fn compare(
    State(app): State<App>,
    Auth(who): Auth,
    Json(mut asked): Json<Value>,
) -> ApiResult<Json<Value>> {
    who.require("blueprints:run")?;
    match asked.get("counts").and_then(Value::as_str) {
        Some("cards") => {
            let project = asked.get("project").and_then(Value::as_str).map(str::to_string);
            let last = last_column(&app, project.as_deref(), &who).await;
            let usual = |(name, topic, thing, by, accepts): &Card| {
                let accepts: Vec<&str> = accepts
                    .iter()
                    .filter_map(|word| if *word == LAST { last.as_deref() } else { Some(word) })
                    .collect();
                let once: &[&str] = if *name == AFTER { &["by"] } else { &[] };
                json!({ "name": name, "topic": topic, "thing": thing, "by": by, "accepts": accepts, "once": once })
            };
            asked["counts"] = CARDS.iter().map(usual).collect();
        }
        Some(other) => {
            return Err(Error::invalid(format!(
                "`counts` is each thing to count, or `cards` for the usual ones of a board's cards; `{other}` is neither"
            ))
            .into());
        }
        None => {}
    }
    let asked: comparisons::Compare = serde_json::from_value(asked)
        .map_err(|wrong| Error::invalid(format!("that is not how a comparison is asked for: {wrong}")))?;
    Ok(Json(json!(app.comparisons.compare(asked, &who).await?)))
}

/// Where a card comes to be done, for the usual counts of a board's cards:
/// the last column of the project's board as it is when the comparison is
/// started, which a board rearranged afterwards does not change (the
/// counting reads no board). A comparison of no project counts cards of
/// every board, and is told the last column of the board that ships; one
/// of a project with no board, or whose board the asker may not read, is
/// told none, and its cards are accepted by their review alone.
async fn last_column(app: &App, project: Option<&str>, who: &kernel::Principal) -> Option<String> {
    let Some(project) = project else {
        return studio::board::DEFAULT_COLUMNS.last().map(|last| last.to_string());
    };
    if !who.can("board:read") {
        return None;
    }
    app.studio.board.columns(project).await.ok()?.pop()
}

/// One comparison, every measurement taken under it since it began, and
/// the same laid beside each other, a column for each text of each way
/// ([`comparisons::beside`]).
async fn comparison(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    let one = app.comparisons.comparison(&name).await?;
    app.journal.flush_metrics().await?;
    let measured = app
        .journal
        .query_metrics(MetricQuery {
            name: "*".into(),
            // From the start of the minute it began in: measurements are kept by the minute.
            since_ms: one.started - one.started.rem_euclid(60_000),
            // Each text a way ran is counted apart: what one did is never added to what another did.
            group_by: comparisons::beside::BY.map(str::to_string).to_vec(),
            labels: vec![(comparisons::COMPARED.to_string(), one.name.clone())],
            ..Default::default()
        })
        .await?;
    let beside = comparisons::beside::beside(&one, &measured);
    Ok(Json(json!({
        "comparison": one, "measured": crate::routes::fleet::with_units(measured), "beside": beside,
    })))
}

async fn end(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    who.require("blueprints:run")?;
    let goes = body.get("goes").and_then(Value::as_str).map(str::to_string);
    Ok(Json(json!(app.comparisons.end_comparison(&name, goes).await?)))
}

/// Its shares said again, with why (`comparisons::Comparisons::share`).
async fn share(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    who.require("blueprints:run")?;
    let shares = serde_json::from_value(body.get("shares").cloned().unwrap_or_default())
        .map_err(|_| Error::invalid("`shares` is each way's share by its name, a whole number each"))?;
    let why = body.get("why").and_then(Value::as_str).map(str::to_string);
    Ok(Json(json!(app.comparisons.share(&name, shares, why, &who).await?)))
}

/// The rule its shares change by given, or taken away with none.
async fn rule(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    who.require("blueprints:run")?;
    let rule = match body.get("rule").filter(|rule| !rule.is_null()) {
        None => None,
        Some(rule) => Some(serde_json::from_value(rule.clone()).map_err(|wrong| {
            Error::invalid(format!(
                "that is not how a rule a comparison's shares change by is written: {wrong}"
            ))
        })?),
    };
    Ok(Json(json!(app.comparisons.rule(&name, rule, &who).await?)))
}
