//! The words a building block answers with (`blueprint::came!`).

blueprint::came! {
    /// What became of a thing, for this test.
    enum Came {
        /// It was kept.
        Kept => "kept",
        /// It was sent back.
        SentBack => "sent_back",
        /// There was none.
        Nothing => "nothing",
    }
}

blueprint::came! {
    enum Only {
        One => "one",
    }
}

#[test]
fn a_blocks_words_are_all_listed_in_the_order_declared_each_with_what_a_blueprint_reads() {
    assert_eq!(Came::ALL, [Came::Kept, Came::SentBack, Came::Nothing]);
    let read: Vec<&str> = Came::ALL.iter().map(|one| one.word()).collect();
    assert_eq!(read, ["kept", "sent_back", "nothing"]);
    assert_eq!(Came::words(), "`kept`, `sent_back` or `nothing`");
    assert_eq!((Only::ALL.len(), Only::words()), (1, "`one`".to_string()));
}
