//! What each function an expression may call is shown by: a whole
//! blueprint whose one step writes in its log what the call comes to,
//! with the params it is asked of. Held as the words' examples are. The
//! blueprints that ship call none of them (an expression in one names a
//! thing and asks one plain question of it), so these are for a project's
//! own texts.

/// A blueprint with these params, saying this in its run's log.
macro_rules! said {
    ($params:literal, $message:literal) => {
        concat!(
            "blueprint: said\n\nparams:\n",
            $params,
            "\nsteps:\n  said:\n    do: log\n    with: { message: \"",
            $message,
            "\" }\n"
        )
    };
}

/// Each function by its name, with its example.
pub const CALLED: &[(&str, &str)] = &[
    ("len", said!("  names: [ada, grace]\n", "There are ${ len(params.names) }.")),
    ("contains", said!("  names: [ada, grace]\n", "Ada is among them: ${ contains(params.names, 'ada') }")),
    ("starts_with", said!("  branch: release/2\n", "A release: ${ starts_with(params.branch, 'release/') }")),
    ("ends_with", said!("  file: notes.md\n", "Markdown: ${ ends_with(params.file, '.md') }")),
    ("lower", said!("  name: Ada\n", "Called ${ lower(params.name) }")),
    ("upper", said!("  name: Ada\n", "CALLED ${ upper(params.name) }")),
    ("trim", said!("  name: \"  Ada  \"\n", "Called ${ trim(params.name) }")),
    ("string", said!("  count: 3\n", "${ string(params.count) } of them")),
    ("int", said!("  count: \"3\"\n", "One more than ${ int(params.count) }")),
    ("float", said!("  share: \"0.5\"\n", "A share of ${ float(params.share) }")),
    ("json", said!("  names: [ada, grace]\n", "As JSON: ${ json(params.names) }")),
    ("keys", said!("  ages: { default: { ada: 36, grace: 85 } }\n", "Who: ${ keys(params.ages) }")),
    ("join", said!("  names: [ada, grace]\n", "Hello, ${ join(params.names, ' and ') }.")),
    ("split", said!("  names: ada grace\n", "The first is ${ split(params.names, ' ')[0] }")),
    ("replace", said!("  title: The old way\n", "${ replace(params.title, 'old', 'new') }")),
    ("matches", said!("  to: \"ops-1\"\n", "A plain name: ${ matches(params.to, '^[a-z0-9-]+$') }")),
    ("slug", said!("  title: A Better Way\n", "Kept as ${ slug(params.title) }")),
    ("min", said!("  asked: 12\n", "At most ${ min(params.asked, 8) }")),
    ("max", said!("  asked: 0\n", "At least ${ max(params.asked, 1) }")),
    (
        "where",
        said!(
            "  people: [{ name: ada, free: true }, { name: grace, free: false }]\n",
            "Free: ${ where(params.people, 'free', true)[0].name }"
        ),
    ),
    (
        "sort",
        said!(
            "  people: [{ name: grace, age: 85 }, { name: ada, age: 36 }]\n",
            "Youngest: ${ sort(params.people, 'age')[0].name }"
        ),
    ),
    ("without", said!("  names: [ada, grace]\n", "Others: ${ without(params.names, 'ada') }")),
    ("range", said!("  count: 3\n", "Counted ${ range(params.count) }")),
    (
        "pluck",
        said!("  people: [{ name: ada }, { name: grace }]\n", "Who: ${ pluck(params.people, 'name') }"),
    ),
    ("at", said!("  names: [ada, grace, alan]\n", "The ends: ${ at(params.names, [1, 3]) }")),
    ("from_json", said!("  written: '{\"name\": \"ada\"}'\n", "Called ${ from_json(params.written).name }")),
    ("sum", said!("  counts: [1, 2, 3]\n", "In all ${ sum(params.counts) }")),
];
