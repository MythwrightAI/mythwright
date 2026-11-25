//! A conductor that starts compiles each text once: for everything alive
//! that runs it, in whichever project the library answers it the same.

use std::sync::Arc;

use conductor::{Apply, Engine, InstanceStatus, Library, MemoryLibrary, Registry};
use journal::MetricQuery;
use serde_json::json;

use crate::Rig;

/// Stays alive, as what a fleet is made of does, and hears what other
/// blueprints say.
const LISTENS: &str = "
blueprint: listens
steps:
  up: { do: test.echo }
on:
  heard:
    event: probe.said
    do: test.echo
";

/// The most the start the rig was last started with measured `name` at.
async fn measured(rig: &Rig, name: &str) -> f64 {
    rig.journal.flush_metrics().await.unwrap();
    let query = MetricQuery { name: name.into(), since_ms: 0, ..Default::default() };
    rig.journal.query_metrics(query).await.unwrap()[0].total.max
}

fn under(rig: &Rig, key: &str, project: Option<&str>) -> Apply {
    let project = project.map(str::to_string);
    Apply { key: Some(key.into()), project, ..rig.request(LISTENS, json!({})) }
}

/// Applies `LISTENS` under each of `keys`, for each project, and waits
/// for all of them to be there.
async fn alive(rig: &Rig, engine: &Engine, keys: &[(&str, Option<&str>)]) {
    for (key, project) in keys {
        let applied = engine.apply(under(rig, key, *project)).await.unwrap();
        crate::handlers::active(rig, &applied.instance).await;
    }
}

#[tokio::test]
async fn a_start_compiles_a_text_once_for_everything_alive_that_runs_it_in_every_project() {
    let mut rig = Rig::new().await;
    let keys =
        [("a", Some("shop")), ("b", Some("shop")), ("c", Some("shop")), ("d", Some("blog")), ("e", None)];
    alive(&rig, &rig.engine.clone(), &keys).await;
    rig.restart().await;
    assert_eq!(measured(&rig, "system.resumed_count").await, 5.0);
    assert_eq!(measured(&rig, "system.resumed_texts_count").await, 1.0, "one text, compiled once");
}

/// One library for every project, and one text more for `blog`.
struct Blog {
    every: MemoryLibrary,
    blog: MemoryLibrary,
}

impl Library for Blog {
    fn load(&self, name: &str, project: Option<&str>) -> Option<String> {
        let own = (project == Some("blog")).then(|| self.blog.load(name, project)).flatten();
        own.or_else(|| self.every.load(name, project))
    }

    fn names(&self, project: Option<&str>) -> Vec<String> {
        let mut names = self.every.names(project);
        if project == Some("blog") {
            names.extend(self.blog.names(project));
        }
        names
    }
}

#[tokio::test]
async fn a_project_whose_library_says_something_the_others_do_not_has_the_text_compiled_for_it() {
    let rig = Rig::new().await;
    let (every, blog) = (MemoryLibrary::new(), MemoryLibrary::new());
    // Only `blog` has a blueprint that says what the text hears.
    let says =
        "blueprint: says\nsteps:\n  say: { do: emit, with: { topic: probe.said, data: { what: hello } } }\n";
    blog.put("says", says);
    let library: Arc<dyn Library> = Arc::new(Blog { every, blog });
    let logs = journal::Logs::new(rig.dir.join("library"));
    let made = async || {
        let registry = crate::registry(&rig.shared);
        Engine::new(rig.journal.clone(), logs.clone(), registry, library.clone()).await.unwrap()
    };
    let first = made().await;
    alive(&rig, &first, &[("a", Some("shop")), ("b", Some("blog")), ("c", Some("blog"))]).await;
    first.halt();
    first.halted().await;

    let second = made().await;
    assert_eq!(second.resume().await.unwrap(), 3);
    assert_eq!(
        measured(&rig, "system.resumed_texts_count").await,
        2.0,
        "once for `shop` and once for `blog`, which the library answers otherwise"
    );
    second.halt();
}

#[tokio::test]
async fn every_instance_whose_text_no_longer_checks_fails_by_itself_from_one_compile() {
    let rig = Rig::new().await;
    alive(&rig, &rig.engine.clone(), &[("a", None), ("b", None)]).await;
    rig.engine.halt();
    rig.engine.halted().await;

    // The next conductor has none of the actions the text uses.
    let bare = Engine::new(rig.journal.clone(), rig.logs.clone(), Registry::new(), rig.library.clone())
        .await
        .unwrap();
    assert_eq!(bare.resume().await.unwrap(), 0);
    let all = bare.instances(conductor::InstanceFilter { limit: 10, ..Default::default() }).await.unwrap();
    assert_eq!(all.len(), 2);
    for one in all {
        assert_eq!(one.status, InstanceStatus::Failed, "{}", one.key);
        let why = one.error.unwrap();
        assert!(why.starts_with("cannot resume: `listens` has"), "{why}");
    }
    assert_eq!(measured(&rig, "system.resumed_texts_count").await, 1.0);
}
