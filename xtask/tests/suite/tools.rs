//! What each program the tooling starts is called on each system.

use xtask::tools;

#[test]
fn a_tool_is_started_by_the_name_its_system_has_for_it() {
    let cases = [
        // What Node brings beside its own program is a command file on
        // Windows, and a program started by name alone is not looked for
        // as one.
        ("npm", "linux", "npm"),
        ("npm", "macos", "npm"),
        ("npm", "windows", "npm.cmd"),
        ("npx", "linux", "npx"),
        ("npx", "macos", "npx"),
        ("npx", "windows", "npx.cmd"),
        // Everything else is a program of its own name everywhere: the
        // system adds its own ending as it looks.
        ("node", "linux", "node"),
        ("node", "macos", "node"),
        ("node", "windows", "node"),
        ("cargo", "linux", "cargo"),
        ("cargo", "macos", "cargo"),
        ("cargo", "windows", "cargo"),
        ("git", "linux", "git"),
        ("git", "macos", "git"),
        ("git", "windows", "git"),
        ("tar", "linux", "tar"),
        ("tar", "macos", "tar"),
        ("tar", "windows", "tar"),
    ];
    for (tool, system, started) in cases {
        assert_eq!(tools::started(tool, system), started, "{tool} on {system}");
    }
    // And here, whichever of them this is.
    assert_eq!(tools::here("npm"), tools::started("npm", std::env::consts::OS));
}
