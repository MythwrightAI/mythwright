//! What a blueprint asks of a person: that its run be let go on
//! (`approval.ask`, [`crate::goes`]). The step makes one ask, tells whoever
//! may decide it, and waits for it to be decided; approved, it gives who
//! let it go on; declined, it fails with why, which `on_fail` or
//! `allow_failure` take as any failure; and past the time it waits it
//! fails as having waited its time. It is the same done twice: done again
//! after a stop it finds its ask by its run, its step and which of
//! everything done under the step's name it is, and goes on waiting or
//! answers what came of it.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionCx, ActionError, ActionResult, Registry};
use journal::Journal;
use kernel::approvals::Status;
use kernel::go::{self, Asking};
use kernel::{BoxFut, Fields};
use serde_json::{Value, json};
use studio::Studio;

use crate::asks::Asks;
use crate::goes::{DECIDED, Going};

/// An example of a run that waits to be let go on, for the Reference.
const GO_ASKED: &str = "\
blueprint: deploy-site
description: Build the site, and put it live once somebody lets it.
permissions: [shell]

steps:
  build:
    do: shell
    with: { cmd: [make, site] }

  approve:
    needs: [build]
    do: approval.ask
    with: { says: The site is built and waits to go live. }

  live:
    needs: [approve]
    do: shell
    with: { cmd: [make, live] }
";

/// Adds `approval.ask`, over the factory's journal and its studio.
pub fn register(registry: &mut Registry, journal: &Journal, studio: &Studio) {
    let (journal, studio) = (journal.clone(), studio.clone());
    registry.add_fn(
        ActionSpec::new(
            "approval.ask",
            "Wait for a person to let the run go on, telling whoever may decide in their inbox.",
        )
        .example(GO_ASKED)
        .required("says", T::String, "what is to go on, in the blueprint's words: whoever may decide is told it")
        .arg(
            "to",
            T::List,
            "whom to tell, of those who may decide, each a person by their id, name or email; where it names \
             none of them, all of them are told",
        )
        .arg(
            "environment",
            T::String,
            "the environment what follows is to go under, whose rule says who approves (default: the one this \
             runs under)",
        )
        .arg("links", T::List, "addresses to look at before deciding, each a page on the web (http or https)")
        .output("approved_by", "who let it go on, as an actor")
        .output("approved_by_name", "who let it go on, by name")
        .output("ask", "the ask, by its id")
        .stands(json!({ "approved_by": "user:somebody", "approved_by_name": "Somebody", "ask": "ask_stood_in" }))
        // The same done twice: its ask keeps which step and which of
        // everything done under its name asked, and done again it finds it.
        .idempotent(),
        move |cx, args| -> BoxFut<'static, ActionResult> {
            let asks = Asks::new(&journal, &studio).reading(cx.engine());
            let journal = journal.clone();
            Box::pin(async move { ask(asks, journal, cx, args).await })
        },
    );
}

fn words(args: &Fields, key: &str) -> Result<Vec<String>, ActionError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::String(one)) => Ok(vec![one.clone()]),
        Some(Value::Array(several)) => several
            .iter()
            .map(|one| one.as_str().map(str::to_string))
            .collect::<Option<Vec<String>>>()
            .ok_or_else(|| ActionError::new(format!("`{key}` is a list of words"))),
        Some(other) => Err(ActionError::new(format!("`{key}` is a list of words; it was given {other}"))),
    }
}

async fn ask(asks: Asks, journal: Journal, cx: ActionCx, args: Fields) -> ActionResult {
    let says = args.get("says").and_then(Value::as_str).unwrap_or_default().trim().to_string();
    let links = words(&args, "links")?;
    if let Some(link) = links.iter().find(|link| !kernel::parts::is_address(link)) {
        return Err(ActionError::new(format!(
            "`{link}` is no address a link leads to: it begins `https://` or `http://`"
        )));
    }
    let environment = match args.get("environment").and_then(Value::as_str).map(str::trim) {
        Some(named) if !named.is_empty() => named.to_string(),
        // The one this runs under, whose rule then says who approves.
        _ => cx.engine().instance(cx.instance()).await?.environment,
    };
    let at = journal.now_ms();
    let asking = Asking {
        says,
        blueprint: cx.blueprint().to_string(),
        instance: cx.instance().to_string(),
        environment,
        links,
        until: go::until(at, cx.limit().map(|limit| limit.as_millis() as u64)),
    };
    // Followed from before the ask is read, so that a decision made in
    // between is heard.
    let mut follow = journal.follow_every(journal.now(), DECIDED);
    let going = Going {
        asker: cx.principal().clone(),
        project: cx.project().map(str::to_string),
        run: cx.run().to_string(),
        step: cx.step().to_string(),
        doing: cx.doing().to_string(),
        asking,
        to: words(&args, "to")?,
        source: cx.source(),
    };
    let id = asks.go(going).await?.id;
    loop {
        let ask = asks.one(&id).await?;
        let asking = Asking::read(&ask.cells);
        let now = journal.now_ms();
        let decided = ask.decided.clone().unwrap_or(Value::Null);
        let name = decided["name"].as_str().unwrap_or_default();
        match go::stands(ask.status(), asking.until, now) {
            Status::Waiting => {}
            Status::Approved => {
                return Ok(json!({ "approved_by": decided["by"], "approved_by_name": name, "ask": id }));
            }
            status => {
                // Written so where it is acted on: nobody approves it after.
                if ask.status() == Status::Waiting {
                    asks.expire_go(&ask).await?;
                }
                let why = decided["why"].as_str().unwrap_or_default();
                let said = go::decided(status, &asking.blueprint, name, why);
                return Err(ActionError::new(said).with_data(json!({ "ask": id, "status": status.as_str() })));
            }
        }
        let left = std::time::Duration::from_millis((asking.until - now).max(0) as u64);
        tokio::select! {
            biased;
            () = cx.cancel().cancelled() => return Err(ActionError::cancelled()),
            heard = follow.next_or_behind() => { heard?; }
            () = cx.clock().sleep(left) => {}
        }
    }
}
