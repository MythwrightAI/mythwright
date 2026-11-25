//! How a project's things are arranged in the side panel, said and
//! changed from a terminal (`project outline`).

use crate::Rig;

#[tokio::test(flavor = "multi_thread")]
async fn a_projects_things_are_put_in_folders_by_their_names_from_a_terminal_and_the_arrangement_forgotten() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let usual = rig.run(&["project", "outline", "shop"]).await.ok();
    assert!(usual.contains("shown as usual"), "{usual}");

    // Each folder of the path that is not there is made, and what is put goes into the last.
    let put = rig
        .run(&[
            "project",
            "outline",
            "shop",
            "--folder",
            "Insights/Runs",
            "--put",
            "repo",
            "blueprints",
            "--in",
            "insights/runs",
        ])
        .await
        .ok();
    assert!(
        put.contains("made the folder `Insights`") && put.contains("made the folder `Insights/Runs`"),
        "{put}"
    );
    assert!(put.contains("  Insights/  (f1)\n    Runs/  (f2)\n      repo\n      blueprints"), "{put}");
    // Made again, nothing is made twice.
    let again = rig.run(&["project", "outline", "shop", "--folder", "Insights"]).await.ok();
    assert!(!again.contains("made the folder"), "{again}");

    let renamed =
        rig.run(&["project", "outline", "shop", "--rename", "Insights/Runs", "--to", "Settings"]).await.ok();
    assert!(renamed.contains("    Settings/  (f2)"), "{renamed}");
    let removed = rig.run(&["project", "outline", "shop", "--remove", "Insights"]).await.ok();
    assert!(removed.contains("\n  Settings/  (f2)\n    repo"), "{removed}");

    // What the project does not have is not put anywhere, and nothing is written.
    let refused = rig.run(&["project", "outline", "shop", "--put", "board"]).await;
    assert_ne!(refused.code, 0);
    assert!(refused.err.contains("`board` is nothing of `shop`'s"), "{}", refused.err);
    // Nor is a folder put in one that is not there.
    let nowhere = rig.run(&["project", "outline", "shop", "--put", "repo", "--in", "Nowhere"]).await;
    assert!(nowhere.err.contains("no folder `Nowhere`"), "{}", nowhere.err);

    // A thing is shown as something else for everybody, by its key, and as it always was again.
    let named = rig.run(&["project", "outline", "shop", "--name", "repo", "--to", "Code"]).await.ok();
    assert!(
        named.contains("showed `repo` as Code") && named.contains("    repo  (shown as Code)"),
        "{named}"
    );
    let usual = rig.run(&["project", "outline", "shop", "--name", "repo", "--to", ""]).await.ok();
    assert!(usual.contains("showed `repo` as it always was") && usual.contains("    repo\n"), "{usual}");
    let alone = rig.run(&["project", "outline", "shop", "--name", "repo"]).await;
    assert!(alone.err.contains("`--name <thing> --to <name>`"), "{}", alone.err);

    let forgot = rig.run(&["project", "outline", "shop", "--forget"]).await.ok();
    assert!(forgot.contains("forgot how it was arranged") && forgot.contains("shown as usual"), "{forgot}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_project_is_called_something_else_from_a_terminal_its_id_as_it_was() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    rig.run(&["project", "create", "--bare", "Depot"]).await.ok();
    let renamed = rig.run(&["project", "rename", "shop", "Shop", "front"]).await.ok();
    assert!(renamed.contains("`shop` is called Shop front now"), "{renamed}");
    let listed = rig.run(&["project", "list"]).await.ok();
    assert!(listed.contains("Shop front"), "{listed}");
    let refused = rig.run(&["project", "rename", "shop", "DEPOT"]).await;
    assert_ne!(refused.code, 0);
    assert!(refused.err.contains("another project is called DEPOT"), "{}", refused.err);
}

const RUNS: &str = "blueprint: runs
permissions: [projects]
steps:
  runs:
    do: collection.ensure
    with:
      name: test-runs
      title: Test runs
      fields: { title: { type: text, required: true } }
";

#[tokio::test(flavor = "multi_thread")]
async fn a_table_is_called_something_else_from_a_terminal_and_back_its_name_as_it_was() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let file = rig.dir.join("runs.bp");
    std::fs::write(&file, RUNS).unwrap();
    rig.run(&["apply", file.to_str().unwrap(), "--project", "shop"]).await.ok();
    let called = rig.run(&["record", "title", "shop", "test-runs", "Runs", "of", "tests"]).await.ok();
    assert!(called.contains("`test-runs` of `shop` is called Runs of tests now"), "{called}");
    // Called what its blueprint calls it again.
    let usual = rig.run(&["record", "title", "shop", "test-runs", "--usual"]).await.ok();
    assert!(usual.contains("is called Test runs now"), "{usual}");
    // Saying both, or neither, is refused before anything is asked; a board keeps its words.
    let both = rig.run(&["record", "title", "shop", "test-runs", "Runs", "--usual"]).await;
    assert!(both.code != 0 && both.err.contains("not both"), "{}", both.err);
    rig.run(&["apply", "std/board", "--project", "shop"]).await.ok();
    let board = rig.run(&["record", "title", "shop", "board", "Cards"]).await;
    assert!(board.code != 0 && board.err.contains("called what they are"), "{}", board.err);
}
