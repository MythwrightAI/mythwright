//! What is asked for of several projects together: `project` said once
//! for each (`project=a&project=b`), and for what was measured the label
//! said once for each (`labels=project:a,project:b`). It answers what
//! each project alone answers, put together as a list of everything is;
//! a project that is not there adds nothing; and each thing is handed to
//! whoever asks as it is handed asked any other way.
//!
//! Which projects are asked for together is whoever asks's to say: the
//! client asks for those of the workspace somebody is looking at, which
//! the daemon is never told, and no route takes a workspace.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::apart::{COUNTER, ids, thing};
use crate::{Rig, soon};

fn runs(listed: &Value) -> Vec<String> {
    let all = listed["runs"].as_array().unwrap_or_else(|| panic!("runs are listed: {listed}"));
    all.iter().map(|one| one["id"].as_str().unwrap().to_string()).collect()
}

fn sorted(mut all: Vec<String>) -> Vec<String> {
    all.sort();
    all
}

/// How many runs were measured, and how many of them failed, by what is asked.
async fn measured(rig: &Rig, labels: &str) -> (u64, u64) {
    let answer = rig.read(&format!("/api/metrics?name=run{labels}")).await;
    match answer["series"].as_array().unwrap().as_slice() {
        [] => (0, 0),
        [one] => (one["total"]["count"].as_u64().unwrap(), one["total"]["errors"].as_u64().unwrap()),
        _ => panic!("one series was asked for: {answer}"),
    }
}

#[tokio::test]
async fn what_is_asked_for_several_projects_is_what_each_alone_answers_put_together() {
    let rig = Rig::new().await;
    for name in ["Shop", "Yard", "Blog"] {
        rig.send("/api/projects", json!({ "name": name })).await;
    }
    let mut ended = rig.follow("run.finished");
    // Each applied under a label of its own, which what is measured of it carries.
    let counter = |project: &'static str, trial: &'static str| {
        let rig = &rig;
        async move {
            let asked =
                json!({ "source": COUNTER, "project": project, "wait": true, "labels": { "trial": trial } });
            let applied = rig.send("/api/apply", asked).await;
            assert_eq!(applied["finished"]["status"], "ok", "{applied}");
            applied["instance"].as_str().unwrap().to_string()
        }
    };
    let (shop, yard, blog) =
        (counter("shop", "a").await, counter("yard", "b").await, counter("blog", "c").await);
    // And something that is nobody's, which no project asked for takes in.
    let asked = json!({ "source": COUNTER.replace("counter", "nobodys"), "wait": true });
    let nobodys = rig.send("/api/apply", asked).await["instance"].as_str().unwrap().to_string();
    // Each hears things, so that each has runs beside the run of its steps:
    // the shop three, the yard two, the blog one.
    for (project, n) in [("shop", 1), ("yard", 2), ("shop", 3), ("blog", 4), ("yard", 5), ("shop", 6)] {
        thing(&rig, project, n).await;
    }
    // Every run is over, and so is measured, before anything is read.
    let mut handled = 0;
    while handled < 6 {
        let event = soon("the six runs that heard a thing to end", ended.next()).await.unwrap();
        handled += usize::from(event.data["flow"] == "on.thing");
    }

    // Instances: each project's alone, and several together.
    let of = |query: &'static str| {
        let rig = &rig;
        async move { ids(&rig.read(&format!("/api/instances?{query}")).await) }
    };
    assert_eq!(of("project=shop").await, std::slice::from_ref(&shop));
    assert_eq!(of("project=yard").await, std::slice::from_ref(&yard));
    let both = of("project=shop&project=yard").await;
    assert_eq!(sorted(both.clone()), sorted(vec![shop.clone(), yard.clone()]));
    // The newest first, as a list of everything is: the yard's was applied after the shop's.
    let all = of("limit=1000").await;
    let among: Vec<String> = all.iter().filter(|id| [&shop, &yard].contains(id)).cloned().collect();
    assert_eq!(both, among, "in the order a list of everything has them");
    assert!(all.contains(&nobodys) && all.contains(&blog));
    assert_eq!(
        sorted(of("project=yard&project=blog&project=shop").await),
        sorted(vec![shop.clone(), yard.clone(), blog.clone()])
    );
    // As many as were asked for, of them all.
    assert_eq!(of("project=shop&project=yard&limit=1").await.len(), 1);
    assert_eq!(of("project=shop&project=yard&live=true").await.len(), 2);
    // A project that is not there adds nothing, and one said twice is said once.
    assert_eq!(of("project=shop&project=nonesuch").await, std::slice::from_ref(&shop));
    assert_eq!(of("project=nonesuch&project=nowhere").await, Vec::<String>::new());
    assert_eq!(of("project=shop&project=shop").await, std::slice::from_ref(&shop));

    // Runs: the most recent of each alone, and of several together.
    let ran = |query: &'static str| {
        let rig = &rig;
        async move { rig.read(&format!("/api/runs?{query}")).await }
    };
    let (of_shop, of_yard) = (ran("project=shop&limit=100").await, ran("project=yard&limit=100").await);
    assert_eq!((runs(&of_shop).len(), runs(&of_yard).len()), (4, 3), "{of_shop} {of_yard}");
    let together = ran("project=shop&project=yard&limit=100").await;
    assert_eq!(sorted(runs(&together)), sorted([runs(&of_shop), runs(&of_yard)].concat()));
    // Each run is handed as it is handed alone.
    for one in [&of_shop, &of_yard].iter().flat_map(|listed| listed["runs"].as_array().unwrap()) {
        let same = together["runs"].as_array().unwrap().iter().find(|run| run["id"] == one["id"]);
        assert_eq!(same, Some(one));
    }
    // The latest begun first, and the most recent of them all where fewer are asked for.
    let began = |listed: &Value| -> Vec<i64> {
        listed["runs"].as_array().unwrap().iter().map(|run| run["started"].as_i64().unwrap()).collect()
    };
    assert!(began(&together).windows(2).all(|pair| pair[0] >= pair[1]), "{together}");
    let few = ran("project=shop&project=yard&limit=3").await;
    assert_eq!(runs(&few).len(), 3);
    assert_eq!(began(&few), began(&together)[..3]);
    assert_eq!(runs(&ran("project=shop&project=nonesuch&limit=100").await), runs(&of_shop));
    assert_eq!(runs(&ran("project=nonesuch&project=nowhere").await), Vec::<String>::new());
    assert_eq!(runs(&ran("project=shop&project=shop&limit=100").await), runs(&of_shop));
    // One instance's runs are that instance's, whatever project is said beside it.
    let its = ran("limit=100").await;
    assert_eq!(runs(&its).len(), 4 + 3 + 2 + 1, "{its}");

    // What was measured: a label said once for each project is any of them, added up as one.
    let (shops, yards, blogs) = (
        measured(&rig, "&labels=project:shop").await,
        measured(&rig, "&labels=project:yard").await,
        measured(&rig, "&labels=project:blog").await,
    );
    assert_eq!((shops, yards, blogs), ((4, 0), (3, 0), (2, 0)));
    assert_eq!(measured(&rig, "&labels=project:shop,project:yard").await, (7, 0));
    assert_eq!(measured(&rig, "&labels=project:yard,project:blog,project:shop").await, (9, 0));
    assert_eq!(measured(&rig, "&labels=project:shop,project:nonesuch").await, shops);
    assert_eq!(measured(&rig, "&labels=project:nonesuch,project:nowhere").await, (0, 0));
    // Another label beside them is asked of all that is found.
    assert_eq!(measured(&rig, "&labels=project:shop,project:yard,flow:on.thing").await, (5, 0));
    // Everything has what is nobody's too, which no project named takes in.
    assert_eq!(measured(&rig, "").await, (10, 0));
    let split = rig.read("/api/metrics?name=run&by=project&labels=project:shop,project:yard").await;
    let mut by: Vec<(String, u64)> = split["series"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| {
            (one["labels"]["project"].as_str().unwrap().to_string(), one["total"]["count"].as_u64().unwrap())
        })
        .collect();
    by.sort();
    assert_eq!(by, [("shop".to_string(), 4), ("yard".to_string(), 3)]);
    // And the labels there are to compare by are found among the same.
    let labels = |of: &'static str| {
        let rig = &rig;
        async move { rig.read(&format!("/api/metrics/labels?name=run&labels={of}")).await["labels"].clone() }
    };
    assert_eq!(labels("project:shop").await, json!({ "trial": ["a"] }));
    assert_eq!(labels("project:shop,project:yard").await, json!({ "trial": ["a", "b"] }));
    assert_eq!(labels("project:blog,project:yard,project:nonesuch").await, json!({ "trial": ["b", "c"] }));
    assert_eq!(labels("project:nonesuch,project:nowhere").await, json!({}));
}

/// A table kept private and a handler that hears a record of it changed,
/// in a project called what `PROJECT` is put for.
const WATCH: &str = r#"
blueprint: watch
permissions: [projects, records]
steps:
  moods:
    do: collection.ensure
    with:
      name: moods
      reads: private:journal
      fields: { title: { type: text, required: true }, went: { type: choice, of: [well, badly] } }
on:
  changed:
    event: record.updated
    for: { project: PROJECT }
    steps:
      read: { do: record.get, with: { id: "${event.data.id}" } }
      said: { do: log, with: { message: "it went ${event.data.chose.went} for ${steps.read.out.data.title}" } }
"#;

#[tokio::test]
async fn a_run_listed_for_several_projects_is_handed_to_whoever_asks_as_it_is_listed_for_one() {
    let rig = Rig::new().await;
    let mut ended = rig.follow("run.finished");
    for (name, project) in [("Life", "life"), ("Work", "work")] {
        rig.send("/api/projects", json!({ "name": name })).await;
        let asked = json!({ "source": WATCH.replace("PROJECT", project), "project": project, "wait": true });
        let applied = rig.send("/api/apply", asked).await;
        assert_eq!(applied["finished"]["status"], "ok", "{applied}");
        let fields = json!({ "fields": { "title": "Saturday", "went": "well" } });
        rig.send(&format!("/api/collections/{project}/moods/records"), fields).await;
        let change = json!({ "fields": { "title": format!("quince of {project}"), "went": "badly" } });
        let (status, changed) = rig
            .call(Method::PATCH, &format!("/api/records/{project}-moods-1"), Some(&rig.token), Some(change))
            .await;
        assert_eq!(status, StatusCode::OK, "{changed}");
    }
    let mut handled = 0;
    while handled < 2 {
        let event = soon("each handler's run to end", ended.next()).await.unwrap();
        handled += usize::from(event.data["flow"] == "on.changed");
    }
    let set_off = |listed: &Value| -> Vec<Value> {
        let all = listed["runs"].as_array().unwrap_or_else(|| panic!("runs are listed: {listed}"));
        all.iter().filter(|run| run["flow"] == "on.changed").cloned().collect()
    };

    // The owner is told what set each off, asked for one project or for both.
    let whole = rig.read("/api/runs?project=life&project=work&limit=100").await;
    assert_eq!(set_off(&whole).len(), 2, "{whole}");
    for run in set_off(&whole) {
        assert!(run.get("kept_from").is_none(), "{run}");
        assert!(run["event"]["data"]["id"].as_str().is_some_and(|id| id.ends_with("-moods-1")), "{run}");
    }

    // Somebody who may read runs and was not given the journal is told
    // that each ran and nothing of what set it off: by one project, and
    // by both, where each run is handed exactly as it is by its own.
    let member = crate::told::key(&rig, kernel::principal::MEMBER_SCOPES).await;
    let ask = |query: &'static str| {
        let (rig, member) = (&rig, member.clone());
        async move {
            let (status, listed) =
                rig.call(Method::GET, &format!("/api/runs?{query}"), Some(&member), None).await;
            assert_eq!(status, StatusCode::OK, "{listed}");
            listed
        }
    };
    let (of_life, of_work) = (ask("project=life&limit=100").await, ask("project=work&limit=100").await);
    let both = ask("project=life&project=work&limit=100").await;
    assert_eq!(set_off(&both).len(), 2, "{both}");
    for run in set_off(&both) {
        assert_eq!(run["kept_from"], true, "{run}");
        assert!(run["event"].get("data").is_none(), "{run}");
        let alone =
            [&of_life, &of_work].iter().flat_map(|listed| set_off(listed)).find(|one| one["id"] == run["id"]);
        assert_eq!(alone.as_ref(), Some(&run));
    }
    for marked in ["quince", "moods-1"] {
        assert!(!both.to_string().contains(marked), "`{marked}` is read in the runs of both: {both}");
    }

    // And whoever may not read runs at all is refused for several as for one.
    let reader = crate::told::key(&rig, &["channels:read"]).await;
    for path in [
        "/api/runs?project=life&project=work",
        "/api/instances?project=life&project=work",
        "/api/metrics?name=run&labels=project:life,project:work",
    ] {
        let (status, answer) = rig.call(Method::GET, path, Some(&reader), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}: {answer}");
    }
}
