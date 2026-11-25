//! The parts of the tooling that are asked things as functions.
//!
//! The program (`main.rs`) is the rest, and its tests run it. A part is
//! here when what it works out is worth asking of it directly: `changes`
//! is given commits and histories a test would otherwise have to make with
//! git, one for each thing it wanted to say; `package` is asked what a
//! package is called and holds on systems this one is not; `menu` what
//! puts the product in a desktop's menu; `sha256` and `zip` are the sum
//! beside a package and the one file of a package for Windows, each held
//! to what its format's own notes say; `tools` what each program the
//! tooling starts is called on a system; `clock` is given what a test
//! program printed and says how long each test took; `sharing` how a
//! program's tests are put in shares; `crowd` is given a test program
//! and runs one test of it many times at once, counting; `left` is given
//! what two looks at what is running found, and says what the tests
//! between them left behind; `plain` is given what a reading of
//! the blueprints that ship found, and holds it to what is still let
//! through; `played` is given what the suites that play wrote down of
//! how many places and plays each test made; `notices` makes the notices
//! of other people's libraries that go with a package, and reads them
//! back, and `shipped` says from the lock and the manifests which crates
//! the programs can be built from, so that a test holds the one to the
//! other with no tool.
//! `generations` says which builds of the client an installed copy
//! keeps, by the rule its build keeps `ui/dist` by.
//! `ways` is the other ways
//! the suites are played, and what a test program is told for each.

pub mod changes;
pub mod clock;
pub mod crowd;
pub mod generations;
pub mod left;
pub mod menu;
pub mod notices;
pub mod package;
pub mod plain;
pub mod played;
pub mod sha256;
pub mod sharing;
pub mod shipped;
pub mod tools;
pub mod ways;
pub mod zip;
