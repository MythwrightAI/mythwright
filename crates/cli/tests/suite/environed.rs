//! A project's environments from the terminal: made, given variables and
//! secrets, marked for agents, applied under, read back and taken away.

use crate::Rig;

#[tokio::test]
async fn a_projects_environment_is_made_given_variables_marked_for_agents_and_taken_away_from_the_terminal() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let none = rig.run(&["project", "environment", "shop"]).await.ok();
    assert!(none.contains("`shop` has no environments yet"), "{none}");

    rig.run(&["project", "environment", "shop", "staging", "add"]).await.ok();
    let set = rig
        .run(&[
            "project",
            "environment",
            "shop",
            "staging",
            "set",
            "api=https://staging.example",
            "replicas=3",
        ])
        .await
        .ok();
    assert!(set.contains("api") && set.contains("https://staging.example"), "{set}");
    assert!(set.contains("replicas") && set.contains('3'), "{set}");
    let marked = rig.run(&["project", "environment", "shop", "staging", "agents", "replicas"]).await.ok();
    let line = marked.lines().find(|line| line.contains("replicas")).unwrap_or_default();
    assert!(line.contains("for agents"), "{marked}");
    let listed = rig.run(&["project", "environment", "shop"]).await.ok();
    assert!(
        listed.contains("staging") && listed.contains("2 variables") && listed.contains("1 for agents"),
        "{listed}"
    );
    // Nothing has run under it, and it names no blueprint it is loaded by: the command that names one.
    let unsaid = "nothing has run under it, and no blueprint loads it from a branch: `";
    assert!(
        listed.contains(unsaid) && listed.contains("project environment shop staging deploy <blueprint>`"),
        "{listed}"
    );

    // What it is refused for is the daemon's sentence.
    let refused = rig.run(&["project", "environment", "shop", "staging", "unset", "region"]).await;
    assert_ne!(refused.code, 0);
    assert!(refused.err.contains("has no variable `region`"), "{}", refused.err);

    // A secret of its own, read from a file and never shown; marked for agents.
    let kept = rig.dir.join("stripe.txt");
    std::fs::write(&kept, "sk_live_from_the_terminal").unwrap();
    let of = ["--project", "shop", "--environment", "staging"];
    let file = kept.to_str().unwrap();
    let set = rig.run(&[&["secret", "set", "stripe", "--file", file][..], &of].concat()).await.ok();
    assert!(set.contains("stripe of `staging` of `shop` is set"), "{set}");
    rig.run(&[&["secret", "agents", "stripe", "STRIPE_KEY"][..], &of].concat()).await.ok();
    let listed = rig.run(&[&["secret", "list"][..], &of].concat()).await.ok();
    let line = listed.lines().find(|line| line.contains("stripe")).unwrap_or_default();
    assert!(line.contains("handed to agents as STRIPE_KEY") && line.contains("set "), "{listed}");
    assert!(!listed.contains("sk_live"), "{listed}");
    let factory = rig.run(&["secret", "list"]).await.ok();
    assert!(!factory.contains("stripe"), "not one of the factory's: {factory}");
    let half = rig.run(&["secret", "list", "--project", "shop"]).await;
    assert_ne!(half.code, 0);
    assert!(half.err.contains("named by both"), "{}", half.err);

    // Applied under it, said by name, and listed so; naming none is refused, naming them.
    let text = rig.dir.join("stays.bp");
    let stays = "blueprint: stays\nsteps:\n  one: { do: noop }\non:\n  poke: { event: s.poke, do: noop }\n";
    std::fs::write(&text, stays).unwrap();
    let file = text.to_str().unwrap();
    let refused = rig.run(&["apply", file, "--project", "shop"]).await;
    assert_ne!(refused.code, 0);
    assert!(refused.err.contains("say one of staging (`--environment staging`)"), "{}", refused.err);
    rig.run(&["apply", file, "--project", "shop", "--environment", "staging"]).await.ok();
    let listed = rig.run(&["instances", "--project", "shop"]).await.ok();
    let line = listed.lines().find(|line| line.contains("stays")).unwrap_or_default();
    assert!(line.contains("under staging"), "{listed}");
    // What it last ran is what was applied under it, as the page says it, with when.
    let listed = rig.run(&["project", "environment", "shop"]).await.ok();
    let line = listed.lines().find(|line| line.starts_with("staging")).unwrap_or_default();
    assert!(line.contains("stays, ") && line.contains(", by ") && line.contains(" ago"), "{listed}");
    let one = rig.run(&["project", "environment", "shop", "staging"]).await.ok();
    assert!(one.contains("  last run: stays, "), "{one}");
    let listed = rig.run(&["instances", "--project", "shop"]).await.ok();
    let line = listed.lines().find(|line| line.contains("stays")).unwrap_or_default();
    let busy = rig.run(&["project", "environment", "shop", "staging", "remove"]).await;
    assert_ne!(busy.code, 0);
    assert!(busy.err.contains("`stays` runs under the environment `staging`"), "{}", busy.err);
    let id = line.split_whitespace().next().unwrap().to_string();
    rig.run(&["stop", &id]).await.ok();

    // An agent added by hand says which it works under, and the list says so.
    let refused = rig.run(&["agent", "add", "helper", "--project", "shop"]).await;
    assert!(refused.err.contains("say one of staging"), "{}", refused.err);
    rig.run(&["agent", "add", "helper", "--project", "shop", "--environment", "staging"]).await.ok();
    let listed = rig.run(&["agent", "list"]).await.ok();
    let line = listed.lines().find(|line| line.starts_with("helper")).unwrap_or_default();
    assert!(line.contains("shop under staging"), "{listed}");
    let busy = rig.run(&["project", "environment", "shop", "staging", "remove"]).await;
    // It and the text it is applied as (`std/agent`, keyed by its name).
    assert!(busy.err.contains("`helper` and 1 more run under the environment `staging`"), "{}", busy.err);
    rig.run(&["agent", "remove", "helper"]).await.ok();

    let removed = rig.run(&["project", "environment", "shop", "staging", "remove"]).await.ok();
    assert!(removed.contains("its 2 variables and its one secret go with it, with its value"), "{removed}");
    assert!(rig.run(&["project", "environment", "shop"]).await.ok().contains("no environments yet"));
}

#[tokio::test]
async fn an_environment_says_from_the_terminal_the_blueprint_it_is_loaded_by_and_is_run_for_a_branch_the_project_has()
 {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    rig.run(&["project", "environment", "shop", "dev", "add"]).await.ok();

    let unknown = rig.run(&["project", "environment", "shop", "dev", "deploy", "nowhere"]).await;
    assert_ne!(unknown.code, 0);
    assert!(unknown.err.contains("there is no blueprint `nowhere` for `shop`"), "{}", unknown.err);
    let said = rig.run(&["project", "environment", "shop", "dev", "deploy", "std/board"]).await.ok();
    assert!(said.contains("loaded by std/board"), "{said}");

    // A project the factory made a folder for has no commit yet, so no branch.
    let branches = rig.run(&["project", "branches", "shop"]).await.ok();
    assert!(branches.contains("`shop` has no branch yet"), "{branches}");
    let refused = rig.run(&["project", "environment", "shop", "dev", "run", "main"]).await;
    assert_ne!(refused.code, 0);
    assert!(refused.err.contains("`shop` has no branch `main`"), "{}", refused.err);
    let fetch = rig.run(&["project", "branches", "shop", "--fetch"]).await;
    assert_ne!(fetch.code, 0);
    assert!(fetch.err.contains("has no remote to fetch from"), "{}", fetch.err);

    let none = rig.run(&["project", "environment", "shop", "dev", "deploy", "--none"]).await.ok();
    assert!(!none.contains("loaded by"), "{none}");
    let nothing = rig.run(&["project", "environment", "shop", "dev", "run", "main"]).await;
    assert!(nothing.err.contains("says no blueprint it is loaded by"), "{}", nothing.err);
}
