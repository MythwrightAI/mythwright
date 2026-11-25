//! Why something failed, as it is passed up.
//!
//! A step that fails fails its run, and a run that failed because a step
//! `call`ed something that failed is one failure, not two: the reason is
//! what the innermost step said, and each run it came up through put the
//! name of its step in front. Both are kept. The whole of it (`error`)
//! says where; the reason alone ([`reason`]) is what a person is told,
//! who has no use for the names of steps inside a blueprint they did not
//! write.
//!
//! What it comes up through is known too. An instance whose steps a `call`
//! is waiting on says so when it fails: which instance called it
//! (`called_by`), and whether that step lets it fail (`let_be`). Its
//! failure is then that step's too. Where the step fails with it, whatever
//! is above says it again; where the step lets it be, it stops there, and
//! is its caller's to speak of or to leave. Whoever tells people of
//! failures can so tell of each once.

pub use kernel::why::{Calls, reason, within};
