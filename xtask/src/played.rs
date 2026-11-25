//! How many places a test passed and how many plays it made of them.
//!
//! Two suites play something stopped at each place it passes: the
//! runner's plays of the language, and the simulation of a factory. What
//! such a test takes is how many plays it makes, and that is said nowhere
//! in what a test program prints unless somebody asks to see it. So a
//! suite that plays is told where to write it down (`PLAYED_TO`, a file),
//! by whatever runs the test programs to time them: the gates and `cargo
//! xtask build-times`. A program run by anybody else is told nothing and
//! writes nothing.
//!
//! A line of that file is the crate, how many places, how many plays, and
//! the test, with a tab between: one line for each thing a test played,
//! so a test that plays two things has two.

/// What a test program is told, to have it write down what it played.
pub const TO: &str = "PLAYED_TO";

/// What one test played.
#[derive(Clone, Debug, PartialEq)]
pub struct Played {
    pub krate: String,
    pub test: String,
    pub places: u64,
    pub plays: u64,
}

/// What was written down, each test once: a test that played several
/// things is all of them together. A line that is not one is passed over.
pub fn read(text: &str) -> Vec<Played> {
    let mut read: Vec<Played> = Vec::new();
    for line in text.lines() {
        let mut fields = line.splitn(4, '\t');
        let (Some(krate), Some(places), Some(plays), Some(test)) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let (Ok(places), Ok(plays)) = (places.parse::<u64>(), plays.parse::<u64>()) else { continue };
        match read.iter_mut().find(|one| one.krate == krate && one.test == test) {
            Some(one) => {
                one.places += places;
                one.plays += plays;
            }
            None => read.push(Played { krate: krate.to_string(), test: test.to_string(), places, plays }),
        }
    }
    read
}

/// What `played` comes to for one crate: its places and its plays.
pub fn of(played: &[Played], krate: &str) -> (u64, u64) {
    played
        .iter()
        .filter(|one| one.krate == krate)
        .fold((0, 0), |(places, plays), one| (places + one.places, plays + one.plays))
}
