//! What is attached to a record (`studio::attaching`): a file of its own
//! beside the database, its row and what is said of it one write, into
//! the Trash and back with its record, gone with it for good, and bounded.

use crate::{Rig, ada, agent};

async fn carded(rig: &Rig) -> String {
    rig.board("shop").await;
    let card = rig.studio.board.add("shop", "Fix the till", "", None, vec![], &ada()).await.unwrap();
    card.id
}

#[tokio::test]
async fn a_file_attached_to_a_card_is_kept_beside_it_said_once_and_read_back_by_whoever_may_read_the_card() {
    let rig = Rig::new().await;
    let card = carded(&rig).await;
    let records = &rig.studio.records;
    let kept =
        records.attach(&card, ("till.png", Some("image/png")), b"png".to_vec(), &ada(), None).await.unwrap();
    assert_eq!((kept.name.as_str(), kept.media.as_str(), kept.size), ("till.png", "image/png", 3));
    assert!(studio::attaching::is_of_a_record(&kept.id), "{}", kept.id);
    // A log says what it is by its name, where nothing else says.
    let log = records.attach(&card, ("run.log", None), b"ok".to_vec(), &ada(), None).await.unwrap();
    assert_eq!(log.media, "text/plain");
    let listed = records.attachments(&card, &ada()).await.unwrap();
    assert_eq!(listed.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["till.png", "run.log"]);
    let (found, file) = records.attachment(&kept.id, &ada()).await.unwrap();
    assert_eq!((found.id.as_str(), std::fs::read(&file).unwrap()), (kept.id.as_str(), b"png".to_vec()));
    let said = rig.events("record.attached").await;
    assert_eq!(said.len(), 2);
    assert_eq!(
        (said[0].data["id"].as_str(), said[0].data["name"].as_str()),
        (Some(card.as_str()), Some("till.png"))
    );

    // Whoever may read the board reads it, and may not add or take away.
    let reader = agent("looker", &["board:read"]);
    assert_eq!(records.attachments(&card, &reader).await.unwrap().len(), 2);
    assert!(records.attachment(&kept.id, &reader).await.is_ok());
    assert!(records.attach(&card, ("mine.txt", None), b"x".to_vec(), &reader, None).await.is_err());
    assert!(records.detach(&card, &kept.id, &reader, None).await.is_err());
    // Whoever may read neither the board nor records is told of nothing.
    let nobody = agent("nobody", &["channels:read"]);
    assert!(records.attachments(&card, &nobody).await.is_err());
    assert!(records.attachment(&kept.id, &nobody).await.is_err());

    // Taken away, it is said, and its file goes.
    let (_, trashed) = records.detach(&card, &kept.id, &ada(), None).await.unwrap();
    assert_eq!(trashed, None, "where the factory keeps no Trash of files, it goes at once");
    assert!(!file.exists());
    assert_eq!(rig.topics("record.detached").await, ["record.detached"]);
    assert_eq!(records.attachments(&card, &ada()).await.unwrap().len(), 1);
}

#[tokio::test]
async fn what_is_attached_is_bounded_and_nothing_is_kept_of_what_is_refused() {
    let rig = Rig::new().await;
    let card = carded(&rig).await;
    let records = &rig.studio.records;
    let empty = records.attach(&card, ("empty.txt", None), Vec::new(), &ada(), None).await.unwrap_err();
    assert!(empty.message.contains("nothing in that file"), "{}", empty.message);
    let big = vec![b'x'; studio::attachments::MAX_ATTACHMENT_BYTES + 1];
    let refused = records.attach(&card, ("big.log", None), big, &ada(), None).await.unwrap_err();
    assert!(refused.message.contains("too large to attach"), "{}", refused.message);
    for at in 0..studio::attaching::MOST {
        records.attach(&card, (&format!("{at}.txt"), None), b"x".to_vec(), &ada(), None).await.unwrap();
    }
    let more = records.attach(&card, ("more.txt", None), b"x".to_vec(), &ada(), None).await.unwrap_err();
    assert!(more.message.contains("the most a record carries"), "{}", more.message);
    assert_eq!(records.attachments(&card, &ada()).await.unwrap().len(), studio::attaching::MOST);
    // A file of the folder for each kept, and none for what was refused.
    let files = std::fs::read_dir(rig.dir.join("attachments")).unwrap().count();
    assert_eq!(files, studio::attaching::MOST);
}

#[tokio::test]
async fn a_card_in_the_trash_keeps_what_is_attached_unread_has_it_again_given_back_and_takes_it_away_for_good()
 {
    let rig = Rig::new().await;
    let card = carded(&rig).await;
    let records = &rig.studio.records;
    let kept = records.attach(&card, ("till.png", None), b"png".to_vec(), &ada(), None).await.unwrap();
    rig.studio.board.remove(&card, &ada()).await.unwrap();
    assert!(
        records.attachment(&kept.id, &ada()).await.is_err(),
        "nothing reads it while its card is not there"
    );
    let (_, file) = (kept.id.clone(), rig.dir.join("attachments").join(&kept.id));
    assert!(file.exists(), "kept in the Trash with its card");
    let rows = rig.journal.records_of(Some("shop"));
    let listed = studio::trash::rows(&rows, None).await.unwrap().pop().unwrap();
    let entry = studio::trash::rows(&rows, Some(listed.id)).await.unwrap().pop().unwrap();
    rig.studio.collections.restore_record(&rows, &entry, &ada()).await.unwrap();
    assert!(records.attachment(&kept.id, &ada()).await.is_ok(), "given back with its card");

    // Gone for good, what was attached goes with it.
    assert_eq!(rig.studio.collections.forget_attached("shop", &card).await.unwrap(), 1);
    assert!(!file.exists());
    assert!(records.attachments(&card, &ada()).await.unwrap().is_empty());

    // Where the factory keeps nothing that is deleted, it goes at once.
    let again = records.attach(&card, ("again.png", None), b"png".to_vec(), &ada(), None).await.unwrap();
    studio::trash::keep_for(&rig.journal, Some(0), &ada()).await.unwrap();
    rig.studio.board.remove(&card, &ada()).await.unwrap();
    assert!(!rig.dir.join("attachments").join(&again.id).exists());
}
