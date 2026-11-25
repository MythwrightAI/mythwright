//! What is said of a project's environments, where it is said
//! (`kept`): an environment made, changed and taken away. The runner
//! hands these out with its own (`conductor::events`).

use blueprint::{EventSpec, ValueType::*};

/// What every event of an environment's life says of it and of the load
/// it is of.
fn life(spec: EventSpec) -> EventSpec {
    spec.field("project", String, "the project whose environment it is")
        .field("environment", String, "what the environment is called")
        .field("branch", String, "the branch it was loaded from, as it was chosen")
        .field("commit", String, "the commit the branch was at, whole")
        .field("blueprint", String, "the blueprint it is loaded by")
        .field("instance", String, "the instance it was loaded as")
        .maybe("run", String, "the run of its steps the load began")
}

pub fn events() -> Vec<EventSpec> {
    let all = vec![
        EventSpec::new(
            "project.environment.made",
            "An environment of a project was made: a named set of variables the project's instances are to be \
             applied under, such as dev, staging or prod. It has no variables yet.",
        )
        .field("project", String, "the project whose environment it is")
        .field("environment", String, "what the environment is called")
        .field("by", String, "who made it, as an actor")
        .field("by_name", String, "who made it, by name"),
        EventSpec::new(
            "project.environment.changed",
            "The variables or the secrets of a project's environment, the blueprint it is loaded by, or what \
             it holds to, were changed. What changed is said by name, and no value is said.",
        )
        .field("project", String, "the project whose environment it is")
        .field("environment", String, "what the environment is called")
        .field("set", List, "the variables that were not set before")
        .field("changed", List, "the variables that have another value than they had")
        .field("removed", List, "the variables that are no longer set")
        .field("marked", List, "the variables now handed to agents, or no longer")
        .field("secrets", List, "the secrets set, or set again: by name, never what they are")
        .field("secrets_removed", List, "the secrets no longer kept")
        .field("secrets_marked", List, "the secrets now handed to agents as another variable, or no longer")
        .field("deploy", Bool, "whether the blueprint it is loaded by, for a branch, was said, changed or taken away")
        .field("rules", Bool, "whether what it holds to (who approves what goes under it) was said, changed or taken away")
        .field("by", String, "who changed them, as an actor")
        .field("by_name", String, "who changed them, by name"),
        EventSpec::new(
            "project.environment.ran",
            "A project's environment was loaded from a branch: the blueprint it says it is loaded by was \
             applied for the project under it, handed the branch. How that goes is the run's to say.",
        )
        .field("project", String, "the project whose environment it is")
        .field("environment", String, "what the environment is called")
        .field("branch", String, "the branch as it was chosen, such as dev or origin/dev")
        .field("commit", String, "the commit the branch was at, whole")
        .field("blueprint", String, "the blueprint that was applied")
        .field("instance", String, "the instance it was applied as")
        .maybe("run", String, "the run of its steps that it began, where it began one")
        .field("by", String, "who ran it, as an actor")
        .field("by_name", String, "who ran it, by name"),
        life(EventSpec::new(
            "project.environment.ready",
            "The run a project's environment was last loaded by ended well: what it is loaded as is ready, with \
             where it is reached and the cards it holds where the blueprint it is loaded by gave them out.",
        ))
        .field("took_ms", Int, "how long the load took, from Run to its run's end, in milliseconds")
        .maybe("address", String, "where it is reached: what the blueprint gave out as `address`")
        .field("cards", List, "the cards it holds: what the blueprint gave out as `cards`; none where it gave none"),
        life(EventSpec::new(
            "project.environment.failed",
            "The run a project's environment was last loaded by did not end well: it failed, or was cut short.",
        ))
        .field("took_ms", Int, "how long the load took before it failed, in milliseconds")
        .field("cards", List, "the cards it would hold, where the blueprint gave them out")
        .maybe("reason", String, "why, as the run said it"),
        life(EventSpec::new(
            "project.environment.down",
            "What a project's environment was loaded as, once ready, was taken down: stopped, or failed, by \
             whatever ended it.",
        ))
        .field("up_ms", Int, "how long it was ready before it was taken down, in milliseconds")
        .maybe("address", String, "where it was reached"),
        EventSpec::new(
            "project.environment.removed",
            "An environment of a project was taken away, with its variables and its secrets.",
        )
        .field("project", String, "the project whose environment it was")
        .field("environment", String, "what the environment was called")
        .field("vars", Number, "how many variables went with it")
        .field("secrets", Number, "how many secrets went with it")
        .field("by", String, "who took it away, as an actor")
        .field("by_name", String, "who took it away, by name"),
        EventSpec::new(
            "project.flag.changed",
            "What a project's feature flag says in one of its environments changed: the flag's variable of the \
             environment (FLAG_<NAME>) was set, or taken away, in the write that says so. It names the flag's \
             row, whose history keeps it.",
        )
        .field("project", String, "the project whose flag it is")
        .field("environment", String, "the environment it now says it in")
        .field("flag", String, "the flag, by its name written plainly, such as new-checkout")
        .field("name", String, "the flag's name, as its row says it")
        .field("record", String, "the flag's row, by its id")
        .field("variable", String, "the variable the environment says it in, such as FLAG_NEW_CHECKOUT")
        .maybe("value", Any, "what it says now: on or off, a whole number from 0 to 100, or a word; nothing, where its variable was taken away")
        .maybe("was", Any, "what it said before; nothing, where it said nothing")
        .field("shown", String, "what it says now, as it is said to people: on, off, 25%, a word")
        .field("by", String, "who wrote it, as an actor")
        .field("by_name", String, "who wrote it, by name")
        .maybe("asked_by", String, "who asked for it, as an actor, where somebody else wrote it once it was let go on")
        .maybe("asked_by_name", String, "who asked for it, by name")
        .maybe("approved_by", String, "who let it go on, as an actor, where it waited for a go-ahead"),
        EventSpec::new(
            "project.flag.asked",
            "Somebody asked for a project's feature flag to be turned on, or wider, in an environment whose rule \
             says who approves what goes under it, and is not one of them: what looks after the project's flags \
             (`std/flags`) asks one of them to let it go on, and writes it once one does, only where it is still \
             the change asked last of that flag there and the flag says what it said when it was asked. Kept on \
             the environment in the write that says it.",
        )
        .field("project", String, "the project whose flag it is")
        .field("flag", String, "the flag, by its name written plainly, such as new-checkout")
        .field("name", String, "the flag's name, as its row says it")
        .field("record", String, "the flag's row, by its id")
        .field("kind", String, "what kind of value it says: switch, percentage or word")
        .field("environment", String, "the environment it is asked for in")
        .field("variable", String, "the variable the environment says it in, such as FLAG_NEW_CHECKOUT")
        .field("asked", String, "the change asked, by its id: it takes the place of any asked before it")
        .field("value", Any, "what it is to say: on, a whole number from 0 to 100, or a word")
        .maybe("was", Any, "what it says now, which it has to say still for the change to be written; nothing, where it says nothing")
        .field("shown", String, "what it is to say, as it is said to people: on, 25%, a word")
        .field("by", String, "who asked, as an actor: the key, where it was asked with a key somebody made")
        .field("by_name", String, "who asked, by name"),
        EventSpec::new(
            "project.flag.unchanged",
            "A change of a project's feature flag that waited for a go-ahead was let go on after the flag was \
             changed, or asked for again, in that environment: nothing was written, so that an approval never \
             undoes a later change. It names the flag's row, whose history keeps it.",
        )
        .field("project", String, "the project whose flag it is")
        .field("environment", String, "the environment it was asked for in")
        .field("flag", String, "the flag, by its name written plainly")
        .field("name", String, "the flag's name, as its row says it")
        .field("record", String, "the flag's row, by its id")
        .field("variable", String, "the variable the environment says it in")
        .maybe("value", Any, "what it was asked to say")
        .maybe("was", Any, "what it said when the change was asked")
        .maybe("now", Any, "what it says now")
        .field("overtaken", String, "`changed`, where it was changed since, or `asked again`, where a later change of it waits in its place")
        .field("shown", String, "what it was asked to say, as it is said to people")
        .field("by", String, "who would have written it, as an actor")
        .field("by_name", String, "who would have written it, by name")
        .maybe("asked_by", String, "who asked for it, as an actor")
        .maybe("asked_by_name", String, "who asked for it, by name")
        .maybe("approved_by", String, "who let it go on, as an actor"),
    ];
    // An environment is a project's, and is said to whoever may read the project.
    blueprint::events::heard(
        all,
        &[("project.environment", "projects:read"), ("project.flag", "projects:read")],
    )
}
