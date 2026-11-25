//! What a new project starts with.
//!
//! Kept small on purpose. A scaffold is a place to begin, and everything in
//! it is an ordinary file the person or their agent will rewrite.

/// Where a new project's first documents are, from its top.
const DOCUMENTS: &str = "docs/";

/// Files for a new, empty project, as `(path, content)`. Its first
/// documents are among them only where `documents`: they are what a
/// library is given to begin with, and a project that is to have none is
/// not told of a folder it has not got.
pub fn project(name: &str, documents: bool) -> Vec<(String, String)> {
    let product = brand::DISPLAY;
    let fleet = brand::blueprint_file("fleet");
    let kept = match documents {
        true => "- `docs/` holds what is known and decided. Start with `docs/overview.md`.\n",
        false => "",
    };
    let mut files = vec![
        (
            "README.md".to_string(),
            format!(
                "# {name}\n\n\
                 What this project is, in a paragraph. Replace this.\n\n\
                 ## Where things are\n\n\
                 {kept}\
                 - Its own blueprints, `{fleet}` among them once the fleet is set up, are kept by {product}\n  \
                   and not in this folder: one with the same name as a standard one replaces it for\n  \
                   this project.\n"
            ),
        ),
        (
            "docs/overview.md".to_string(),
            format!(
                "# {name}: overview\n\n\
                 ## Goal\n\nWhat this project is for, and how you will know it is working.\n\n\
                 ## Who it is for\n\n\
                 ## Constraints\n\nBudget, hardware, deadlines, things that must not change.\n\n\
                 ## Open questions\n\n- \n"
            ),
        ),
        (
            "docs/architecture.md".to_string(),
            format!(
                "# {name}: architecture\n\n\
                 | Component | What it does | Where it runs |\n\
                 |---|---|---|\n\
                 | | | |\n\n\
                 ## Decisions\n\nRecord each decision that would be expensive to reverse, with the reason.\n"
            ),
        ),
    ];
    files.retain(|(path, _)| documents || !path.starts_with(DOCUMENTS));
    files
}

/// The fleet blueprint written when a project's fleet is first set up:
/// `std/fleet` called for the project, with where the fleet is said (its
/// channel, and what may be asked of the repository's remote). Who is in
/// it is not here: that is the project's tables, which the fleet makes. The text is a blueprint as it stands
/// (`fleet.bp`, beside this), with the project's name and id put in.
pub fn fleet(name: &str, project: &str) -> String {
    include_str!("fleet.bp").replace("__name__", name).replace("__project__", project)
}
