//! A record deleted goes into the Trash in the one write that takes it out
//! of its table (`studio::trash`); what reads, gives back and empties the
//! Trash is the `trash` crate's, and its suite's.

use crate::{Rig, ada};

#[tokio::test]
async fn a_card_deleted_leaves_its_board_for_the_trash_in_one_write_and_its_number_is_not_given_again() {
    let rig = Rig::new().await;
    rig.board("shop").await;
    let board = &rig.studio.board;
    let card = board.add("shop", "Gone", "", None, vec![], &ada()).await.unwrap();
    board.remove(&card.id, &ada()).await.unwrap();
    assert!(board.list("shop").await.unwrap().is_empty());
    assert!(board.get(&card.id).await.is_err());
    let kept = studio::trash::rows(&rig.journal.records_of(Some("shop")), None).await.unwrap();
    assert_eq!(kept.len(), 1);
    assert_eq!((kept[0].thing.as_str(), kept[0].table.as_deref()), (card.id.as_str(), Some("board")));
    let said = rig.events("board.item.removed,trash.put").await;
    assert_eq!(
        said.iter().map(|event| event.topic.as_str()).collect::<Vec<_>>(),
        ["board.item.removed", "trash.put"]
    );
    assert_eq!(said[0].ts, said[1].ts, "said in the one write");
    let next = board.add("shop", "Next", "", None, vec![], &ada()).await.unwrap();
    assert!(next.number > card.number, "numbered after what is in the Trash");

    // Kept for nought days, what is deleted goes at once.
    studio::trash::keep_for(&rig.journal, Some(0), &ada()).await.unwrap();
    board.remove(&next.id, &ada()).await.unwrap();
    let kept = studio::trash::rows(&rig.journal.records_of(Some("shop")), None).await.unwrap();
    assert_eq!(kept.len(), 1);
}

#[test]
fn what_hearing_of_a_card_in_the_trash_takes_goes_by_the_boards_own_name() {
    // The Trash's rows are written below the studio (`projects::trashed`),
    // which names the board's table as the board does.
    assert_eq!(studio::projects::trashed::BOARD, studio::board::BOARD);
}
