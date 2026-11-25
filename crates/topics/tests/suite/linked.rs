//! Which pages link to a page, read from what they say as they are now:
//! by the body and the summary, never by code or a page's link to itself,
//! each with the wiki it is in, and the series box through which alone it
//! links where that is so.

use serde_json::json;

use crate::{Rig, ada};

#[tokio::test]
async fn the_pages_that_link_to_a_page_are_read_from_what_they_say_now() {
    let rig = Rig::new().await;
    let wiki = rig.topics.make("Amphibians", "", (None, None), None, &ada()).await.unwrap();
    let wiki = wiki["id"].as_str().unwrap().to_string();
    let fungus = rig.topics.make_under("Chytrid fungus", "", &wiki, None, &ada()).await.unwrap();
    let fungus = fungus["id"].as_str().unwrap().to_string();
    let link = format!("[the fungus](topic:{fungus})");
    rig.topics.make_under("Frogs", &format!("Many died of {link}."), &wiki, None, &ada()).await.unwrap();
    rig.topics.make("Code", &format!("```\n{link}\n```"), (None, None), None, &ada()).await.unwrap();
    rig.topics.make("Self", "", (None, None), None, &ada()).await.unwrap();
    let linked = rig.topics.linked(&fungus, &ada()).await.unwrap();
    assert_eq!(
        linked,
        [
            json!({ "id": linked[0]["id"], "title": "Frogs", "wiki": { "id": wiki, "title": "Amphibians" }, "series": null })
        ]
    );
    assert!(rig.topics.linked(&wiki, &ada()).await.unwrap().is_empty());
}
