//! The examples the words of the language are shown by: each a whole
//! blueprint, short enough to read at a glance, written with the word it
//! is the example of and with as little else as it can be.
//!
//! A word names its example where it is said (`.example(shown::FETCH)`),
//! and several words that are best seen together share one: `retry` and
//! the four words inside it are one step that is tried again. Each is held
//! to reading, to checking with no warning against every action there is,
//! and to being laid out as the formatter lays it out, so that one copied
//! from the page can be applied as it stands. They name no service
//! anywhere outside the machine, and say plainly what they do, as the
//! blueprints that ship do.

pub const NIGHTLY: &str = "\
blueprint: nightly-copy
description: Copy the database somewhere safe every night.
permissions: [shell]

on:
  night:
    every: 24h
    do: shell
    with: { cmd: [cp, data.db, backup/data.db] }
";

pub const VERSIONED: &str = "\
blueprint: greeting
version: 2
description: Say hello in the run's log.

steps:
  greet:
    do: log
    with: { message: Hello. }
";

pub const GREETING: &str = "\
blueprint: greeting

params:
  who: { type: string, default: everyone, description: Who is greeted. }

steps:
  greet:
    do: log
    with: { message: \"Hello, ${params.who}.\" }
";

pub const BUILD_PARAMS: &str = "\
blueprint: build
permissions: [shell]

params:
  profile: { type: string, default: release, enum: [debug, release] }
  target: { type: string, required: true, description: The system it is built for. }

steps:
  build:
    do: shell
    with: { cmd: [cargo, build, --profile, \"${params.profile}\", --target, \"${params.target}\"] }
";

pub const LAST_SEEN: &str = "\
blueprint: last-seen

state:
  last: \"\"

on:
  seen:
    event: visitor.seen
    outside: true
    do: set
    with: { last: \"${event.data.name}\" }
";

pub const DISK: &str = "\
blueprint: disk-room
permissions: [shell]

steps:
  room:
    do: shell
    with: { cmd: [df, -h, .] }
";

pub const BUILD_ENV: &str = "\
blueprint: build
permissions: [shell]

env:
  CARGO_TERM_COLOR: never

steps:
  build:
    do: shell
    with: { cmd: [cargo, build] }
";

pub const STEP_ENV: &str = "\
blueprint: build
permissions: [shell]

steps:
  build:
    do: shell
    with: { cmd: [make] }
    env: { RELEASE: \"1\" }
";

pub const ATTACHED: &str = "\
blueprint: order-news

uses:
  watch:
    blueprint: std/watch
    with: { channel: general, collection: orders }
";

pub const FETCH: &str = "\
blueprint: health
permissions: [shell]

steps:
  fetch:
    do: shell
    with: { cmd: [curl, -fsS, \"http://localhost:8080/health\"] }
    retry: { max: 3, backoff: 2s, factor: 2, max_backoff: 30s }
";

pub const DONE_NEWS: &str = "\
blueprint: shipped-news
permissions: [channels:write]

on:
  shipped:
    event: order.shipped
    outside: true
    when: ${ event.data.to != \"\" }
    do: channel.post
    with: { channel: general, body: \"Order ${event.data.order} is on its way to ${event.data.to}.\" }
";

pub const DONE_STEPS: &str = "\
blueprint: shipped-news
permissions: [channels:write]

on:
  shipped:
    event: order.shipped
    outside: true
    steps:
      said:
        do: channel.post
        with: { channel: general, body: \"Order ${event.data.order} is on its way.\" }

      kept:
        do: emit
        with: { topic: news.said, order: \"${event.data.order}\" }
";

pub const OUTPUTS: &str = "\
blueprint: where-it-runs
permissions: [shell]

steps:
  host:
    do: shell
    with: { cmd: [hostname] }

outputs:
  host: ${steps.host.out.stdout}
";

pub const RELEASE: &str = "\
blueprint: release
permissions: [shell, channels:write]

steps:
  build:
    do: shell
    with: { cmd: [make, release] }

on_success:
  - emit: { topic: release.built }

on_fail:
  - do: channel.post
    with: { channel: ops, body: \"The release failed: ${run.error}\" }

on_finish:
  - do: log
    with: { message: \"The release ended: ${run.status}\" }
";

pub const STOPPED: &str = "\
blueprint: on-duty
permissions: [channels:write]

steps:
  here:
    do: channel.post
    with: { channel: ops, body: On duty. }

on:
  hourly:
    every: 1h
    do: channel.post
    with: { channel: ops, body: Still on duty. }

on_stop:
  - do: channel.post
    with: { channel: ops, body: Off duty. }
";

pub const CARRIED: &str = "\
blueprint: last-seen
version: 2

state:
  last: \"\"

on_update:
  - do: log
    with: { message: \"Carried over from version ${was.version}.\" }

on:
  seen:
    event: visitor.seen
    outside: true
    do: set
    with: { last: \"${event.data.name}\" }
";

pub const PARALLEL: &str = "\
blueprint: pictures
permissions: [shell]
max_parallel: 2

steps:
  small:
    needs: []
    do: shell
    with: { cmd: [convert, photo.jpg, -resize, 64x64, small.jpg] }

  large:
    needs: []
    do: shell
    with: { cmd: [convert, photo.jpg, -resize, 512x512, large.jpg] }
";

pub const SHORT_LIVED: &str = "\
blueprint: ping
durable: false
permissions: [shell]

steps:
  ping:
    do: shell
    with: { cmd: [ping, -c, \"1\", localhost] }
";

pub const LOCK: &str = "\
blueprint: lock
one_at_a_time: true

state:
  holder: \"\"

on:
  take:
    event: lock.take
    outside: true
    when: ${ state.holder == \"\" }
    do: set
    with: { holder: \"${event.data.who}\" }

  give:
    event: lock.give
    outside: true
    do: set
    with: { holder: \"\" }
";

pub const CALLED: &str = "\
blueprint: order-news
permissions: [blueprints:run]

steps:
  watch:
    call: std/watch
    with: { channel: general, collection: orders }
";

pub const SPAWNED: &str = "\
blueprint: order-news
permissions: [blueprints:run]

steps:
  watch:
    spawn: std/watch
    as: order-watch
    with: { channel: general, collection: orders }
";

pub const DEPLOYED: &str = "\
blueprint: release
permissions: [blueprints:run, shell]

params:
  branch: { type: string, default: main }
  loads: { type: string, default: deploy }

steps:
  build: { do: shell, with: { cmd: [make] } }

  deploy:
    needs: [build]
    call: \"${params.loads}\"
    as: deploy-prod
    environment: prod
    with: { branch: \"${params.branch}\" }
";

pub const TASK: &str = "\
blueprint: release
permissions: [shell]

steps:
  build:
    description: Everything it takes to have something to ship.
    task:
      compile: { do: shell, with: { cmd: [make] } }
      test: { do: shell, with: { cmd: [make, test] } }
    retry: 1
    timeout: 30m
";

pub const NEEDS: &str = "\
blueprint: deploy
permissions: [shell]

steps:
  build:
    do: shell
    with: { cmd: [make] }

  lint:
    needs: []
    do: shell
    with: { cmd: [make, lint] }

  ship:
    needs: [build, lint]
    do: shell
    with: { cmd: [make, ship] }
";

pub const WHEN: &str = "\
blueprint: deploy
permissions: [shell]

params:
  ship: false

steps:
  ship:
    when: ${params.ship}
    do: shell
    with: { cmd: [make, ship] }
";

pub const TIMED: &str = "\
blueprint: tests
permissions: [shell, channels:write]

steps:
  test:
    do: shell
    with: { cmd: [make, test] }
    timeout: 10m
    on_timeout:
      - do: channel.post
        with: { channel: ops, body: The tests ran for ten minutes and were stopped. }
";

pub const LINT: &str = "\
blueprint: tidy
permissions: [shell]

steps:
  lint:
    do: shell
    with: { cmd: [make, lint] }
    allow_failure: true

  said:
    do: log
    with: { message: \"The lint went well: ${steps.lint.ok}\" }
";

pub const REBUILT: &str = "\
blueprint: build
permissions: [shell]

steps:
  build:
    do: shell
    with: { cmd: [make] }
    again: true
";

pub const WATCHED: &str = "\
blueprint: build
permissions: [shell]

steps:
  build:
    do: shell
    with: { cmd: [make] }
    watch:
      - after: 5m
        emit: { topic: build.slow }
      - every: 1m
        when: ${ self.duration_ms > 120000 }
        do: log
        with: { message: \"Still building after ${self.duration_ms} ms.\" }
      - log: \"^error\"
        emit: { topic: build.error }
        fail: true
";

pub const BUILD_HOOKS: &str = "\
blueprint: build
permissions: [shell]

steps:
  build:
    do: shell
    with: { cmd: [make] }
    on_start:
      - emit: { topic: build.started }
    on_success:
      - do: log
        with: { message: \"Built in ${self.duration_ms} ms.\" }
    on_finish:
      - emit: { topic: build.over, went: \"${self.status}\" }
";

pub const TOLD: &str = "\
blueprint: build
permissions: [shell, channels:write]

params:
  tell: true

steps:
  build:
    do: shell
    with: { cmd: [make] }
    on_success:
      - when: ${params.tell}
        do: channel.post
        with: { channel: ops, body: Built. }
";

pub const FAILED: &str = "\
blueprint: build
permissions: [shell, channels:write]

steps:
  build:
    do: shell
    with: { cmd: [make] }
    on_fail:
      - when: ${ self.attempts < 3 }
        do: retry
      - emit: { topic: build.broken, why: \"${self.error}\" }
      - do: channel.post
        with: { channel: ops, body: \"The build failed: ${self.error}\" }
";

pub const ELSEWHERE: &str = "\
blueprint: build-there
permissions: [shell]

params:
  device: { type: string, description: The device to build on. }

steps:
  build:
    device: ${params.device}
    do: shell
    with: { cmd: [make] }
";

pub const MEASURED: &str = "\
blueprint: build
permissions: [shell]

steps:
  build:
    do: shell
    with: { cmd: [make] }
    measure: build
";

pub const LABELLED: &str = "\
blueprint: two-builds
permissions: [shell]

steps:
  quick:
    task:
      fast: { do: shell, with: { cmd: [make, fast] } }
    labels: { experiment: build, variant: quick }

  whole:
    needs: []
    task:
      full: { do: shell, with: { cmd: [make] } }
    labels: { experiment: build, variant: whole }
";

pub const ASKED: &str = "\
blueprint: build
permissions: [shell, agents:write]

steps:
  build:
    do: shell
    with: { cmd: [make] }
    if_fail_ask_agent: builder
    else: build-broken
";

pub const EACH: &str = "\
blueprint: greet-each

params:
  names: [ada, grace]

steps:
  greet:
    for_each: ${params.names}
    do: log
    with: { message: \"Hello, ${item}, number ${index}.\" }
";

pub const HOURLY: &str = "\
blueprint: hourly-copy
permissions: [shell]

on:
  hourly:
    every: 1h
    do: shell
    with: { cmd: [cp, data.db, backup/data.db] }
";

pub const BEFORE_DAWN: &str = "\
blueprint: copy-before-dawn
description: Copy the database somewhere safe at half past two each night, by the clock in Zagreb.
permissions: [shell]

on:
  copied:
    at: 02:30 Europe/Zagreb
    do: shell
    with: { cmd: [cp, data.db, backup/data.db] }
";

pub const POLLED: &str = "\
blueprint: code-moved
permissions: [shell]

on:
  moved:
    poll: { every: 1m, do: shell, with: { cmd: [git, rev-parse, HEAD] } }
    do: emit
    with: { topic: code.moved, head: \"${event.data.stdout}\" }
";

pub const ONE_ORDER: &str = "\
blueprint: one-order

params:
  order: { type: string, description: The order to follow. }

state:
  where: \"\"

on:
  shipped:
    event: order.shipped
    outside: true
    for: { order: \"${params.order}\" }
    do: set
    with: { where: \"${event.data.to}\" }
";

pub const EVERY_FAILURE: &str = "\
blueprint: every-failure
permissions: [channels:write]

on:
  failed:
    event: run.failed
    scope: fleet
    do: channel.post
    with: { channel: ops, body: \"${event.data.blueprint} failed: ${event.data.error}\" }
";

pub const BURST: &str = "\
blueprint: burst-news

on:
  changed:
    event: record.updated
    debounce: 10s
    do: emit
    with: { topic: news.changed, record: \"${event.data.id}\" }
";

pub const ORDERS: &str = "\
blueprint: orders
permissions: [shell]

on:
  paid:
    event: order.paid
    outside: true
    turn: \"${event.data.order}\"
    do: shell
    with: { cmd: [./pack, \"${event.data.order}\"] }
";

pub const ON_ASKING: &str = "\
blueprint: copy-on-asking
permissions: [blueprints:run]

on:
  asked:
    event: copy.asked
    outside: true
    call: std/snapshot
    with: { to: \"${event.data.to}\" }
";

pub const WHO: &str = "\
blueprint: who-am-i

steps:
  said:
    do: log
    with: { message: \"I am ${instance.id}, of ${instance.name}.\" }
";

pub const SHADOW: &str = "\
blueprint: beside

steps:
  said:
    do: log
    with: { message: \"Attached to ${target.id}.\" }
";

pub const CAREFUL: &str = "\
blueprint: careful
permissions: [shell]

steps:
  ship:
    when: ${ profile.name != \"prod\" }
    do: shell
    with: { cmd: [make, ship] }
";

/// The environment of its project's an instance was applied under.
pub const STAGED: &str = "\
blueprint: staged
permissions: [shell]

steps:
  migrate:
    when: ${ environment.name == \"prod\" }
    do: shell
    env: { API: \"${environment.vars.api}\" }
    with: { cmd: [./migrate] }
";
