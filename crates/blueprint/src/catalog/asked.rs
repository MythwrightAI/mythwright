//! What the actions that are the language's own are shown by: each a
//! whole blueprint that asks it, held as every action's example is.

use super::ActionSpec;

/// The language's own actions, each with its example.
pub(super) fn shown(specs: Vec<ActionSpec>) -> Vec<ActionSpec> {
    specs
        .into_iter()
        .map(|spec| match SHOWN.iter().find(|(name, _)| *name == spec.name) {
            Some((_, example)) => spec.example(example),
            None => spec,
        })
        .collect()
}

const SHOWN: &[(&str, &str)] = &[
    ("wait", WAIT),
    ("emit", EMIT),
    ("set", SET),
    ("keep", KEEP),
    ("drop", DROP),
    ("measure", MEASURE),
    ("end", END),
    ("fail", FAIL),
    ("noop", NOOP),
    ("log", LOG),
    ("stop", STOP),
];

pub(crate) const WAIT: &str = "\
blueprint: asked-and-answered

steps:
  asked:
    do: wait
    with:
      event: question.answered
      timeout: 1h
      say: { topic: question.asked, what: Ship today? }

  said:
    do: log
    with: { message: \"Answered after the asking: ${ !steps.asked.out.timed_out }\" }
";

const EMIT: &str = "\
blueprint: built

steps:
  said:
    do: emit
    with: { topic: build.done, version: \"1.2\" }
";

const SET: &str = "\
blueprint: last-build

state:
  last: \"\"

on:
  built:
    event: build.done
    outside: true
    do: set
    with: { last: \"${event.data.version}\" }
";

const KEEP: &str = "\
blueprint: who-came

state:
  here: []

on:
  came:
    event: visitor.came
    outside: true
    do: keep
    with: { in: here, what: \"${event.data.name}\" }
";

const DROP: &str = "\
blueprint: who-left

state:
  here: []

on:
  left:
    event: visitor.left
    outside: true
    do: drop
    with: { from: here, is: \"${event.data.name}\" }
";

const MEASURE: &str = "\
blueprint: checkout

on:
  paid:
    event: order.paid
    outside: true
    do: measure
    with: { name: checkout.done, value: \"${event.data.total}\", labels: { shop: main } }
";

const END: &str = "\
blueprint: maybe-ship

params:
  dry: true

steps:
  enough:
    when: ${params.dry}
    do: end
    with: { reason: Only a dry run. }

  ship:
    do: emit
    with: { topic: release.asked }
";

const FAIL: &str = "\
blueprint: tagged-or-not
permissions: [shell]

steps:
  version:
    do: shell
    with: { cmd: [git, describe, --tags] }
    allow_failure: true

  refused:
    when: ${steps.version.failed}
    do: fail
    with: { message: There is no tag to ship from. }
";

const NOOP: &str = "\
blueprint: both-done
permissions: [shell]

steps:
  docs:
    needs: []
    do: shell
    with: { cmd: [make, docs] }

  tests:
    needs: []
    do: shell
    with: { cmd: [make, test] }

  both:
    needs: [docs, tests]
    description: Where the two come together.
    do: noop
";

const LOG: &str = "\
blueprint: started

steps:
  said:
    do: log
    with: { message: \"Started for ${instance.project}.\" }
";

const STOP: &str = "\
blueprint: news-for-a-while
permissions: [blueprints:run]

steps:
  watch:
    spawn: std/watch
    as: order-watch
    with: { channel: general, collection: orders }

  over:
    do: wait
    with: { for: 1h }

  stopped:
    do: stop
    with: { instance: order-watch, missing_ok: true }
";
