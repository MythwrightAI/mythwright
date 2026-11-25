//! The events the agents crate emits: what becomes of a slot, a turn, a
//! sign-in and a package.

use blueprint::{EventSpec, ValueType::*};

pub fn events() -> Vec<EventSpec> {
    let slot =
        |topic: &str, summary: &str| EventSpec::new(topic, summary).field("slot", String, "the agent's name");
    let turn = |topic: &str, summary: &str| slot(topic, summary).field("turn", String, "the turn");
    let signin = |topic: &str, summary: &str| {
        EventSpec::new(topic, summary).field("signin", String, "the sign-in's name")
    };
    let all = vec![
        slot("agent.slot.created", "An agent was brought into the fleet.")
            .field("adapter", String, "what runs it")
            .maybe("project", String, "the project it works in")
            .maybe("role", String, "the role it was brought up in")
            .maybe("wall", Bool, "whether its program works behind a wall"),
        slot("agent.slot.updated", "An agent's slot was changed, or its program said which model it is.")
            .maybe("adapter", String, "what runs it")
            .maybe("project", String, "the project it works in")
            .maybe("role", String, "the role it is in")
            .maybe("wall", Bool, "whether its program works behind a wall")
            .maybe("model", String, "the model its program says it runs on"),
        slot("agent.slot.stopped", "An agent was stopped taking work.").maybe("project", String, "the project its agent is in, where it is in one"),
        slot("agent.slot.removed", "An agent was taken away for good, by a person: its slot, its credential and its place in every channel. What it said and did stays.")
            .field("turns", Int, "how many pieces of work it was asked, which stay as they are")
            .field("channels", List, "the channels it was in, by name")
            .maybe("project", String, "the project it worked in, where it worked in one"),
        slot(
            "agent.slot.invited",
            "An invitation was made for an agent this factory does not start to join by. The invitation is in no event.",
        )
        .field("expires", Int, "when the invitation stops working, in milliseconds since 1970")
        .maybe("project", String, "the project the agent is listed with, where it is with one"),
        slot("agent.slot.joined", "An agent this factory does not start handed its invitation over, and has its credential.")
            .maybe("project", String, "the project the agent is listed with, where it is with one"),
        slot("agent.slot.unjoined", "An agent this factory does not start gave its credential up: it is let in again by an invitation.")
            .maybe("project", String, "the project the agent is listed with, where it is with one"),
        slot(
            "agent.slot.leaving",
            "An agent is given no more work, and is stopped when it has finished what it has.",
        )
        .field("in_hand", Int, "how many turns it has yet to finish")
        .maybe("project", String, "the project its agent is in, where it is in one"),
        turn("agent.turn.queued", "A turn was put in an agent's queue.").maybe("project", String, "the project its agent is in, where it is in one"),
        turn("agent.turn.started", "An agent began a turn.")
            .maybe("project", String, "the project its agent is in, where it is in one")
            .field("kind", String, "what kind of turn: a message, a task, a consultation")
            .maybe("channel", String, "the channel it answers in")
            .maybe("thread", String, "the conversation it is part of: for a card, `lane:<card>`, `plan:<card>` or `review:<card>:<seat>`")
            // Said again of a turn a daemon was started and found in hand.
            .maybe("again", Int, "how many times a daemon has now been started and found it in hand")
            .maybe("taken_up", Bool, "of one found in hand: whether its program had gone on without the daemon and was taken up where it was, and not begun again"),
        turn("agent.turn.finished", "An agent's turn ended.")
            .maybe("project", String, "the project its agent is in, where it is in one")
            .field("kind", String, "what kind of turn it was")
            .maybe("channel", String, "the channel it answered in")
            .field("ok", Bool, "whether it ended well")
            .maybe("error", String, "why it did not")
            .maybe("called_off", Bool, "that it did not because somebody called it off, which is not the agent failing at it")
            .field("duration_ms", Int, "how long it took")
            .maybe("cost_usd", Number, "what its program says it cost")
            .maybe("input_tokens", Int, "what it read")
            .maybe("output_tokens", Int, "what it wrote")
            .maybe("decision", String, "what it chose, where it was given a choice")
            .maybe("refused", List, "what its program said it was refused as it went, each by a word, where its package says how it tells"),
        turn("agent.output", "Something an agent's program printed as it worked.")
            .maybe("project", String, "the project its agent is in, where it is in one")
            .field("text", String, "what it printed")
            .passing(),
        signin("agent.signin.added", "A sign-in was given to the factory to hold.")
            .field("kind", String, "`subscription` or `key`")
            .field("for", String, "the package whose program signs in with it")
            .field("at_once", Int, "how many turns may hold it at once"),
        signin("agent.signin.removed", "A sign-in was taken away."),
        signin("agent.signin.restored", "A sign-in that was set aside can be handed out again."),
        signin("agent.signin.held", "A sign-in was handed to an agent for a turn.")
            .field("kind", String, "`subscription` or `key`")
            .field("slot", String, "the agent's name")
            .field("turn", String, "the turn"),
        signin("agent.signin.released", "A sign-in was handed back at the end of a turn.")
            .field("slot", String, "the agent's name")
            .field("turn", String, "the turn"),
        signin("agent.signin.set_aside", "A sign-in that was refused is not handed out for a while.")
            .field("slot", String, "the agent whose turn it was refused in")
            .field("turn", String, "the turn")
            .field("until", Int, "when it may be handed out again, in milliseconds since 1970")
            .field("why", String, "what its program said"),
        turn("agent.signin.waiting", "A turn is waiting for a sign-in to be free.").field(
            "until",
            Int,
            "when one is next expected, in milliseconds since 1970",
        ),
        EventSpec::new("extension.added", "A package was installed.")
            .field("extension", String, "the package's name")
            .maybe("program", String, "the program it runs")
            .field("actions", List, "the actions it brings")
            .field("blueprints", List, "the blueprints it brings"),
        EventSpec::new("extension.removed", "A package was taken away.").field(
            "extension",
            String,
            "the package's name",
        ),
        EventSpec::new("extension.broken", "A package that is installed could not be read.").field(
            "problem",
            String,
            "what is wrong with it",
        ),
    ];
    // Who may be told of each: whoever may see the agents, what they run on
    // and what they sign in with (never the sign-in itself, which no event
    // carries).
    blueprint::events::heard(all, &[("agent", "agents:read"), ("extension", "agents:read")])
}
