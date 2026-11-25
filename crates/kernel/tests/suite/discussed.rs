//! What a channel's discussions come to, by their cases.

use kernel::discussed::{Message, Next, Settings, Stretch, Takes, Thread, brief, ending, next, takes};
use serde_json::json;

fn peers() -> Settings {
    Settings::read(
        &json!({ "agents": [{ "name": "ann", "does": "writes" }, { "name": "bo", "does": "checks" }] }),
    )
    .unwrap()
}

fn led() -> Settings {
    Settings::read(&json!({
        "lead": "helper",
        "advisors": [{ "name": "architect", "for": "technical advice, and confirming a plan" }],
    }))
    .unwrap()
}

fn said(author: &str, names: &[&str], mark: &str) -> Message {
    Message {
        id: "m2".into(),
        author: author.into(),
        names: names.iter().map(|one| one.to_string()).collect(),
        mark: mark.into(),
        thread: None,
        reply_to: None,
        answers: None,
    }
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|one| one.to_string()).collect()
}

#[test]
fn settings_say_everything_on_six_turns_one_agree_and_every_ending_told_where_nothing_is_said() {
    let settings = peers();
    assert_eq!(settings.on, names(&["tagged", "plans", "asked"]));
    assert_eq!((settings.turns, settings.agreeing.as_str()), (6, "one"));
    assert_eq!(settings.tell_on, names(&["agree", "needs", "cap"]));
    assert_eq!(led().taking(), names(&["helper", "architect"]));
    // A line of a form says one as `name: what it is for`.
    let lines =
        Settings::read(&json!({ "lead": "helper", "advisors": ["architect: technical advice"] })).unwrap();
    assert_eq!(lines.advisors[0].does, "technical advice");
}

#[test]
fn settings_that_make_no_sense_are_refused_in_a_sentence() {
    for (said, why) in [
        (json!({ "agents": ["ann"] }), "two agents at least"),
        (json!({ "agents": ["ann", "ann"] }), "said twice"),
        (json!({ "agents": ["ann", "bo"], "turns": 21 }), "from 1 to 20"),
        (json!({ "agents": ["ann", "bo"], "turns": 0 }), "from 1 to 20"),
        (json!({ "agents": ["ann", "bo"], "agreeing": "most" }), "`one` or `all`"),
        (json!({ "agents": ["ann", "bo"], "on": ["shouted"] }), "`shouted` in `on`"),
        (json!({ "agents": ["ann", "bo"], "tell_on": ["never"] }), "`never` in `tell_on`"),
        (json!({ "lead": "helper", "agents": ["ann"] }), "say who advises `helper`"),
        (json!({ "agents": ["ann", { "does": "x" }] }), "with none"),
    ] {
        let refused = Settings::read(&said).unwrap_err();
        assert!(refused.contains(why), "{said}: {refused}");
    }
}

#[test]
fn each_way_a_message_comes_to_a_discussion_or_not() {
    let (peers, led) = (peers(), led());
    let started = |settings: &Settings, message: &Message| match takes(settings, message, |_| Thread::Free) {
        Takes::Starts { root, agents, spoken, asker, why } => Some((root, agents, spoken, asker, why)),
        _ => None,
    };
    // An agent naming another that takes part asks it: it speaks first.
    let tag = said("agent:ann", &["bo"], "");
    assert_eq!(
        started(&peers, &tag),
        Some(("m2".into(), names(&["bo", "ann"]), names(&["ann"]), None, "tagged"))
    );
    // A plan is discussed with everybody else, and asks the person it answered.
    let mut plan = said("agent:ann", &[], "plan");
    plan.answers = Some("user:ada".into());
    plan.reply_to = Some("m1".into());
    let (root, agents, _, asker, why) = started(&peers, &plan).unwrap();
    assert_eq!(
        (root.as_str(), agents, asker, why),
        ("m2", names(&["bo", "ann"]), Some("user:ada".into()), "plans")
    );
    // A person asking two to discuss, the first named first.
    let asked = said("user:ada", &["bo", "ann"], "discuss");
    assert_eq!(started(&peers, &asked).unwrap().1, names(&["bo", "ann"]));
    assert_eq!(started(&peers, &asked).unwrap().3, Some("user:ada".into()));
    // Naming two without the mark is two answers, as anywhere.
    assert_eq!(takes(&peers, &said("user:ada", &["bo", "ann"], ""), |_| Thread::Free), Takes::Not);
    // Asked of one is no discussion.
    assert_eq!(takes(&peers, &said("user:ada", &["bo"], "discuss"), |_| Thread::Free), Takes::Not);
    // A bot naming agents begins nothing, nor does an agent that takes no part.
    assert_eq!(takes(&peers, &said("bot:uptime", &["bo"], ""), |_| Thread::Free), Takes::Not);
    assert_eq!(takes(&peers, &said("agent:carl", &["bo"], ""), |_| Thread::Free), Takes::Not);
    // A trigger that is off begins nothing.
    let off = Settings::read(&json!({ "agents": ["ann", "bo"], "on": ["asked"] })).unwrap();
    assert_eq!(takes(&off, &tag, |_| Thread::Free), Takes::Not);
    assert_eq!(takes(&off, &plan, |_| Thread::Free), Takes::Not);
    // In a discussion's thread a message is the discussion's, and a person's lets one that waits go on.
    let within = |mut message: Message| {
        message.thread = Some("m0".into());
        message
    };
    assert_eq!(takes(&peers, &within(tag.clone()), |_| Thread::Going), Takes::Within { resumes: false });
    let person = within(said("user:ada", &[], ""));
    assert_eq!(takes(&peers, &person, |_| Thread::Waiting), Takes::Within { resumes: true });
    let agent = within(said("agent:bo", &[], ""));
    assert_eq!(takes(&peers, &agent, |_| Thread::Waiting), Takes::Within { resumes: false });
    // In a thread of no discussion, the discussion hangs from that thread.
    let mut threaded = tag.clone();
    threaded.thread = Some("m0".into());
    assert_eq!(started(&peers, &threaded).unwrap().0, "m0");

    // In lead's way a person naming nobody is the lead's alone, and naming one is as anywhere.
    assert_eq!(takes(&led, &said("user:ada", &[], ""), |_| Thread::Free), Takes::Lead("helper".into()));
    assert_eq!(takes(&led, &said("user:ada", &["architect"], ""), |_| Thread::Free), Takes::Not);
    assert_eq!(takes(&peers, &said("user:ada", &[], ""), |_| Thread::Free), Takes::Not);
    // The lead consulting, or planning, hangs the discussion under what the person asked.
    let mut consults = said("agent:helper", &["architect"], "");
    consults.reply_to = Some("m1".into());
    consults.answers = Some("user:ada".into());
    assert_eq!(
        started(&led, &consults),
        Some((
            "m1".into(),
            names(&["architect", "helper"]),
            names(&["helper"]),
            Some("user:ada".into()),
            "tagged"
        ))
    );
    let mut plans = consults.clone();
    plans.names.clear();
    plans.mark = "plan".into();
    assert_eq!(started(&led, &plans).unwrap().1, names(&["architect", "helper"]));
    // An advisor discusses with the lead alone.
    // What the lead says under a discussion that was agreed (its answer to the person) begins none again.
    let agreed = |root: &str| if root == "m1" { Thread::Agreed } else { Thread::Free };
    assert_eq!(takes(&led, &consults, agreed), Takes::Within { resumes: false });
    let mut again = said("user:ada", &["architect", "helper"], "discuss");
    again.thread = Some("m1".into());
    assert!(matches!(takes(&led, &again, agreed), Takes::Starts { .. }));
    let mut advises = said("agent:architect", &[], "plan");
    advises.reply_to = None;
    assert_eq!(started(&led, &advises).unwrap().1, names(&["helper", "architect"]));
}

fn stretch(agents: &[&str], spoken: &[&str]) -> Stretch {
    Stretch { agents: names(agents), spoken: names(spoken), ..Stretch::default() }
}

fn after(settings: &Settings, was: &Stretch, last: &str, word: &str) -> Next {
    let mut was = was.clone();
    was.last = last.into();
    was.word = word.into();
    was.nth += 1;
    next(settings, &was)
}

#[test]
fn turns_go_round_until_agreed_needed_or_capped() {
    let settings = peers();
    // The first turn is whoever was said to be first, else the first of them.
    let mut begun = stretch(&["bo", "ann"], &["ann"]);
    let Next::Turn { slot, then } = next(&settings, &begun) else { panic!() };
    assert_eq!((slot.as_str(), then.nth), ("bo", 0));
    begun.first = "ann".into();
    assert!(matches!(next(&settings, &begun), Next::Turn { slot, .. } if slot == "ann"));

    // One agree once everybody has spoken ends it.
    let begun = stretch(&["bo", "ann"], &["ann"]);
    assert_eq!(after(&settings, &begun, "bo", "agree"), Next::Agreed);
    // Not before everybody has spoken.
    let unheard = stretch(&["bo", "ann"], &[]);
    let Next::Turn { slot, then } = after(&settings, &unheard, "bo", "agree") else { panic!() };
    assert_eq!(
        (slot.as_str(), then.spoken.clone(), then.agreed.clone()),
        ("ann", names(&["bo"]), names(&["bo"]))
    );
    assert_eq!(after(&settings, &then, "ann", "agree"), Next::Agreed);
    // A reply goes on to the other, and a turn that chose nothing is a reply.
    for word in ["reply", ""] {
        assert!(matches!(after(&settings, &begun, "bo", word), Next::Turn { slot, .. } if slot == "ann"));
    }
    // Needs a person whoever says so.
    assert_eq!(after(&settings, &begun, "bo", "needs"), Next::Needs);
    // Capped where the stretch has had its turns.
    let mut long = begun.clone();
    long.nth = 5;
    assert_eq!(after(&settings, &long, "bo", "reply"), Next::Capped);
    assert!(matches!(after(&settings, &begun, "bo", "reply"), Next::Turn { .. }));
    // Three go round in order.
    let three = stretch(&["a", "b", "c"], &[]);
    assert!(matches!(after(&settings, &three, "c", "reply"), Next::Turn { slot, .. } if slot == "a"));
    assert!(matches!(after(&settings, &three, "a", "reply"), Next::Turn { slot, .. } if slot == "b"));
}

#[test]
fn where_everybody_is_to_agree_a_reply_after_an_agree_begins_the_agreeing_again() {
    let all = Settings::read(&json!({ "agents": ["ann", "bo"], "agreeing": "all" })).unwrap();
    let begun = stretch(&["bo", "ann"], &["ann"]);
    let Next::Turn { then, .. } = after(&all, &begun, "bo", "agree") else { panic!() };
    let Next::Turn { then, .. } = after(&all, &then, "ann", "reply") else { panic!() };
    assert!(then.agreed.is_empty());
    let Next::Turn { then, .. } = after(&all, &then, "bo", "agree") else { panic!() };
    assert_eq!(after(&all, &then, "ann", "agree"), Next::Agreed);
}

#[test]
fn an_ending_says_a_line_tells_by_what_tell_on_says_and_asks_the_lead_to_answer_an_agreement() {
    let settings = peers();
    let agents = names(&["bo", "ann"]);
    let agreed = ending(&settings, "agreed", &agents, 3, "design", "Fine.");
    assert_eq!((agreed.status, agreed.tells), ("agreed", true));
    assert_eq!(agreed.line, "bo and ann agreed, after 3 turns.");
    assert!(agreed.lead.is_empty());
    let needs = ending(&settings, "needs", &agents, 2, "design", "Which database?");
    assert_eq!(needs.status, "waiting");
    assert_eq!(needs.says, "bo and ann need you to decide, in #design: Which database?");
    let capped = ending(&settings, "capped", &agents, 6, "design", "");
    assert!(capped.says.contains("did not agree in 6 turns"));
    assert_eq!(
        ending(&settings, "agreed", &agents, 1, "design", "").line,
        "bo and ann agreed, after 1 turn."
    );
    let quiet = Settings::read(&json!({ "agents": ["ann", "bo"], "tell_on": ["needs"] })).unwrap();
    assert!(!ending(&quiet, "agreed", &agents, 1, "design", "").tells);
    assert!(!ending(&quiet, "capped", &agents, 1, "design", "").tells);
    assert!(ending(&quiet, "needs", &agents, 1, "design", "").tells);
    let lead = ending(&led(), "agreed", &names(&["architect", "helper"]), 2, "design", "Use one table.");
    assert!(
        lead.lead.contains("Answer the person now") && lead.lead.contains("Use one table."),
        "{}",
        lead.lead
    );
    assert!(ending(&led(), "needs", &names(&["architect", "helper"]), 2, "design", "").lead.is_empty());
}

#[test]
fn a_brief_says_who_else_takes_part_and_how_each_is_asked() {
    let lead = brief(&led(), "helper");
    assert!(lead.contains("people talk to you") && lead.contains("`architect`: technical advice"), "{lead}");
    let advisor = brief(&led(), "architect");
    assert!(
        advisor.contains("you advise `helper`") && advisor.contains("only where they name you"),
        "{advisor}"
    );
    let peer = brief(&peers(), "ann");
    assert!(peer.contains("`bo`: checks") && !peer.contains("`ann`"), "{peer}");
    assert!(peer.contains("6 turns at most") && peer.contains("`plan`"), "{peer}");
    assert_eq!(brief(&peers(), "carl"), "");
}
