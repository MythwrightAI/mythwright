//! `schema`: the blueprint language as a schema, for an editor.
//!
//! A blueprint is a strict subset of YAML, and the editors people already
//! have complete and check YAML against a schema. This prints the one for
//! blueprints: the daemon's, which knows the actions there are on this
//! device and what its packages bring; or, with no daemon to ask, the
//! language's words and the actions that are part of the language, which
//! is said.

use crate::client::Client;

pub async fn schema(client: &Client) -> Result<(), String> {
    let schema = match client.get("/api/blueprints/schema").await {
        Ok(schema) => schema,
        // No daemon to ask: the language alone, as `check` checks a file alone.
        Err(error) if Client::is_unreachable(&error) => {
            eprintln!(
                "note: {error}; the schema has the language's words and no action but those that are part \
                 of the language"
            );
            blueprint::schema::schema(&blueprint::intrinsics())
        }
        Err(error) => return Err(error),
    };
    println!("{}", serde_json::to_string_pretty(&schema).map_err(|err| err.to_string())?);
    Ok(())
}
