//! The first words of the client's addresses that no project may be
//! called.
//!
//! The client names a thing by a path (`/shop/test-runs`, `/run/<id>`),
//! which the daemon answers with the client's page wherever it has no route
//! of its own (`served::client::serve`). So a project's id is never one of the
//! daemon's own first words (its routes), of the folders the client's files
//! are served from, or of the grammar's own words for what is no project's;
//! and a few more are kept for the factory's pages that have no address
//! yet, so that no project takes them first. A project is refused such a
//! name where it is made and where it is called something else
//! (`studio::Projects`), in [`refused`]'s sentence. The client holds the
//! same list (`RESERVED` in `ui/src/tab.ts`), and a test of its suite holds
//! the two to each other word for word. A project made before a word was
//! kept keeps it, and the client writes its addresses as they were written
//! before addresses were paths.

/// Every word kept, grouped as the client groups them: the daemon's own,
/// the client's files' folders, the grammar's, and the factory's pages.
pub const RESERVED: &[&str] = &[
    "api",
    "library",
    "mockups",
    "in",
    "assets",
    "lib",
    "run",
    "instance",
    "inbox",
    "reference",
    "blueprint",
    "topic",
    "settings",
    "activity",
    "measurements",
    "agents",
    "fleet",
    "trash",
    "secrets",
    "releases",
    "about",
    "profile",
    "extensions",
    "channels",
];

/// Why a project whose id would be `id` cannot be called so, or nothing
/// where it can.
pub fn refused(id: &str) -> Option<String> {
    RESERVED.contains(&id).then(|| {
        format!(
            "a project cannot be called `{id}`: its address would be `/{id}`, and that is where this \
             factory's own pages and files are found. Call it something else, `{id}-project` say"
        )
    })
}
