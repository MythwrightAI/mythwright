//! The stand-in for a program, as this suite says things to it: what
//! the suites share is beside the program (`crates/hosting/standin`).

#[path = "../../../hosting/standin/cmd.rs"]
mod cmd;
#[path = "../../../hosting/standin/line.rs"]
mod line;
pub use cmd::cmd;
pub use line::line;
