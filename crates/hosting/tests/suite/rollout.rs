//! What a rollout works out (`hosting::rollout`): what the next version is
//! called, and what watching it came to. Each a table of what is handed
//! and what it comes to; and the second holds that no word it can answer
//! goes unreached.

use hosting::rollout::{Came, Watched, next, watched};
use serde_json::{Value, json};

#[test]
fn the_next_version_is_one_generation_on_and_is_called_after_what_is_rolled_out() {
    type Comes<'a> = Result<(i64, &'a str), &'a str>;
    let cases: [(&str, i64, Comes<'_>); 6] = [
        // The first there is, and those after it.
        ("web", 0, Ok((1, "web-g1"))),
        ("web", 1, Ok((2, "web-g2"))),
        ("shop-store", 41, Ok((42, "shop-store-g42"))),
        // A name is taken as it is, whatever is in it.
        ("web-g2", 2, Ok((3, "web-g2-g3"))),
        // What makes no sense is refused.
        ("", 0, Err("`name` is what is being rolled out, and it has none")),
        (
            "web",
            -1,
            Err(
                "`generation` is how many versions of `web` there have been, and is not less than none; it is -1",
            ),
        ),
    ];
    for (name, generation, comes_to) in cases {
        let comes_to = comes_to.map(|(next, called)| (next, called.to_string())).map_err(str::to_string);
        assert_eq!(next(name, generation), comes_to, "`{name}` after {generation}");
    }
}

fn stage(connections: u64) -> Value {
    json!({ "weight": 10, "connections": connections, "failures": 0 })
}

#[test]
fn watching_comes_to_none_watched_or_unwatched_by_whether_any_connection_reached_any_stage() {
    let came = |came, stages, unwatched| Watched { came, stages, unwatched };
    let cases: Vec<(Value, Watched)> = vec![
        // No stages: replaced at once, or the first there is. Nothing said is none.
        (json!([]), came(Came::None, 0, 0)),
        (json!(null), came(Came::None, 0, 0)),
        // Connections in every stage, in one of several, and in the last alone.
        (json!([stage(4), stage(9), stage(30)]), came(Came::Watched, 3, 0)),
        (json!([stage(0), stage(2), stage(0)]), came(Came::Watched, 3, 2)),
        (json!([stage(0), stage(0), stage(1)]), came(Came::Watched, 3, 2)),
        (json!([stage(7)]), came(Came::Watched, 1, 0)),
        // None in any: nothing was learnt of it.
        (json!([stage(0)]), came(Came::Unwatched, 1, 1)),
        (json!([stage(0), stage(0), stage(0)]), came(Came::Unwatched, 3, 3)),
        // A stage says more than this reads, and only its connections are read.
        (json!([{ "connections": 3 }]), came(Came::Watched, 1, 0)),
    ];
    let mut reached = Vec::new();
    for (stages, comes_to) in cases {
        let answer = watched(&stages);
        assert_eq!(answer, Ok(comes_to), "{stages}");
        reached.push(answer.unwrap().came);
    }
    for word in Came::ALL {
        assert!(reached.contains(word), "no case reaches `{}`", word.word());
    }
}

#[test]
fn stages_that_do_not_say_what_reached_them_are_refused_and_never_counted_as_watched() {
    let cases: Vec<(Value, &str)> = vec![
        (
            json!("three"),
            "`shifts` is a list of what each shift gave, or nothing where there were none; it is \"three\"",
        ),
        (json!({ "connections": 0 }), "`shifts` is a list of what each shift gave"),
        (
            json!([stage(1), { "weight": 50 }]),
            "shift 2 of `shifts` does not say how many `connections` reached the version in it: {\"weight\":50}",
        ),
        (
            json!([null]),
            "shift 1 of `shifts` does not say how many `connections` reached the version in it: null",
        ),
        (json!([{ "connections": "many" }]), "shift 1 of `shifts` does not say how many `connections`"),
        (json!([{ "connections": -2 }]), "shift 1 of `shifts` does not say how many `connections`"),
    ];
    for (stages, why) in cases {
        let refused = watched(&stages).unwrap_err();
        assert!(refused.contains(why), "{stages}: {refused}");
    }
}
