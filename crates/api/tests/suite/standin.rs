//! The stand-in for a program, as this suite says things to it: what
//! the suites share is beside the program (`crates/hosting/standin`).

#[path = "../../../hosting/standin/argv.rs"]
mod argv;
#[path = "../../../hosting/standin/cmd.rs"]
mod cmd;
#[path = "../../../hosting/standin/holds.rs"]
mod holds;
#[path = "../../../hosting/standin/let_go.rs"]
mod let_go;
#[path = "../../../hosting/standin/line.rs"]
mod line;
#[path = "../../../hosting/standin/listening.rs"]
mod listening;
pub use argv::argv;
pub use cmd::cmd;
pub use holds::{hold, pass};
pub use let_go::let_go;
pub use line::line;
pub use listening::Listening;
