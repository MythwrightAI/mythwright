//! What is said of a topic beyond what is said of any record: that what it
//! says was revised, what its status came to, that it was put somewhere
//! else or moved in its wiki's tree, and who follows it. Each is said in the write that keeps the topic's row, in
//! the factory's own journal whatever it names (`journal::HERE`), and
//! carries `project` where the topic is a project's, so that it is heard
//! by project; one that is a workspace's is nobody's to whatever hears by
//! project. Each is heard by whoever may read records, and takes what
//! reading the topic's table takes.

use blueprint::EventSpec;
use blueprint::ValueType::*;

fn topic(name: &str, says: &str) -> EventSpec {
    EventSpec::new(name, says)
        .field("record", String, "the topic, by its id")
        .field("title", String, "what it is called")
        .maybe("project", String, "the project it is of, where it is a project's")
        .maybe("workspace", String, "the workspace it is the own of, where it is a workspace's")
        .maybe("by", String, "who did it, as an actor")
}

pub fn events() -> Vec<EventSpec> {
    let all = vec![
        topic(
            "topic.revised",
            "What a topic says was changed (its title, its body or its summary): a revision of it, kept whole, \
             numbered from its first, which was its making.",
        )
        .field("nth", Int, "which revision it is, the first being the topic as it was made"),
        topic("topic.answered", "A topic was answered: a reply was marked as what answered it, or its status said so.")
            .maybe("answer", String, "the reply that answered it, by the number it was said at"),
        topic("topic.closed", "A topic was closed."),
        topic(
            "topic.opened",
            "A topic is open: made so, opened again once it was answered, closed or standing, or given a status where \
             it had none.",
        ),
        topic("topic.standing", "A topic is standing: it is never answered, and is kept up to date."),
        topic(
            "topic.untracked",
            "A topic's status was taken away: it is not tracked as a question, and stands nowhere until it is \
             given one again.",
        ),
        topic("topic.placed", "A topic was put somewhere else: in a workspace of its own, or in a project.")
            .maybe("was", String, "where it was, as `workspace:<id>` or `project:<id>`; nothing where it was nowhere"),
        topic(
            "topic.moved",
            "A page of a wiki was moved: under another page, or to the top, where it is a wiki of its own, or to \
             another place among the pages beside it. Every page under it went with it.",
        )
        .maybe("parent", String, "the page it is under now, by its id; nothing where it is a wiki")
        .field("position", Number, "where it stands among the pages beside it now")
        .field("was", Object, "where it was: `parent`, the page it was under, and `position`"),
        topic("topic.followed", "Somebody follows a topic now, or follows it no longer.")
            .field("follows", Bool, "whether they follow it now"),
    ];
    blueprint::events::heard(all, &[("topic", "records:read")])
}
