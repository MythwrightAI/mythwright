//! Words shown working: an example run in a sandbox, with a few things
//! done to it, and a sentence that says what to notice in what came of it.
//!
//! A scenario is of an example, and is offered wherever that example is
//! shown: the words of the language that are hardest to read from a
//! sentence (a handler, what it is held to and how it takes turns, a step
//! tried again, a wait, a blueprint called or spawned) are the ones that
//! have one. What is done is said in the shape a sandbox's routes take,
//! each thing as `POST /api/sandboxes/{id}/acts` takes it, in order: the
//! sandbox is made with the text not yet applied, and applying it is one
//! of the things done, so that what has to be there first (a channel it
//! says things in) is made first. Each comes to what its sentence says,
//! which a test of the API suite holds by running it in a sandbox and
//! reading the sandbox's own answer (`scenarios.rs` there): a scenario
//! with no checks of its own there fails it. Nothing of one reaches the
//! factory in use, which is a sandbox's rule and not this file's.

use serde::Serialize;

use super::shown;
use crate::catalog::asked;

/// An example run in a sandbox, with what is done to it and what to notice.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Scenario {
    /// What it is known by: the word it shows, mostly.
    pub name: &'static str,
    /// The text that is run: an example a word or an action is shown by.
    pub example: &'static str,
    /// Whether a stand-in waits to be answered (`asked`) or answers at
    /// once what it usually answers (`default`), as a sandbox is made.
    pub answers: &'static str,
    /// What is done, in order: a JSON list of what a sandbox's acts take.
    pub acts: &'static str,
    /// What to notice in what came of it, in a sentence or two.
    pub notice: &'static str,
}

pub const SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "handler",
        example: shown::DONE_NEWS,
        answers: "default",
        acts: r#"[
            { "do": { "action": "channel.ensure", "with": { "name": "general" } } },
            { "apply": {} },
            { "say": { "topic": "order.shipped", "data": { "order": "7", "to": "Ada" } } },
            { "say": { "topic": "order.shipped", "data": { "order": "8", "to": "" } } }
        ]"#,
        notice: "Applied, it ran nothing and stayed, listening. Each time `order.shipped` was said its handler \
                 was woken: the first was said in general, and the second, which names nobody to send it to, did \
                 not hold to its `when`, so nothing ran for it.",
    },
    Scenario {
        name: "for",
        example: shown::ONE_ORDER,
        answers: "default",
        acts: r#"[
            { "apply": { "params": { "order": "7" } } },
            { "say": { "topic": "order.shipped", "data": { "order": "8", "to": "Paris" } } },
            { "say": { "topic": "order.shipped", "data": { "order": "7", "to": "Lyon" } } }
        ]"#,
        notice: "It was told to follow order 7, and `for` held its handler to that order: order 8 being shipped \
                 woke nothing, and order 7 being shipped was heard once, and kept where it went.",
    },
    Scenario {
        name: "turn",
        example: shown::ORDERS,
        answers: "asked",
        acts: r#"[
            { "apply": {} },
            { "say": { "topic": "order.paid", "data": { "order": "7" } } },
            { "say": { "topic": "order.paid", "data": { "order": "7" } } },
            { "say": { "topic": "order.paid", "data": { "order": "8" } } },
            { "answer": { "asked": 1, "gives": { "code": 0 } } }
        ]"#,
        notice: "Order 7 was paid for twice, and the second took its turn behind the first: its command was \
                 asked for only once the first had been answered. Order 8 has a turn of its own, and was asked \
                 for at once, beside the first.",
    },
    Scenario {
        name: "debounce",
        example: shown::BURST,
        answers: "default",
        acts: r#"[
            { "apply": {} },
            { "say": { "topic": "record.updated", "data": { "project": "sandbox", "collection": "notes", "id": "sandbox-notes-1", "changed": ["title"] } } },
            { "say": { "topic": "record.updated", "data": { "project": "sandbox", "collection": "notes", "id": "sandbox-notes-1", "changed": ["title"] } } },
            { "say": { "topic": "record.updated", "data": { "project": "sandbox", "collection": "notes", "id": "sandbox-notes-1", "changed": ["title"] } } },
            { "advance": { "by": "10s" } }
        ]"#,
        notice: "The record was changed three times in a moment and nothing ran. Once the clock had moved ten \
                 seconds past the last change, its handler ran once, for the three of them.",
    },
    Scenario {
        name: "when",
        example: shown::WHEN,
        answers: "default",
        acts: r#"[
            { "apply": {} },
            { "apply": { "params": { "ship": true } } }
        ]"#,
        notice: "Applied as it is, `params.ship` is false, so its step was passed over and nothing was shipped. \
                 Applied again told `ship: true`, the same step ran.",
    },
    Scenario {
        name: "one_at_a_time",
        example: shown::LOCK,
        answers: "default",
        acts: r#"[
            { "apply": {} },
            { "say": { "topic": "lock.take", "data": { "who": "ada" } } },
            { "say": { "topic": "lock.take", "data": { "who": "grace" } } },
            { "say": { "topic": "lock.give", "data": {} } },
            { "say": { "topic": "lock.take", "data": { "who": "grace" } } }
        ]"#,
        notice: "Its handlers take turns, so each sees what the one before it kept. Grace asking first found \
                 the lock Ada's, did not hold to the `when`, and ran nothing; once it was given back, her second \
                 asking took it.",
    },
    Scenario {
        name: "retry",
        example: shown::FETCH,
        answers: "asked",
        acts: r#"[
            { "apply": {} },
            { "answer": { "asked": 1, "fails": "Connecting to localhost:8080... failed: Connection refused." } },
            { "advance": { "by": "2s" } },
            { "answer": { "asked": 2, "gives": { "code": 0, "stdout": "ok" } } }
        ]"#,
        notice: "Its first try failed, so the step waited two seconds, its `backoff`, and tried again. The second \
                 try went well, and so did the step: one step, whose command was asked for twice.",
    },
    Scenario {
        name: "on_fail",
        example: shown::FAILED,
        answers: "asked",
        acts: r#"[
            { "do": { "action": "channel.ensure", "with": { "name": "ops" } } },
            { "apply": {} },
            { "answer": { "asked": 1, "fails": "make: *** [build] Error 2" } },
            { "answer": { "asked": 2, "fails": "make: *** [build] Error 2" } },
            { "answer": { "asked": 3, "fails": "make: *** [build] Error 2" } }
        ]"#,
        notice: "Each time the build failed, every entry of its `on_fail` was done in order. The first two times \
                 its `when` held and the step was tried again, and the failure was said all the same, as an event \
                 and in ops: what follows `retry` is done too. The third time nothing tried it again, and the run \
                 failed.",
    },
    Scenario {
        name: "wait",
        example: asked::WAIT,
        answers: "default",
        acts: r#"[
            { "apply": {} },
            { "advance": { "by": "30m" } },
            { "say": { "topic": "question.answered", "data": { "yes": true } } }
        ]"#,
        notice: "Its first step asked its question and waited. Half an hour on it was still waiting; when \
                 `question.answered` was said it went on, and the next step says it was answered in time.",
    },
    Scenario {
        name: "call",
        example: shown::CALLED,
        answers: "default",
        acts: r#"[{ "apply": {} }]"#,
        notice: "`call` waited for what it called: std/watch made its channel and joined it before the step \
                 `watch` was over, and only then did order-news go on.",
    },
    Scenario {
        name: "spawn",
        example: shown::SPAWNED,
        answers: "default",
        acts: r#"[{ "apply": {} }]"#,
        notice: "`spawn` did not wait: the step `watch` was over, and order-news was up, before std/watch had \
                 run a step of its own. What it started is called order-watch, as its `as` says.",
    },
];
