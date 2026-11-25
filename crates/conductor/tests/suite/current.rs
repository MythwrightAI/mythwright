//! Which text an instance runs and whether it is the one now there; and
//! what applying a parent again does for what its handlers started.

use conductor::{Apply, InstanceFilter, InstanceInfo, InstanceStatus, RunStatus, Shown, Takes};
use kernel::{ErrorKind, Principal, PrincipalKind, Source};
use serde_json::{Value, json};

use crate::handlers::{active, handled};
use crate::{Rig, step, user};

/// What somebody applies. Its handler starts a child for each card; none
/// of its own steps names one.
const PARENT: &str = "\
blueprint: parent
on:
  ready:
    event: board.item.moved
    steps:
      go: { spawn: child, as: \"kid-${ event.data.id }\", with: { id: ${ event.data.id } } }
";

/// The child, as it is when it is started.
const CHILD: &str = "\
blueprint: child
version: 1
params: { id: 0 }
state: { seen: 0 }
steps:
  claim: { do: test.count, with: { key: claimed } }
on:
  poke:
    event: test.poke
    steps:
      note: { do: set, with: { seen: ${ state.seen + 1 } } }
      say: { do: test.count, with: { key: old } }
";

/// The same with another handler, and saying so of itself.
fn later() -> String {
    CHILD.replace("key: old", "key: new").replace("version: 1", "version: 2")
}

async fn moved(rig: &Rig, id: u64) {
    rig.journal.emit("board.item.moved", Source::default(), json!({ "id": id })).await.unwrap();
}

async fn poke(rig: &Rig) {
    rig.journal.emit("test.poke", Source::default(), json!({})).await.unwrap();
}

async fn live(rig: &Rig) -> Vec<InstanceInfo> {
    rig.engine.instances(InstanceFilter { live_only: true, ..Default::default() }).await.unwrap()
}

async fn keyed(rig: &Rig, key: &str) -> InstanceInfo {
    live(rig).await.into_iter().find(|one| one.key == key).unwrap_or_else(|| panic!("no live `{key}`"))
}

async fn shown(rig: &Rig, id: &str) -> Shown {
    rig.engine.shown_one(rig.instance(id).await)
}

/// The parent applied, and one child of it started by its handler and
/// settled: `(parent, child)`.
async fn started(rig: &Rig) -> (String, String) {
    rig.library.put("parent", PARENT);
    rig.library.put("child", CHILD);
    let applied = rig.apply(PARENT, json!({})).await;
    active(rig, &applied.instance).await;
    moved(rig, 1).await;
    handled(rig, "on.ready", 1).await;
    let kid = keyed(rig, "kid-1").await;
    active(rig, &kid.id).await;
    (applied.instance, kid.id)
}

/// Applies the parent again as it is, as Ada.
async fn again(rig: &Rig) {
    let applied = rig.engine.apply(rig.request(PARENT, json!({}))).await.unwrap();
    assert!(!applied.changed, "the parent's own text is what it was");
}

/// What was said of instances taking a text where they are: `(instance, from, to)`.
async fn taken(rig: &Rig) -> Vec<(String, String, String)> {
    rig.events("instance.updated")
        .await
        .into_iter()
        .filter(|event| event.data["in_place"] == true)
        .map(|event| {
            let text = |key: &str| event.data[key].as_str().unwrap().to_string();
            (event.source.instance.clone().unwrap(), text("from"), text("revision"))
        })
        .collect()
}

#[tokio::test]
async fn an_instance_says_which_text_it_runs_and_is_behind_when_the_text_now_there_is_another() {
    let rig = Rig::new().await;
    let (parent, kid) = started(&rig).await;
    let is = shown(&rig, &kid).await;
    assert_eq!(is.instance.revision, blueprint::revision(CHILD));
    assert_eq!(is.version.as_deref(), Some("1"));
    assert_eq!(is.behind, None, "it runs what is there");
    assert_eq!(shown(&rig, &parent).await.version, None, "the parent's text says no version");

    rig.library.put("child", &later());
    let behind = shown(&rig, &kid).await.behind.expect("it is behind");
    assert_eq!(behind.revision, blueprint::revision(&later()));
    assert_eq!(behind.version.as_deref(), Some("2"));
    assert_eq!(behind.takes, Takes::InPlace);
    assert_eq!(
        behind.why,
        "The text of `child` now there differs from the one this runs only in what it does later: its \
         handlers, what it does when it is stopped, what it may use. It takes that text where it is, with \
         none of its steps run again, when what started it, or it itself, is applied again."
    );
    assert_eq!(shown(&rig, &parent).await.behind, None, "nothing of the parent's moved");

    // As JSON it is the instance, with these beside what it always said.
    let said = serde_json::to_value(shown(&rig, &kid).await).unwrap();
    assert_eq!(said["id"], kid);
    assert_eq!(said["version"], "1");
    assert_eq!(said["behind"]["takes"], "in_place");

    // Once it has stopped there is nothing to be behind; and what was
    // applied from a text no library has is behind nothing.
    rig.engine.stop(&kid).await.unwrap();
    assert_eq!(shown(&rig, &kid).await.behind, None);
    let loose =
        rig.apply("blueprint: loose\non:\n  x:\n    event: test.poke\n    do: noop\n", json!({})).await;
    active(&rig, &loose.instance).await;
    assert_eq!(shown(&rig, &loose.instance).await.behind, None);
}

#[tokio::test]
async fn applying_a_parent_again_has_what_its_handler_started_take_a_text_that_differs_only_in_what_it_does_later()
 {
    let rig = Rig::new().await;
    let (parent, kid) = started(&rig).await;
    poke(&rig).await;
    handled(&rig, "on.poke", 1).await;
    assert_eq!((rig.shared.count("old"), rig.shared.count("new")), (1, 0));

    rig.library.put("child", &later());
    again(&rig).await;

    let is = rig.instance(&kid).await;
    assert_eq!(is.revision, blueprint::revision(&later()));
    assert_eq!(is.status, InstanceStatus::Active, "it was not started again");
    assert_eq!(is.state["seen"], 1, "its state is kept");
    assert_eq!(rig.shared.count("claimed"), 1, "its steps were not run again");
    assert_eq!(
        taken(&rig).await,
        [(kid.clone(), blueprint::revision(CHILD), blueprint::revision(&later()))],
        "and it is said, of the child and of nothing else"
    );
    assert_eq!(shown(&rig, &kid).await.behind, None);
    assert_eq!(shown(&rig, &kid).await.version.as_deref(), Some("2"));
    let versions = rig.engine.versions(&kid, 10).await.unwrap();
    assert_eq!(versions.len(), 2, "the text it took is its next version");
    assert_eq!((versions[0].outcome.as_str(), versions[0].by_name.as_str()), ("applied", "Ada"));

    // The next firing is the new text's, from where the old one had got to.
    poke(&rig).await;
    handled(&rig, "on.poke", 2).await;
    assert_eq!((rig.shared.count("old"), rig.shared.count("new")), (1, 1));
    assert_eq!(rig.instance(&kid).await.state["seen"], 2);

    // Asking again for what is there does nothing more.
    again(&rig).await;
    assert_eq!(taken(&rig).await.len(), 1);
    assert_eq!(rig.instance(&parent).await.status, InstanceStatus::Active);
}

#[tokio::test]
async fn a_run_under_way_when_its_instance_takes_a_text_finishes_on_the_one_it_began_with() {
    let rig = Rig::new().await;
    let held = CHILD.replace(
        "      note: { do: set, with: { seen: ${ state.seen + 1 } } }\n",
        "      wait: { do: test.gate, with: { name: hold } }\n",
    );
    rig.library.put("parent", PARENT);
    rig.library.put("child", &held);
    let applied = rig.apply(PARENT, json!({})).await;
    active(&rig, &applied.instance).await;
    moved(&rig, 1).await;
    handled(&rig, "on.ready", 1).await;
    let kid = keyed(&rig, "kid-1").await;
    active(&rig, &kid.id).await;

    poke(&rig).await;
    rig.reached("hold").await;
    // A second, while the first is still being answered.
    poke(&rig).await;
    rig.library.put("child", &later());
    again(&rig).await;
    assert_eq!(rig.instance(&kid.id).await.revision, blueprint::revision(&later()));
    assert_eq!((rig.shared.count("old"), rig.shared.count("new")), (0, 0), "nothing was cut short");

    rig.open("hold");
    handled(&rig, "on.poke", 2).await;
    assert_eq!(
        (rig.shared.count("old"), rig.shared.count("new")),
        (1, 1),
        "the run under way finished as the old text, and what came in meanwhile was the new text's"
    );
    let runs = rig.engine.runs(&kid.id, 10).await.unwrap();
    let pokes: Vec<(&str, RunStatus)> = runs
        .iter()
        .rev()
        .filter(|run| run.flow == "on.poke")
        .map(|run| (run.revision.as_str(), run.status))
        .collect();
    assert_eq!(
        pokes,
        [
            (blueprint::revision(&held).as_str(), RunStatus::Ok),
            (blueprint::revision(&later()).as_str(), RunStatus::Ok)
        ],
        "each run says which text it was, and neither was run twice"
    );
}

#[tokio::test]
async fn a_child_whose_text_now_begins_otherwise_is_left_as_it_is_and_says_it_must_be_applied_again() {
    let rig = Rig::new().await;
    let (_, kid) = started(&rig).await;
    for changed in [
        later().replace("key: claimed", "key: claimed-otherwise"),
        later().replace("params: { id: 0 }", "params: { id: 0, room: general }"),
        later().replace("state: { seen: 0 }", "state: { seen: 0, rounds: 0 }"),
    ] {
        rig.library.put("child", &changed);
        again(&rig).await;
        assert_eq!(rig.instance(&kid).await.revision, blueprint::revision(CHILD), "left as it was");
        let behind = shown(&rig, &kid).await.behind.expect("and behind");
        assert_eq!(behind.takes, Takes::Again);
        assert_eq!(
            behind.why,
            "The text of `child` now there begins otherwise than the one this runs: its steps, its \
             params, or something else it says at the top. It runs that text only when it is itself \
             applied again, which runs its steps again."
        );
    }
    assert_eq!(taken(&rig).await, [], "nothing was taken");
    assert_eq!(rig.shared.count("claimed"), 1);
    poke(&rig).await;
    handled(&rig, "on.poke", 1).await;
    assert_eq!((rig.shared.count("old"), rig.shared.count("new")), (1, 0), "it goes on as the text it has");
}

#[tokio::test]
async fn a_child_whose_text_no_longer_checks_here_is_left_as_it_is_and_says_what_is_wrong_with_it() {
    let rig = Rig::new().await;
    // Named only when the handler runs, so the parent's apply cannot see it.
    let parent = PARENT.replace("spawn: child,", "spawn: \"${ event.data.kind }\",");
    rig.library.put("parent", &parent);
    rig.library.put("child", CHILD);
    let applied = rig.apply(&parent, json!({})).await;
    active(&rig, &applied.instance).await;
    rig.journal
        .emit("board.item.moved", Source::default(), json!({ "id": 1, "kind": "child" }))
        .await
        .unwrap();
    handled(&rig, "on.ready", 1).await;
    let kid = keyed(&rig, "kid-1").await.id;
    active(&rig, &kid).await;

    rig.library.put("child", &later().replace("key: new", "key: new, loudly: true"));
    let over = rig.engine.apply(rig.request(&parent, json!({}))).await.unwrap();
    assert!(!over.changed);
    assert_eq!(rig.instance(&kid).await.revision, blueprint::revision(CHILD));
    let behind = shown(&rig, &kid).await.behind.expect("it is behind");
    assert_eq!(behind.takes, Takes::Unfit);
    assert_eq!(
        behind.why,
        format!(
            "The text of `child` now there cannot be applied here, so this goes on with the one it has.\n\
             `child` has 1 problem(s):\n\
             child.bp:12:7: error: `test.count` has no argument `loudly`; it takes: key\n\
             `child` may have been written for a newer version than this one, which is {}",
            env!("CARGO_PKG_VERSION")
        )
    );
    poke(&rig).await;
    handled(&rig, "on.poke", 1).await;
    assert_eq!(rig.shared.count("old"), 1, "and it goes on");
}

#[tokio::test]
async fn what_is_further_down_is_reached_too_and_only_what_is_behind_is_touched() {
    let rig = Rig::new().await;
    let grand = "blueprint: grand\non:\n  poke:\n    event: test.poke\n    do: test.count\n    with: { key: grand-old }\n";
    let middle = CHILD.replace(
        "  claim: { do: test.count, with: { key: claimed } }\n",
        "  claim: { do: test.count, with: { key: claimed } }\n  under: { spawn: grand, as: grand }\n",
    );
    rig.library.put("grand", grand);
    rig.library.put("parent", PARENT);
    rig.library.put("child", &middle);
    let applied = rig.apply(PARENT, json!({})).await;
    active(&rig, &applied.instance).await;
    moved(&rig, 1).await;
    handled(&rig, "on.ready", 1).await;
    let kid = keyed(&rig, "kid-1").await.id;
    active(&rig, &kid).await;
    let under = keyed(&rig, "grand").await.id;
    active(&rig, &under).await;

    rig.library.put("grand", &grand.replace("grand-old", "grand-new"));
    again(&rig).await;
    assert_eq!(
        taken(&rig).await.into_iter().map(|(instance, ..)| instance).collect::<Vec<_>>(),
        std::slice::from_ref(&under),
        "the one two below, and not the one between, whose text is what it was"
    );
    poke(&rig).await;
    handled(&rig, "on.poke", 2).await;
    assert_eq!((rig.shared.count("grand-old"), rig.shared.count("grand-new")), (0, 1));
    assert_eq!(rig.shared.count("claimed"), 1);
}

/// An agent as agents are unless given more.
fn builder(scopes: &[&str]) -> Principal {
    Principal {
        kind: PrincipalKind::Agent,
        id: "builder".into(),
        name: "builder".into(),
        scopes: scopes.iter().map(|scope| scope.to_string()).collect(),
        key: None,
    }
}

#[tokio::test]
async fn a_child_that_takes_a_text_acts_as_whoever_applied_its_parent_again_and_is_left_if_they_may_not_use_what_it_lists()
 {
    let rig = Rig::new().await;
    let needs = |key: &str| {
        format!(
            "blueprint: child\npermissions: [shell]\nparams: {{ id: 0 }}\nsteps:\n  claim: {{ do: test.count, with: \
             {{ key: claimed }} }}\non:\n  poke:\n    event: test.poke\n    steps:\n      say: {{ do: test.count, \
             with: {{ key: {key} }} }}\n      run: {{ do: test.shell }}\n"
        )
    };
    rig.library.put("parent", PARENT);
    rig.library.put("child", &needs("old"));
    let applied = rig.apply(PARENT, json!({})).await;
    active(&rig, &applied.instance).await;
    moved(&rig, 1).await;
    handled(&rig, "on.ready", 1).await;
    let kid = keyed(&rig, "kid-1").await.id;
    active(&rig, &kid).await;
    assert_eq!(rig.instance(&kid).await.principal.name, "Ada");

    // Somebody who may apply blueprints and may not run a shell.
    rig.library.put("child", &needs("new"));
    let theirs = Apply { principal: builder(&["blueprints"]), ..rig.request(PARENT, json!({})) };
    rig.engine.apply(theirs).await.unwrap();
    let is = rig.instance(&kid).await;
    assert_eq!(
        (is.revision.as_str(), is.principal.name.as_str()),
        (blueprint::revision(&needs("old")).as_str(), "Ada")
    );
    let failed = rig.events("instance.update_failed").await;
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].source.instance.as_deref(), Some(kid.as_str()));
    assert_eq!(
        failed[0].data["error"],
        "builder may not use `shell`, which `child` lists under `permissions`, so builder cannot apply it \
         over `kid-1`, which is running, and nothing was changed. Someone who holds `shell` can apply it."
    );
    assert!(shown(&rig, &kid).await.behind.is_some(), "it is still behind");

    // Somebody who may do both: the child takes it, and is theirs from then on.
    let theirs = Apply { principal: builder(&["blueprints", "shell"]), ..rig.request(PARENT, json!({})) };
    rig.engine.apply(theirs).await.unwrap();
    let is = rig.instance(&kid).await;
    assert_eq!(
        (is.revision.as_str(), is.principal.name.as_str()),
        (blueprint::revision(&needs("new")).as_str(), "builder")
    );
    assert_eq!(rig.instance(&applied.instance).await.principal.name, "Ada", "the parent was not changed");
    poke(&rig).await;
    handled(&rig, "on.poke", 1).await;
    let run = &rig.engine.runs(&kid, 1).await.unwrap()[0];
    let run = rig.engine.run(&run.id).await.unwrap();
    assert_eq!(step(&run, "run").output, json!({ "ran_as": "builder" }));
}

#[tokio::test]
async fn when_a_parent_is_changed_its_steps_run_first_and_what_its_handlers_started_is_reached_once_they_settle()
 {
    let rig = Rig::new().await;
    let parent = format!("{PARENT}steps:\n  first: {{ call: named, as: named }}\n");
    let named = "blueprint: named\nsteps:\n  claim: { do: test.count, with: { key: named-claimed } }\non:\n  poke:\n    event: test.poke\n    do: test.count\n    with: { key: named-old }\n";
    rig.library.put("named", named);
    rig.library.put("parent", &parent);
    rig.library.put("child", CHILD);
    let applied = rig.apply(&parent, json!({})).await;
    active(&rig, &applied.instance).await;
    moved(&rig, 1).await;
    handled(&rig, "on.ready", 1).await;
    let kid = keyed(&rig, "kid-1").await.id;
    active(&rig, &kid).await;

    // Both children's handlers change, and the parent is given a param.
    rig.library.put("child", &later());
    rig.library.put("named", &named.replace("named-old", "named-new"));
    let changed = format!("{parent}params: {{ loud: false }}\n");
    let over = rig.engine.apply(rig.request(&changed, json!({}))).await.unwrap();
    assert!(over.changed);
    rig.finished(&over).await;

    // The child its steps name is applied by them, as it always was: its
    // own steps run again. The one a handler started takes the text.
    assert_eq!(rig.shared.count("named-claimed"), 2);
    assert_eq!(rig.shared.count("claimed"), 1);
    assert_eq!(
        taken(&rig).await.into_iter().map(|(instance, ..)| instance).collect::<Vec<_>>(),
        std::slice::from_ref(&kid)
    );
    assert_eq!(rig.instance(&kid).await.revision, blueprint::revision(&later()));
}

#[tokio::test]
async fn a_step_that_spawns_a_child_already_there_brings_it_to_the_text_now_there_as_any_apply_does() {
    let rig = Rig::new().await;
    let (_, kid) = started(&rig).await;

    // A text that differs only in what it does later is taken where it is.
    rig.library.put("child", &later());
    moved(&rig, 1).await;
    handled(&rig, "on.ready", 2).await;
    let is = rig.instance(&kid).await;
    assert_eq!(is.revision, blueprint::revision(&later()));
    assert_eq!(rig.shared.count("claimed"), 1, "none of its steps ran again");
    assert_eq!(taken(&rig).await.len(), 1);
    let mains = |runs: Vec<conductor::RunInfo>| runs.into_iter().filter(|run| run.flow == "main").count();
    assert_eq!(mains(rig.engine.runs(&kid, 10).await.unwrap()), 1);

    // One whose steps are others has them run.
    let otherwise = later().replace("key: claimed", "key: claimed-otherwise");
    rig.library.put("child", &otherwise);
    moved(&rig, 1).await;
    handled(&rig, "on.ready", 3).await;
    let runs = rig.engine.runs(&kid, 10).await.unwrap();
    let again = runs.iter().find(|run| run.flow == "main").unwrap();
    rig.soon("the child's steps to finish", rig.engine.wait_run(&again.id)).await.unwrap();
    assert_eq!(mains(rig.engine.runs(&kid, 10).await.unwrap()), 2, "a second run of its own steps");
    assert_eq!(rig.instance(&kid).await.revision, blueprint::revision(&otherwise));
    assert_eq!(rig.shared.count("claimed-otherwise"), 1);
    assert_eq!(taken(&rig).await.len(), 1, "that is an update, not a text taken where it is");
}

#[tokio::test]
async fn one_instance_is_applied_again_as_it_is_with_the_params_and_the_place_it_has() {
    let rig = Rig::new().await;
    let (parent, kid) = started(&rig).await;
    let otherwise = later().replace("key: claimed", "key: claimed-otherwise");
    rig.library.put("child", &otherwise);
    again(&rig).await;
    assert_eq!(shown(&rig, &kid).await.behind.map(|behind| behind.takes), Some(Takes::Again));

    let applied = rig.engine.apply_again(&kid, user(), false).await.unwrap();
    assert_eq!((applied.instance.as_str(), applied.created, applied.changed), (kid.as_str(), false, true));
    assert_eq!(rig.finished(&applied).await.status, RunStatus::Ok);
    let is = rig.instance(&kid).await;
    assert_eq!(is.revision, blueprint::revision(&otherwise));
    assert_eq!(is.params, kernel::fields(json!({ "id": 1 })), "the params it was started with");
    assert_eq!((is.parent.as_deref(), is.key.as_str()), (Some(parent.as_str()), "kid-1"));
    assert_eq!(rig.shared.count("claimed-otherwise"), 1, "its steps ran again, as the new text has them");
    assert_eq!(shown(&rig, &kid).await.behind, None);
    assert_eq!(live(&rig).await.len(), 2, "and no second one was started beside it");

    // What it runs already is not run again unless that is asked for.
    let same = rig.engine.apply_again(&kid, user(), false).await.unwrap();
    assert!(!same.changed);
    let forced = rig.engine.apply_again(&kid, user(), true).await.unwrap();
    rig.finished(&forced).await;
    assert_eq!(rig.shared.count("claimed-otherwise"), 2);

    // A text that takes other params cannot be given the ones it has.
    rig.library.put("child", &otherwise.replace("params: { id: 0 }", "params: { card: 0 }"));
    let refused = rig.engine.apply_again(&kid, user(), false).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid);
    assert_eq!(
        refused.message,
        "`kid-1` cannot be applied again with the params it was started with: `child` has no param `id`; \
         it takes: card. Nothing was changed. Stop it and let what started it start it again."
    );

    // It needs what applying needs, and that is said before anything of the instance is.
    let may_not = rig.engine.apply_again(&kid, builder(&["board"]), false).await.unwrap_err();
    assert_eq!(may_not.kind, ErrorKind::Denied, "{}", may_not.message);

    // One that is not running is not applied over.
    rig.engine.stop(&kid).await.unwrap();
    let stopped = rig.engine.apply_again(&kid, user(), false).await.unwrap_err();
    assert_eq!(
        stopped.message,
        "`kid-1` was stopped, so there is nothing running to apply `child` over. Apply `child` to start it."
    );
}

#[tokio::test]
async fn what_an_instance_says_of_a_text_it_went_on_with_is_still_said_when_its_steps_settle() {
    let rig = Rig::new().await;
    let worker = "blueprint: worker\nsteps:\n  work: { do: test.count, with: { key: worked } }\n";
    // The name is only known when the step runs, so applying does not see it.
    let caller = "blueprint: caller\nparams: { kind: worker }\nsteps:\n  go: { call: \"${ params.kind }\", as: the-worker }\non:\n  poke:\n    event: test.poke\n    do: noop\n";
    rig.library.put("worker", worker);
    let first = rig.apply(caller, json!({})).await;
    rig.finished(&first).await;
    active(&rig, &first.instance).await;

    rig.library.put("worker", &worker.replace("key: worked", "key: worked, loudly: true"));
    let forced = rig.engine.apply(Apply { force: true, ..rig.request(caller, json!({})) }).await.unwrap();
    assert_eq!(rig.finished(&forced).await.status, RunStatus::Ok);
    let said: Vec<Value> =
        rig.events("instance.call_kept").await.into_iter().map(|event| event.data).collect();
    assert_eq!(said.len(), 1);
    let is = rig.instance(&first.instance).await;
    assert_eq!(is.status, InstanceStatus::Active);
    let note = is.error.expect("it still says so");
    assert!(
        note.starts_with(
            "`worker` cannot be applied here as it is now, so `go` went on with it as it last checked."
        ),
        "{note}"
    );
}

#[tokio::test]
async fn a_run_under_way_is_finished_on_the_text_it_began_with_even_when_the_conductor_was_stopped_meanwhile()
{
    let mut rig = Rig::new().await;
    let held = CHILD.replace(
        "      note: { do: set, with: { seen: ${ state.seen + 1 } } }\n",
        "      wait: { do: test.durable, with: { name: hold } }\n",
    );
    rig.library.put("parent", PARENT);
    rig.library.put("child", &held);
    let applied = rig.apply(PARENT, json!({})).await;
    active(&rig, &applied.instance).await;
    moved(&rig, 1).await;
    handled(&rig, "on.ready", 1).await;
    let kid = keyed(&rig, "kid-1").await;
    active(&rig, &kid.id).await;
    poke(&rig).await;
    rig.reached("hold").await;

    // The text it takes has neither step of the run under way.
    let other = later().replace("      note: { do: set, with: { seen: ${ state.seen + 1 } } }\n", "");
    rig.library.put("child", &other);
    again(&rig).await;
    assert_eq!(rig.instance(&kid.id).await.revision, blueprint::revision(&other));

    rig.restart().await;
    rig.open("hold");
    handled(&rig, "on.poke", 1).await;
    assert_eq!(
        (rig.shared.count("old"), rig.shared.count("new")),
        (1, 0),
        "it went on from where it was, in the text it began with"
    );
    poke(&rig).await;
    handled(&rig, "on.poke", 2).await;
    assert_eq!((rig.shared.count("old"), rig.shared.count("new")), (1, 1), "and the next is the new text's");
}

#[tokio::test]
async fn a_handler_the_text_it_takes_adds_begins_to_listen_and_one_it_drops_stops() {
    let rig = Rig::new().await;
    let (_, kid) = started(&rig).await;
    // The new text hears `test.nudge`, and no longer `test.poke`.
    let other = CHILD
        .replace("  poke:\n    event: test.poke", "  nudge:\n    event: test.nudge")
        .replace("key: old", "key: nudged");
    rig.library.put("child", &other);
    // Said before the text is taken: it is nobody's.
    rig.journal.emit("test.nudge", Source::default(), json!({})).await.unwrap();
    again(&rig).await;
    assert_eq!(rig.instance(&kid).await.revision, blueprint::revision(&other));

    poke(&rig).await;
    rig.journal.emit("test.nudge", Source::default(), json!({})).await.unwrap();
    handled(&rig, "on.nudge", 1).await;
    assert_eq!(
        (rig.shared.count("nudged"), rig.shared.count("old")),
        (1, 0),
        "what was said after it took the text, once; and nothing for the handler that is gone"
    );
    let flows: Vec<String> =
        rig.engine.runs(&kid, 10).await.unwrap().into_iter().map(|run| run.flow).collect();
    assert_eq!(flows, ["on.nudge", "main"]);
}
