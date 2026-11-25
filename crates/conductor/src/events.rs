//! The events the conductor itself emits: what becomes of a run, a step
//! and an instance.
//!
//! Each is said once, here, with what it carries. A handler is held to
//! this when its blueprint is checked, and the conductor is held to it as
//! it runs wherever it is run to be tested (`blueprint::events`).

use blueprint::{EventSpec, ValueType::*};

pub fn events() -> Vec<EventSpec> {
    let run = |topic: &str, summary: &str| {
        EventSpec::new(topic, summary)
            .field("flow", String, "which flow: `main`, or a handler as `on.<name>`")
            .field("blueprint", String, "the blueprint whose flow it is")
    };
    let step = |topic: &str, summary: &str| EventSpec::new(topic, summary);
    let instance = |topic: &str, summary: &str| EventSpec::new(topic, summary);
    let all = vec![
        run("run.started", "A run of a flow began. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it."),
        run("run.resumed", "A run that was under way when the conductor stopped was taken up again. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it."),
        run("run.failed", "A run failed. It is said just before the run says it finished. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it.")
            .field("error", String, "why, with each step it came up through named before it")
            .maybe("reason", String, "why, alone: what the innermost step that failed said")
            .maybe("duration_ms", Int, "how long it ran"),
        run("run.finished", "A run ended, however it went. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it.")
            .field("status", String, "`ok`, `failed` or `cancelled`")
            .maybe("duration_ms", Int, "how long it ran")
            .maybe("error", String, "why, when it did not end well"),
        step("step.started", "A step began, or began again. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it.")
            .field("attempt", Int, "which try this is, from 1")
            .field("verb", String, "`do`, `call`, `spawn` or `task`")
            .field("target", String, "the action or blueprint it names")
            .field("resumed", Bool, "taken up again after the conductor stopped"),
        step("step.finished", "A step that began has ended, however it went. Said after `step.failed` or `step.cancelled`. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it.")
            .field("attempt", Int, "which try it was")
            .field("status", String, "`ok`, `failed` or `cancelled`")
            .field("duration_ms", Int, "how long, from its first try"),
        step("step.failed", "A step failed. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it.")
            .field("attempt", Int, "which try it was")
            .field("error", String, "why")
            .maybe("code", String, "the code the step was said to fail with, if it was")
            .maybe("timed_out", Bool, "it ran past its `timeout`")
            .maybe("tolerated", Bool, "the run goes on all the same (`allow_failure`)")
            .maybe("duration_ms", Int, "how long, from its first try"),
        step("step.retrying", "A step that failed is to be tried again. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it.")
            .field("attempt", Int, "the try that failed")
            .field("error", String, "why it failed")
            .field("in_ms", Int, "how long until the next try")
            .field("by", String, "what asked for it: `retry`, or a `hook`"),
        step(
            "step.skipped",
            "A step was not run. Passed on as it is said, and kept in the journal only while some \
             instance's text hears it or waits for it.",
        )
        .maybe("when", String, "the `when` that did not hold"),
        step("step.cancelled", "A step was stopped before it ended, or never came to run. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it.").maybe(
            "attempt",
            Int,
            "the try that was under way",
        ),
        step("step.waiting", "A `wait` step began waiting. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it.")
            .maybe("event", String, "the topic it waits for")
            .maybe("ended", List, "the keys of the children it waits to have ended")
            .maybe("until", Int, "when it gives up, in milliseconds since 1970"),
        instance("instance.started", "A blueprint was applied and an instance of it began.")
            .field("blueprint", String, "the blueprint")
            .field("key", String, "its name within its project or its parent")
            .maybe("parent", String, "the instance that called or spawned it")
            .maybe("project", String, "the project it is in")
            .field("revision", String, "the text it runs"),
        instance("instance.active", "An instance's steps ended well and it stays, listening.").field(
            "outputs",
            Object,
            "what its blueprint gives out",
        ),
        instance(
            "instance.finished",
            "An instance has ended, however it did: its steps went well and it had nothing to stay for, or it \
             failed, or it was stopped. Said after `instance.failed` or `instance.stopped`.",
        )
        .field("status", String, "`done`, `failed` or `stopped`")
        .maybe("key", String, "what it was called by whatever started it: for a child, what its step said with `as`")
        .maybe("outputs", Object, "what its blueprint gives out, when its steps went well")
        .maybe("error", String, "why, when it failed"),
        instance("instance.failed", "An instance's first run failed, or it could not be taken up again.")
            .field("error", String, "why, with each step it came up through named before it")
            .maybe("reason", String, "why, alone: what the innermost step that failed said")
            .maybe("called_by", String, "the instance whose step called it and is waiting: its failure is that step's too")
            .maybe("let_be", Bool, "the step that called it lets it fail (`allow_failure`) or may (`on_fail`), so it may go no further up")
            .maybe("run", String, "the run whose steps failed, where it was one"),
        instance("instance.stopped", "An instance was stopped.").field("blueprint", String, "the blueprint").field(
            "why",
            String,
            "`asked`, by somebody or by a step that says so; `parent`, what it was started under ended; `restarted`, it keeps nothing as it goes and the conductor was started again; `withdrawn`, the key it was applied with was withdrawn",
        ),
        instance("instance.updated", "An instance came to run another text, or other params. Nothing said after this is handled by the text from before.")
            .field("blueprint", String, "the blueprint")
            .field("revision", String, "the text it runs now")
            .field("from", String, "the text it ran before")
            .maybe("in_place", Bool, "it took the text where it was; nothing ran again")
            .maybe("carried", Bool, "it was carried over by the text's `on_update`"),
        instance("instance.update_failed", "An instance could not take what was applied over it.")
            .field("error", String, "why")
            .field("kept", String, "the text it goes on running"),
        instance("instance.call_kept", "A step went on with another text of a blueprint than the one now there.")
            .field("blueprint", String, "the blueprint that was called")
            .field("key", String, "the child's name")
            .field("error", String, "why: what is wrong with the text there now, or that nobody vouched for it")
            .field("kept", String, "the text that was used"),
        instance("instance.state", "An instance's state changed.").open(),
        EventSpec::new("handler.failed", "A handler could not start listening.")
            .field("handler", String, "its name")
            .field("error", String, "why"),
        EventSpec::new(
            "handler.runaway",
            "An event was not handled: it was at the end of a long chain of events, each said while the one \
             before was being handled, in a short time.",
        )
        .field("handler", String, "the handler that did not take it")
        .field("topic", String, "the event's topic")
        .field("seq", Int, "its place in the journal")
        .field("depth", Int, "how many events deep in the chain it was")
        .field("within_ms", Int, "how long the chain had been going")
        .field("error", String, "what happened and what to do about it, in a sentence"),
        EventSpec::new("hook.failed", "A hook could not be done.")
            .field("action", String, "what it was to do")
            .field("error", String, "why"),
        EventSpec::new("poll.failed", "What a handler asks on a timer could not be asked.")
            .field("handler", String, "its name")
            .field("error", String, "why"),
        EventSpec::new("log.line", "A line something printed.")
            .field("stream", String, "`stdout` or `stderr`")
            .field("text", String, "the line")
            .maybe("service", String, "the service that printed it")
            .passing(),
        EventSpec::new("system.profile", "The variables of this profile, the copy of the factory, were changed.")
            .field("set", List, "the names newly set")
            .field("changed", List, "the names given another value")
            .field("removed", List, "the names taken away")
            .maybe("by", String, "who did it, as an actor")
            .field("by_name", String, "who did it, by name"),
        EventSpec::new("system.texts", "How what is running comes by new texts, or how a taken one lands, was changed.")
            .field("come", String, "`held`, `follow`, or `usual` where nothing is said")
            .field("land", String, "`in_place` or `again`")
            .field("was", String, "how texts were come by before")
            .field("landed", String, "how a taken one landed before")
            .field("by", String, "who said so, as an actor")
            .field("by_name", String, "who said so, by name")
            .maybe("gives_up", String, "where having texts followed was turned on: what that gives up, in words"),
        EventSpec::new("system.logs", "How long the detail of runs' steps is kept, or where their output is written, was said.")
            .maybe("days", Int, "for how many days, where it is said; nothing for ever")
            .maybe("was", Any, "what was said before: days, or a folder")
            .maybe("folder", String, "where runs' steps write, where it is said; nothing for the data folder")
            .field("by", String, "who said so, as an actor")
            .field("by_name", String, "who said so, by name"),
        EventSpec::new(
            "system.logs.let_go",
            "What runs' steps printed and what the runner said of them as they went, older than the factory keeps, \
             was let go of. What runs came to is kept.",
        )
        .field("before", Int, "what was last written before this went, in milliseconds since 1970")
        .field("days", Int, "how many days the factory keeps it")
        .field("events", Int, "how many events went")
        .field("files", Int, "how many files of steps went")
        .field("bytes", Int, "how many bytes those held")
        .maybe("held", String, "what was kept for being owed, in a sentence"),
        EventSpec::new(
            "project.forked",
            "A blueprint that ships was forked for a project: its text was copied into the project, where the \
             copy is what runs under the name from then on. What it was copied from is kept.",
        )
        .field("project", String, "the project whose fork it is")
        .field("blueprint", String, "the blueprint that ships, by its name")
        .maybe("version", String, "the version the text it was copied from says of itself")
        .field("revision", String, "the text it was copied from")
        .field("path", String, "the file the copy is, within the project")
        .field("by", String, "who forked it, as an actor")
        .field("by_name", String, "who forked it, by name"),
        EventSpec::new(
            "project.unforked",
            "A project's fork of a blueprint that ships was taken away: the one that ships is what runs under \
             the name there again, once what is running is applied. What the fork was made from is forgotten.",
        )
        .field("project", String, "the project whose fork it was")
        .field("blueprint", String, "the blueprint that ships, by its name")
        .field("path", String, "the file that was taken away, within the project")
        .maybe("version", String, "the version it had been forked from, where that was kept")
        .field("lost", Bool, "its text said something else than the one that ships, and is gone")
        .field("by", String, "who took it away, as an actor")
        .field("by_name", String, "who took it away, by name"),
        instance(
            "instance.text_left",
            "Where texts are followed: the text for an instance changed and does not check, so it was not taken. Said once for a text.",
        )
        .field("blueprint", String, "the blueprint whose text it is")
        .field("error", String, "what is wrong with the text")
        .field("kept", String, "the revision the instance goes on with")
        .field("left", String, "the revision that was not taken"),
    ];
    // What is said of a project's environments is said where they are kept
    // (`environments::events`).
    let all = all.into_iter().chain(environments::events::events()).collect();
    // Who may be told of each: whoever may read blueprints, their instances
    // and their runs, which is what all of these are about. A fork is a
    // file of a project's, and is said to whoever may read the project.
    blueprint::events::heard(
        all,
        &[
            ("project.forked", "projects:read"),
            ("project.unforked", "projects:read"),
            ("run", "blueprints:read"),
            ("step", "blueprints:read"),
            ("instance", "blueprints:read"),
            ("handler", "blueprints:read"),
            ("hook", "blueprints:read"),
            ("poll", "blueprints:read"),
            ("log", "blueprints:read"),
            ("system.profile", "blueprints:read"),
            ("system.texts", "blueprints:read"),
            ("system.logs", "blueprints:read"),
        ],
    )
}

impl crate::Engine {
    /// Whether an instance acting as `who` is told of `event`: whether
    /// `who` holds what reading the thing it is about takes, as a caller
    /// of the routes would have to. What the instance or anything it
    /// started said is its own to hear whatever it holds.
    pub async fn hears(&self, who: &kernel::Principal, instance: &str, event: &kernel::Event) -> bool {
        // What an event says hearing it takes is held to first, and for
        // everybody: it was said of something kept from those without it.
        if !self.journal().may_tell(who, event) {
            return false;
        }
        if self.may_hear(who, &event.topic) {
            return true;
        }
        match event.source.instance.as_deref() {
            // Asked of the conductor that keeps the instance: what is
            // under it is that one's too.
            Some(from) => self.keeper(instance).await.is_within(from, instance).await,
            None => false,
        }
    }

    /// Whether `who` may be told of an event of this topic at all.
    pub fn may_hear(&self, who: &kernel::Principal, topic: &str) -> bool {
        let described = self.registry().own_events().iter().find(|spec| spec.topic == topic);
        let scope = blueprint::events::scope_to_hear(described, topic);
        scope == blueprint::events::ANYONE || who.can(scope)
    }
}
