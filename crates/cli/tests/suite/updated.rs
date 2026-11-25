//! `extension update`: the real program asked to take the package that
//! came with it in place of the copy that is installed.
//!
//! The program is a copy in a folder of its own, with a package beside it
//! as an install leaves one, and what is beside it is written again as an
//! upgrade writes it. The package names a program that is nowhere:
//! nothing is run.

use std::path::Path;

use crate::places::{PROGRAM, Started, copied};

fn manifest(wall: &str) -> String {
    format!(
        "extension: sample\ntitle: Sample\ndescription: Not a real agent.\nagent:\n  program: no-such-program-here\n  \
         input: ${{ turn.prompt }}\n  output:\n    - reply: ${{ event.text }}\n  wall:\n    {wall}\n"
    )
}

fn beside(app: &Path, source: &str) {
    std::fs::create_dir_all(app.join("extensions/sample")).unwrap();
    std::fs::write(app.join("extensions/sample/extension.yaml"), source).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_package_that_came_with_the_program_is_taken_in_place_of_the_installed_copy_when_asked() {
    let cli = brand::CLI;
    let (made, app) = copied("updated");
    let earlier = manifest("keeps: [\".sample\"]");
    beside(&app, &earlier);
    let copy = Started::from(&app.join(PROGRAM)).await;
    copy.register().await;
    copy.run(&["extension", "add", "sample"]).await;
    let listed = copy.run(&["extension", "list"]).await;
    assert!(!listed.contains("not the one"), "{listed}");
    let none = copy.run(&["extension", "update", "--all"]).await;
    assert_eq!(none.trim(), "every installed package is already the one that came with this installation");
    // Asked for as JSON, all of them are one list, here with nothing in it.
    assert_eq!(copy.run(&["extension", "update", "--all", "--json"]).await.trim(), "[]");

    // The program is replaced by one whose package says more.
    beside(&app, &manifest("keeps: [\".sample\"]\n    signin: [\".sample/auth\"]"));
    let listed = copy.run(&["extension", "list"]).await;
    assert!(listed.contains("not the one that came with this"), "{listed}");
    assert!(listed.contains(&format!("`{cli} extension update <name>` (or `--all`)")), "{listed}");
    assert!(listed.contains("  sample: the one that came says `agent.wall.signin` besides"), "{listed}");
    let status = copy.run(&["status"]).await;
    assert!(status.contains("not the ones that came with this installation: sample"), "{status}");

    // A name nothing that came has is refused, and nothing is replaced.
    let refused = copy.ran(&["extension", "update", "nonesuch"]).await;
    assert_ne!(refused.code, 0);
    assert!(
        refused.err.contains("no package called `nonesuch` came with this installation"),
        "{}",
        refused.err
    );
    let unnamed = copy.ran(&["extension", "update"]).await;
    assert!(unnamed.code != 0 && unnamed.err.contains("`--all`"), "{}", unnamed.err);

    // Asked, it says what it replaced and where the two differed.
    let taken = copy.run(&["extension", "update", "sample"]).await;
    assert!(taken.contains("`sample` is now the one that came with this installation"), "{taken}");
    assert!(
        taken.contains("It replaced the copy that was installed, whatever was changed in that"),
        "{taken}"
    );
    assert!(taken.contains("`extension.was.yaml`"), "{taken}");
    assert!(
        taken.contains("Against that copy, the one that came says `agent.wall.signin` besides."),
        "{taken}"
    );
    assert!(!copy.run(&["extension", "list"]).await.contains("not the one"));

    // Asked again, and for all of them, there is nothing to take.
    let again = copy.run(&["extension", "update", "sample"]).await;
    assert_eq!(again.trim(), "`sample` is already the one that came with this installation");

    // And every one that differs is taken at a word: here a copy its
    // owner changed, which reads as an earlier one does.
    let changed = manifest("keeps: [\".sample\"]\n    signin: [\".sample/auth\", \".sample/more\"]");
    let file = made.0.join("changed.yaml");
    std::fs::write(&file, changed).unwrap();
    copy.run(&["extension", "add", file.to_str().unwrap()]).await;
    let all = copy.run(&["extension", "update", "--all"]).await;
    assert!(all.contains("`sample` is now the one that came with this installation"), "{all}");
    assert!(all.contains("the one that came says something else of `agent.wall.signin`."), "{all}");
}
