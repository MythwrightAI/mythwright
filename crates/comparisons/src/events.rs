//! The events said of comparisons: that one was started, that one whose
//! ways are named by the start had ways added, that one was ended, that
//! one was given a rule its shares change by, and that its shares were
//! said again.
//!
//! Each is said once, here, with what it carries, which is the comparison
//! as it then is (`blueprint::events`).

use blueprint::{EventSpec, ValueType::*};

pub fn events() -> Vec<EventSpec> {
    let compared = |topic: &str, summary: &str| {
        EventSpec::new(topic, summary)
            .field("name", String, "what the comparison is called")
            .maybe("project", String, "the project it is in")
            .field("of", String, "the blueprint whose starting it is about")
            .maybe("per", String, "the param that says which thing a start is for")
            .maybe("also", List, "blueprints that carry a thing's way when started for it")
            .maybe("also_per", String, "the param that says which thing a start of one of those is for, where it is another than `per`")
            .maybe(
                "counts",
                List,
                "what is counted of what comes of each thing: each its `name`, the `topic` of the event, the field that says the `thing`, the fields it is counted `by`, and what of that it `accepts` as the thing being accepted",
            )
            .field(
                "ways",
                List,
                "its ways: each its `name`, `share`, what it starts and hands, and how many `went`",
            )
            .field("by", String, "who started it, as an actor")
            .field("held", List, "what they held then")
            .field("started", Int, "when")
            .maybe("ended", Int, "when it was ended, once it has been")
            .maybe(
                "goes",
                String,
                "once ended: the way everything goes from then on; as it is written where none",
            )
            .maybe(
                "until",
                Object,
                "once ended, while what was said before it ended is still to be counted under it: where among the journals it ended",
            )
            .maybe(
                "asked",
                Bool,
                "true where its ways are named by the start: a start that names it and one of its ways (params `compared` and `way`) goes that way",
            )
            .maybe(
                "rule",
                Object,
                "the rule its shares change by: the way that `grows`, the one it is held `against`, the `count` and what it `came` to, `at_least` where that is held to coming at least as often, `over` how many things of each, the `step` and the `most` of a hundred, and the `least` it goes back to a step at a time where one is said",
            )
            .maybe("rolling", Object, "what the rule has read since the shares were last set (`since`), and whether it is `done`")
            .maybe("shared", List, "each time its shares were said again: `at`, `by`, what they `was` and are `now`, and `why`")
    };
    let all = vec![
        compared(
            "system.comparison.started",
            "A comparison was started: from now, one blueprint is started one of several ways.",
        ),
        compared(
            "system.comparison.added",
            "Ways or counts were added to a comparison whose ways are named by the start, as a blueprint asked for it again.",
        ),
        compared(
            "system.comparison.ended",
            "A comparison was ended, or which way everything goes after it was said again.",
        ),
        compared(
            "system.comparison.ruled",
            "A comparison was given a rule its shares change by, or had it taken away.",
        ),
        EventSpec::new(
            "system.comparison.shared",
            "A comparison's shares were said again: by its rule, as what it counts came to, or by somebody; the same where the rule held them.",
        )
        .field("name", String, "what the comparison is called")
        .maybe("project", String, "the project it is in")
        .field("at", Int, "when")
        .field("by", String, "who said them: an actor, or `rule` where the rule did")
        .field("was", Object, "each way's share before")
        .field("now", Object, "each way's share after: the same as before where the rule held them")
        .field("why", String, "why, in a sentence: what the rule read and what that came to"),
    ];
    // Who may be told of each: whoever may read blueprints, whose starting
    // these are about.
    blueprint::events::heard(all, &[("system.comparison", "blueprints:read")])
}
