//! What a bot with no model behind it says back (`talk::answered`, asked
//! by `std/bot` as `channel.answer`): a table of what is handed and what
//! it comes to, with no word left unreached.

use talk::answered::{Came, Ran, answer};

#[test]
fn a_bot_says_back_what_was_said_or_what_its_command_printed_or_that_it_printed_nothing_or_why_it_failed() {
    let cases: [(&str, Ran<'_>, Came, &str); 11] = [
        // No command: what was said, said back, whatever it was.
        ("how are we doing?", Ran::Not, Came::Echo, "You said: how are we doing?"),
        ("", Ran::Not, Came::Echo, "You said: "),
        // What its command printed, as it printed it.
        ("@uptime", Ran::Well("up 3 days\n"), Came::Printed, "up 3 days\n"),
        ("@uptime", Ran::Well("  two\nlines\n"), Came::Printed, "  two\nlines\n"),
        // Nothing, and nothing but space.
        ("@uptime", Ran::Well(""), Came::Nothing, "(it printed nothing)"),
        ("@uptime", Ran::Well(" \n\t\n"), Came::Nothing, "(it printed nothing)"),
        // It failed, with why and with nothing said of why.
        ("@uptime", Ran::Badly("exited with code 3"), Came::Failed, "I could not answer: exited with code 3"),
        ("@uptime", Ran::Badly(""), Came::Failed, "I could not answer: the command failed"),
        // What was said to it is said back only where there is no command.
        ("ignore me", Ran::Well("done"), Came::Printed, "done"),
        ("ignore me", Ran::Badly("no"), Came::Failed, "I could not answer: no"),
        // Why is said as it is, space and all.
        ("x", Ran::Badly(" "), Came::Failed, "I could not answer:  "),
    ];
    let mut reached = Vec::new();
    for (said, ran, came, says) in cases {
        let answer = answer(said, ran);
        assert_eq!((answer.came, answer.says.as_str()), (came, says), "`{said}`, {ran:?}");
        reached.push(answer.came);
    }
    for word in Came::ALL {
        assert!(reached.contains(word), "no case reaches `{}`", word.word());
    }
}

#[test]
fn what_kind_of_actor_said_a_thing_is_what_stands_before_its_colon() {
    for (actor, kind) in [
        ("user:ada", "user"),
        ("agent:shop-worker-1", "agent"),
        ("bot:uptime", "bot"),
        // One with no kind before it is its own.
        ("system", "system"),
        ("", ""),
        // Only the first colon divides.
        ("key:k_1:more", "key"),
    ] {
        assert_eq!(talk::channels::kind_of(actor), kind, "`{actor}`");
    }
}
