//! A card's A/B test, played through the fleet that ships with stand-in
//! agents: set up as a row of `contests`, laid into where the card is
//! worked, each contender working the card in a place of its own, a
//! judge picking one by its letter, the winner's work the card's and
//! reviewed and merged as any card's, the loser's kept as a branch, and
//! what the judge said counted under each contender's way.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{self, add, card, given, rig as fleet_rig, says, shop};
use crate::held::{key, said};
use crate::lanes::{git, move_card, moved};

/// The fleet's rows: three roles that each write (the contenders), the
/// reviewer, and so many judges.
fn agents(judges: u64, judged_by: &str) -> Value {
    json!([
        { "title": "worker", "role": "worker", "count": 1, "adapter": "writer" },
        { "title": "otter", "role": "otter", "count": 1, "adapter": "writer" },
        { "title": "slate", "role": "slate", "count": 1, "adapter": "writer" },
        { "title": "lark", "role": "lark", "count": 1, "adapter": "writer" },
        { "title": "reviewer", "role": "reviewer", "count": 1, "adapter": "critic", "permission": "read" },
        { "title": "judge", "role": "judge", "count": judges, "adapter": judged_by, "permission": "read" },
    ])
}

/// A fleet with A/B tests, given as the board's page gives them, by
/// applying what that start applies (the start asks for a fleet by the
/// project's own `fleet`, which this rig has not got): the factory, where
/// its stand-ins are told what to say, and the project's folder.
async fn tested(judges: u64) -> (Rig, PathBuf, PathBuf) {
    let (rig, at) = fleet_rig().await;
    given_ab(rig, at, judges, "critic").await
}

async fn given_ab(rig: Rig, at: PathBuf, judges: u64, judged_by: &str) -> (Rig, PathBuf, PathBuf) {
    let dir = shop(&rig, json!({ "agents": agents(judges, judged_by) })).await;
    let ab =
        json!({ "blueprint": "std/ab", "project": "shop", "params": { "project": "shop" }, "wait": true });
    let given = rig.send("/api/apply", ab).await;
    assert_eq!(given["finished"]["status"], "ok", "{given}");
    (rig, at, dir)
}

/// A card with a test set up as `fields` say, laid into where it is worked,
/// and made ready: the card and the test.
async fn contested(rig: &Rig, fields: Value) -> (String, String) {
    let id = add(rig, "Add a cart", &[]).await;
    let mut fields = fields;
    fields["card"] = json!(id);
    let before = rig.mark();
    let test = rig.send("/api/collections/shop/contests/records", json!({ "fields": fields })).await;
    let test = test["id"].as_str().unwrap().to_string();
    rig.event("record.created", move |event| event.seq > before && event.data["collection"] == "segments")
        .await;
    // And the fleet has taken the row into what its stage goes through.
    rig.event("fleet.arranged", move |event| event.seq > before).await;
    assert_eq!(rows(rig, "segments", "title", &format!("ab-{test}")).await.len(), 1);
    move_card(rig, &id, "ready", None).await;
    (id, test)
}

/// What each entry of the test came to, by its letter.
async fn entries(rig: &Rig, test: &str) -> BTreeMap<String, Value> {
    let rows = rows(rig, "entries", "contest", test).await;
    rows.into_iter()
        .map(|row| (row["data"]["letter"].as_str().unwrap().to_string(), row["data"].clone()))
        .collect()
}

/// The rows of `table` that say `of` in `field`.
async fn rows(rig: &Rig, table: &str, field: &str, of: &str) -> Vec<Value> {
    let all = rig.read(&format!("/api/collections/shop/{table}/records")).await;
    let all = all["records"].as_array().cloned().unwrap_or_default();
    all.into_iter().filter(|row| row["data"][field] == of).collect()
}

/// Waits until the card has come to the end of the board, or was left.
async fn played_to_the_end(rig: &Rig, id: &str) {
    let card = id.to_string();
    rig.event("card.left,board.item.moved", move |event| {
        let of = event.data["card"] == card.as_str() || event.data["id"] == card.as_str();
        of && (event.topic == "card.left" || event.data["to"] == "done")
    })
    .await;
}

fn branches(dir: &Path) -> Vec<String> {
    git(dir, &["branch", "--format=%(refname:short)"]).lines().map(str::to_string).collect()
}

#[tokio::test]
async fn a_card_worked_two_ways_at_once_is_judged_and_the_winners_work_is_reviewed_merged_and_counted() {
    let (rig, at, dir) = tested(1).await;
    // The judge picks A, whichever way that is.
    says(&at, "shop-judge-1", "A does what the card asks; B does not.", "A");
    let (id, test) = contested(&rig, json!({ "contenders": ["otter", "slate"], "judges": "judge" })).await;
    let laid = format!("ab-{test}");
    played_to_the_end(&rig, &id).await;
    assert_eq!(card(&rig, &id).await["status"], "done", "{}", card(&rig, &id).await);

    // Each contender worked it, kept, and was judged; the one picked by its letter won.
    let entries = rows(&rig, "entries", "contest", &test).await;
    assert_eq!(entries.len(), 2, "{entries:?}");
    let of =
        |letter: &str| entries.iter().find(|row| row["data"]["letter"] == letter).unwrap()["data"].clone();
    let (won, lost) = (of("A"), of("B"));
    assert_eq!(
        (won["verdict"].as_str(), lost["verdict"].as_str()),
        (Some("won"), Some("lost")),
        "{entries:?}"
    );
    assert_eq!((won["state"].as_str(), lost["state"].as_str()), (Some("kept"), Some("kept")));
    let winner = won["way"].as_str().unwrap().to_string();
    assert_eq!(won["agent"], format!("shop-{winner}-1"));
    // The tests hold no walls: each could see the other's work, and says so.
    assert_eq!((won["not_held_apart"].as_bool(), lost["not_held_apart"].as_bool()), (Some(true), Some(true)));
    assert_eq!(lost["branch"], format!("lane/{}", lost["key"].as_str().unwrap()));
    assert!(
        lost["scores"].as_array().is_some_and(Vec::is_empty) && !won["scores"].as_array().unwrap().is_empty()
    );

    // The test says what it came to.
    let row = rig.read(&format!("/api/records/{test}")).await;
    assert_eq!(
        (row["data"]["state"].as_str(), row["data"]["winner"].as_str()),
        (Some("came"), Some(winner.as_str()))
    );
    // Its row of the card's list now gives the card's rounds to the winner's agent.
    let handed = rows(&rig, "segments", "title", &laid).await;
    assert_eq!(handed.len(), 1, "{handed:?}");
    assert_eq!(handed[0]["data"]["does"], "std/work", "{handed:?}");
    assert_eq!(handed[0]["data"]["with"], json!([format!("agent=shop-{winner}-1")]), "{handed:?}");

    // The winner's work is what was merged; the loser's is kept as its branch, and no place is left.
    let merged = git(&dir, &["log", "--format=%an", "-5"]);
    assert!(merged.contains(&format!("shop-{winner}-1")), "{merged}");
    let kept = format!("lane/{}", lost["key"].as_str().unwrap());
    assert!(branches(&dir).contains(&kept), "{:?}", branches(&dir));
    assert!(!dir.join(".lanes").join(lost["key"].as_str().unwrap()).exists());
    // The card is the winner's agent's from now.
    assert_eq!(card(&rig, &id).await["assignee"], format!("agent:shop-{winner}-1"));
    // And its history says what the test came to, once, naming that nothing held them apart.
    let history = rig.read(&format!("/api/records/{id}/history")).await.to_string();
    assert_eq!(history.matches("\"topic\":\"card.contested\"").count(), 1, "{history}");
    assert!(history.contains("not held apart"), "{history}");

    // Each contender's own start is no way of the comparison: only the work it calls is, so nothing
    // was started otherwise than it was written.
    assert_eq!(rig.count_of("instance.call_kept").await, 0);
    // What the judge said is counted under each way of the test's own comparison, which is over once
    // the card's review has passed, which it counts.
    let compared = crate::soon("the test's own comparison to be over", async {
        loop {
            let compared = rig.read(&format!("/api/comparisons/ab-{test}")).await;
            if compared["comparison"]["ended"].is_number() {
                return compared;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    let shown = compared.to_string();
    assert!(shown.contains("judged") && shown.contains("won") && shown.contains("lost"), "{shown}");
}

#[tokio::test]
async fn three_contenders_judged_by_a_council_of_three_have_the_losers_work_thrown_away_where_the_test_says()
{
    let (rig, at, dir) = tested(3).await;
    for judge in ["shop-judge-1", "shop-judge-2", "shop-judge-3"] {
        says(&at, judge, "A is the one.", "A");
    }
    let test = json!({
        "contenders": ["otter", "slate", "lark"], "judges": "judge", "seats": 3, "needed": 2, "losers": "discard",
    });
    let (id, test) = contested(&rig, test).await;
    played_to_the_end(&rig, &id).await;
    assert_eq!(card(&rig, &id).await["status"], "done", "{}", card(&rig, &id).await);

    let entries = entries(&rig, &test).await;
    assert_eq!(entries.keys().cloned().collect::<Vec<_>>(), ["A", "B", "C"], "{entries:?}");
    let verdicts: Vec<_> = entries.values().map(|entry| entry["verdict"].as_str().unwrap_or("")).collect();
    assert_eq!(verdicts, ["won", "lost", "lost"], "{entries:?}");
    // Each of the three judges said its word, each another agent.
    assert_eq!(entries["A"]["scores"].as_array().map(Vec::len), Some(3), "{entries:?}");
    for judge in ["shop-judge-1", "shop-judge-2", "shop-judge-3"] {
        let turns = rig.read(&format!("/api/agents/{judge}/turns")).await.to_string();
        assert!(turns.contains(&format!("judge:{id}:seat-")), "{judge}: {turns}");
    }
    // The losers' work is gone with their places; the winner's was merged.
    let branches = branches(&dir);
    for letter in ["B", "C"] {
        let key = entries[letter]["key"].as_str().unwrap();
        assert!(!branches.contains(&format!("lane/{key}")), "{key}: {branches:?}");
        assert!(!dir.join(".lanes").join(key).exists());
    }
    let winner = entries["A"]["way"].as_str().unwrap();
    assert!(git(&dir, &["log", "--format=%an", "-5"]).contains(&format!("shop-{winner}-1")));
}

#[tokio::test]
async fn scores_left_to_a_person_ask_on_the_card_and_the_entry_a_person_names_is_taken() {
    let (rig, at, dir) = tested(1).await;
    says(&at, "shop-judge-1", "A: 3\nB: 8", "A");
    let test = json!({ "contenders": ["otter", "slate"], "judges": "judge", "judging": "std/judge-scores" });
    let (id, test) = contested(&rig, test).await;
    // The scores are a person's to take: the card asks which.
    let card_id = id.clone();
    let asked = rig.event("card.asked", move |event| event.data["card"] == card_id.as_str()).await;
    let question = asked.data["question"].as_str().unwrap_or_default().to_string();
    assert!(question.contains('A') && question.contains('B'), "{}", asked.data);
    let row = rig.read(&format!("/api/records/{test}")).await;
    assert_eq!(row["data"]["state"], "asking", "{row}");
    let scored = entries(&rig, &test).await;
    assert!(scored.values().all(|entry| entry["scores"].as_array().is_some_and(|said| !said.is_empty())));

    // A person names B, and B's work is the card's.
    let said = rig.send(&format!("/api/records/{id}/comments"), json!({ "body": "B" })).await;
    assert!(said.get("error").is_none(), "{said}");
    played_to_the_end(&rig, &id).await;
    assert_eq!(card(&rig, &id).await["status"], "done", "{}", card(&rig, &id).await);
    let entries = entries(&rig, &test).await;
    assert_eq!(
        (entries["A"]["verdict"].as_str(), entries["B"]["verdict"].as_str()),
        (Some("lost"), Some("won")),
        "{entries:?}"
    );
    let winner = entries["B"]["way"].as_str().unwrap();
    assert!(git(&dir, &["log", "--format=%an", "-5"]).contains(&format!("shop-{winner}-1")));
    let row = rig.read(&format!("/api/records/{test}")).await;
    assert_eq!((row["data"]["state"].as_str(), row["data"]["winner"].as_str()), (Some("came"), Some(winner)));
}

#[tokio::test]
async fn a_test_is_set_up_only_by_whoever_could_give_the_fleets_agents_its_work() {
    let (rig, _at, _dir) = tested(1).await;
    let id = add(&rig, "Add a cart", &[]).await;
    let test = json!({ "fields": { "card": id, "contenders": ["otter", "slate"] } });
    let asks = ["projects", "channels", "board", "records", "blueprints", "fleet", "search"];
    let narrow = key(&rig, &asks).await;
    let (status, refused) = rig
        .call(Method::POST, "/api/collections/shop/contests/records", Some(&narrow), Some(test.clone()))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("`agents:write`"), "{}", said(&refused));
    // Nothing of it was laid into the card's list.
    assert!(rows(&rig, "contests", "card", &id).await.is_empty());

    let wider = key(&rig, &[&asks[..], &["agents"]].concat()).await;
    let (status, made) =
        rig.call(Method::POST, "/api/collections/shop/contests/records", Some(&wider), Some(test)).await;
    assert_eq!(status, StatusCode::OK, "{made}");
}

#[tokio::test]
async fn a_card_sent_back_from_review_after_its_test_goes_to_the_winners_agent() {
    let (rig, at, _dir) = tested(1).await;
    says(&at, "shop-judge-1", "A is the one.", "A");
    says(&at, "shop-reviewer-1", "The total is wrong.", "send_back");
    let (id, test) = contested(&rig, json!({ "contenders": ["otter", "slate"], "judges": "judge" })).await;
    let taken = moved(&rig, &id, "doing", 0).await;
    // Sent back from review.
    let back = moved(&rig, &id, "doing", taken).await;
    std::fs::remove_file(at.join("says-shop-reviewer-1")).unwrap();
    moved(&rig, &id, "done", back).await;

    // The test is done once: the round it was sent back to is the winner's agent's, as any card's.
    let entries = entries(&rig, &test).await;
    let winner = entries.values().find(|entry| entry["verdict"] == "won").unwrap()["way"].as_str().unwrap();
    let worked = given(&rig, &format!("shop-{winner}-1")).await;
    assert_eq!(worked.len(), 2, "{worked:?}");
    assert!(
        worked[1].starts_with("This card was sent back, for you to take up again")
            && worked[1].contains("shop-reviewer-1: The total is wrong."),
        "{}",
        worked[1]
    );
    assert!(given(&rig, "shop-worker-1").await.is_empty());
    assert_eq!(given(&rig, "shop-judge-1").await.len(), 1);
    assert_eq!(entries.len(), 2);
}

#[tokio::test]
async fn a_contender_that_is_a_projects_own_fork_of_the_work_works_the_card_and_is_counted_under_its_way() {
    let (rig, at, _dir) = tested(1).await;
    says(&at, "shop-judge-1", "A is the one.", "A");
    // The project's own text of the work, called otherwise, saved as the owner.
    let shipped = include_str!("../../../../blueprints/std/work.bp");
    let fork = shipped.replacen("blueprint: std/work", "blueprint: std/mywork", 1).replacen(
        "version: 8",
        "version: 1",
        1,
    );
    assert_ne!(fork, shipped);
    let saved =
        json!({ "path": format!("@blueprints/std/{}", brand::blueprint_file("mywork")), "content": fork });
    let (status, kept) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    // A comparison of the two, shared: one way a role, the other the fork worked by another role.
    let ways = json!([
        { "name": "otter", "with": { "role": "otter" } },
        { "name": "mine", "blueprint": "std/mywork", "with": { "role": "slate" } },
    ]);
    let asked = json!({ "name": "forked", "project": "shop", "of": "std/work", "per": "entry", "asked": true, "ways": ways });
    let started = rig.send("/api/comparisons", asked).await;
    assert!(started.get("error").is_none(), "{started}");

    let test = json!({ "contenders": ["otter", "mine"], "compared": "forked", "judges": "judge" });
    let (id, test) = contested(&rig, test).await;
    played_to_the_end(&rig, &id).await;
    assert_eq!(card(&rig, &id).await["status"], "done", "{}", card(&rig, &id).await);

    // The fork's entry worked the card, by the role its way hands it.
    let entries = entries(&rig, &test).await;
    let mine = entries.values().find(|entry| entry["way"] == "mine").unwrap();
    assert_eq!(
        (mine["state"].as_str(), mine["agent"].as_str()),
        (Some("kept"), Some("shop-slate-1")),
        "{entries:?}"
    );
    assert!(given(&rig, "shop-slate-1").await.len() == 1);
    // Run as the fork's text and no other.
    let runs = rig.read("/api/instances?limit=200").await.to_string();
    assert!(runs.contains("std/mywork"), "{runs}");
    // And counted under its way of the shared comparison, which goes on.
    let compared = rig.read("/api/comparisons/forked").await;
    assert!(compared["comparison"]["ended"].is_null(), "{compared}");
    let shown = compared.to_string();
    assert!(shown.contains("\"mine\"") && shown.contains("judged"), "{shown}");
}

#[tokio::test]
async fn a_council_that_talks_it_over_takes_another_stretch_until_it_agrees_each_seat_shown_what_was_said() {
    // A judge that is not settled until it has heard the council once, and then takes A.
    let (rig, at) = fleets::rig_told(|config| {
        let talks = [
            "read",
            "where",
            "Stretch 1:",
            "(",
            "say",
            "Having heard the others, A.",
            "say",
            "DECISION: A",
            ")",
            "say",
            "I would hear the others first.",
        ];
        config.adapters.push(("talker".into(), crate::standin::argv(&talks)));
    })
    .await;
    let (rig, _at, dir) = given_ab(rig, at, 2, "talker").await;
    let test = json!({ "contenders": ["otter", "slate"], "judges": "judge", "seats": 2, "council": "talks" });
    let (id, test) = contested(&rig, test).await;
    played_to_the_end(&rig, &id).await;
    assert_eq!(card(&rig, &id).await["status"], "done", "{}", card(&rig, &id).await);

    // Two stretches: in the first neither would take an entry, and in the second, shown that, both took A.
    for judge in ["shop-judge-1", "shop-judge-2"] {
        let asked = given(&rig, judge).await;
        assert_eq!(asked.len(), 2, "{judge}: {asked:?}");
        assert!(
            asked[0].contains("This is stretch\n1") || asked[0].contains("This is stretch 1"),
            "{}",
            asked[0]
        );
        assert!(!asked[0].contains("What the council has said so far"), "{}", asked[0]);
        assert!(
            asked[1].contains("What the council has said so far")
                && asked[1].contains("(would take none yet): I would hear the others first."),
            "{}",
            asked[1]
        );
    }
    let entries = entries(&rig, &test).await;
    assert_eq!(
        (entries["A"]["verdict"].as_str(), entries["B"]["verdict"].as_str()),
        (Some("won"), Some("lost")),
        "{entries:?}"
    );
    // What the seats that took it said is kept on the winner's row.
    let said = entries["A"]["scores"].to_string();
    assert!(said.contains("Having heard the others, A."), "{said}");
    let winner = entries["A"]["way"].as_str().unwrap();
    assert!(git(&dir, &["log", "--format=%an", "-5"]).contains(&format!("shop-{winner}-1")));
}

#[tokio::test]
async fn a_card_sent_back_while_its_list_still_says_its_test_is_worked_by_the_winners_agent_and_not_the_stages_role()
 {
    // No reviewer: the card waits in review for a person, who sends it back.
    let (rig, at) = fleet_rig().await;
    let mut rows = agents(1, "critic");
    rows.as_array_mut().unwrap().retain(|row| row["title"] != "reviewer");
    let dir = shop(&rig, json!({ "agents": rows })).await;
    let ab =
        json!({ "blueprint": "std/ab", "project": "shop", "params": { "project": "shop" }, "wait": true });
    assert_eq!(rig.send("/api/apply", ab).await["finished"]["status"], "ok");
    says(&at, "shop-judge-1", "A is the one.", "A");
    let (id, test) = contested(&rig, json!({ "contenders": ["otter", "slate"], "judges": "judge" })).await;
    let in_review = moved(&rig, &id, "review", 0).await;
    // The card's list handed to the winner, and then put back as a stop before the fleet had it leaves it.
    let laid = rows_called(&rig, &format!("ab-{test}")).await;
    if laid["data"]["does"] != "std/work" {
        rig.event("record.updated", |event| event.data["collection"] == "segments").await;
    }
    let laid = rows_called(&rig, &format!("ab-{test}")).await;
    assert_eq!(laid["data"]["does"], "std/work", "{laid}");
    let before = rig.mark();
    let put = json!({ "fields": { "does": "std/contest", "with": [] } });
    let (status, said) = rig
        .call(
            Method::PATCH,
            &format!("/api/records/{}", laid["id"].as_str().unwrap()),
            Some(&rig.token),
            Some(put),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    rig.event("fleet.arranged", move |event| event.seq > before).await;

    // Sent back by a person: worked again by the winner's agent, told why, and by nobody of the stage's role.
    move_card(&rig, &id, "doing", Some("The total is wrong.")).await;
    moved(&rig, &id, "review", in_review).await;
    let entries = entries(&rig, &test).await;
    let winner = entries.values().find(|entry| entry["verdict"] == "won").unwrap()["way"].as_str().unwrap();
    let worked = given(&rig, &format!("shop-{winner}-1")).await;
    assert_eq!(worked.len(), 2, "{worked:?}");
    assert!(worked[1].contains("The total is wrong."), "{}", worked[1]);
    assert!(given(&rig, "shop-worker-1").await.is_empty());
    assert!(dir.join(".lanes").join(&id).exists());
    // Worked from the test's own segment, the card's list not yet saying otherwise.
    let ran = rig.read("/api/instances?limit=500").await.to_string();
    assert!(ran.contains(&format!("again-{id}")), "{ran}");
}

/// The row of the card's list called `title`.
async fn rows_called(rig: &Rig, title: &str) -> Value {
    rows(rig, "segments", "title", title).await.into_iter().next().unwrap_or(Value::Null)
}

#[tokio::test]
async fn a_test_in_a_project_that_is_no_repository_is_not_done_and_its_card_is_left_saying_why() {
    let (rig, at, dir) = tested(1).await;
    says(&at, "shop-judge-1", "A is the one.", "A");
    // No repository with history (one with none at all is found as the checkout the tests are in):
    // every contender would work in the project's own folder, over the others.
    std::fs::remove_dir_all(dir.join(".git")).unwrap();
    git(&dir, &["init", "--quiet", "."]);
    let (id, test) = contested(&rig, json!({ "contenders": ["otter", "slate"], "judges": "judge" })).await;
    let card_id = id.clone();
    rig.event("card.left", move |event| event.data["card"] == card_id.as_str()).await;
    let entries = entries(&rig, &test).await;
    assert!(entries.values().all(|entry| entry["state"] == "failed"), "{entries:?}");
    let row = rig.read(&format!("/api/records/{test}")).await;
    assert_eq!(row["data"]["state"], "left", "{row}");
    // Nobody worked the card, and nothing was taken.
    assert!(given(&rig, "shop-otter-1").await.is_empty() && given(&rig, "shop-slate-1").await.is_empty());
    let left = card(&rig, &id).await.to_string();
    assert!(!left.contains("no place to give"), "{left}");
}

#[tokio::test]
async fn a_shadow_tests_incumbent_goes_on_though_the_judge_takes_the_challenger_whose_work_is_only_judged_and_kept()
 {
    let (rig, at, dir) = tested(1).await;
    // The judge knows the entries by their ways, and takes the challenger.
    says(&at, "shop-judge-1", "slate does what the card asks; otter does not.", "slate");
    let test = json!({ "contenders": ["slate"], "shadow": "otter", "judges": "judge", "known_by": "ways" });
    let (id, test) = contested(&rig, test).await;
    played_to_the_end(&rig, &id).await;
    assert_eq!(card(&rig, &id).await["status"], "done", "{}", card(&rig, &id).await);

    // Both worked it, beside each other, and both were judged: the judge took the challenger.
    let entries = entries(&rig, &test).await;
    assert_eq!(entries.keys().cloned().collect::<Vec<_>>(), ["otter", "slate"], "{entries:?}");
    let (otter, slate) = (&entries["otter"], &entries["slate"]);
    assert_eq!(
        (otter["verdict"].as_str(), slate["verdict"].as_str()),
        (Some("lost"), Some("won")),
        "{entries:?}"
    );
    assert_eq!(
        (otter["incumbent"].as_bool(), slate["incumbent"].as_bool()),
        (Some(true), None),
        "{entries:?}"
    );
    assert!(!slate["scores"].as_array().unwrap().is_empty(), "{entries:?}");
    // The incumbent's work is the card's all the same: merged, and the card its agent's.
    let row = rig.read(&format!("/api/records/{test}")).await;
    let said = |key: &str| row["data"][key].as_str().unwrap_or_default().to_string();
    assert_eq!(
        (said("state"), said("winner"), said("agent")),
        ("came".into(), "otter".into(), "shop-otter-1".into())
    );
    assert!(
        said("why").starts_with("a shadow test") && said("why").contains("the judges took slate"),
        "{row}"
    );
    let merged = git(&dir, &["log", "--format=%an", "-5"]);
    assert!(merged.contains("shop-otter-1") && !merged.contains("shop-slate-1"), "{merged}");
    assert_eq!(card(&rig, &id).await["assignee"], "agent:shop-otter-1");
    // The challenger's is never the card's: kept as its branch, its place gone.
    let key = slate["key"].as_str().unwrap();
    assert!(branches(&dir).contains(&format!("lane/{key}")), "{:?}", branches(&dir));
    assert!(!dir.join(".lanes").join(key).exists());
    // Its history says it was a shadow test, what the judge took and that the incumbent went on, once.
    let history = rig.read(&format!("/api/records/{id}/history")).await.to_string();
    assert_eq!(history.matches("\"topic\":\"card.contested\"").count(), 1, "{history}");
    assert!(
        history.contains("its shadow test of") && history.contains("its incumbent, otter, goes on"),
        "{history}"
    );
    // What the judge said is counted under each way; and the card, whose work is the incumbent's, is
    // written down as that way's, so that its review passing is the incumbent's accepted card and not
    // the challenger's the judge took. The test's own comparison is then over.
    let compared = crate::soon("the shadow test's own comparison to be over", async {
        loop {
            let compared = rig.read(&format!("/api/comparisons/ab-{test}")).await;
            if compared["comparison"]["ended"].is_number() {
                return compared;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    let shown = compared.to_string();
    assert!(shown.contains("judged") && shown.contains("won") && shown.contains("lost"), "{shown}");
    let columns = compared["beside"]["columns"].as_array().cloned().unwrap_or_default();
    let column = |way: &str| columns.iter().position(|column| column["way"] == way).unwrap();
    let accepted =
        compared["beside"]["rows"].as_array().unwrap().iter().find(|row| row["kind"] == "accepted");
    let by = &accepted.unwrap_or_else(|| panic!("{shown}"))["by"];
    assert_eq!(
        (&by[column("otter")]["count"], &by[column("slate")]["count"]),
        (&json!(1), &json!(0)),
        "{shown}"
    );
}

#[tokio::test]
async fn a_shadow_test_beside_the_usual_setup_whose_challengers_are_only_counted_asks_no_judge_and_throws_their_work_away()
 {
    let (rig, at, dir) = tested(1).await;
    says(&at, "shop-judge-1", "A is the one.", "A");
    // The usual setup is the stage's role, which the contenders leave out.
    let test = json!({
        "contenders": ["slate"], "shadow": "worker", "challengers": "counted", "judges": "judge", "losers": "discard",
    });
    let (id, test) = contested(&rig, test).await;
    played_to_the_end(&rig, &id).await;
    assert_eq!(card(&rig, &id).await["status"], "done", "{}", card(&rig, &id).await);

    // Each worked it once; nobody judged, and nothing was said of either.
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 1);
    assert_eq!(given(&rig, "shop-slate-1").await.len(), 1);
    assert!(given(&rig, "shop-judge-1").await.is_empty());
    let entries = rows(&rig, "entries", "contest", &test).await;
    assert_eq!(entries.len(), 2, "{entries:?}");
    assert!(
        entries.iter().all(|entry| entry["data"]["verdict"].as_str().unwrap_or("").is_empty()
            && entry["data"]["state"] == "kept"),
        "{entries:?}"
    );
    // The usual setup's work went on, and the challenger's is gone with its place.
    let row = rig.read(&format!("/api/records/{test}")).await;
    assert_eq!(
        (row["data"]["winner"].as_str(), row["data"]["state"].as_str()),
        (Some("worker"), Some("came"))
    );
    let merged = git(&dir, &["log", "--format=%an", "-5"]);
    assert!(merged.contains("shop-worker-1") && !merged.contains("shop-slate-1"), "{merged}");
    let slate = entries.iter().find(|entry| entry["data"]["way"] == "slate").unwrap()["data"]["key"].clone();
    let slate = slate.as_str().unwrap();
    assert!(!branches(&dir).contains(&format!("lane/{slate}")), "{:?}", branches(&dir));
    let history = rig.read(&format!("/api/records/{id}/history")).await.to_string();
    assert!(history.contains("was counted and not judged"), "{history}");
}

/// A fleet with A/B tests whose `slate` cannot finish: its stand-in says so
/// and ends saying it was blocked, so that its work is not kept.
async fn one_kept() -> (Rig, PathBuf, PathBuf) {
    let quits = ["say", "I could not finish: the tests would not run.", "say", "DECISION: blocked"];
    let (rig, at) =
        fleets::rig_told(|config| config.adapters.push(("quitter".into(), crate::standin::argv(&quits))))
            .await;
    let mut agents = agents(1, "critic");
    agents[2]["adapter"] = json!("quitter");
    let dir = shop(&rig, json!({ "agents": agents })).await;
    let ab =
        json!({ "blueprint": "std/ab", "project": "shop", "params": { "project": "shop" }, "wait": true });
    assert_eq!(rig.send("/api/apply", ab).await["finished"]["status"], "ok");
    (rig, at, dir)
}

/// What the card was asked, once its test asks a person, after `after`.
async fn asked_of(rig: &Rig, id: &str, after: u64) -> String {
    let card = id.to_string();
    let asked =
        rig.event("card.asked", move |event| event.seq > after && event.data["card"] == card.as_str()).await;
    asked.data["question"].as_str().unwrap_or_default().to_string()
}

#[tokio::test]
async fn a_test_that_kept_one_entry_asks_a_person_saying_what_it_is_and_takes_it_on_yes_and_leaves_the_card_on_no()
 {
    let (rig, at, _dir) = one_kept().await;
    says(&at, "shop-judge-1", "A is the one.", "A");
    let (id, test) = contested(&rig, json!({ "contenders": ["otter", "slate"], "judges": "judge" })).await;
    let question = asked_of(&rig, &id, 0).await;
    let entries = entries(&rig, &test).await;
    let (otter, slate) = {
        let of = |way: &str| entries.values().find(|entry| entry["way"] == way).unwrap().clone();
        (of("otter"), of("slate"))
    };
    assert_eq!(
        (otter["state"].as_str(), slate["state"].as_str()),
        (Some("kept"), Some("unkept")),
        "{entries:?}"
    );
    // What the card asks says, of the entry it names, its letter, whose work it is, the agent, its
    // branch and how much it changed; of the other, why it was not kept and what its agent said.
    let lettered = format!(
        "Only otter's work could be kept: {} is otter's work (shop-otter-1, branch {}, 1 file, +",
        otter["letter"].as_str().unwrap(),
        otter["branch"].as_str().unwrap()
    );
    assert!(question.starts_with(&lettered), "{question}");
    assert!(
        question.contains(
            "; slate's was not kept (it ended with nothing kept; its agent said \u{201c}I could not finish: the tests would not run.\u{201d})"
        ),
        "{question}"
    );
    assert!(
        question
            .ends_with("Take otter's work? Each is marked in the card's A/B testing tab. Answer yes or no."),
        "{question}"
    );

    // A person says yes, and that one's work is the card's.
    let said = rig.send(&format!("/api/records/{id}/comments"), json!({ "body": "yes" })).await;
    assert!(said.get("error").is_none(), "{said}");
    played_to_the_end(&rig, &id).await;
    assert_eq!(card(&rig, &id).await["status"], "done", "{}", card(&rig, &id).await);
    let row = rig.read(&format!("/api/records/{test}")).await;
    assert_eq!(
        (row["data"]["state"].as_str(), row["data"]["winner"].as_str()),
        (Some("came"), Some("otter"))
    );
    assert_eq!(card(&rig, &id).await["assignee"], "agent:shop-otter-1");

    // Another card, and a person says no: nothing is taken, and the card is left for a person, saying so.
    let before = rig.mark();
    let (other, test) = contested(&rig, json!({ "contenders": ["otter", "slate"], "judges": "judge" })).await;
    asked_of(&rig, &other, before).await;
    let said = rig.send(&format!("/api/records/{other}/comments"), json!({ "body": "No." })).await;
    assert!(said.get("error").is_none(), "{said}");
    let card_id = other.clone();
    let left = rig
        .event("card.left", move |event| event.seq > before && event.data["card"] == card_id.as_str())
        .await;
    let why = left.data["why"].as_str().unwrap_or_default();
    assert!(why.contains("otter's work is not taken"), "{}", left.data);
    let row = rig.read(&format!("/api/records/{test}")).await;
    let winner = row["data"]["winner"].as_str().unwrap_or("");
    assert_eq!((row["data"]["state"].as_str(), winner), (Some("left"), ""), "{row}");
    assert!(
        entries_of(&rig, &test).await.iter().all(|entry| entry["verdict"].as_str().unwrap_or("").is_empty())
    );
}

async fn entries_of(rig: &Rig, test: &str) -> Vec<Value> {
    entries(rig, test).await.into_values().collect()
}

#[tokio::test]
async fn a_contender_is_told_how_the_card_is_finished_where_it_is_worked_and_is_held_to_it_as_the_worker_is()
{
    let (rig, _at, _dir) = tested(1).await;
    fleets::change(&rig, "stages", "work", json!({ "finish": ["its tests pass"] })).await;
    let (id, test) = contested(&rig, json!({ "contenders": ["otter", "slate"], "judges": "judge" })).await;
    played_to_the_end(&rig, &id).await;
    // Neither says anything of the step: each is given it again once,
    // told what is left, and its work is not kept.
    for agent in ["shop-otter-1", "shop-slate-1"] {
        let asked = given(&rig, agent).await;
        assert_eq!(asked.len(), 2, "{agent}: {asked:?}");
        assert!(asked[0].starts_with(&format!("Before you say {id} is done")), "{}", asked[0]);
        assert!(asked[1].starts_with(&format!("You said {id} was done")), "{}", asked[1]);
    }
    let states: Vec<Value> = entries(&rig, &test).await.values().map(|row| row["state"].clone()).collect();
    assert_eq!(states, [json!("unkept"), json!("unkept")], "{states:?}");
}
