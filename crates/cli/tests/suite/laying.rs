//! `fmt`: blueprints laid out from the terminal, with no daemon to ask
//! and nobody signed in.

use std::path::{Path, PathBuf};

use blueprint::format::SETTINGS_FILE;

use crate::{run_in, scratch};

/// Nothing answers here, and `fmt` never asks.
const NOWHERE: &str = "http://127.0.0.1:1";

fn bp(stem: &str) -> String {
    brand::blueprint_file(stem)
}

fn write(file: &Path, text: &str) {
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, text).unwrap();
}

fn read(file: &Path) -> String {
    std::fs::read_to_string(file).unwrap()
}

const WORKER: &str = "\
blueprint: worker
steps:
    claim: {do: emit, with: {topic: shop.claimed, room: \"${params.room}\", told: \"said so\"}}
";

#[tokio::test(flavor = "multi_thread")]
async fn the_blueprints_that_ship_are_laid_out_the_one_way() {
    let top: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let dir = scratch("shipped");
    let (std, examples) = (top.join("blueprints/std"), top.join("examples"));
    let ran =
        run_in(&dir, NOWHERE, &[], &["fmt", "--check", std.to_str().unwrap(), examples.to_str().unwrap()])
            .await;
    assert_eq!(
        ran.code,
        0,
        "a blueprint that ships is not laid out as `{cli} fmt` lays blueprints out. From the top of the \
         repository run `cargo run -q -p cli -- fmt blueprints/std examples` (that is `{cli} fmt` as this \
         checkout builds it), and read what it changed.\n{}{}",
        ran.out,
        ran.err,
        cli = brand::CLI
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn fmt_lays_out_what_it_is_given_as_the_nearest_settings_say_and_check_only_says_what_would_change() {
    let dir = scratch("fmt");
    // A repository, with a settings file above it that is nobody's here.
    let shop = dir.join("shop");
    std::fs::create_dir_all(shop.join(".git")).unwrap();
    write(&dir.join(SETTINGS_FILE), "indent: 8\n");
    write(&shop.join(SETTINGS_FILE), "width: 60\nexpressions: spaced\n");
    let worker = shop.join("blueprints").join(bp("worker"));
    let deep = shop.join("blueprints/more").join(bp("deep"));
    let broken = shop.join("blueprints").join(bp("broken"));
    write(&worker, WORKER);
    write(&deep, &WORKER.replace("worker", "deep"));
    write(&shop.join("blueprints/more").join(SETTINGS_FILE), "indent: 4\n");
    write(&broken, "blueprint: broken\n  steps: oops\n");
    // Other tools' folders, and a lane's worktree, are not gone into.
    for theirs in [".lanes/shop-1/blueprints", "target", "node_modules/pkg"] {
        write(&shop.join(theirs).join(bp("left")), WORKER);
    }
    let shop_arg = shop.to_str().unwrap();

    let checked = run_in(&dir, NOWHERE, &[], &["fmt", "--check", shop_arg]).await;
    assert_eq!(checked.code, 1, "{}{}", checked.out, checked.err);
    assert_eq!(checked.out.matches("would change").count(), 2, "{}", checked.out);
    assert!(checked.out.contains(&bp("worker")) && checked.out.contains(&bp("deep")), "{}", checked.out);
    assert!(
        checked.err.contains(&format!("{}:2:3: unexpected indentation", broken.display())),
        "{}",
        checked.err
    );
    assert!(
        checked.err.contains(&format!("run `{} fmt {shop_arg}` to rewrite them", brand::CLI)),
        "it says how to lay them out: {}",
        checked.err
    );
    assert_eq!(read(&worker), WORKER, "`--check` rewrites nothing");

    let laid = run_in(&dir, NOWHERE, &[], &["fmt", shop_arg]).await;
    assert_eq!(laid.code, 1, "one of them does not read: {}{}", laid.out, laid.err);
    assert_eq!(laid.out.lines().filter(|line| line.starts_with("laid out ")).count(), 2, "{}", laid.out);
    assert!(laid.out.contains("3 blueprints looked at, 2 laid out again"), "{}", laid.out);
    // The project's own settings: narrower, and a space each side.
    assert_eq!(
        read(&worker),
        "blueprint: worker\n\nsteps:\n  claim:\n    do: emit\n    with:\n      topic: shop.claimed\n      room: \"${ params.room }\"\n      told: \"said so\"\n"
    );
    // The nearest settings are the ones that count, by themselves.
    assert_eq!(
        read(&deep),
        "blueprint: deep\n\nsteps:\n    claim: { do: emit, with: { topic: shop.claimed, room: \"${params.room}\", told: \"said so\" } }\n"
    );
    assert_eq!(
        read(&broken),
        "blueprint: broken\n  steps: oops\n",
        "one that does not read is left as it is"
    );
    assert_eq!(read(&shop.join("target").join(bp("left"))), WORKER);
    assert_eq!(read(&shop.join(".lanes/shop-1/blueprints").join(bp("left"))), WORKER);

    std::fs::remove_file(&broken).unwrap();
    let settled = run_in(&dir, NOWHERE, &[], &["fmt", "--check", shop_arg]).await;
    assert_eq!(settled.code, 0, "{}{}", settled.out, settled.err);
    assert!(!settled.out.contains("would change"), "{}", settled.out);

    // One file by name; and a setting that is not one is said with its file.
    let one = run_in(&dir, NOWHERE, &[], &["fmt", worker.to_str().unwrap()]).await.ok();
    assert!(one.contains("1 blueprint looked at, 0 laid out again"), "{one}");
    write(&shop.join("blueprints/more").join(SETTINGS_FILE), "tabs: true\n");
    let refused = run_in(&dir, NOWHERE, &[], &["fmt", shop_arg]).await;
    assert_eq!(refused.code, 1);
    assert!(
        refused.err.contains(SETTINGS_FILE) && refused.err.contains("`tabs` is not something the layout"),
        "{}",
        refused.err
    );
    let missing = run_in(&dir, NOWHERE, &[], &["fmt", dir.join("nothing").to_str().unwrap()]).await;
    assert_eq!(missing.code, 1);
    assert!(missing.err.contains("there is no file or folder"), "{}", missing.err);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn fmt_takes_a_blueprint_whose_file_is_called_as_yaml_and_leaves_other_yaml_alone() {
    let dir = scratch("fmt-yaml");
    let shop = dir.join("shop");
    let (ours, also) = (shop.join("worker.bp.yml"), shop.join("blueprints/deep.bp.yaml"));
    // A project is full of YAML that is not a blueprint, whatever it says inside.
    let theirs = shop.join("compose.yml");
    write(&ours, WORKER);
    write(&also, &WORKER.replace("worker", "deep"));
    write(&theirs, WORKER);

    let laid = run_in(&dir, NOWHERE, &[], &["fmt", shop.to_str().unwrap()]).await;
    assert_eq!(laid.code, 0, "{}{}", laid.out, laid.err);
    assert!(laid.out.contains("2 blueprints looked at, 2 laid out again"), "{}", laid.out);
    assert!(read(&ours).starts_with("blueprint: worker\n\nsteps:\n  claim:"), "{}", read(&ours));
    assert!(read(&also).starts_with("blueprint: deep\n\nsteps:\n  claim:"), "{}", read(&also));
    assert_eq!(read(&theirs), WORKER, "a file that is YAML and no more is not ours to lay out");
    // Named by itself it is laid out as any file that is named is.
    let one = run_in(&dir, NOWHERE, &[], &["fmt", ours.to_str().unwrap()]).await.ok();
    assert!(one.contains("1 blueprint looked at, 0 laid out again"), "{one}");
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn check_says_of_a_file_called_as_yaml_what_an_editor_would_mark_and_fmt_puts_it_right() {
    let dir = scratch("marked");
    let text = "\
blueprint: marked
description: A board: its cards
steps:
  say: { do: emit, with: { topic: shop.said, data: { rounds: ${ 1 + 1 } } } }
";
    let (as_yaml, plain) = (dir.join("marked.bp.yml"), dir.join(bp("marked")));
    write(&as_yaml, text);
    write(&plain, text);

    // Called as YAML, it is a file an editor reads as YAML: each value it
    // would mark is said with its line and with what puts it right. They
    // are warnings, and the file checks.
    let named = as_yaml.to_str().unwrap();
    let checked = run_in(&dir, NOWHERE, &[], &["check", named]).await;
    assert_eq!(checked.code, 0, "{}{}", checked.out, checked.err);
    let said: Vec<&str> = checked.out.lines().collect();
    assert_eq!(said.len(), 3, "{}", checked.out);
    assert!(said[0].starts_with(&format!("{named}:2:14: warning: this is written plain, and YAML does not")));
    assert!(said[0].contains(&format!("`{} fmt {named}` puts it in quotes", brand::CLI)), "{}", said[0]);
    assert!(said[1].starts_with(&format!("{named}:4:62: warning: ")), "{}", said[1]);
    assert_eq!(said[2], format!("{named}: ok"));

    // Called `.bp`, the same text is no editor's YAML until somebody says
    // so, and nothing is said of it.
    let checked = run_in(&dir, NOWHERE, &[], &["check", plain.to_str().unwrap()]).await;
    assert_eq!(checked.out.trim(), format!("{}: ok", plain.display()), "{}", checked.out);

    // Laid out, it is in quotes, and there is nothing left to say.
    assert_eq!(run_in(&dir, NOWHERE, &[], &["fmt", named]).await.code, 0);
    let laid = read(&as_yaml);
    assert!(
        laid.contains("description: \"A board: its cards\"") && laid.contains("rounds: \"${ 1 + 1 }\""),
        "{laid}"
    );
    let checked = run_in(&dir, NOWHERE, &[], &["check", named]).await;
    assert_eq!(checked.out.trim(), format!("{named}: ok"), "{}", checked.out);
    let _ = std::fs::remove_dir_all(dir);
}
