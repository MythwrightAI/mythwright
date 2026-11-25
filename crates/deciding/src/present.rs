//! Who of a project's agents is there to be told of: which of them take
//! part in a channel, and how many there are of some roles, a function
//! of the agents there are and of the roles a blueprint hands, which it
//! asks as `agent.present` (whose action is the agents', since it reads
//! them: `agents::present`, which offers this under the names it always
//! gave). It came out of `agents` when that reached the most lines a
//! crate may have. An agent takes part in a channel where it is in it to
//! listen there: one that is in it to say what it is asked there takes
//! no part in what is said naming nobody.

blueprint::came! {
    /// Who is there.
    pub enum Came {
        /// Somebody takes part in the channel, and somebody is counted.
        Both => "both",
        /// Somebody takes part in the channel, and nobody is counted.
        Heard => "heard",
        /// Nobody takes part in the channel, and somebody is counted.
        Counted => "counted",
        /// Neither.
        Nobody => "nobody",
    }
}

/// An agent, as [`present`] reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct One {
    pub name: String,
    pub role: String,
    /// Whether it takes work: one that was stopped, or is leaving, does not.
    pub up: bool,
    /// Whether it takes part in the channel asked about.
    pub in_channel: bool,
}

/// Who is there, of a project's agents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Present {
    pub came: Came,
    /// Those of the roles asked for that take part in the channel, by
    /// name, in the order given.
    pub heard: Vec<String>,
    /// The first of them, or nothing.
    pub first: String,
    /// Whether the first of them is the one agent in the channel: what is
    /// said there with nobody named is then said to it.
    pub alone: bool,
    /// How many there are of the roles to be counted.
    pub count: usize,
}

/// Who of `agents` is there: those of the roles `here` that take part in
/// the channel, and how many there are of the roles `counted`. Only
/// agents that are up are either. A role may be in both lists, and an
/// agent then is both heard and counted.
pub fn present(agents: &[One], here: &[String], counted: &[String]) -> Present {
    let of = |roles: &[String], one: &One| one.up && roles.contains(&one.role);
    let heard: Vec<String> =
        agents.iter().filter(|one| one.in_channel && of(here, one)).map(|one| one.name.clone()).collect();
    let count = agents.iter().filter(|one| of(counted, one)).count();
    // Anybody's agent that takes part there, of whatever role: one more
    // than the first is somebody else a word with no name could be for.
    let in_channel = agents.iter().filter(|one| one.up && one.in_channel).count();
    let came = match (heard.is_empty(), count) {
        (false, 1..) => Came::Both,
        (false, 0) => Came::Heard,
        (true, 1..) => Came::Counted,
        (true, 0) => Came::Nobody,
    };
    Present {
        came,
        first: heard.first().cloned().unwrap_or_default(),
        alone: !heard.is_empty() && in_channel == 1,
        heard,
        count,
    }
}
