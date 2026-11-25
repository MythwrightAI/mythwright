//! What is being built of this program itself and may be turned on before it
//! is whole (Settings, Experiments): none is listed today, which
//! anybody signed in is told; a name the program does not list is refused
//! to whoever manages the factory, saying there are none, and anybody else
//! is refused turning anything.

use reqwest::{Method, StatusCode};
use serde_json::json;

use crate::Rig;
use crate::held::said;
use crate::kept::person;

#[tokio::test]
async fn no_experiment_is_listed_today_and_none_is_turned_on_by_its_name() {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let grace = person(&rig, "Grace").await;
    let (status, listed) = rig.call(Method::GET, "/api/experiments", Some(&grace.token), None).await;
    assert_eq!((status, &listed["experiments"]), (StatusCode::OK, &json!([])), "{listed}");
    assert_eq!(rig.read("/api/meta").await["experiments"], json!([]));

    let (status, refused) =
        rig.call(Method::PUT, "/api/experiments/topics", Some(&rig.token), Some(json!({ "on": true }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    assert!(said(&refused).contains("there are none now"), "{refused}");
    let (status, refused) = rig
        .call(Method::PUT, "/api/experiments/topics", Some(&grace.token), Some(json!({ "on": true })))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!(rig.count_of("system.experiment").await, 0);
}
