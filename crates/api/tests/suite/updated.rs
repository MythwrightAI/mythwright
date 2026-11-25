//! An installed package that is not the one that came with the
//! installation: said to be, to whoever may read what agents run on, and
//! replaced by the one that came only when its owner asks.
//!
//! The package here names a program that is nowhere: nothing is run.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::held::{key, said};

/// A package for a made-up agent, called `name`, that says `wall` of its program.
fn manifest(name: &str, wall: &str) -> String {
    format!(
        "extension: {name}\ntitle: Sample\ndescription: Not a real agent.\nagent:\n  program: no-such-program-here\n  \
         input: ${{ turn.prompt }}\n  output:\n    - reply: ${{ event.text }}\n  wall:\n    {wall}\n"
    )
}

fn came_with(dir: &std::path::Path, name: &str, source: &str) {
    let folder = dir.join("offered").join(name);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("extension.yaml"), source).unwrap();
}

fn sample(listing: &Value) -> &Value {
    listing["extensions"].as_array().unwrap().iter().find(|entry| entry["name"] == "sample").unwrap()
}

#[tokio::test]
async fn a_package_that_is_not_the_one_that_came_is_said_to_be_and_replaced_only_when_its_owner_asks() {
    // What an earlier program shipped, installed as a copy.
    let earlier = manifest("sample", "keeps: [\".sample\"]");
    let rig = Rig::with("api-updated", |config, dir| {
        came_with(dir, "sample", &manifest("sample", "keeps: [\".sample\"]"));
        config.packages_dir = Some(dir.join("offered"));
    })
    .await;
    let mut rig = Rig::owned(rig).await;
    rig.send("/api/extensions", json!({ "name": "sample" })).await;
    let listed = rig.read("/api/extensions").await;
    assert_eq!(sample(&listed)["behind"], false);
    assert!(sample(&listed).get("differs").is_none());
    let installed = rig.dir.join("data/extensions/sample/extension.yaml");
    assert_eq!(std::fs::read_to_string(&installed).unwrap(), earlier);

    // A newer program comes with one that says more, and with another
    // nobody installed.
    let came = manifest("sample", "keeps: [\".sample\"]\n    signin: [\".sample/auth\"]");
    came_with(&rig.dir, "sample", &came);
    came_with(&rig.dir, "spare", &manifest("spare", "keeps: [\".spare\"]"));

    // Whoever may read what agents run on is told, with where the two differ.
    let reader = key(&rig, &["agents:read", "fleet:read"]).await;
    for token in [rig.token.clone(), reader.clone()] {
        let (status, listed) = rig.call(Method::GET, "/api/extensions", Some(&token), None).await;
        assert_eq!(status, StatusCode::OK, "{listed}");
        assert_eq!(sample(&listed)["behind"], true);
        assert_eq!(sample(&listed)["differs"]["besides"], json!(["agent.wall.signin"]));
        assert_eq!(sample(&listed)["differs"]["words"], "says `agent.wall.signin` besides");
        let (_, fleet) = rig.call(Method::GET, "/api/fleet", Some(&token), None).await;
        assert_eq!(fleet["device"]["packages_behind"], json!(["sample"]));
    }
    // And nobody else is.
    let other = key(&rig, &["fleet:read", "board"]).await;
    let (status, _) = rig.call(Method::GET, "/api/extensions", Some(&other), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (_, fleet) = rig.call(Method::GET, "/api/fleet", Some(&other), None).await;
    assert_eq!(fleet["device"]["packages_behind"], json!([]));

    // Nothing takes the one that came unasked: not a start, and not being asked which differ.
    rig.restart().await;
    assert_eq!(sample(&rig.read("/api/extensions").await)["behind"], true);
    assert_eq!(std::fs::read_to_string(&installed).unwrap(), earlier, "the copy is as it was, to the byte");
    assert!(!installed.with_file_name("extension.was.yaml").exists());
    assert_eq!(rig.count_of("extension.added").await, 1);

    // Taking it is the owner's, as installing a package is: a key that
    // holds less is refused, and nothing is replaced.
    let update = "/api/extensions/sample/update";
    for narrow in [&reader, &key(&rig, &["agents", "fleet", "extensions"]).await] {
        let (status, refused) = rig.call(Method::POST, update, Some(narrow), Some(json!({}))).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    }
    assert_eq!(std::fs::read_to_string(&installed).unwrap(), earlier);

    // A name nothing that came has, and one that came and is not installed.
    let (status, refused) = rig.post("/api/extensions/nonesuch/update", json!({})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(said(&refused).contains("no package called `nonesuch` came with this installation"), "{refused}");
    assert!(said(&refused).ends_with("there are: sample, spare"), "{refused}");
    let (status, refused) = rig.post("/api/extensions/spare/update", json!({})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(said(&refused).contains("is not installed, so there is nothing to update"), "{refused}");
    assert!(
        said(&refused).contains(&format!("`{} extension add spare` installs it", brand::CLI)),
        "{refused}"
    );
    assert_eq!(rig.read("/api/extensions").await["extensions"].as_array().unwrap().len(), 1);

    // Asked by its owner, it is the one that came: said as a package
    // installed is, with the copy it replaced kept beside it.
    let mark = rig.mark();
    let taken = rig.send(update, json!({})).await;
    assert_eq!(taken["updated"], true);
    assert_eq!(taken["name"], "sample");
    assert_eq!(taken["differs"]["words"], "says `agent.wall.signin` besides");
    let heard = rig.event("extension.added", |event| event.seq > mark).await;
    assert_eq!(heard.data["extension"], "sample");
    assert_eq!(std::fs::read_to_string(&installed).unwrap(), came);
    assert_eq!(std::fs::read_to_string(installed.with_file_name("extension.was.yaml")).unwrap(), earlier);
    let listed = rig.read("/api/extensions").await;
    assert_eq!(sample(&listed)["behind"], false);
    assert_eq!(rig.read("/api/fleet").await["device"]["packages_behind"], json!([]));

    // The same asked twice replaces nothing and says nothing.
    let again = rig.send(update, json!({})).await;
    assert_eq!(again["updated"], false);
    assert!(again.get("differs").is_none());
    assert_eq!(rig.count_of("extension.added").await, 2);
    assert_eq!(std::fs::read_to_string(installed.with_file_name("extension.was.yaml")).unwrap(), earlier);
}
