//! The stand-in for a program, as this suite says things to it: what
//! the suites share is beside the program (`crates/hosting/standin`).

#[path = "../../../hosting/standin/argv.rs"]
mod argv;
#[path = "../../../hosting/standin/cmd.rs"]
mod cmd;
// Unix's, as is what holds a stand-in here (`drained.rs`).
#[cfg(unix)]
#[path = "../../../hosting/standin/hold.rs"]
mod hold;
pub use argv::argv;
pub use cmd::cmd;
#[cfg(unix)]
pub use hold::Hold;
