//! A comparison's shares said again: by a rule its user writes, as what
//! it counts comes to, or by somebody, with why.
//!
//! A comparison that shares its things out gives each new one the way
//! that has had least of its share since the shares were last set
//! (`kernel::compared::next`). A rule (`kernel::compared::rolled`) has one
//! way given more of them while what is counted of its things holds
//! against another's: a new setup given a part of the work that grows
//! while its counts hold, as a service's canary is rolled out. Whoever
//! counts reads each event of the rule's count into what the rule has
//! read ([`Comparisons::roll`]), and the block says what that comes to;
//! the shares, what was read and what is said of it are one write. A
//! person, or a blueprint (`compare.share`), says the shares again the
//! same way, and the rule goes on from there.
//!
//! Nothing here scores or knows what a thing is: the rule is whoever
//! wrote it's, the block reads the counts as they are, and what was
//! decided is said with the counts it was decided by.

use std::collections::BTreeMap;

use kernel::compared::rolled::{self, Rolled, Rolling, Seen};
pub use kernel::compared::rolled::{Rule, Shared};
use kernel::{Error, Principal, Result, Source};
use serde_json::{Value, json};

use crate::Comparisons;
use crate::compared::{Comparison, NAMESPACE, covers};

/// `one` given `rule`, checked against it as it is, the rule reading
/// from `since`.
pub(crate) fn ruled(mut one: Comparison, rule: Rule, since: i64) -> Result<Comparison> {
    if let Some(wrong) = rolled::wrong(&rule, &one) {
        return Err(Error::invalid(wrong));
    }
    let done = rolled::percent(&one.ways, &rule.grows) >= rule.most;
    one.rolling = Some(Rolling { since, seen: BTreeMap::new(), done });
    one.rule = Some(rule);
    Ok(one)
}

/// What is said with `one` kept.
enum Said {
    /// Its shares were said again ([`Shared`]).
    Shared(Shared),
    /// It was given a rule, or had it taken away.
    Ruled,
}

impl Comparisons {
    /// Keeps `one` and says what is to be said of it, as one write.
    async fn keep_saying(&self, one: &Comparison, said: Said) -> Result<()> {
        let (key, value) = (one.name.clone(), json!(one));
        let (name, project) = (one.name.clone(), one.project.clone());
        self.engine
            .journal()
            .write(move |writing| {
                writing.keep(NAMESPACE, &key, &value)?;
                match said {
                    Said::Shared(shared) => {
                        let mut data = json!(shared);
                        data["name"] = json!(name);
                        data["project"] = json!(project);
                        writing.say("system.comparison.shared", Source::default(), data)?;
                    }
                    Said::Ruled => {
                        writing.say("system.comparison.ruled", Source::default(), value)?;
                    }
                }
                Ok(())
            })
            .await?;
        self.remember(one);
        Ok(())
    }

    /// Says `name`'s shares again, as `by`, with why: each way named is
    /// given the share said, the others keeping theirs, and what each way
    /// is given from then is held to them. A rule goes on from there,
    /// reading what comes next. The same done twice: shares that are what
    /// they are change nothing and say nothing.
    pub async fn share(
        &self,
        name: &str,
        shares: BTreeMap<String, u32>,
        why: Option<String>,
        by: &Principal,
    ) -> Result<Comparison> {
        let _one = self.kept.choosing.lock().await;
        let mut one = self.comparison(name).await?;
        if one.asked {
            return Err(Error::invalid(format!(
                "each start that names `{name}` names its way, so its shares say nothing; it has none to change"
            )));
        }
        if one.ended.is_some() {
            return Err(Error::invalid(format!(
                "`{name}` has ended, and its shares are no longer changed; `compare end {name} --goes <way>` says \
                 which way everything goes"
            )));
        }
        if shares.is_empty() {
            return Err(Error::invalid(format!("say each way's share, such as `{}=1`", one.ways[0].name)));
        }
        for (way, share) in &shares {
            if one.way(way).is_none() {
                let ways: Vec<&str> = one.ways.iter().map(|way| way.name.as_str()).collect();
                return Err(Error::invalid(format!(
                    "`{name}` has no way called `{way}`; its ways are {}",
                    ways.join(", ")
                )));
            }
            if *share == 0 {
                return Err(Error::invalid(format!(
                    "the way `{way}` would have a share of 0 and be given nothing; ending the comparison with \
                     the way that goes on is how a way is given everything"
                )));
            }
        }
        if !covers(&by.scopes, &one.held) {
            return Err(Error::denied(format!(
                "changing `{name}`'s shares changes what its starts run, and {} started it holding more than you \
                 do; somebody who holds that changes them",
                one.by
            )));
        }
        let was = rolled::shares(&one.ways);
        let mut now = was.clone();
        now.extend(shares);
        if now == was {
            return Ok(one);
        }
        let why = why.map(|why| why.trim().to_string()).filter(|why| !why.is_empty());
        let why = why.unwrap_or_else(|| format!("{} said them", by.actor()));
        let at = self.engine.journal().now_ms();
        for way in &mut one.ways {
            way.share = now[&way.name];
        }
        let shared = Shared { at, by: by.actor(), was, now, why };
        self.set_again(&mut one, shared.clone(), None);
        self.keep_saying(&one, Said::Shared(shared)).await?;
        Ok(one)
    }

    /// Gives `name` a rule its shares change by, or takes it away, as `by`:
    /// checked against the comparison as it is, reading what comes from now.
    pub async fn rule(&self, name: &str, rule: Option<Rule>, by: &Principal) -> Result<Comparison> {
        let _one = self.kept.choosing.lock().await;
        let mut one = self.comparison(name).await?;
        if !covers(&by.scopes, &one.held) {
            return Err(Error::denied(format!(
                "a rule changes what `{name}`'s starts run, and {} started it holding more than you do; somebody \
                 who holds that gives it one",
                one.by
            )));
        }
        if one.rule == rule {
            return Ok(one);
        }
        let one = match rule {
            Some(rule) => ruled(one, rule, self.engine.journal().now_ms())?,
            None => {
                (one.rule, one.rolling) = (None, None);
                one
            }
        };
        self.keep_saying(&one, Said::Ruled).await?;
        // Whoever counts reads the comparisons again, and so the rule it now has.
        self.count_from_here().await;
        Ok(one)
    }

    /// The shares of `one` set again as `shared` says: what each way is
    /// given is held to them from what it had gone, where they are others,
    /// and the rule reads afresh; `done` where it is done, else as far as the way that grows
    /// is from the most.
    fn set_again(&self, one: &mut Comparison, shared: Shared, done: Option<bool>) {
        // Held where they were, the shares go on as they went: only shares
        // that are others are what comes next is shared by.
        if shared.now != shared.was {
            for way in &mut one.ways {
                way.before = way.went;
            }
        }
        if let Some(rule) = &one.rule {
            let done = done.unwrap_or_else(|| rolled::percent(&one.ways, &rule.grows) >= rule.most);
            one.rolling = Some(Rolling { since: shared.at, seen: BTreeMap::new(), done });
        }
        one.shared.push(shared);
    }

    /// Reads into what `name`'s rule has read that `thing`, of the way
    /// `way`, was said by the count to have come to `came` at `at`; and
    /// where that decides something, says the shares again with why. In
    /// one write, so that a daemon that stops anywhere and counts the
    /// same event again finds it read: a thing is read once for each time
    /// the shares are set.
    pub(crate) async fn roll(&self, name: &str, thing: &str, way: &str, count: &str, came: &str, at: i64) {
        let _one = self.kept.choosing.lock().await;
        let Ok(mut one) = self.comparison(name).await else { return };
        let Some(rule) = one.rule.clone().filter(|rule| rule.count == count) else { return };
        if one.ended.is_some() || (way != rule.grows && way != rule.against) {
            return;
        }
        let Some(rolling) = one.rolling.as_mut() else { return };
        // Said before the shares were last set: no part of what is read now.
        if rolling.done || at < rolling.since {
            return;
        }
        let hit = rolled::came_to(&rule.came, came);
        let before = rolling.seen.get(thing).cloned();
        let seen =
            rolling.seen.entry(thing.to_string()).or_insert(Seen { way: way.to_string(), came: false });
        seen.came |= hit;
        if before.as_ref() == Some(seen) {
            return;
        }
        let read = (rolled::tally(rolling, &rule.grows), rolled::tally(rolling, &rule.against));
        let now = rolled::percent(&one.ways, &rule.grows);
        let at = self.engine.journal().now_ms();
        let was = rolled::shares(&one.ways);
        let (shared, done) = match rolled::decide(&rule, now, read.0, read.1) {
            Rolled::Wait => {
                let _ = self.keep_compared(&one).await;
                return;
            }
            Rolled::Held { why } => {
                (Shared { at, by: rolled::RULE.into(), now: was.clone(), was, why }, false)
            }
            Rolled::Lowered { to, why } => {
                let shares = rolled::shares_at(&one.ways, &rule.grows, to);
                for (way, share) in one.ways.iter_mut().zip(shares) {
                    way.share = share;
                }
                let now = rolled::shares(&one.ways);
                (Shared { at, by: rolled::RULE.into(), was, now, why }, false)
            }
            Rolled::Raised { to, why, done } => {
                let shares = rolled::shares_at(&one.ways, &rule.grows, to);
                for (way, share) in one.ways.iter_mut().zip(shares) {
                    way.share = share;
                }
                let now = rolled::shares(&one.ways);
                (Shared { at, by: rolled::RULE.into(), was, now, why }, done)
            }
        };
        self.set_again(&mut one, shared.clone(), Some(done));
        let _ = self.keep_saying(&one, Said::Shared(shared)).await;
    }
}

/// What a comparison's rule and its shares come to, as it is shown
/// ([`crate::beside`]): the rule in a sentence, what it has read of each
/// of its two ways since the shares were last set, whether it is done,
/// and each time the shares were said again with why, the oldest first.
pub(crate) fn shown(one: &Comparison) -> Value {
    let reading = one.rule.as_ref().map(|rule| {
        let rolling = one.rolling.clone().unwrap_or_default();
        let read = |way: &str| {
            let tally = rolled::tally(&rolling, way);
            json!({ "way": way, "things": tally.things, "came": tally.came })
        };
        json!({
            "said": rolled::said(rule),
            "since": rolling.since,
            "done": rolling.done,
            "read": [read(&rule.grows), read(&rule.against)],
            "over": rule.over,
        })
    });
    let parts: Vec<Value> = one
        .ways
        .iter()
        .map(|way| json!({ "way": way.name, "share": way.share, "percent": rolled::percent(&one.ways, &way.name) }))
        .collect();
    json!({ "rule": reading, "parts": parts, "shared": one.shared })
}
