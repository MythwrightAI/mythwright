//! A project's files from a terminal (`file write`, `read`, `remove`,
//! `mkdir`), with
//! the real program: byte for byte, as whoever is signed in or as an agent
//! by `--as`, held as the routes hold anybody, and into the Trash.

use std::path::Path;
use std::process::Stdio;

use tokio::io::AsyncWriteExt;

use crate::external::{handed, invitation};
use crate::{Ran, Rig, command_in, scratch, soon};

/// A picture of one pixel: bytes that are not text (the first is 0x89),
/// a nought among them, and a line's end that a reading as text would turn.
const PIXEL: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00,
    0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0xF0, 0x1F, 0x00, 0x05,
    0x00, 0x01, 0xFF, 0x89, 0x99, 0x3D, 0x1D, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42,
    0x60, 0x82,
];

/// Runs the program in the home `dir`, handed `bytes` on standard input
/// as they are, and gives back what it printed as it is.
async fn fed(dir: &Path, url: &str, args: &[&str], bytes: &[u8]) -> (Ran, Vec<u8>) {
    let mut command = command_in(dir, url, args);
    command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    input.write_all(bytes).await.unwrap();
    drop(input);
    let output = soon(&format!("`{}`", args.join(" ")), child.wait_with_output()).await.unwrap();
    let ran = Ran {
        code: output.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&output.stdout).into_owned(),
        err: String::from_utf8_lossy(&output.stderr).into_owned(),
    };
    (ran, output.stdout)
}

/// What a file read printed, as it is.
async fn read(dir: &Path, url: &str, args: &[&str]) -> (Ran, Vec<u8>) {
    let mut command = command_in(dir, url, args);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = soon(&format!("`{}`", args.join(" ")), command.output()).await.unwrap();
    let ran = Ran {
        code: output.status.code().unwrap_or(-1),
        out: String::new(),
        err: String::from_utf8_lossy(&output.stderr).into_owned(),
    };
    (ran, output.stdout)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_picture_is_written_read_back_byte_for_byte_and_removed_into_the_trash_from_a_terminal() {
    let rig = Rig::signed_in().await;
    let cli = brand::CLI;
    rig.run(&["project", "create", "shop", "--bare"]).await.ok();

    // From standard input, into a folder that is not there yet.
    let (wrote, _) = fed(&rig.dir, &rig.url, &["file", "write", "shop", "art/logo.png"], PIXEL).await;
    assert_eq!(wrote.code, 0, "{}{}", wrote.out, wrote.err);
    assert_eq!(wrote.out.trim(), format!("wrote `art/logo.png` in shop ({} bytes)", PIXEL.len()));
    assert_eq!(
        std::fs::read(rig.project("shop").join("art/logo.png")).unwrap(),
        PIXEL,
        "on the disk as it was"
    );
    let (ran, back) = read(&rig.dir, &rig.url, &["file", "read", "shop", "art/logo.png"]).await;
    assert_eq!(ran.code, 0, "{}", ran.err);
    assert_eq!(back, PIXEL, "read back byte for byte");
    assert!(rig.run(&["project", "tree", "shop"]).await.ok().lines().any(|line| line == "art/logo.png"));

    // From a file, over what is there; and text read back as text.
    let note = rig.dir.join("note.md");
    std::fs::write(&note, "# Opening hours\n\nNine till five.\n").unwrap();
    let from = note.to_str().unwrap();
    rig.run(&["file", "write", "shop", "notes/hours.md", "--from", from]).await.ok();
    std::fs::write(&note, "# Opening hours\n\nTen till six.\n").unwrap();
    rig.run(&["file", "write", "shop", "notes/hours.md", "--from", from]).await.ok();
    assert_eq!(
        rig.run(&["file", "read", "shop", "notes/hours.md"]).await.ok(),
        "# Opening hours\n\nTen till six.\n"
    );
    // With nothing to write from, nothing is written: a terminal is no file.
    let none = rig.run(&["file", "write", "shop", "notes/empty.md", "--from", "/nowhere/at/all"]).await;
    assert!(none.code != 0 && none.err.contains("`/nowhere/at/all` could not be read"), "{}", none.err);
    assert!(!rig.project("shop").join("notes/empty.md").exists());

    // Removed, it is in the Trash, which says how to give it back.
    let gone = rig.run(&["file", "remove", "shop", "art/logo.png"]).await.ok();
    assert!(gone.starts_with("moved `art/logo.png` of shop to the Trash as `"), "{gone}");
    assert!(
        gone.contains("it is kept 30 days") && gone.contains(&format!("`{cli} trash restore ")),
        "{gone}"
    );
    assert!(!rig.project("shop").join("art/logo.png").exists());
    let trash = gone.split('`').nth(3).unwrap().to_string();
    rig.run(&["trash", "restore", &trash]).await.ok();
    let (_, back) = read(&rig.dir, &rig.url, &["file", "read", "shop", "art/logo.png"]).await;
    assert_eq!(back, PIXEL, "given back as it was");

    // What is not there is said in the daemon's words.
    let missing = rig.run(&["file", "read", "shop", "art/none.png"]).await;
    assert!(missing.code != 0 && missing.err.contains("there is no file `art/none.png`"), "{}", missing.err);
    // A folder made empty, and a name that is there refused in the daemon's words.
    let made = rig.run(&["file", "mkdir", "shop", "notes/2026"]).await.ok();
    assert_eq!(made.trim(), "made the folder `notes/2026` in shop");
    assert!(rig.project("shop").join("notes/2026").is_dir());
    let again = rig.run(&["file", "mkdir", "shop", "notes/2026"]).await;
    assert!(again.code != 0 && again.err.contains("there is a folder called `notes/2026`"), "{}", again.err);
    let help = rig.run(&["help", "file"]).await.ok();
    assert!(help.starts_with(&format!("{cli} file write <project> <path> [--from FILE]")), "{help}");
}

/// A library kept private beside one that is not, as `private.rs` of the API suite has it.
const NOTES: &str = "blueprint: notes
permissions: [projects, records]
steps:
  days:
    do: collection.ensure
    with:
      name: days
      store: documents
      at: notes/days
      reads: private:journal
      fields: { title: { type: text, required: true }, body: { type: text }, path: { type: text } }
";

#[tokio::test(flavor = "multi_thread")]
async fn a_key_that_holds_less_is_refused_a_projects_files_as_the_routes_refuse_it() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "Life", "--bare"]).await.ok();
    let file = rig.dir.join("notes.bp");
    std::fs::write(&file, NOTES).unwrap();
    rig.run(&["apply", file.to_str().unwrap(), "--project", "life"]).await.ok();
    let (wrote, _) = fed(
        &rig.dir,
        &rig.url,
        &["file", "write", "life", "notes/days/sat.md"],
        b"# Saturday\n\nMarmalade.\n",
    )
    .await;
    assert_eq!(wrote.code, 0, "{}", wrote.err);
    rig.run(&["file", "write", "life", "open.md", "--from", file.to_str().unwrap()]).await.ok();

    // One that may read the project's files and not write them.
    let made = rig.run(&["key", "reader", "--scope", "projects:read"]).await.ok();
    let reader = made.lines().next().unwrap().to_string();
    let (refused, _) =
        fed(&rig.dir, &rig.url, &["--token", &reader, "file", "write", "life", "open.md"], b"over it").await;
    assert!(refused.code != 0 && refused.err.contains("`projects:write`"), "{}", refused.err);
    let taken = rig.run(&["--token", &reader, "file", "remove", "life", "open.md"]).await;
    assert!(taken.code != 0 && taken.err.contains("`projects:write`"), "{}", taken.err);
    assert_eq!(rig.run(&["--token", &reader, "file", "read", "life", "open.md"]).await.ok(), NOTES);

    // One that may write them, and does not hold what the days are kept under.
    let made = rig.run(&["key", "helper", "--scope", "projects", "--scope", "records"]).await.ok();
    let helper = made.lines().next().unwrap().to_string();
    let day = ["file", "read", "life", "notes/days/sat.md"];
    let kept = |ran: &Ran, what: &str| {
        assert!(ran.code != 0, "{what}: {}", ran.out);
        assert!(
            ran.err.contains("kept private") && ran.err.contains("`private:journal`"),
            "{what}: {}",
            ran.err
        );
    };
    kept(&rig.run(&[&["--token", helper.as_str()][..], &day[..]].concat()).await, "read");
    let (over, _) =
        fed(&rig.dir, &rig.url, &["--token", &helper, "file", "write", "life", "notes/days/sat.md"], b"x")
            .await;
    kept(&over, "written over");
    kept(&rig.run(&["--token", &helper, "file", "remove", "life", "notes/days/sat.md"]).await, "removed");
    assert_eq!(rig.run(&day).await.ok(), "# Saturday\n\nMarmalade.\n", "as its owner wrote it");
    // What is not kept, it writes.
    rig.run(&["--token", &helper, "file", "write", "life", "open.md", "--from", file.to_str().unwrap()])
        .await
        .ok();
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_writes_a_document_into_a_library_with_its_own_credential() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "Shop", "--bare"]).await.ok();
    rig.run(&["apply", "std/docs", "--project", "shop"]).await.ok();
    // An agent this factory does not start, of no project, holding what one usually does.
    let printed = rig.run(&["agent", "invite", "writer"]).await.ok();
    let home = scratch("writer");
    let joined = handed(&home, &rig.url, &["agent", "join"], &invitation(&printed)).await;
    assert_eq!(joined.code, 0, "{}{}", joined.out, joined.err);

    let before = crate::fleets::now(&rig);
    let page = b"# Hello\n\nWhat we sell, and why.\n";
    let (wrote, _) =
        fed(&home, &rig.url, &["--as", "writer", "file", "write", "shop", "docs/guides/hello.md"], page)
            .await;
    assert_eq!(wrote.code, 0, "{}{}", wrote.out, wrote.err);
    // A document of the library, said to have been written by the agent.
    let made = crate::fleets::event_after(&rig, before, "record.created", |event| {
        event.data["id"] == "shop-docs:guides:hello"
    })
    .await;
    assert_eq!(made.source.actor.as_deref(), Some("agent:writer"), "{made:?}");
    let listed = rig.run(&["record", "list", "shop", "docs"]).await.ok();
    assert!(listed.lines().any(|line| line.starts_with("shop-docs:guides:hello")), "{listed}");
    let read =
        run_as(&home, &rig.url, &["--as", "writer", "file", "read", "shop", "docs/guides/hello.md"]).await;
    assert_eq!(read.as_bytes(), page);
}

async fn run_as(dir: &Path, url: &str, args: &[&str]) -> String {
    crate::run_in(dir, url, &[], args).await.ok()
}
