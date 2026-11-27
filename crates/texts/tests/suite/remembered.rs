//! A check remembered is given only where every question it asked is
//! answered the same (`texts::remembered`): whatever the catalog answers,
//! what is given is what checking the text there and then comes to.

use std::cell::Cell;

use blueprint::{ActionList, ActionSpec, Catalog, Checked, ValueType};

/// A catalog of so many actions and blueprints, counting how often it is
/// asked after an action.
struct Counted {
    actions: ActionList,
    blueprints: Vec<&'static str>,
    asked: Cell<usize>,
}

impl Counted {
    fn new(helps: &str, blueprints: &[&'static str]) -> Counted {
        let tell =
            ActionSpec::new("tell", "Tells somebody something.").required("what", ValueType::String, helps);
        let actions = ActionList::new(vec![tell, ActionSpec::new("noop", "Does nothing.")]);
        Counted { actions, blueprints: blueprints.to_vec(), asked: Cell::new(0) }
    }
}

impl Catalog for Counted {
    fn action(&self, name: &str) -> Option<&ActionSpec> {
        self.asked.set(self.asked.get() + 1);
        self.actions.action(name)
    }

    fn action_names(&self) -> Vec<&str> {
        self.actions.action_names()
    }

    fn has_blueprint(&self, name: &str) -> Option<bool> {
        Some(self.blueprints.contains(&name))
    }
}

/// What a check came to, as somebody reading it is told.
fn said(checked: &Checked) -> Vec<String> {
    checked
        .diagnostics
        .iter()
        .map(|said| format!("{} {}: {}", said.span.start.line, said.code, said.message))
        .collect()
}

#[test]
fn a_check_is_given_again_only_where_every_question_it_asked_is_answered_the_same() {
    // A text of its own, so that nothing else this program checks is
    // remembered under it.
    let source = format!(
        "blueprint: remembered-{}\nsteps:\n  a: {{ do: tell, with: {{ what: hello }} }}\n  b: {{ do: tell, with: {{ what: again }} }}\n  c: {{ do: tell }}\n  d: {{ call: other }}\n",
        kernel::id::secret()
    );
    let usual = || Counted::new("what to say", &[]);
    let catalogs = [
        // Each answers one question otherwise than the usual: an action
        // described another way, a blueprint that is there, an action
        // that is not.
        usual(),
        Counted::new("what is said", &[]),
        Counted::new("what to say", &["other"]),
        Counted { actions: ActionList::new(vec![ActionSpec::new("noop", "Does nothing.")]), ..usual() },
        usual(),
    ];
    let mut came_to = Vec::new();
    for catalog in &catalogs {
        let fresh = said(&blueprint::check(&source, catalog));
        catalog.asked.set(0);
        let given = said(&texts::remembered::checked(&source, catalog));
        assert_eq!(given, fresh);
        came_to.push(given);
    }
    // They came to something else each, where they answered otherwise.
    assert_ne!(came_to[0], came_to[2], "a blueprint that is there is no warning");
    assert_ne!(came_to[0], came_to[3], "an action that is not there is an error");
    assert_eq!(came_to[0], came_to[4]);

    // Asked again of a catalog that answers the same, it is given what
    // it came to, asking after each action it takes once: checked, it
    // asks at each step that takes it.
    let again = usual();
    let given = said(&texts::remembered::checked(&source, &again));
    assert_eq!(given, came_to[0]);
    let checking = usual();
    let _ = blueprint::check(&source, &checking);
    assert!(again.asked.get() < checking.asked.get(), "{} and {}", again.asked.get(), checking.asked.get());
}
