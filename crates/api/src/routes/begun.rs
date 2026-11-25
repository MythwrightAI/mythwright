//! The run a press of Run on a project's Pipelines page began, where the
//! project's triggers are kept (`std/pipeline`): the page names its asking
//! (`asked` of `pipeline.asked`), the triggers carry the name to the run
//! that calls the pipeline (`pipeline.fired`), and the run that call
//! begins keeps which step asked for it (`runs.asked_by`), so that two
//! presses at one moment each open their own run and never the other's.

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use conductor::InstanceFilter;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

/// What keeps a project's triggers, whose runs call its pipelines.
const TRIGGERS: &str = "std/pipeline";

pub fn routes() -> Router<App> {
    Router::new().route("/pipelines/begun", get(begun))
}

#[derive(Deserialize)]
struct Asked {
    project: String,
    /// What the press named its asking.
    asked: String,
}

/// `{ "run": <id> }` once the run the asking began is there and may be
/// read by whoever asks; `{ "run": null }` until then.
async fn begun(
    State(app): State<App>,
    Auth(who): Auth,
    Query(asked): Query<Asked>,
) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    let filter = InstanceFilter {
        live_only: true,
        project: Some(asked.project.clone()),
        blueprint: Some(TRIGGERS.to_string()),
        ..Default::default()
    };
    for triggers in app.engine.instances(filter).await? {
        let Some(run) = app.engine.begun_for(&triggers.id, &asked.asked).await? else { continue };
        let Ok(run) = app.engine.run(&run).await else { continue };
        if app.tells_of(&who, &run) {
            return Ok(Json(json!({ "run": run.id })));
        }
    }
    Ok(Json(json!({ "run": null })))
}
