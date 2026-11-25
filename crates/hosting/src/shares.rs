//! What each backend of a router is sent, as whoever asks writes it.
//!
//! A weight is a number, or for one backend the word `rest`: what the
//! others leave of a hundred. A rollout that sends its candidate a share
//! says the share once, and what the stable version keeps is worked out
//! here and not where the rollout is written.

use serde_json::Value;

use crate::router::Backend;

/// What one backend may say in place of a number.
pub const REST: &str = "rest";

/// What `rest` is the rest of.
const WHOLE: u64 = 100;

/// The backends a router is to have, from what was written. One with no
/// service and no address (no previous version, say) is left out, as it
/// always was.
pub fn shares(written: &Value) -> Result<Vec<Backend>, String> {
    let Some(written) = written.as_array() else {
        return Err("it is a list, of `{ service: <name>, weight: <n> }` or \
                    `{ address: <host:port>, weight: <n> }`"
            .to_string());
    };
    let rests: Vec<&Value> = written.iter().filter(|backend| backend["weight"] == REST).collect();
    if rests.len() > 1 {
        return Err(format!(
            "{} of them say `weight: {REST}`, and what the others leave of a hundred can be given to one",
            rests.len()
        ));
    }
    let mut numbered = Vec::new();
    for backend in written {
        if backend["weight"] == REST {
            let others: u64 = written.iter().filter_map(|other| other["weight"].as_u64()).sum();
            let Some(left) = WHOLE.checked_sub(others) else {
                return Err(format!(
                    "the others' weights come to {others}, which leaves nothing of a hundred for the one that \
                     says `weight: {REST}`"
                ));
            };
            let mut backend = backend.clone();
            backend["weight"] = Value::from(left);
            numbered.push(backend);
        } else if let Some(word) = backend["weight"].as_str() {
            return Err(format!(
                "one says its weight is `{word}`; a weight is a number, or `{REST}` for what the others leave \
                 of a hundred"
            ));
        } else {
            numbered.push(backend.clone());
        }
    }
    let backends: Vec<Backend> =
        serde_json::from_value(Value::Array(numbered)).map_err(|err| err.to_string())?;
    Ok(backends
        .into_iter()
        .filter(|backend| backend.service.is_some() || backend.address.is_some())
        .collect())
}
