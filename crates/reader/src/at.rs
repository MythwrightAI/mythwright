//! A time of day a handler fires at (`at:`), and when that next comes.
//!
//! It is written as a clock of twenty-four hours says it, after the days
//! it is on where it is not every day, and before the zone it is in,
//! which is always said: `02:00 Europe/Zagreb`, `mon,thu 09:30 UTC`. A
//! time with no zone is refused, so that nothing is ever taken to be in
//! a zone nobody wrote: the daemon knows
//! no zone of its own, and the one written here is the only one it reads.
//! The zone is a name of the zone database, and a day in it follows its
//! daylight saving: 02:00 in Zagreb is 02:00 there in winter and in
//! summer. Where a time is not there on a day (the hour the clocks go
//! forward) it is the first moment after it that is; where it is there
//! twice (the hour they go back), the first of the two.
//!
//! Everything here is a function of the text and of a moment, in
//! milliseconds since 1970: the runner keeps when a handler last fired
//! and asks what is next, or what was missed while it was not there.

use jiff::Timestamp;
use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;
use serde_json::{Map, Value, json};

/// The days, from Monday, as they are written.
const DAYS: [(&str, Weekday); 7] = [
    ("mon", Weekday::Monday),
    ("tue", Weekday::Tuesday),
    ("wed", Weekday::Wednesday),
    ("thu", Weekday::Thursday),
    ("fri", Weekday::Friday),
    ("sat", Weekday::Saturday),
    ("sun", Weekday::Sunday),
];

/// How far a day is looked for from a moment: a week and a day, which
/// holds every day a time can be on.
const LOOKED: usize = 9;

/// A time of day, the days it is on (none: every day) and its zone.
#[derive(Clone, Debug)]
pub struct At {
    days: Vec<Weekday>,
    hour: i8,
    minute: i8,
    zone: TimeZone,
    /// The zone as it was written.
    named: String,
}

/// What `text` says, or why it is not a time of day with its zone, in a
/// sentence that says how one is written.
pub fn read(text: &str) -> Result<At, String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let (days, clock, zone) = match words.as_slice() {
        [clock, zone] => ("", *clock, *zone),
        [days, clock, zone] => (*days, *clock, *zone),
        [clock] if clock.contains(':') => {
            return Err(format!(
                "`{text}` says no zone: write the zone the time is in after it, as `{clock} Europe/Zagreb` or `{clock} UTC`"
            ));
        }
        _ => {
            return Err(format!(
                "`{text}` is not a time of day: write one as `02:00 Europe/Zagreb`, the days before it where it is \
                 not every day (`mon,thu 09:30 UTC`)"
            ));
        }
    };
    let (hour, minute) = clock
        .split_once(':')
        .and_then(|(hour, minute)| {
            let whole = |part: &str| part.len() == 2 && part.bytes().all(|b| b.is_ascii_digit());
            (whole(hour) && whole(minute)).then(|| (hour.parse::<i8>().ok(), minute.parse::<i8>().ok()))
        })
        .and_then(|(hour, minute)| Some((hour?, minute?)))
        .filter(|(hour, minute)| *hour < 24 && *minute < 60)
        .ok_or_else(|| {
            format!("`{clock}` is no time of day: write it as hours and minutes on a clock of twenty-four hours, as `02:00`")
        })?;
    let mut on = Vec::new();
    for day in days.split(',').filter(|day| !day.is_empty()) {
        let found = DAYS.iter().find(|(name, _)| name.eq_ignore_ascii_case(day));
        match found {
            Some((_, weekday)) if !on.contains(weekday) => on.push(*weekday),
            Some(_) => return Err(format!("`{day}` is said twice in `{days}`")),
            None => {
                return Err(format!(
                    "`{day}` is no day: the days are `mon`, `tue`, `wed`, `thu`, `fri`, `sat` and `sun`, with commas \
                     between them (`mon,thu`)"
                ));
            }
        }
    }
    let found = if zone == "UTC" { Ok(TimeZone::UTC) } else { TimeZone::get(zone) };
    let zone_found = found.map_err(|_| {
        format!(
            "`{zone}` is no zone this machine knows: write a zone by its name in the zone database, as \
             `Europe/Zagreb` or `America/New_York`, or `UTC`"
        )
    })?;
    Ok(At { days: on, hour, minute, zone: zone_found, named: zone.to_string() })
}

impl At {
    /// The moment this comes on `date` in its zone, where `date` is one
    /// of its days.
    fn on(&self, date: Date) -> Option<i64> {
        if !self.days.is_empty() && !self.days.contains(&date.weekday()) {
            return None;
        }
        let zoned = date.at(self.hour, self.minute, 0, 0).to_zoned(self.zone.clone()).ok()?;
        Some(zoned.timestamp().as_millisecond())
    }

    /// The day `ms` is in, in its zone.
    fn day_of(&self, ms: i64) -> Option<Date> {
        Some(Timestamp::from_millisecond(ms).ok()?.to_zoned(self.zone.clone()).date())
    }

    /// The first moment it comes after `ms`, and never at it.
    pub fn next_after(&self, ms: i64) -> Option<i64> {
        let mut date = self.day_of(ms)?.yesterday().ok()?;
        for _ in 0..LOOKED {
            if let Some(at) = self.on(date).filter(|at| *at > ms) {
                return Some(at);
            }
            date = date.tomorrow().ok()?;
        }
        None
    }

    /// The last moment it came at or before `ms`.
    pub fn last_upto(&self, ms: i64) -> Option<i64> {
        let mut date = self.day_of(ms)?.tomorrow().ok()?;
        for _ in 0..LOOKED {
            if let Some(at) = self.on(date).filter(|at| *at <= ms) {
                return Some(at);
            }
            date = date.yesterday().ok()?;
        }
        None
    }

    /// What a handler that last fired at `last` does at `now`: fires for
    /// the last time that came since (`Ok`), or waits for the next
    /// (`Err`, with nothing where none comes).
    pub fn due(&self, last: i64, now: i64) -> Result<i64, Option<i64>> {
        self.last_upto(now).filter(|due| *due > last).ok_or_else(|| self.next_after(now))
    }

    /// What it is, in words: `at 02:00 in Europe/Zagreb`, `on Monday and
    /// Thursday at 09:30 in UTC`.
    pub fn said(&self) -> String {
        let clock = format!("{:02}:{:02}", self.hour, self.minute);
        let names: Vec<&str> = self
            .days
            .iter()
            .map(|day| match day {
                Weekday::Monday => "Monday",
                Weekday::Tuesday => "Tuesday",
                Weekday::Wednesday => "Wednesday",
                Weekday::Thursday => "Thursday",
                Weekday::Friday => "Friday",
                Weekday::Saturday => "Saturday",
                Weekday::Sunday => "Sunday",
            })
            .collect();
        match names.as_slice() {
            [] => format!("every day at {clock} in {}", self.named),
            [one] => format!("on {one} at {clock} in {}", self.named),
            [before @ .., last] => {
                format!("on {} and {last} at {clock} in {}", before.join(", "), self.named)
            }
        }
    }
}

/// A handler's `at:` as written, its holes filled from `scope` (what is
/// fixed for as long as it lives, as for `for:`), and what that says.
pub fn filled(written: &str, scope: &Map<String, Value>) -> Result<(String, At), String> {
    let text = crate::template::render(&json!(written), scope)?;
    let text = text.as_str().ok_or("what it says is not text")?.to_string();
    read(&text).map(|at| (text, at))
}

/// What a handler that fires at `text` goes from as it listens, and
/// whether that is to be kept: when it last fired, where that was kept of
/// the same time (`kept`, as [`kept`] writes it); from now, where another
/// time was kept, so that nothing it missed is fired; and from when its
/// instance was `made`, the first time, so that a time that came between
/// that and its listening is fired.
pub fn begun(kept: Option<&Value>, text: &str, now: i64, made: Option<i64>) -> (i64, bool) {
    match kept {
        Some(kept) if kept["at"] == text => (kept["last"].as_i64().unwrap_or(now), false),
        Some(_) => (now, true),
        None => (made.unwrap_or(now), true),
    }
}

/// What is kept of a handler that fires at `text` and last fired at `last`.
pub fn kept(text: &str, last: i64) -> Value {
    json!({ "at": text, "last": last })
}
