//! The variables of a profile, as a person says them on the command
//! line and a blueprint reads them: one file, two factories.
//!
//! Every command here says which factory it means, by `--profile`.
//!
//! Unix's alone: its factories are the real program as processes of their
//! own (`profiles::Home`), each stopped by a signal.
#![cfg(unix)]

use crate::profiles::{Home, own_address};

/// The outputs line of what `apply` printed.
fn outputs(applied: &str) -> &str {
    applied
        .lines()
        .find(|line| line.starts_with("outputs: "))
        .unwrap_or_else(|| panic!("no outputs: {applied}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn one_blueprint_reads_what_the_owner_of_each_factory_said_of_its_profile() {
    let home = Home::new("vars");
    // `prod` is stopped and started again where it was, so where that
    // is, is the test's own to say.
    let (_own, listen) = own_address();
    let mut prod = home.start(&["--profile", "prod", "--listen", &listen]).await;
    let mut dev = home.start(&["--profile", "dev1"]).await;
    home.register("prod", "Ada").await;
    home.register("dev1", "Ada").await;

    // One file, not edited for either.
    let file = home.0.join("reads.bp");
    std::fs::write(
        &file,
        "blueprint: reads\npermissions: []\nsteps:\n  read: { do: noop }\noutputs:\n  \
         api: \"${ profile.vars.api ?? 'http://localhost' }\"\n  \
         replicas: ${ profile.vars.replicas ?? 1 }\n  where: \"${ profile.name }\"\n",
    )
    .unwrap();
    let file = file.to_str().unwrap();
    let home = &home;
    let read = |env: &'static str| async move { home.run(&["--profile", env, "apply", file]).await.ok() };
    // Nothing has been said of either: each name is nothing, and `??`
    // gives what to use instead.
    assert_eq!(
        outputs(&read("prod").await),
        r#"outputs: {"api":"http://localhost","replicas":1,"where":"prod"}"#
    );

    // The variables are one param, a map, given as JSON.
    let said = home
        .run(&[
            "--profile",
            "prod",
            "apply",
            "std/profile",
            "--param",
            r#"vars={"api":"https://api.example","replicas":3}"#,
        ])
        .await
        .ok();
    assert!(said.contains(" ok in ") && said.contains(r#""set":["api","replicas"]"#), "{said}");
    home.run(&[
        "--profile",
        "dev1",
        "apply",
        "std/profile",
        "--param",
        r#"vars={"api":"http://127.0.0.1:8080"}"#,
    ])
    .await
    .ok();
    assert_eq!(
        outputs(&read("prod").await),
        r#"outputs: {"api":"https://api.example","replicas":3,"where":"prod"}"#
    );
    assert_eq!(
        outputs(&read("dev1").await),
        r#"outputs: {"api":"http://127.0.0.1:8080","replicas":1,"where":"dev1"}"#
    );

    // A value that is not plain is refused in a sentence, and what was
    // said stands.
    let listed = home
        .run(&["--profile", "prod", "apply", "std/profile", "--param", r#"vars={"hosts":["a","b"]}"#])
        .await;
    assert_eq!(listed.code, 1, "{}", listed.out);
    assert!(
        listed.err.contains(
            "the variable `hosts` is a list, and a variable is plain: text, a number, or true or false"
        ),
        "{}",
        listed.err
    );
    // And a map is what the param is: anything else is not taken for one.
    let text = home.run(&["--profile", "prod", "apply", "std/profile", "--param", "vars=api"]).await;
    assert_eq!(text.code, 1, "{}", text.out);
    assert!(text.err.contains("param `vars` of `std/profile` must be object"), "{}", text.err);

    // They are the factory's: there after its daemon has been stopped and
    // started again, with nothing left running to hold them.
    assert_eq!(home.run(&["--profile", "prod", "instances"]).await.ok().trim(), "nothing is running");
    prod.stop().await;
    let mut prod = home.start(&["--profile", "prod"]).await;
    assert_eq!(
        outputs(&read("prod").await),
        r#"outputs: {"api":"https://api.example","replicas":3,"where":"prod"}"#
    );
    let log = home.run(&["--profile", "prod", "events", "--topics", "system.profile"]).await.ok();
    assert_eq!(log.lines().filter(|line| line.contains("system.profile")).count(), 1, "{log}");

    // Applied with none, there are none.
    home.run(&["--profile", "prod", "apply", "std/profile"]).await.ok();
    assert_eq!(
        outputs(&read("prod").await),
        r#"outputs: {"api":"http://localhost","replicas":1,"where":"prod"}"#
    );
    prod.stop().await;
    dev.stop().await;
}
