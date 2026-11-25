//! A project of a factory from before is moved to a journal of its own
//! when its owner asks (`api::moved`): while the factory is stopped, at
//! rest, with everything that was running for it taken up afterwards by
//! a conductor of the project's own; left where it is, and said why,
//! while something of it is under way; and cut off anywhere it is as it
//! was or it is moved, with what the cut left taken away by the daemon
//! as it starts.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use api::moved::Stop;
use kernel::ManualClock;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::apart::{COUNTER, counter, heard, ids, rig, rows, thing};

/// Waits, in its steps, for something nobody says: a run under way for
/// as long as it is there.
const WAITS: &str = r#"
blueprint: waits
steps:
  for_it: { do: wait, with: { event: s.never } }
"#;

/// Makes every project there is one of a factory from before: no row
/// says it has a journal, and no file is kept for one.
pub(crate) fn from_before(data: &Path) {
    let connection = journal::copy::Connection::open(data.join("journal.db")).unwrap();
    connection.execute("DELETE FROM journals", []).unwrap();
    let _ = std::fs::remove_dir_all(data.join("journals"));
}

/// Has the journal of `project` keep only `keeps` of it
/// (`journal::keeps`), as a program before this one listed or made it:
/// its row says so, and its file holds nothing of the tables such a
/// journal does not keep, which a file made now was given a few rows of
/// as the project was added.
pub(crate) fn kept_less(data: &Path, project: &str, keeps: i64) {
    let connection = journal::copy::Connection::open(data.join("journal.db")).unwrap();
    let said = connection.execute("UPDATE journals SET keeps = ?1 WHERE project = ?2", (keeps, project));
    assert_eq!(said.unwrap(), 1, "`{project}` has a journal of its own");
    let own = journal::copy::Connection::open(data.join("journals").join(format!("{project}.db"))).unwrap();
    let above = studio::apart::KEPT_APART.iter().chain(agents::apart::KEPT_APART);
    for (table, _) in above.filter(|(_, level)| keeps < *level) {
        own.execute(&format!("DELETE FROM {table}"), []).unwrap();
    }
}

/// The rows of `journals`, as (project, file, keeps, whether it was
/// moved).
pub(crate) fn listed(data: &Path) -> Vec<(String, String, i64, bool)> {
    let connection = journal::copy::Connection::open(data.join("journal.db")).unwrap();
    let listed = journal::listed(&connection).unwrap();
    listed.into_iter().map(|one| (one.project, one.file, one.keeps, one.moved.is_some())).collect()
}

/// Says a thing of `project` and waits until its counter has heard it
/// and is at rest again: the run of its handler over, which is when a
/// project may be moved.
async fn told(rig: &Rig, project: &str, n: u64) {
    let mark = rig.mark();
    thing(rig, project, n).await;
    rig.event("run.finished", |event| event.seq > mark && event.data["flow"] == "on.thing").await;
}

/// How often the counter of `project` said it heard something.
async fn heard_times(rig: &Rig, project: &str) -> usize {
    let query = journal::EventQuery { topics: "s.heard".into(), limit: 10_000, ..Default::default() };
    let instances = ids(&rig.read(&format!("/api/instances?project={project}")).await);
    let all = rig.said(query).await;
    all.iter().filter(|event| event.source.instance.as_ref().is_some_and(|id| instances.contains(id))).count()
}

#[tokio::test]
async fn a_project_from_before_is_moved_to_a_journal_of_its_own_and_goes_on_from_where_it_was() {
    if crate::one_journal() {
        return;
    }
    let (mut rig, data) = rig("moved").await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.stop().await;
    from_before(&data);
    rig.start().await;
    // Something of the project from before, alive and at rest, which
    // has heard one thing; a project added by this program beside it;
    // and something that is nobody's.
    let (shop, run) = counter(&rig, "shop").await;
    told(&rig, "shop", 1).await;
    rig.send("/api/projects", json!({ "name": "Yard" })).await;
    let (yard, _) = counter(&rig, "yard").await;
    let asked = json!({ "source": COUNTER.replace("counter", "nobodys"), "wait": true });
    let nobodys = rig.send("/api/apply", asked).await["instance"].as_str().unwrap().to_string();
    let (factory, own) = (data.join("journal.db"), data.join("journals").join("shop.db"));
    assert_eq!(rows(&factory, "instances"), 2, "the project's from before, and nobody's");

    // It is asked of a factory that is stopped, and of no other.
    let running = api::moved::look(&data, None).expect_err("a factory that is running");
    assert!(running.message.contains("is running"), "{}", running.message);
    rig.stop().await;

    // Asked first, it says what there is and changes nothing.
    let asked = api::moved::look(&data, None).unwrap();
    let said: Vec<_> = asked
        .projects
        .iter()
        .map(|project| (project.id.as_str(), project.moves(), project.apart()))
        .collect();
    assert_eq!(said, [("shop", true, false), ("yard", false, true)]);
    assert!(asked.projects[0].held() >= 4, "an instance, its version, its runs and their steps");
    assert!(!own.exists());
    assert!(
        api::moved::look(&data, Some("nothing")).unwrap_err().message.contains("has no project `nothing`")
    );

    let mut shown = Vec::new();
    let moved = api::moved::run(&data, None, None, |asked| {
        shown = asked.projects.iter().map(|project| project.id.clone()).collect();
    })
    .await
    .unwrap();
    assert_eq!(shown, ["shop", "yard"]);
    assert_eq!(moved.len(), 1);
    assert_eq!((moved[0].project.as_str(), moved[0].file.as_str()), ("shop", "shop.db"));
    // Every row of it is in its own file, and the factory's keeps what
    // is nobody's.
    assert_eq!((rows(&factory, "instances"), rows(&own, "instances")), (1, 1));
    assert!(rows(&own, "runs") >= 2 && rows(&own, "steps") >= 2 && rows(&own, "places") >= 1);
    assert_eq!(
        listed(&data),
        [
            ("shop".to_string(), "shop.db".to_string(), journal::keeps::MADE_AT, true),
            ("yard".to_string(), "yard.db".to_string(), journal::keeps::MADE_AT, false)
        ]
    );
    // Asked again there is nothing to do.
    assert!(api::moved::look(&data, None).unwrap().projects.iter().all(|project| project.apart()));
    assert!(api::moved::run(&data, None, None, |_| {}).await.unwrap().is_empty());

    // The daemon that next starts says it was done, once, and takes up
    // what was running for the project in the project's own conductor:
    // the same instance, which goes on hearing from where it had got to.
    rig.start().await;
    let said = rig.event("system.moved_apart", |_| true).await;
    assert_eq!(
        (said.data["moved"].as_str(), said.data["journal"].as_str()),
        (Some("shop"), Some("project:shop"))
    );
    assert_eq!(said.data["rows"]["instances"], json!(1), "{}", said.data);
    assert_eq!(said.data["at"], json!(moved[0].at));
    assert_eq!(rig.count_of("system.move_cleared").await, 0);
    let one = rig.read(&format!("/api/instances/{shop}")).await;
    assert_eq!(
        (&one["instance"]["status"], &one["instance"]["state"]["heard"]),
        (&json!("active"), &json!(1))
    );
    thing(&rig, "shop", 2).await;
    thing(&rig, "shop", 3).await;
    assert_eq!(heard(&rig, &shop).await, json!(3));
    assert_eq!(heard_times(&rig, "shop").await, 3, "each thing heard once, before the move and after");
    // What ran before it was moved is read as ever, and what is applied
    // for it now is the one there is.
    assert_eq!(rig.read(&format!("/api/runs/{run}")).await["instance"], json!(shop));
    let again = rig.send("/api/apply", json!({ "source": COUNTER, "project": "shop", "wait": true })).await;
    assert_eq!((again["instance"].as_str(), again["created"].as_bool()), (Some(shop.as_str()), Some(false)));
    assert_eq!(ids(&rig.read("/api/instances?project=shop").await), std::slice::from_ref(&shop));
    // What it does from now on is written in its own file.
    let runs = rows(&own, "runs");
    told(&rig, "shop", 4).await;
    assert!(rows(&own, "runs") > runs);
    // The others are as they were.
    told(&rig, "yard", 1).await;
    assert_eq!(heard(&rig, &yard).await, json!(1));
    assert_eq!(rig.read(&format!("/api/instances/{nobodys}")).await["instance"]["status"], "active");

    // Said once: a daemon started again says nothing of it.
    rig.restart().await;
    assert_eq!(rig.count_of("system.moved_apart").await, 1);
    thing(&rig, "shop", 5).await;
    assert_eq!(heard(&rig, &shop).await, json!(5));
}

#[tokio::test]
async fn a_project_with_something_under_way_is_left_where_it_is_and_whoever_asked_is_told_what() {
    if crate::one_journal() {
        return;
    }
    let (mut rig, data) = rig("moved-under-way").await;
    for name in ["Shop", "Yard"] {
        rig.send("/api/projects", json!({ "name": name })).await;
    }
    rig.stop().await;
    from_before(&data);
    rig.start().await;
    // One project at rest, and one with a run of its steps waiting.
    let (yard, _) = counter(&rig, "yard").await;
    let waits = rig.send("/api/apply", json!({ "source": WAITS, "project": "shop" })).await;
    let waits = waits["instance"].as_str().unwrap().to_string();
    rig.event("step.started", |event| event.source.instance.as_deref() == Some(waits.as_str())).await;
    rig.stop().await;

    let asked = api::moved::look(&data, Some("shop")).unwrap();
    assert!(!asked.projects[0].moves());
    let why = asked.projects[0].in_the_way.join("; ");
    assert!(why.contains("`waits`") && why.contains(&waits) && why.contains("under way"), "{why}");
    // Asked for by name it is refused, saying what is in the way and
    // what to do; asked for with the rest, the rest are moved.
    let refused = api::moved::run(&data, Some("shop"), None, |_| {}).await.expect_err("something under way");
    assert!(refused.message.contains("is moved only at rest"), "{}", refused.message);
    assert!(
        refused.message.contains(&waits) && refused.message.contains("Nothing was moved"),
        "{}",
        refused.message
    );
    assert!(listed(&data).is_empty());
    // A turn of one of a project's agents that is waiting, or running
    // (its program may be going on without the daemon), is in the way
    // as a run is.
    let turn = |sql: &str| {
        let connection = journal::copy::Connection::open(data.join("journal.db")).unwrap();
        connection.execute_batch(sql).unwrap();
    };
    turn(
        "INSERT INTO slots (name, adapter, project, instructions, scopes, permission_mode, active, created, updated)
             VALUES ('yard-worker-1', 'builtin', 'yard', '', '[]', 'edit', 1, 1, 1);
         INSERT INTO turns (id, slot, kind, thread, prompt, options, status, created)
             VALUES ('turn_1', 'yard-worker-1', 'task', 'lane:yard-1', 'do it', '{}', 'running', 2);",
    );
    let asked = api::moved::look(&data, Some("yard")).unwrap();
    assert_eq!(asked.projects[0].in_the_way, ["a turn of the agent `yard-worker-1` is running (turn_1)"]);
    assert!(api::moved::run(&data, None, None, |_| {}).await.unwrap().is_empty());
    turn("DELETE FROM turns WHERE id = 'turn_1'; DELETE FROM slots WHERE name = 'yard-worker-1';");
    let moved = api::moved::run(&data, None, None, |_| {}).await.unwrap();
    assert_eq!(moved.iter().map(|moved| moved.project.as_str()).collect::<Vec<_>>(), ["yard"]);
    assert_eq!(listed(&data), [("yard".to_string(), "yard.db".to_string(), journal::keeps::MADE_AT, true)]);

    // What was under way goes on where it was, with the factory's own
    // conductor; stopped, and the factory stopped, it is moved.
    rig.start().await;
    let one = rig.read(&format!("/api/instances/{waits}")).await;
    assert_ne!(one["instance"]["status"], "stopped", "{one}");
    told(&rig, "yard", 1).await;
    assert_eq!(heard(&rig, &yard).await, json!(1));
    // Stopped, and the run it had under way over, which is said a
    // moment after the stopping is answered.
    let mark = rig.mark();
    rig.send(&format!("/api/instances/{waits}/stop"), json!({})).await;
    rig.event("run.finished", |event| {
        event.seq > mark && event.source.instance.as_deref() == Some(waits.as_str())
    })
    .await;
    rig.stop().await;
    let moved = api::moved::run(&data, Some("shop"), None, |_| {}).await.unwrap();
    assert_eq!(moved.len(), 1);
    rig.start().await;
    assert_eq!(rows(&data.join("journals").join("shop.db"), "instances"), 1);
    assert_eq!(rig.count_of("system.moved_apart").await, 2);
}

#[tokio::test]
async fn a_move_cut_off_leaves_the_project_as_it_was_or_moved_and_the_daemon_clears_what_it_left() {
    if crate::one_journal() {
        return;
    }
    let (mut rig, data) = rig("moved-cut").await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.stop().await;
    // As a program from before this one listed a journal: a file that
    // keeps nothing of what is run.
    kept_less(&data, "shop", journal::keeps::NOTHING);
    rig.start().await;
    let (shop, _) = counter(&rig, "shop").await;
    told(&rig, "shop", 1).await;
    rig.stop().await;
    let (factory, journals) = (data.join("journal.db"), data.join("journals"));
    let files = |journals: &Path| -> Vec<String> {
        let mut files: Vec<String> = std::fs::read_dir(journals)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".db"))
            .collect();
        files.sort();
        files
    };

    // Cut off with the project's file written and nothing decided: the
    // project is as it was, and the daemon takes the file away as it
    // starts, and says so.
    let moved = api::moved::run(&data, None, Some(Stop::Written), |_| {}).await.unwrap();
    assert!(moved.is_empty());
    assert_eq!(files(&journals), ["shop.2.db", "shop.db"]);
    assert_eq!(listed(&data), [("shop".to_string(), "shop.db".to_string(), 0, false)]);
    assert_eq!(rows(&factory, "instances"), 1);
    rig.start().await;
    let cleared = rig.event("system.move_cleared", |_| true).await;
    assert_eq!(cleared.data["files"], json!(1), "{}", cleared.data);
    assert_eq!(files(&journals), ["shop.db"]);
    assert_eq!(rig.count_of("system.moved_apart").await, 0);
    told(&rig, "shop", 2).await;
    assert_eq!(heard(&rig, &shop).await, json!(2));
    rig.stop().await;

    // Cut off with the move decided and the file it had before not yet
    // taken away: the project is moved, into the file its row names,
    // and the one no row names is cleared.
    let moved = api::moved::run(&data, None, Some(Stop::Decided), |_| {}).await.unwrap();
    assert!(moved.is_empty());
    assert_eq!(files(&journals), ["shop.2.db", "shop.db"]);
    assert_eq!(listed(&data), [("shop".to_string(), "shop.2.db".to_string(), journal::keeps::MADE_AT, true)]);
    assert_eq!((rows(&factory, "instances"), rows(&journals.join("shop.2.db"), "instances")), (0, 1));
    rig.start().await;
    assert_eq!(files(&journals), ["shop.2.db"]);
    assert_eq!(rig.count_of("system.move_cleared").await, 2);
    let said = rig.event("system.moved_apart", |_| true).await;
    assert_eq!(said.data["moved"], "shop");
    thing(&rig, "shop", 3).await;
    assert_eq!(heard(&rig, &shop).await, json!(3));
    let _: Value = rig.read(&format!("/api/instances/{shop}")).await;
}

/// What is applied for a project that keeps something of every kind
/// beside its rows: it starts a text that ships by its name, one of the
/// project's own, and one whose name is only known as the step runs;
/// and it reads a file of the project's on a timer.
const TREE: &str = r#"
blueprint: tree
params: { of: "" }
on:
  ready:
    event: s.ready
    steps:
      kid: { spawn: kid, as: "kid-${ event.data.n }", with: { of: "${params.of}", n: "${event.data.n}" } }
      own: { spawn: own, as: "own-${ event.data.n }", with: { of: "${params.of}", n: "${event.data.n}" } }
  find:
    event: s.find
    steps:
      found:
        spawn: "${ event.data.name }"
        as: "found-${ event.data.n }"
        with: { of: "${params.of}", n: "${event.data.n}" }
  read:
    poll: { every: 30s, do: fs.read, with: { path: watched.txt } }
    steps:
      said:
        do: emit
        with: { topic: s.read, of: "${params.of}", now: "${event.data.content}", was: "${event.previous.content}" }
"#;

/// A text called `name` that says `says` of itself where it is run.
fn text(name: &str, says: &str) -> String {
    format!(
        "blueprint: {name}\nparams: {{ of: \"\", n: 0 }}\nsteps:\n  said:\n    do: emit\n    \
         with: {{ topic: s.{name}, of: \"${{params.of}}\", n: \"${{params.n}}\", says: {says} }}\n"
    )
}

/// The blueprint that ships which each project of the scene has a fork of.
const FORKED: &str = "std/pass-by-hand";

/// The names under which what goes with a project is kept beside its
/// rows.
const BESIDE: [&str; 7] = [
    runs::beside::PINNED,
    runs::beside::TEXTS,
    runs::beside::VOUCHED,
    runs::beside::POLL,
    runs::beside::FITS,
    runs::beside::FORKED,
    runs::beside::PROJECT_ENVIRONMENTS,
];
/// One thing kept under such a name: the name, its key and what is kept.
type Entry = (String, String, String);
/// The projects of the scene.
const PROJECTS: [&str; 2] = ["shop", "yard"];

/// A factory from before whose trees keep something of every kind
/// beside their rows.
struct Scene {
    rig: Rig,
    data: PathBuf,
    clock: ManualClock,
    /// The folder what ships is read from.
    ships: PathBuf,
    /// Where each project's files are.
    folders: Vec<PathBuf>,
    /// The tree applied for each project, by the project.
    trees: Vec<(String, String)>,
}

impl Scene {
    /// A factory whose texts are held, with two projects whose rows the
    /// factory's own conductor keeps: `shop` as a program from before
    /// kept it, with no journal of its own, and `yard` with a journal
    /// that says it keeps nothing. Each has the tree applied for it by
    /// its owner: held to `kid` and `found` as they ship, with `own`
    /// vouched for as the project's file stood, the last of each text
    /// that checked remembered, and `watched.txt` read twice, as `one`
    /// and as `two`.
    async fn made(label: &str) -> Scene {
        let (clock, mut data, mut ships) = (ManualClock::new(), PathBuf::new(), PathBuf::new());
        let theirs = clock.clone();
        let rig = Rig::with(label, |config, dir| {
            ships = dir.join("ships");
            std::fs::create_dir_all(&ships).unwrap();
            config.blueprints_dir = Some(ships.clone());
            config.clock = Some(Arc::new(theirs));
            data = config.data_dir.clone();
        });
        let mut scene = Scene {
            rig: Rig::owned(rig.await).await,
            data,
            clock,
            ships,
            folders: Vec::new(),
            trees: Vec::new(),
        };
        scene.shipped("one");
        let rig = &mut scene.rig;
        rig.send("/api/system/texts", json!({ "come": "held" })).await;
        for project in PROJECTS {
            let added = rig.send("/api/projects", json!({ "name": project })).await;
            scene.folders.push(PathBuf::from(added["path"].as_str().unwrap()));
        }
        rig.stop().await;
        scene.listed("DELETE FROM journals WHERE project = 'shop'");
        journal::moved::put_away(&scene.data.join("journals").join("shop.db"));
        kept_less(&scene.data, "yard", journal::keeps::NOTHING);
        scene.rig.start().await;
        for project in PROJECTS {
            let own = format!("@blueprints/{}", brand::blueprint_file("own"));
            scene.write(project, &own, &text("own", "vouched")).await;
            scene.write(project, "watched.txt", "one").await;
            // Waited for: a handler hears from when its instance is active, and what is
            // said to the tree next is said as soon as this is answered.
            let asked =
                json!({ "source": TREE, "project": project, "params": { "of": project }, "wait": true });
            let tree = scene.rig.send("/api/apply", asked).await["instance"].as_str().unwrap().to_string();
            scene.trees.push((project.to_string(), tree));
            assert_eq!(scene.ready(project, 1).await, ("one".to_string(), "vouched".to_string()));
            assert_eq!(scene.find(project, 1).await, "one");
            scene.first_read(project).await;
            scene.write(project, "watched.txt", "two").await;
            assert_eq!(scene.read(project, "two").await, "one");
            // And a fork of a blueprint that ships, with what it was made from kept.
            scene.rig.send(&format!("/api/projects/{project}/own"), json!({ "blueprint": FORKED })).await;
            // And an environment, with a variable.
            scene
                .rig
                .send(&format!("/api/projects/{project}/environments"), json!({ "name": "staging" }))
                .await;
            let var = format!("/api/projects/{project}/environments/staging/vars/api");
            let body = Some(json!({ "value": format!("https://{project}.example") }));
            let (status, set) = scene.rig.call(Method::PUT, &var, Some(&scene.rig.token), body).await;
            assert_eq!(status, StatusCode::OK, "{set}");
        }
        scene
    }

    /// Changes one row of `journals` while the daemon is stopped.
    fn listed(&self, change: &str) {
        let connection = journal::copy::Connection::open(self.data.join("journal.db")).unwrap();
        assert_eq!(connection.execute(change, []).unwrap(), 1, "{change}");
    }

    /// How often a child the tree of `project` started by name said it
    /// was run: once for each thing made ready, where nothing was heard
    /// again.
    async fn kids(&self, project: &str) -> usize {
        let query = journal::EventQuery { topics: "s.kid".into(), limit: 10_000, ..Default::default() };
        self.rig.said(query).await.iter().filter(|event| event.data["of"] == project).count()
    }

    /// Has `kid` and `found` ship saying `says`, as another program does.
    fn shipped(&self, says: &str) {
        for name in ["kid", "found"] {
            std::fs::write(self.ships.join(brand::blueprint_file(name)), text(name, says)).unwrap();
        }
    }

    /// Saves a file of `project`'s through the daemon, as its owner.
    async fn write(&self, project: &str, path: &str, content: &str) {
        let (rig, body) = (&self.rig, json!({ "path": path, "content": content }));
        let route = format!("/api/projects/{project}/file");
        let (status, answer) = rig.call(Method::PUT, &route, Some(&rig.token), Some(body)).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
    }

    /// What somebody did to the projects' files with no daemon to save
    /// them through, and what another program brought: `own` and
    /// `watched.txt` are other than they were, and so is what ships.
    fn changed(&self) {
        self.shipped("two");
        for (dir, project) in self.folders.iter().zip(PROJECTS) {
            let place = conductor::library::folders::kept_at(&self.data, project);
            std::fs::write(place.join(brand::blueprint_file("own")), text("own", "changed")).unwrap();
            std::fs::write(dir.join("watched.txt"), "three").unwrap();
        }
    }

    /// Has what the tree of `project` reads on a timer say `now`, with
    /// no daemon to save it through.
    fn watched(&self, project: &str, now: &str) {
        let at = PROJECTS.iter().position(|one| *one == project).unwrap();
        std::fs::write(self.folders[at].join("watched.txt"), now).unwrap();
    }

    /// The tree that was applied for `project`.
    fn tree(&self, project: &str) -> &str {
        let found = self.trees.iter().find(|(of, _)| of == project);
        &found.unwrap_or_else(|| panic!("no tree was applied for `{project}`")).1
    }

    /// Waits for the run of the handler `handler` of the project's tree
    /// that ended after `mark`: a project is moved at rest, and a
    /// handler that has said what it says may not be over.
    async fn over(&self, project: &str, handler: &str, mark: u64) {
        let (tree, flow) = (self.tree(project), format!("on.{handler}"));
        let wanted = |event: &kernel::Event| {
            event.seq > mark
                && event.data["flow"] == flow.as_str()
                && event.source.instance.as_deref() == Some(tree)
        };
        self.rig.event("run.finished", wanted).await;
    }

    /// What a child of the project's tree said of itself on `topic`, for
    /// the thing `n`, after `mark`, once its steps are over.
    async fn says(&self, topic: &str, project: &str, n: u64, mark: u64) -> String {
        let wanted = |event: &kernel::Event| {
            event.seq > mark && event.data["of"] == project && event.data["n"] == json!(n)
        };
        let said = self.rig.event(topic, wanted).await;
        let child = said.source.instance.clone().expect("a step's event says its instance");
        self.rig
            .event("run.finished", |event| event.source.instance.as_deref() == Some(child.as_str()))
            .await;
        said.data["says"].as_str().unwrap().to_string()
    }

    /// Has a thing made ready in `project`: what the text that ships and
    /// the project's own each said, started for it.
    async fn ready(&self, project: &str, n: u64) -> (String, String) {
        let mark = self.rig.mark();
        self.rig.send("/api/emit/s.ready", json!({ "project": project, "n": n })).await;
        let said = (self.says("s.kid", project, n, mark).await, self.says("s.own", project, n, mark).await);
        self.over(project, "ready", mark).await;
        said
    }

    /// Has the tree start `found`, by a name it is only told now.
    async fn find(&self, project: &str, n: u64) -> String {
        let mark = self.rig.mark();
        self.rig.send("/api/emit/s.find", json!({ "project": project, "n": n, "name": "found" })).await;
        let said = self.says("s.found", project, n, mark).await;
        self.over(project, "find", mark).await;
        said
    }

    /// Lets half a minute pass, which is when a poll is due, and waits
    /// for something to have gone back to waiting on the clock.
    async fn tick(&self) {
        let since = self.clock.sleep_count();
        self.clock.advance(Duration::from_secs(33));
        self.clock.sleeps_started(since + 1).await;
    }

    /// Lets time pass until the project's tree has read `watched.txt`
    /// once, which is what it tells the next reading from.
    async fn first_read(&self, project: &str) {
        let (journal, key) = (self.rig.node().app.journal.clone(), format!("{}/read", self.tree(project)));
        crate::soon("what a poll first read to be kept", async {
            while journal.kv_get(runs::beside::POLL, &key).await.unwrap().is_none() {
                self.tick().await;
            }
        })
        .await;
    }

    /// Lets time pass until the tree of `project` says that what it
    /// reads has come to `now`, and its saying so is over: what it says
    /// it was before.
    async fn read(&self, project: &str, now: &str) -> String {
        let query = journal::EventQuery { topics: "s.read".into(), limit: 10_000, ..Default::default() };
        let what = format!("`{project}` to read `{now}`");
        let said = crate::soon(&what, async {
            loop {
                let all = self.rig.said(query.clone()).await;
                let said =
                    all.into_iter().find(|event| event.data["of"] == project && event.data["now"] == now);
                if let Some(said) = said {
                    return said;
                }
                self.tick().await;
            }
        })
        .await;
        self.over(project, "read", said.seq).await;
        said.data["was"].as_str().unwrap_or_default().to_string()
    }
}

/// What a file keeps beside its rows under the names that go with a
/// project, read while no daemon has it.
fn beside(file: &Path) -> Vec<Entry> {
    let connection = journal::copy::Connection::open(file).unwrap();
    let mut statement =
        connection.prepare("SELECT namespace, key, value FROM kv ORDER BY namespace, key").unwrap();
    let all = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
    let all: Vec<Entry> = all.collect::<rusqlite::Result<_>>().unwrap();
    all.into_iter().filter(|(name, ..)| BESIDE.contains(&name.as_str())).collect()
}

/// The instances a file's rows hold for `project`.
fn instances(file: &Path, project: &str) -> Vec<String> {
    let connection = journal::copy::Connection::open(file).unwrap();
    let mut statement = connection.prepare("SELECT id FROM instances WHERE project = ?1").unwrap();
    statement.query_map([project], |row| row.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap()
}

/// Of what a file keeps beside its rows, what is `project`'s: what is
/// kept for one of its instances, `ids`, and what is kept for it by name.
fn of_project(all: &[Entry], ids: &[String], project: &str) -> Vec<Entry> {
    use runs::beside::{Whose, instance_of, project_of, whose};
    let its = |(name, key, _): &&Entry| match whose(name) {
        Some(Whose::Instance) => ids.iter().any(|id| id == instance_of(key)),
        Some(Whose::Project) => project_of(key) == Some(project),
        _ => false,
    };
    all.iter().filter(its).cloned().collect()
}

/// The texts a file keeps by revision, each with what is kept of it.
fn texts(all: &[Entry]) -> Vec<(String, String)> {
    let texts = all.iter().filter(|(name, ..)| name == runs::beside::TEXTS);
    texts.map(|(_, revision, kept)| (revision.clone(), kept.clone())).collect()
}

/// The revisions that what is pinned for a project's trees names.
fn held_to(its: &[Entry]) -> Vec<String> {
    let pins = its.iter().filter(|(name, ..)| name == runs::beside::PINNED);
    let mut revisions: Vec<String> = pins
        .flat_map(|(.., kept)| {
            let kept: Value = serde_json::from_str(kept).unwrap();
            let texts = kept["texts"].as_object().unwrap().values();
            texts.map(|revision| revision.as_str().unwrap().to_string()).collect::<Vec<_>>()
        })
        .collect();
    revisions.sort();
    revisions.dedup();
    revisions
}

#[tokio::test]
async fn what_is_kept_beside_a_projects_rows_goes_where_they_go_wherever_a_move_is_cut_off_or_takes_the_rest()
{
    if crate::one_journal() {
        return;
    }
    let mut scene = Scene::made("moved-beside").await;
    scene.rig.stop().await;
    scene.changed();
    let (factory, journals) = (scene.data.join("journal.db"), scene.data.join("journals"));
    let own = journals.join("shop.db");
    let before = beside(&factory);
    let ids = |project: &str| instances(&factory, project);
    let (shops, yards) =
        (of_project(&before, &ids("shop"), "shop"), of_project(&before, &ids("yard"), "yard"));
    // Something of every kind is kept for each of them.
    for name in BESIDE.iter().filter(|name| **name != runs::beside::TEXTS) {
        for (project, its) in [("shop", &shops), ("yard", &yards)] {
            assert!(
                its.iter().any(|(kept, ..)| kept == name),
                "nothing under `{name}` for `{project}`: {its:?}"
            );
        }
    }
    assert_eq!(held_to(&shops).len(), 2, "`kid` and `found`, as they shipped: {shops:?}");

    // Asked first, it says how much of each kind would go with the rows,
    // each in words, and changes nothing.
    let would: Vec<(String, u64)> = BESIDE
        .iter()
        .map(|name| match *name == runs::beside::TEXTS {
            true => (name.to_string(), held_to(&shops).len() as u64),
            false => (name.to_string(), shops.iter().filter(|(kept, ..)| kept == name).count() as u64),
        })
        .collect();
    let said = |counted: &[api::moved::Counted]| -> Vec<(String, u64)> {
        api::moved::went(counted, true).into_iter().filter(|(_, kept)| *kept > 0).collect()
    };
    let asked = api::moved::look(&scene.data, Some("shop")).unwrap();
    assert_eq!(said(&asked.projects[0].rows), would);
    for name in BESIDE {
        assert!(!api::moved::said_of(name).contains('`'), "`{name}` is said to nobody in words");
    }
    assert_eq!(beside(&factory), before, "asking changes nothing");
    let as_it_was = [("yard".to_string(), "yard.db".to_string(), 0, false)];

    // Cut off with the project's file written and nothing decided: all
    // of it is in the factory's file with the rows, as it was, and what
    // was written of it is in a file that no row names, which whoever
    // next holds the folder takes away (the daemon as it starts, in the
    // test of a move cut off above; here, the move asked again).
    let moved = api::moved::run(&scene.data, Some("shop"), Some(Stop::Written), |_| {}).await.unwrap();
    assert!(moved.is_empty());
    assert_eq!(listed(&scene.data), as_it_was);
    assert_eq!(beside(&factory), before);
    assert_eq!(of_project(&beside(&own), &instances(&own, "shop"), "shop"), shops);

    // Cut off with the move decided: what was kept for the project and
    // its instances is in its own file as it was, and the factory's
    // keeps none of it; what is kept for the project that was not moved
    // is where it was, as it was.
    let moved = api::moved::run(&scene.data, Some("shop"), Some(Stop::Decided), |_| {}).await.unwrap();
    assert!(moved.is_empty());
    let apart = ("shop".to_string(), "shop.db".to_string(), journal::keeps::MADE_AT, true);
    assert_eq!(listed(&scene.data), [apart.clone(), as_it_was[0].clone()]);
    let (left, there) = (beside(&factory), beside(&own));
    assert!(instances(&factory, "shop").is_empty());
    assert_eq!(of_project(&there, &instances(&own, "shop"), "shop"), shops);
    assert_eq!(of_project(&left, &instances(&own, "shop"), "shop"), Vec::<Entry>::new());
    assert_eq!(of_project(&left, &ids("yard"), "yard"), yards);
    // The texts its trees are held to are kept with them, each once in
    // each file: the factory's has them still, for the other project's.
    let kept: Vec<(String, String)> =
        texts(&before).into_iter().filter(|(revision, _)| held_to(&shops).contains(revision)).collect();
    assert_eq!(texts(&there), kept);
    assert_eq!(texts(&left), texts(&before));
    assert_eq!(there.len(), shops.len() + kept.len(), "nothing else went: {there:?}");

    // Started, the daemon says how much of each kind went, and each tree
    // goes on held to the texts it had reached, whatever ships now; with
    // the project's own file as it was vouched for, whatever the file
    // says now; and telling what it reads from what it read before.
    scene.rig.start().await;
    let done = scene.rig.event("system.moved_apart", |_| true).await;
    let went: serde_json::Map<String, Value> =
        would.iter().map(|(name, kept)| (name.clone(), json!(kept))).collect();
    let told: serde_json::Map<String, Value> = done.data["beside"]
        .as_object()
        .unwrap()
        .clone()
        .into_iter()
        .filter(|(_, kept)| *kept != json!(0))
        .collect();
    assert_eq!(told, went, "{}", done.data);
    for project in PROJECTS {
        assert_eq!(scene.ready(project, 2).await, ("one".to_string(), "vouched".to_string()), "{project}");
        assert_eq!(scene.find(project, 2).await, "one", "{project}");
        assert_eq!(scene.read(project, "three").await, "two", "{project}");
    }
    // Asked for again, nothing is taken that nobody said to take: the
    // texts that ship, and the project's own, are said to have been left.
    let asked = json!({ "source": TREE, "project": "shop", "params": { "of": "shop" } });
    let again = scene.rig.send("/api/apply", asked).await;
    let left: Vec<&str> = again["reached"]["left"]
        .as_array()
        .unwrap()
        .iter()
        .map(|text| text["name"].as_str().unwrap())
        .collect();
    assert_eq!(left, ["found", "kid"], "{again}");
    assert_eq!(again["vouched"], json!({ "left": ["own"] }), "{again}");
    // And each thing is where its instance's rows are, while it runs.
    crate::apart::held_to_the_lists(&scene.rig.node().app.journal.clone()).await;
    // What each project's fork was made from is read where the project now is.
    for project in PROJECTS {
        let origin = scene.rig.node().app.engine.forked_from(project, FORKED).await;
        assert_eq!(origin.map(|origin| origin.blueprint).as_deref(), Some(FORKED), "{project}");
        // And its environments.
        let staging = scene.rig.read(&format!("/api/projects/{project}/environments/staging")).await;
        assert_eq!(staging["vars"]["api"]["value"], format!("https://{project}.example"), "{project}");
    }
    assert_eq!(rows(&factory, "instances"), instances(&factory, "yard").len() as i64);

    // The rest of the way. As a project taken away and added again: the
    // other project's journal says it keeps what is run and what is
    // said, while what was running for it is in the factory's own rows.
    scene.rig.stop().await;
    scene.listed("UPDATE journals SET keeps = 1 WHERE project = 'yard'");
    scene.watched("yard", "four");
    let yards = of_project(&beside(&factory), &ids("yard"), "yard");
    assert!(
        BESIDE.iter().all(|name| *name == runs::beside::TEXTS || yards.iter().any(|(kept, ..)| kept == name))
    );
    assert_eq!(rows(&journals.join("yard.db"), "instances"), 0);
    // It is not apart while the factory's rows hold something of it;
    // what they hold goes where the rest of it is kept, with what is
    // kept beside it, into a file beside the one it had.
    let asked = api::moved::look(&scene.data, None).unwrap();
    let state: Vec<_> = asked.projects.iter().map(|project| (project.apart(), project.moves())).collect();
    assert_eq!(state, [(true, false), (false, true)]);
    assert_eq!(said(&asked.projects[1].rows).len(), BESIDE.len());
    let moved = api::moved::run(&scene.data, None, None, |_| {}).await.unwrap();
    assert_eq!(moved.iter().map(|moved| moved.file.as_str()).collect::<Vec<_>>(), ["yard.2.db"]);
    // What was said of it is not moved, since its journal says it keeps
    // that: so its row does not say it was moved from anywhere, which is
    // what a place in the journal it was moved from would go by. Nothing
    // kept beside the rows is a place: it is all read by its key.
    let rest = ("yard".to_string(), "yard.2.db".to_string(), journal::keeps::MADE_AT, false);
    assert_eq!(listed(&scene.data), [apart, rest]);
    let own = journals.join("yard.2.db");
    assert!(!journals.join("yard.db").exists());
    assert_eq!(rows(&factory, "instances"), 0);
    assert_eq!(of_project(&beside(&own), &instances(&own, "yard"), "yard"), yards);
    assert_eq!(of_project(&beside(&factory), &instances(&own, "yard"), "yard"), Vec::<Entry>::new());
    let held: Vec<String> = texts(&beside(&own)).into_iter().map(|(revision, _)| revision).collect();
    assert_eq!(held, held_to(&yards));

    // Its own conductor has it from the next start, and it goes on as
    // it was held, hearing nothing again, with what it does written in
    // its own file.
    scene.rig.start().await;
    let runs = rows(&own, "runs");
    assert_eq!(scene.ready("yard", 3).await, ("one".to_string(), "vouched".to_string()));
    assert_eq!(scene.find("yard", 3).await, "one");
    assert_eq!(scene.read("yard", "four").await, "three");
    assert_eq!(scene.kids("yard").await, 3, "each thing made ready heard once, before the move and after");
    assert!(rows(&own, "runs") > runs);
    assert_eq!(scene.rig.count_of("system.moved_apart").await, 2);
    crate::apart::held_to_the_lists(&scene.rig.node().app.journal.clone()).await;
}

/// Waits, in its steps, for a thing said in any project, and says that
/// it woke: nobody's, so that it is under way while a project is moved.
const WAKES: &str = r#"
blueprint: wakes
steps:
  for_it: { do: wait, with: { event: s.thing, scope: fleet } }
  woke: { do: emit, with: { topic: s.woke } }
"#;

/// The numbers of the events in the database `file` that are of
/// `project`: those whose data names it.
fn said_of(file: &Path, project: &str) -> Vec<i64> {
    let connection = journal::copy::Connection::open(file).unwrap();
    let mut statement = connection
        .prepare("SELECT seq FROM events WHERE json_extract(data, '$.project') = ?1 ORDER BY seq")
        .unwrap();
    statement.query_map([project], |row| row.get(0)).unwrap().map(Result::unwrap).collect()
}

/// The numbers of the events in the database `file` that are of
/// `project` and the factory's for good (`journal::HERE`): what is said
/// of a channel of it, and that it was put in a workspace, each kept
/// where its row is.
fn said_in_channels(file: &Path, project: &str) -> Vec<i64> {
    let connection = journal::copy::Connection::open(file).unwrap();
    let mut statement = connection
        .prepare(
            "SELECT seq FROM events WHERE (topic LIKE 'channel.%' OR topic = 'project.placed')
             AND json_extract(data, '$.project') = ?1 ORDER BY seq",
        )
        .unwrap();
    statement.query_map([project], |row| row.get(0)).unwrap().map(Result::unwrap).collect()
}

fn holed(file: &Path) -> bool {
    let connection = journal::copy::Connection::open(file).unwrap();
    connection
        .query_row(
            "SELECT EXISTS (SELECT 1 FROM kv WHERE namespace = 'journal' AND key = 'holes')",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[tokio::test]
async fn what_was_said_of_a_project_is_moved_with_it_heard_once_by_whoever_follows_and_kept_through_a_repack_and_a_copy()
 {
    if crate::one_journal() {
        return;
    }
    let (mut rig, data) = rig("moved-said").await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.stop().await;
    from_before(&data);
    rig.start().await;
    // A handler of the project that has heard two things; something of
    // nobody's that waits for the next thing said anywhere, begun after
    // them; and a page that read as far as the first.
    let (shop, _) = counter(&rig, "shop").await;
    told(&rig, "shop", 1).await;
    let page = rig.read("/api/events/recent?topics=s.thing").await;
    let cursor = page["cursor"].as_str().unwrap_or_else(|| panic!("a place, as text: {page}")).to_string();
    told(&rig, "shop", 2).await;
    // And something is said in a channel of the project's, which is the
    // factory's for good whatever project the channel says.
    rig.send("/api/channels", json!({ "name": "shopfloor", "project": "shop" })).await;
    rig.send("/api/channels/shopfloor/messages", json!({ "body": "the till is in" })).await;
    let waits = rig.send("/api/apply", json!({ "source": WAKES })).await;
    assert!(waits["instance"].is_string(), "{waits}");
    rig.event("step.started", |event| event.data["target"] == "wait").await;
    let (factory, own) = (data.join("journal.db"), data.join("journals").join("shop.db"));
    rig.stop().await;
    let before = said_of(&factory, "shop");
    assert!(before.len() >= 2, "the two things said of it, at least: {before:?}");
    let in_channels = said_in_channels(&factory, "shop");
    assert!(
        in_channels.len() >= 3,
        "that it was put in a workspace, a channel made and a thing said there: {in_channels:?}"
    );
    let its_own: Vec<i64> = before.iter().copied().filter(|seq| !in_channels.contains(seq)).collect();

    // Moved, what was said of it is in its own journal, each event
    // under the number it had, and the factory's keeps none of it; both
    // say that their events have holes now.
    let moved = api::moved::run(&data, None, None, |_| {}).await.unwrap();
    assert_eq!(moved.len(), 1);
    assert_eq!(
        moved[0].rows.iter().find(|counted| counted.table == "events").map(|counted| counted.moves),
        Some(rows(&own, "events") as u64)
    );
    // What was said of its channels, and that it was put in a
    // workspace, stay in the factory's journal, where their rows are.
    assert_eq!((said_of(&factory, "shop"), said_of(&own, "shop")), (in_channels.clone(), its_own.clone()));
    assert!(holed(&factory) && holed(&own));
    assert_eq!(listed(&data), [("shop".to_string(), "shop.db".to_string(), journal::keeps::MADE_AT, true)]);

    rig.start().await;
    // Nothing is heard again for having been moved: not by the handler,
    // whose place was in the factory's journal, and not by what waits.
    assert_eq!(heard(&rig, &shop).await, json!(2));
    assert_eq!(heard_times(&rig, "shop").await, 2);
    // What was said in its channel is read there as before.
    let there = rig.read("/api/channels/shopfloor/messages").await;
    assert_eq!(there["messages"].as_array().map(Vec::len), Some(1), "{there}");
    let woke = journal::EventQuery { topics: "s.woke".into(), ..Default::default() };
    // The next thing is heard once by each.
    let mark = rig.mark();
    told(&rig, "shop", 3).await;
    assert_eq!(heard(&rig, &shop).await, json!(3));
    assert_eq!(heard_times(&rig, "shop").await, 3);
    let woken = rig.event("s.woke", |_| true).await;
    assert!(woken.seq > mark, "it woke for the thing said since, and not for one moved");
    assert_eq!(rig.said(woke).await.len(), 1);
    // The page that read as far as the first thing is handed the second
    // and the third, the newest first as it asks, and not the first.
    let read = rig.read(&format!("/api/events/recent?topics=s.thing&after={cursor}")).await;
    let read: Vec<_> =
        read["events"].as_array().unwrap().iter().map(|event| event["data"]["n"].clone()).collect();
    assert_eq!(read, [json!(3), json!(2)]);
    rig.stop().await;

    // Asked to be written out again, each file is looked at and what was
    // said is where it was moved to afterwards, under the same numbers.
    let said = said_of(&own, "shop");
    assert!(said.len() > its_own.len(), "what was said before the move, and since: {said:?}");
    let at = data.clone();
    tokio::task::spawn_blocking(move || api::repack::run(&at, |_| {})).await.unwrap().unwrap();
    assert_eq!((said_of(&factory, "shop"), said_of(&own, "shop")), (in_channels.clone(), said.clone()));
    rig.start().await;
    told(&rig, "shop", 4).await;
    assert_eq!((heard(&rig, &shop).await, heard_times(&rig, "shop").await), (json!(4), 4));

    // A copy takes the project's file with the factory's: what was said
    // of the project before it was moved is in the copy's file for it.
    let to = data.parent().unwrap().join("copy");
    let taken = crate::snapshots::snapshot(&rig, &to).await;
    assert_eq!(taken["status"], "ok", "{taken}");
    let copied: Vec<i64> = said_of(&to.join("journals").join("shop.db"), "shop");
    assert_eq!(copied[..said.len()], said[..]);
    rig.stop().await;

    // The newest so many of each journal are counted over the holes: of
    // the project's journal the newest three are kept and no more.
    let clock = std::sync::Arc::new(kernel::ManualClock::new());
    let journals = journal::moved::made(&own, "project:shop", clock).await.unwrap();
    let all = rows(&own, "events");
    let gone = journals.prune_events(i64::MAX, 3).await.unwrap();
    assert_eq!((gone as i64, rows(&own, "events")), (all - 3, 3));
}
