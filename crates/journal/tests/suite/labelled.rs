//! What is measured, asked for by a label named more than once: carried
//! with any of the values the label was named with. That is how what was
//! measured for several projects is asked for together, so that what is
//! typical of them is worked out from all of it and not from each apart.

use journal::MetricQuery;

#[tokio::test]
async fn a_label_asked_for_several_times_over_is_carried_with_any_of_what_it_was_asked_with() {
    let (journal, _clock) = crate::journal().await;
    for (project, flow, ms, ok) in [
        ("shop", "main", 100.0, true),
        ("shop", "on.push", 200.0, false),
        ("blog", "main", 400.0, true),
        ("yard", "main", 800.0, true),
    ] {
        journal.measure("run", &[("project", project), ("flow", flow)], ms, ok);
    }
    // And one that is nobody's: it carries no project at all.
    journal.measure("run", &[("flow", "main")], 1_600.0, true);
    journal.flush_metrics().await.unwrap();
    let since_ms = journal.now_ms() - 3_600_000;
    let asked = |labels: &[(&str, &str)]| MetricQuery {
        name: "run".into(),
        since_ms,
        labels: labels.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect(),
        ..Default::default()
    };
    let counted = |found: Vec<journal::MetricSeries>| match found.as_slice() {
        [] => (0, 0, 0.0),
        [one] => (one.total.count, one.total.errors, one.total.max),
        more => panic!("one series was asked for, and {} came", more.len()),
    };
    let of = |labels: &'static [(&'static str, &'static str)]| {
        let journal = journal.clone();
        async move { counted(journal.query_metrics(asked(labels)).await.unwrap()) }
    };

    // Asked once, a label is carried with that value, as it always was.
    assert_eq!(of(&[("project", "shop")]).await, (2, 1, 200.0));
    assert_eq!(of(&[("project", "blog")]).await, (1, 0, 400.0));
    // Asked twice, with either: the two put together, added up as one.
    assert_eq!(of(&[("project", "shop"), ("project", "blog")]).await, (3, 1, 400.0));
    assert_eq!(of(&[("project", "blog"), ("project", "shop")]).await, (3, 1, 400.0), "in whichever order");
    // A value nothing carries adds nothing, and asked alone finds nothing.
    assert_eq!(of(&[("project", "shop"), ("project", "nonesuch")]).await, (2, 1, 200.0));
    assert_eq!(of(&[("project", "nonesuch")]).await, (0, 0, 0.0));
    // Another label beside it is still asked of everything found.
    assert_eq!(of(&[("project", "shop"), ("project", "blog"), ("flow", "main")]).await, (2, 0, 400.0));
    assert_eq!(of(&[("flow", "main"), ("flow", "on.push"), ("project", "shop")]).await, (2, 1, 200.0));
    // What carries no project is of no project that is asked for, and is there where none is.
    assert_eq!(of(&[]).await, (5, 1, 1_600.0));

    // What there is to compare by is found among the same.
    let labels =
        journal.metric_labels(asked(&[("project", "shop"), ("project", "blog")]), &[]).await.unwrap();
    let projects: Vec<&str> = labels["project"].iter().map(String::as_str).collect();
    assert_eq!(projects, ["blog", "shop"]);
    let flows: Vec<&str> = labels["flow"].iter().map(String::as_str).collect();
    assert_eq!(flows, ["main", "on.push"]);
}
