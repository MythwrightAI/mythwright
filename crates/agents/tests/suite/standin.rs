//! The stand-in for a program, as this suite says things to it: what
//! the suites share is beside the program (`crates/hosting/standin`).

#[path = "../../../hosting/standin/argv.rs"]
mod argv;
#[path = "../../../hosting/standin/holds.rs"]
mod holds;
pub use argv::argv;
pub use holds::{hold, pass};
