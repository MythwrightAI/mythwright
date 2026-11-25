//! What is done to a play from outside: each thing as a person would do
//! it, looking first, so that one the conductor went away in the middle
//! of is done again and does nothing twice.

use std::future::Future;
use std::pin::Pin;

use conductor::{InstanceFilter, Library};

use super::Does;
use crate::Rig;

/// What doing one thing came to.
pub(super) enum Did {
    /// It was done, and answered.
    It,
    /// It was asked by somebody the play does not wait for.
    Asked(tokio::task::JoinHandle<()>),
    /// It is to be done by somebody who does not give way: begun by
    /// whoever holds the play's databases, which are let go for it.
    AtOnce(Pin<Box<dyn Future<Output = ()> + Send>>),
}

/// Does one thing from outside. `may_be_refused` says that the play is
/// stopped somewhere: an apply that is refused then is no fault of the
/// scenario's.
pub(super) async fn does(rig: &Rig, does: &Does, may_be_refused: bool) -> Did {
    match does {
        Does::Apply(source, params)
        | Does::Offer(source, params)
        | Does::Asks(source, params)
        | Does::ApplyFor(_, source, params)
        | Does::ApplyUnder(_, _, source, params)
        | Does::ApplyBeside(_, _, source, params) => {
            // Looked for first, as somebody whose asking got no
            // answer would: where an instance has this very text it
            // was applied, whatever became of it since, and applied
            // again it would be a second one.
            let filter = InstanceFilter { limit: 10_000, ..Default::default() };
            let there = rig.engine.instances(filter).await.unwrap_or_default();
            if !there.iter().any(|one| one.source == *source && one.parent.is_none()) {
                let mut asked = rig.request(source, params.clone());
                if let Does::ApplyFor(project, ..) = does {
                    asked.project = Some(project.to_string());
                }
                if let Does::ApplyUnder(project, environment, ..) = does {
                    asked.project = Some(project.to_string());
                    asked.environment = Some(environment.to_string());
                }
                if let Does::ApplyBeside(project, ..) = does {
                    asked.project = Some(project.to_string());
                    asked.environment = Some(String::new());
                }
                if let Does::Asks(..) = does {
                    let (engine, shared) = (rig.engine.clone(), rig.shared.clone());
                    return Did::Asked(tokio::spawn(async move {
                        let answered = match engine.apply(asked).await {
                            Ok(applied) if applied.created => "made".to_string(),
                            Ok(_) => "applied over what was there".to_string(),
                            Err(refused) => refused.message,
                        };
                        shared.note("answered", &answered);
                    }));
                }
                // What it came to is read from what is kept; a text
                // that is refused outright is the scenario's fault.
                if let Err(err) = rig.engine.apply(asked).await {
                    assert!(
                        matches!(does, Does::Offer(..)) || may_be_refused,
                        "it was refused: {}",
                        err.message
                    );
                }
            }
        }
        Does::Say(topic, data) => {
            // Looked for first: said again it would be two things said.
            let act = data["act"].clone();
            let said = rig.events(topic).await.iter().any(|event| event.data["act"] == act);
            if !said {
                let _ = rig.journal.emit(topic, kernel::Source::actor("user:ada"), data.clone()).await;
            }
        }
        Does::Put(text) => {
            let read = blueprint::check(text, &blueprint::NoCatalog).blueprint.expect("a text that reads");
            rig.library.put(&read.name, text);
        }
        Does::Again(key) => {
            let filter = InstanceFilter { live_only: true, limit: 1_000, ..Default::default() };
            let alive = rig.engine.instances(filter).await.unwrap_or_default();
            if let Some(found) = alive.iter().find(|one| one.key == *key)
                && let Some(there) = rig.library.load(&found.blueprint, found.project.as_deref())
                && found.revision != blueprint::revision(&there)
                && let Err(err) = rig.engine.apply_again(&found.id, crate::user(), false).await
            {
                assert!(may_be_refused, "it was refused: {}", err.message);
            }
        }
        Does::Stop(blueprint) | Does::Stops(blueprint) | Does::StopsAtOnce(blueprint) => {
            let filter = InstanceFilter { live_only: true, limit: 1_000, ..Default::default() };
            let alive = rig.engine.instances(filter).await.unwrap_or_default();
            if let Some(found) = alive.iter().find(|one| one.blueprint == *blueprint && one.parent.is_none())
            {
                let (engine, id) = (rig.engine.clone(), found.id.clone());
                let stopping = async move {
                    let _ = engine.stop(&id).await;
                };
                match does {
                    Does::Stops(_) => return Did::Asked(tokio::spawn(stopping)),
                    Does::StopsAtOnce(_) => return Did::AtOnce(Box::pin(stopping)),
                    _ => stopping.await,
                }
            }
        }
    }
    Did::It
}
