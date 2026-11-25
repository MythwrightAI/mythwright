//! Durations as people write them: `90s`, `5m`, `1h30m`, `300 minutes`.

const UNITS: &[(&[&str], u64)] = &[
    (&["ms", "msec", "millis", "millisecond", "milliseconds"], 1),
    (&["s", "sec", "secs", "second", "seconds"], 1_000),
    (&["m", "min", "mins", "minute", "minutes"], 60_000),
    (&["h", "hr", "hrs", "hour", "hours"], 3_600_000),
    (&["d", "day", "days"], 86_400_000),
];

/// Parses a duration into milliseconds. A unit is always required: a bare
/// `300` could mean seconds or minutes, and guessing is how timeouts go wrong.
pub fn parse(text: &str) -> Result<u64, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("expected a duration such as `30s` or `5m`".to_string());
    }
    let mut total = 0f64;
    let mut rest = text;
    while !rest.is_empty() {
        let digits = rest.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(rest.len());
        let amount: f64 = rest[..digits]
            .parse()
            .map_err(|_| format!("`{text}` is not a duration; write it like `30s` or `1h30m`"))?;
        rest = rest[digits..].trim_start();
        let letters = rest.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(rest.len());
        let unit = &rest[..letters];
        if unit.is_empty() {
            return Err(format!("`{text}` needs a unit: ms, s, m, h or d"));
        }
        let scale = UNITS
            .iter()
            .find(|(names, _)| names.contains(&unit.to_ascii_lowercase().as_str()))
            .map(|(_, scale)| *scale)
            .ok_or_else(|| format!("unknown time unit `{unit}`; use ms, s, m, h or d"))?;
        total += amount * scale as f64;
        rest = rest[letters..].trim_start();
    }
    Ok(total.round() as u64)
}

/// The shortest exact spelling: `90000` is `1m30s`.
pub fn format(millis: u64) -> String {
    if millis == 0 {
        return "0s".to_string();
    }
    let mut left = millis;
    let mut out = String::new();
    for (unit, scale) in [("d", 86_400_000), ("h", 3_600_000), ("m", 60_000), ("s", 1_000), ("ms", 1)] {
        if left >= scale {
            out.push_str(&format!("{}{unit}", left / scale));
            left %= scale;
        }
    }
    out
}
