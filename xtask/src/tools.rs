//! How each program the tooling asks is started on a system.
//!
//! Most are a program of their own name everywhere. Node's are not: on
//! Windows `npm` and `npx` are command files (`npm.cmd`), and a program
//! started by name alone is looked for there as a program (`npm.exe`) and
//! not found. So whatever starts a tool asks here what it is called on
//! the system it is started on, and nothing writes a tool's file name out
//! for itself.

/// The tools that are command files on Windows: what Node brings beside
/// its own program.
const COMMAND_FILES: [&str; 2] = ["npm", "npx"];

/// What `tool` is started as on `system` (as `std::env::consts::OS` says a
/// system: `linux`, `macos`, `windows`).
pub fn started(tool: &str, system: &str) -> String {
    if system == "windows" && COMMAND_FILES.contains(&tool) {
        format!("{tool}.cmd")
    } else {
        tool.to_string()
    }
}

/// What `tool` is started as on the system this is running on.
pub fn here(tool: &str) -> String {
    started(tool, std::env::consts::OS)
}
