//! What a handler hears where it says no `scope`, as the terminal says
//! it: an apply warns of a handler that will hear nothing and goes
//! ahead, and something a program from before applied is said to hear
//! every project's events still, until it is applied again.

use crate::Rig;

/// Hears a card being moved, and says no `scope`.
const LISTENER: &str = "\
blueprint: listener
on:
  moved:
    event: board.item.moved
    steps:
      said: { do: log, with: { message: moved } }
";

#[tokio::test]
async fn an_apply_for_no_project_warns_of_a_handler_that_will_hear_nothing_and_for_its_project_says_nothing()
{
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let blueprints = rig.placed("shop");
    std::fs::create_dir_all(&blueprints).unwrap();
    let file = blueprints.join(brand::blueprint_file("listener"));
    std::fs::write(&file, LISTENER).unwrap();
    let file = file.to_str().unwrap();

    // For its project there is nothing to say.
    let applied = rig.run(&["apply", file, "--project", "shop"]).await;
    assert!(applied.code == 0 && !applied.err.contains("warning:"), "{}", applied.err);

    // For no project the apply goes ahead, and says what will not be
    // heard, where the handler is written, and the two ways to put it right.
    let applied = rig.run(&["apply", file]).await;
    assert!(applied.code == 0, "{}", applied.err);
    let warned: Vec<&str> = applied.err.lines().filter(|line| line.starts_with("warning:")).collect();
    assert_eq!(warned.len(), 1, "{}", applied.err);
    assert!(
        warned[0].starts_with(
            "warning: line 3: this is applied for no project, and `board.item.moved`, which `moved` listens \
             for, is always some project's."
        ) && warned[0].contains("(`--project`)")
            && warned[0].contains("`scope: fleet`"),
        "{}",
        warned[0]
    );
}

#[tokio::test]
async fn what_a_program_from_before_applied_is_said_to_hear_every_projects_events_until_it_is_applied_again()
{
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let blueprints = rig.placed("shop");
    std::fs::create_dir_all(&blueprints).unwrap();
    let file = blueprints.join(brand::blueprint_file("listener"));
    std::fs::write(&file, LISTENER).unwrap();
    let file = file.to_str().unwrap();
    rig.run(&["apply", file, "--project", "shop"]).await.ok();

    // Applied by this program, nothing is said of how it hears.
    let listed = rig.run(&["instances"]).await.ok();
    assert!(listed.contains("listener") && !listed.contains("every project's"), "{listed}");
    let id =
        listed.lines().find(|line| line.contains("listener")).unwrap().split_whitespace().next().unwrap();
    let id = id.to_string();

    // As a program from before left it: its row says nothing of how it
    // hears, which is how such a one is known.
    // In whichever database keeps it: its project's own, where the
    // project has a journal of its own.
    let journal = &rig.node.as_ref().unwrap().app.journal;
    let mut changed = 0;
    for db in journal.dbs() {
        let of = id.clone();
        let done = db.call(move |connection| {
            connection.execute("UPDATE instances SET heard = NULL WHERE id = ?1", [&of])
        });
        changed += done.await.unwrap_or(0);
    }
    assert_eq!(changed, 1, "the instance's row was not found");
    let says = "every project's events where it says no `scope`, as when it was applied; applied again \
                (`--force` for the same text) it hears its own project's and the factory's own";
    let listed = rig.run(&["instances"]).await.ok();
    assert_eq!(listed.lines().last(), Some(format!("{id}: 1 hear {says}").as_str()), "{listed}");
    let one = rig.run(&["instance", &id]).await.ok();
    assert!(one.lines().any(|line| line == format!("  hears {says}")), "{one}");

    // Applied again it is as anything this program applies.
    rig.run(&["apply", file, "--project", "shop", "--force"]).await.ok();
    let listed = rig.run(&["instances"]).await.ok();
    assert!(listed.contains("listener") && !listed.contains("every project's"), "{listed}");
    assert!(!rig.run(&["instance", &id]).await.ok().contains("  hears "));
}
