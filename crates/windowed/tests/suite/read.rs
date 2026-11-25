use kernel::window::figures::Counting;
use serde_json::json;
use windowed::{Answer, Counts, RECORD, read_record};

#[test]
fn a_record_is_read_from_its_row_by_the_columns_said_for_it() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    let statement = format!(
        "SELECT {RECORD} FROM (SELECT 'shop-3' AS id, 'shop' AS project, 'board' AS collection, 3 AS number, \
         '{{\"title\":\"Tea\"}}' AS data, 2.5 AS position, 'user:ada' AS created_by, 10 AS created, 20 AS updated)"
    );
    let record = connection.query_row(&statement, [], read_record).unwrap();
    assert_eq!(record.id, "shop-3");
    assert_eq!(record.number, 3);
    assert_eq!(record.data.get("title"), Some(&json!("Tea")));
    assert_eq!((record.position, record.created, record.updated), (2.5, 10, 20));
}

#[test]
fn a_window_says_where_it_stands_and_what_was_read_whole_only_where_there_is_something_to_say() {
    let plain = Answer { looked: 4, ..Answer::default() }.json();
    assert_eq!(plain, json!({ "rows": [], "after": null, "before": null, "looked": 4, "short": false }));
    let said = Answer { rank: Some(7), whole_read: Some(90_000), ..Answer::default() }.json();
    assert_eq!(said["rank"], json!(7));
    assert_eq!(said["whole_read"], json!(90_000));
}

#[test]
fn what_was_counted_of_a_view_is_kept_only_while_the_table_is_as_it_was() {
    let counts = Counts::default();
    let counting = Counting { looked: 3, count: 2, ..Counting::default() };
    counts.keep("view".into(), Some(5), (counting.clone(), 42, true));
    assert_eq!(counts.kept("view", Some(5)), Some((counting, 42, true)));
    // Written since: counted again.
    assert_eq!(counts.kept("view", Some(6)), None);
    // A table whose writes are not marked keeps nothing.
    counts.keep("other".into(), None, (Counting::default(), 0, false));
    assert_eq!(counts.kept("other", None), None);
}

#[test]
fn a_record_written_as_a_file_is_read_back_as_it_was() {
    let fields: Vec<String> = ["title", "status", "body"].map(str::to_string).to_vec();
    let mut data = serde_json::Map::new();
    data.insert("title".into(), json!("Tea"));
    data.insert("status".into(), json!("ready"));
    data.insert("body".into(), json!("Steep it."));
    let record = windowed::Record {
        id: windowed::record_id("shop", "notes", 4),
        project: "shop".into(),
        collection: "notes".into(),
        number: 4,
        data,
        position: 4.0,
        created_by: "user:ada".into(),
        created: 1,
        updated: 2,
    };
    let text = windowed::files::write(&fields, &record);
    assert!(text.starts_with("---\nstatus: \"ready\"\n"));
    assert_eq!(windowed::files::read("shop", "notes", &fields, 4, &text), record);
    assert_eq!(windowed::record_id("shop", "board", 4), "shop-4");
}
