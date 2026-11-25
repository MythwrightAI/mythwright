//! Who of a project's agents is there to be told of (`agents::present`,
//! asked by `std/fleet` as `agent.present` for what it says when it is
//! first there): a table of who there is and what is asked, and what it
//! comes to, with no word left unreached. The roles are handed, and some
//! cases call them something else than a fleet usually does: a role's name
//! written into the block fails those.

use agents::present::{Came, One, present};

/// An agent: its name, its role, whether it is up and whether it takes
/// part in the channel.
fn one(name: &str, role: &str, up: bool, in_channel: bool) -> One {
    One { name: name.into(), role: role.into(), up, in_channel }
}

fn roles(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_string()).collect()
}

#[test]
fn who_is_there_is_those_of_some_roles_in_the_channel_by_name_and_how_many_there_are_of_others() {
    let architect = one("shop-architect-1", "architect", true, true);
    let worker = |n: u8| one(&format!("shop-worker-{n}"), "worker", true, false);
    // The agents; the roles heard; the roles counted; what it came to, who is heard, alone, how many.
    type Case = (
        Vec<One>,
        &'static [&'static str],
        &'static [&'static str],
        Came,
        &'static [&'static str],
        bool,
        usize,
    );
    let cases: Vec<Case> = vec![
        // The usual: one to talk to, who is the one agent in the channel, and one who works.
        (
            vec![architect.clone(), worker(1)],
            &["architect"],
            &["worker"],
            Came::Both,
            &["shop-architect-1"],
            true,
            1,
        ),
        // Three who work and nobody to talk to.
        (vec![worker(1), worker(2), worker(3)], &[], &["worker"], Came::Counted, &[], false, 3),
        (vec![worker(1), worker(2), worker(3)], &["architect"], &["worker"], Came::Counted, &[], false, 3),
        // Somebody to talk to and nobody who works.
        (vec![architect.clone()], &["architect"], &["worker"], Came::Heard, &["shop-architect-1"], true, 0),
        // Nobody at all; and nothing asked.
        (vec![], &["architect"], &["worker"], Came::Nobody, &[], false, 0),
        (vec![architect.clone(), worker(1)], &[], &[], Came::Nobody, &[], false, 0),
        // One of the role that is not in the channel is not heard.
        (
            vec![one("shop-architect-1", "architect", true, false), worker(1)],
            &["architect"],
            &["worker"],
            Came::Counted,
            &[],
            false,
            1,
        ),
        // One that was stopped is neither heard nor counted.
        (
            vec![
                one("shop-architect-1", "architect", false, true),
                one("shop-worker-1", "worker", false, false),
                worker(2),
            ],
            &["architect"],
            &["worker"],
            Came::Counted,
            &[],
            false,
            1,
        ),
        // Two to talk to: both by name, in the order given, and neither alone.
        (
            vec![architect.clone(), one("shop-architect-2", "architect", true, true), worker(1)],
            &["architect"],
            &["worker"],
            Came::Both,
            &["shop-architect-1", "shop-architect-2"],
            false,
            1,
        ),
        // One to talk to, and another agent of some other role in the
        // channel: a word with nobody named could be for either.
        (
            vec![architect.clone(), one("shop-worker-1", "worker", true, true)],
            &["architect"],
            &["worker"],
            Came::Both,
            &["shop-architect-1"],
            false,
            1,
        ),
        // That other one stopped, the first is alone again.
        (
            vec![architect.clone(), one("shop-worker-1", "worker", false, true), worker(2)],
            &["architect"],
            &["worker"],
            Came::Both,
            &["shop-architect-1"],
            true,
            1,
        ),
        // A role in both lists: its agent is heard and counted.
        (vec![one("solo-1", "solo", true, true)], &["solo"], &["solo"], Came::Both, &["solo-1"], true, 1),
        // Several roles counted together, and roles called anything.
        (
            vec![
                one("a-1", "drafts", true, true),
                one("b-1", "builds", true, false),
                one("c-1", "tests", true, false),
            ],
            &["drafts"],
            &["builds", "tests"],
            Came::Both,
            &["a-1"],
            true,
            2,
        ),
        // The usual names mean nothing where the roles are called something else.
        (vec![architect.clone(), worker(1)], &["drafts"], &["builds"], Came::Nobody, &[], false, 0),
    ];
    let mut reached = Vec::new();
    for (agents, here, counted, came, heard, alone, count) in cases {
        let got = present(&agents, &roles(here), &roles(counted));
        assert_eq!(
            (got.came, got.heard.iter().map(String::as_str).collect::<Vec<_>>(), got.alone, got.count),
            (came, heard.to_vec(), alone, count),
            "{agents:?}, here {here:?}, counted {counted:?}"
        );
        assert_eq!(got.first, heard.first().copied().unwrap_or_default());
        reached.push(got.came);
    }
    for word in Came::ALL {
        assert!(reached.contains(word), "no case reaches `{}`", word.word());
    }
}
