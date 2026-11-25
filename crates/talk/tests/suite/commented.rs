//! Somebody named in what is said on a record (`talk::commented`): told
//! in their box where they read the record as things stand, once however
//! often the comment is heard, by a line that keeps no words, opens the
//! record and is passed over once they may read it no longer.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use kernel::Principal;
use talk::commented::{Comment, OnRecord, OnRecords};
use talk::followed::Change;
use talk::inbox::Kind;

use crate::kept::who;
use crate::{Rig, agent};

/// What a rig's records answer: what reading each takes, by its id, and
/// the words said on each, by the comment's number.
#[derive(Default)]
struct Records {
    takes: Mutex<HashMap<String, Vec<String>>>,
    said: HashMap<u64, String>,
}

impl OnRecords for Records {
    fn takes(&self, record: &str) -> Option<Vec<String>> {
        self.takes.lock().unwrap().get(record).cloned()
    }

    fn said(
        &self,
        by: Principal,
        record: String,
        seq: u64,
    ) -> Pin<Box<dyn Future<Output = Option<OnRecord>> + Send>> {
        let takes = self.takes(&record);
        let body = self.said.get(&seq).cloned();
        Box::pin(async move {
            let reads = takes?.iter().all(|scope| kernel::principal::covers(&by.scopes, scope));
            let body = body.filter(|_| reads)?;
            Some(OnRecord {
                record,
                project: "shop".into(),
                table: "board".into(),
                title: Some("Fix the login".into()),
                path: Vec::new(),
                who: "Ada".into(),
                body,
                topic: "record.commented".into(),
                did: None,
                seq,
                created: 1,
            })
        })
    }
}

const CARD: &str = "shop-4";
const BOARD: &[&str] = &["board:read"];

/// A rig that knows these people by their ids, what they and the agents
/// hold, which agents the factory does not start, and the records.
async fn rig_of(people: &[Principal], agents: &[Principal], outside: &[&str], records: Arc<Records>) -> Rig {
    let rig = Rig::new().await;
    let named = people.to_vec();
    rig.channels.names_with(Arc::new(move |_, names| {
        Ok(named.iter().filter(|one| names.contains(&one.id)).cloned().collect())
    }));
    let all: Vec<Principal> = people.iter().chain(agents).cloned().collect();
    rig.channels.inbox.holds_with(Arc::new(move |_, actor| {
        Ok(all.iter().find(|one| one.actor() == actor).map(|one| one.scopes.clone()).unwrap_or_default())
    }));
    let outside: Vec<String> = outside.iter().map(|name| format!("agent:{name}")).collect();
    rig.channels.outside_with(Arc::new(move |actor| outside.iter().any(|one| one == actor)));
    rig.channels.inbox.records_with(records);
    rig
}

fn records(said: &[(u64, &str)]) -> Arc<Records> {
    let records = Records {
        said: said.iter().map(|(seq, body)| (*seq, body.to_string())).collect(),
        ..Default::default()
    };
    records.takes.lock().unwrap().insert(CARD.into(), BOARD.iter().map(|scope| scope.to_string()).collect());
    Arc::new(records)
}

fn comment(seq: u64, body: &str, by: &str) -> Comment {
    Comment {
        record: CARD.into(),
        seq,
        from: by.into(),
        by: by.into(),
        body: body.into(),
        takes: BOARD.iter().map(|scope| scope.to_string()).collect(),
        at: 10,
        followers: Vec::new(),
        above: Vec::new(),
    }
}

#[tokio::test]
async fn whoever_a_comment_names_and_may_read_the_record_is_told_once_by_a_line_that_keeps_no_words() {
    let mira = who("mira", &["board:read", "inbox"]);
    let nell = who("nell", &["records", "inbox"]);
    let ada = who("ada", &["*"]);
    let rig = rig_of(
        &[mira.clone(), nell.clone(), ada.clone()],
        &[],
        &[],
        records(&[(7, "@mira and @nell, look")]),
    )
    .await;
    let told = rig.channels.tell_commented(comment(7, "@mira and @nell, look", "user:ada")).await.unwrap();
    // Mira reads the board; Nell holds no reading of it, and is told nothing.
    assert_eq!(told, 1);
    let lines = rig.channels.inbox.list(&mira, false).await.unwrap();
    assert_eq!(lines.len(), 1);
    let line = &lines[0];
    assert_eq!(
        (line.kind, line.about.as_str(), line.place.as_str(), line.from.as_str()),
        (Kind::Commented, "7", CARD, "user:ada")
    );
    assert_eq!(line.says, None);
    assert!(rig.channels.inbox.list(&nell, false).await.unwrap().is_empty());
    // Heard again (a daemon started again before it kept where it had read to): nothing more.
    assert_eq!(
        rig.channels.tell_commented(comment(7, "@mira and @nell, look", "user:ada")).await.unwrap(),
        0
    );
    assert_eq!(rig.events("inbox.told").await.len(), 1);
    // No words in what is said of the box; they are shown from the record as its owner reads it.
    assert!(!rig.events("inbox.told").await[0].data.to_string().contains("look"));
    let (shown, unread) = rig.channels.inbox_of(&mira, false).await.unwrap();
    assert_eq!(unread, 1);
    let on = shown[0].on_record.as_ref().unwrap();
    assert_eq!((on.body.as_str(), on.seq, on.record.as_str()), ("@mira and @nell, look", 7, CARD));
    // Another comment that names her is a line of its own.
    rig.channels.tell_commented(comment(9, "@mira again", "user:ada")).await.unwrap();
    assert_eq!(rig.channels.inbox.unread(&mira).await.unwrap(), 2);
}

#[tokio::test]
async fn whoever_said_it_is_not_told_and_a_comment_that_names_nobody_writes_nothing() {
    let mira = who("mira", &["board:read", "inbox"]);
    let rig = rig_of(std::slice::from_ref(&mira), &[], &[], records(&[])).await;
    assert_eq!(rig.channels.tell_commented(comment(3, "a note to @mira", "user:mira")).await.unwrap(), 0);
    // Said with a key of hers: the key is who said it, and she whose doing it was.
    let keyed = Comment { from: "key:cred_1".into(), ..comment(4, "@mira", "user:mira") };
    assert_eq!(rig.channels.tell_commented(keyed).await.unwrap(), 0);
    assert_eq!(
        rig.channels.tell_commented(comment(5, "nobody named, mira@example.com", "user:ada")).await.unwrap(),
        0
    );
    assert!(rig.events("inbox.*").await.is_empty());
}

#[tokio::test]
async fn an_agent_the_factory_does_not_start_is_told_as_a_person_and_one_it_starts_is_given_nothing() {
    let scout = agent("scout", &["board:read", "inbox"]);
    let worker = agent("worker", &["board:read", "inbox"]);
    let blind = agent("blind", &["inbox"]);
    let rig = rig_of(
        &[],
        &[scout.clone(), worker.clone(), blind.clone()],
        &["scout", "blind"],
        records(&[(2, "@scout @worker @blind")]),
    )
    .await;
    assert_eq!(
        rig.channels.tell_commented(comment(2, "@scout @worker @blind", "user:ada")).await.unwrap(),
        1
    );
    assert_eq!(rig.channels.inbox.list(&scout, false).await.unwrap().len(), 1);
    // One the factory starts is no box's owner by being named on a record; one that may not read the record is told nothing.
    assert!(rig.channels.inbox.list(&worker, false).await.unwrap().is_empty());
    assert!(rig.channels.inbox.list(&blind, false).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_line_of_a_record_its_owner_may_no_longer_read_is_passed_over_and_comes_back_when_they_may() {
    let mira = who("mira", &["board:read", "inbox"]);
    let held = records(&[(7, "@mira look")]);
    let rig = rig_of(std::slice::from_ref(&mira), &[], &[], held.clone()).await;
    rig.channels.tell_commented(comment(7, "@mira look", "user:ada")).await.unwrap();
    assert_eq!(rig.channels.inbox.unread(&mira).await.unwrap(), 1);
    // Its table is kept private: the line is not listed, not counted and not found by its id.
    held.takes.lock().unwrap().insert(CARD.into(), vec!["board:read".into(), "private:shop".into()]);
    let id = rig
        .channels
        .inbox
        .list(&Principal { scopes: vec!["*".into()], ..mira.clone() }, false)
        .await
        .unwrap()[0]
        .id
        .clone();
    assert!(rig.channels.inbox.list(&mira, false).await.unwrap().is_empty());
    assert_eq!(rig.channels.inbox.unread(&mira).await.unwrap(), 0);
    assert!(rig.channels.inbox_line(&mira, &id).await.is_err());
    // A record no table owns any more is passed over too.
    held.takes.lock().unwrap().remove(CARD);
    assert!(rig.channels.inbox.list(&mira, false).await.unwrap().is_empty());
    // And it is as it was once she may read it again.
    held.takes.lock().unwrap().insert(CARD.into(), vec!["board:read".into()]);
    let (shown, unread) = rig.channels.inbox_of(&mira, false).await.unwrap();
    assert_eq!((shown.len(), unread), (1, 1));
}

#[tokio::test]
async fn a_follower_is_told_by_one_line_of_the_page_which_each_change_raises_and_a_change_heard_again_does_not()
 {
    let mira = who("mira", &["board:read", "inbox"]);
    let nell = who("nell", &["inbox"]);
    let rig = rig_of(&[mira.clone(), nell.clone()], &[], &[], records(&[(7, "Linking."), (9, "")])).await;
    let followers = vec![mira.actor(), nell.actor(), "user:ada".to_string()];
    let said = Comment { followers: followers.clone(), ..comment(7, "Linking.", "user:ada") };
    // Mira follows and reads the page; Nell follows and may not; Ada made the change.
    assert_eq!(rig.channels.tell_commented(said.clone()).await.unwrap(), 1);
    let line = rig.channels.inbox.list(&mira, false).await.unwrap()[0].clone();
    assert_eq!(
        (line.kind, line.about.as_str(), line.place.as_str(), line.why.as_str(), line.said_by.as_deref()),
        (Kind::Followed, CARD, CARD, "follows", Some("7"))
    );
    assert!(rig.channels.inbox.list(&nell, false).await.unwrap().is_empty());
    // Another change raises the same line; one heard again, or one before it, raises nothing.
    let revised = Change { seq: 9, ..said.change() };
    assert_eq!(rig.channels.tell_followed(revised.clone()).await.unwrap(), 1);
    assert_eq!(rig.channels.tell_followed(revised).await.unwrap(), 0);
    assert_eq!(rig.channels.tell_commented(said).await.unwrap(), 0);
    let lines = rig.channels.inbox.list(&mira, false).await.unwrap();
    assert_eq!((lines.len(), lines[0].count, lines[0].said_by.as_deref()), (1, 2, Some("9")));
    assert_eq!(rig.events("inbox.told").await.len(), 2);
    // Shown with the change that raised it last.
    let (shown, _) = rig.channels.inbox_of(&mira, false).await.unwrap();
    assert_eq!(shown[0].on_record.as_ref().map(|on| on.seq), Some(9));
}

#[tokio::test]
async fn a_follower_named_in_a_reply_is_told_by_their_line_of_the_page_and_not_named_besides() {
    let mira = who("mira", &["board:read", "inbox"]);
    let nell = who("nell", &["board:read", "inbox"]);
    let rig =
        rig_of(&[mira.clone(), nell.clone()], &[], &[], records(&[(7, "@mira @nell which tag?")])).await;
    let said = Comment { followers: vec![mira.actor()], ..comment(7, "@mira @nell which tag?", "user:ada") };
    assert_eq!(rig.channels.tell_commented(said).await.unwrap(), 2);
    let kinds = |lines: Vec<talk::inbox::Line>| lines.into_iter().map(|line| line.kind).collect::<Vec<_>>();
    assert_eq!(kinds(rig.channels.inbox.list(&mira, false).await.unwrap()), [Kind::Followed]);
    assert_eq!(kinds(rig.channels.inbox.list(&nell, false).await.unwrap()), [Kind::Commented]);
}
