//! The events said of accounts and keys.

use blueprint::{EventSpec, ValueType::*};

pub fn events() -> Vec<EventSpec> {
    let all = vec![
        EventSpec::new("identity.registered", "An account was made.")
            .field("account", String, "the account's id")
            .field("role", String, "`owner` or `member`")
            .maybe("made", Bool, "made by the factory as it first started, with no email and no password"),
        EventSpec::new("identity.renamed", "Somebody called themselves something else.")
            .field("account", String, "whose account")
            .field("name", String, "what they are called now")
            .field("was", String, "what they were called"),
        EventSpec::new(
            "identity.password",
            "An account that had no password was given one, with an email to sign in by where it had none, to \
             sign in another way than by a link.",
        )
        .field("account", String, "whose account"),
        EventSpec::new("identity.tier", "The plan this installation is on was set.")
            .field("tier", String, "the plan"),
        EventSpec::new(
            "identity.signed_out",
            "Somebody ended every session they had, wherever each was begun.",
        )
        .field("account", String, "whose sessions they were")
        .field("sessions", Int, "how many there were"),
        EventSpec::new("identity.key.made", "A key was made: a credential for a script.")
            .field("key", String, "the key's id, which is what withdraws it")
            .field("name", String, "what its maker called it")
            .field("account", String, "whose it is")
            .field("scopes", List, "what it holds")
            .maybe("with", String, "the key it was made with, which withdraws it with itself"),
        EventSpec::new(
            "identity.key.revoked",
            "A key was withdrawn, and is nobody's credential any more; so is every key made with it, each said \
             too. What it applied is stopped unless whoever withdrew it said to leave it running.",
        )
        .field("key", String, "the key's id")
        .field("name", String, "what its maker called it")
        .field("account", String, "whose it was")
        .maybe("with", String, "the key whose withdrawing withdrew it: the one it was made with, or that one's maker, all the way up")
        .maybe("leave", Bool, "what it applied was left running, as whoever withdrew it said to (`--leave`)"),
        EventSpec::new(
            "identity.key.rolled",
            "A key was given another secret: the one it had is refused from then, and it is the same key, \
             holding what it held, so what it applied goes on.",
        )
        .field("key", String, "the key's id")
        .field("name", String, "what its maker called it")
        .field("account", String, "whose it is"),
        EventSpec::new(
            "identity.given",
            "A person was given a scope beyond what their role holds, by somebody who holds it and `admin`.",
        )
        .field("account", String, "whose account")
        .field("scope", String, "what was given")
        .field("given", List, "everything they are given now"),
        EventSpec::new(
            "identity.taken",
            "A scope a person was given was taken away from them, and from the keys they made.",
        )
        .field("account", String, "whose account")
        .field("scope", String, "what was taken away")
        .field("given", List, "everything they are given now")
        .field("keys", Int, "how many of their keys held it, and hold it no longer"),
        EventSpec::new("identity.group.made", "A group of people and agents was made, by whoever manages the factory.")
            .field("group", String, "the group's id")
            .field("name", String, "what it is called"),
        EventSpec::new("identity.group.renamed", "A group was called something else.")
            .field("group", String, "the group's id")
            .field("name", String, "what it is called now")
            .field("was", String, "what it was called"),
        EventSpec::new(
            "identity.group.held",
            "What a group's members may ask of the factory beside what each holds itself was said: heard by \
             whoever manages the factory.",
        )
        .field("group", String, "the group's id")
        .field("name", String, "what it is called")
        .field("holds", List, "the scopes its members hold by it now")
        .field("keys", Int, "how many keys of its people were narrowed to what they hold now"),
        EventSpec::new("identity.group.joined", "A person or an agent was put in a group.")
            .field("group", String, "the group's id")
            .field("name", String, "what it is called")
            .field("member", String, "who, as an actor"),
        EventSpec::new("identity.group.left", "A person or an agent was taken out of a group.")
            .field("group", String, "the group's id")
            .field("name", String, "what it is called")
            .field("member", String, "who, as an actor")
            .field("keys", Int, "how many of their keys were narrowed to what they hold now"),
        EventSpec::new("identity.group.removed", "A group was taken away, with everybody's place in it.")
            .field("group", String, "the group's id")
            .field("name", String, "what it was called")
            .field("members", List, "who was in it, as actors")
            .field("keys", Int, "how many keys of its people were narrowed to what they hold now"),
        EventSpec::new(
            "theme.kept",
            "A theme somebody made was made or changed: heard by them alone, and by whoever may read themes \
             while it is offered to everybody.",
        )
        .field("owner", String, "whose theme it is, as an actor")
        .field(
            "theme",
            Object,
            "the theme as it now is: its id, its name, its eight colours, its light version where it has one, its \
             look (what it says of letters, size, density and corners), and who offered it",
        ),
        EventSpec::new(
            "theme.gone",
            "A theme somebody made was taken away: heard by them alone, and by whoever may read themes where it \
             was offered to everybody.",
        )
        .field("owner", String, "whose theme it was, as an actor")
        .field("theme", String, "the theme's id")
        .field("offered", Bool, "whether it was offered to everybody"),
        EventSpec::new("theme.chosen", "Somebody chose another theme: heard by them alone.")
            .field("owner", String, "who chose, as an actor")
            .field("theme", Any, "the theme's id, or nothing for the usual one"),
        EventSpec::new("theme.offered", "A theme was offered to everybody, beside the ones that ship.")
            .field("owner", String, "whose theme it is, as an actor")
            .field("theme", Object, "the theme as it now is")
            .field("by", String, "who offered it, as an actor"),
        EventSpec::new(
            "theme.withdrawn",
            "A theme offered to everybody was withdrawn, and is its owner's alone again.",
        )
        .field("owner", String, "whose theme it is, as an actor")
        .field("theme", String, "the theme's id")
        .field("by", String, "who withdrew it, as an actor"),
        EventSpec::new(
            "arranged.kept",
            "Somebody changed how they arranged the side panel (their groups of channels, the channels pinned to \
             the top, what is pinned in Resources): heard by them alone.",
        )
        .field("owner", String, "whose arrangement it is, as an actor")
        .field(
            "arrangement",
            Object,
            "the arrangement as it now is: its groups, each an id, a name and its channels; the channels pinned; \
             and the addresses of the things pinned",
        )
        .field("nth", Int, "how many times it has been written, which grows with each write"),
        EventSpec::new(
            "clock.kept",
            "Somebody changed how a time is written for them (the zone and whether on a clock of twelve hours or \
             twenty-four): heard by them alone.",
        )
        .field("owner", String, "whose setting it is, as an actor")
        .field("clock", Object, "the setting as it now is: the zone by its name, and `12h` or `24h`")
        .field("nth", Int, "how many times it has been written, which grows with each write"),
        EventSpec::new(
            "appearance.kept",
            "Somebody changed how what they read looks for them (the size of its text, its width, and light or \
             dark): heard by them alone.",
        )
        .field("owner", String, "whose setting it is, as an actor")
        .field("appearance", Object, "the setting as it now is: `text`, `width`, `colour`, `contents` and `typeface`, each a word")
        .field("nth", Int, "how many times it has been written, which grows with each write"),
    ];
    // What is said of an account is the owner's to hear; what is said of
    // themes is for whoever may read their own, and each event of one
    // that is somebody's alone is held to them by what it takes
    // (`crate::themes`).
    blueprint::events::heard(
        all,
        &[
            ("identity", "admin"),
            // Who is in a group is read as who has an account here is
            // (`/people`); what a group holds is the factory's manager's.
            ("identity.group", "channels:read"),
            ("identity.group.held", "admin"),
            ("theme", crate::themes::READ),
            ("arranged", crate::arranged::READ),
            ("clock", crate::clocked::READ),
            ("appearance", crate::appearance::READ),
        ],
    )
}
