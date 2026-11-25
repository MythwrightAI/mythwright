//! A run that waits for a person to let it go on
//! (`docs/approvals.md`). A step asks
//! (`approval.ask`, [`crate::actions`]); what it asks is an ask of its
//! own kind (`go`) beside a change of a record's cells, a row of the same
//! table kept where the run's project's records are, told to whoever may
//! decide it as a line of their inbox with a `go` part, and decided at
//! the same door ([`Asks::decide`]), from the line or the run's page.
//!
//! Who may decide is one rule, `kernel::go::may`, asked as things stand
//! each time: a person holding what the run's next stage takes, one of
//! those the environment's rule names where the ask names an
//! environment that says who approves (`environments::project::Rules`),
//! and whoever started the run only where that rule names them.
//!
//! The ask names the run, the step and which of everything done under
//! the step's name it is (`ActionCx::doing`), so that a step done again
//! after a stop finds the ask it made and goes on waiting, or answers
//! what came of it; and what it lets happen is the run going on, so
//! nothing else can be let go by it. Its row and what is said of it are
//! one write; its lines are the factory's, told after and owed on the row
//! until they are, as a change's are. Deciding is one write too, said as
//! `project.approval.decided`, which is what the waiting step hears.
//!
//! It waits as long as its step is given and seven days where that says
//! nothing (`kernel::go::until`); past that it stands as expired wherever
//! it is read, is written so where it is acted on, and nobody approves it.

use conductor::Engine;
use journal::Journal;
use kernel::approvals::{self, Status};
use kernel::go::{self, Asking, Deciding};
use kernel::{Error, Principal, PrincipalKind, Result, Source};
use rusqlite::params;
use serde_json::{Value, json};
use talk::inbox::{Kind, Tell};

use crate::asks::{Ask, Asks, COLUMNS, actor, named_by};

/// What a step asks to be let go on with, and where it was asked.
pub struct Going {
    pub asker: Principal,
    pub project: Option<String>,
    pub run: String,
    pub step: String,
    /// Which of everything done under the step's name asked (`ActionCx::doing`).
    pub doing: String,
    pub asking: Asking,
    /// Whom to tell of those who may decide: each by their id, name or email.
    pub to: Vec<String>,
    /// What it is said as asked from: the step.
    pub source: Source,
}

/// What every event of a go ask carries: the ask, what runs, and whose.
fn about(ask: &Ask, asking: &Asking) -> Value {
    let mut about = json!({
        "ask": ask.id, "run": ask.record, "step": ask.place["step"], "instance": asking.instance,
        "blueprint": asking.blueprint, "environment": asking.environment, "asker": ask.asker().actor(),
    });
    // Kept where its project's events are; one of no project's is nobody's.
    if !ask.project.is_empty() {
        about["project"] = json!(ask.project);
    }
    about
}

impl Asks {
    fn engine(&self) -> Result<&Engine> {
        self.engine
            .as_ref()
            .ok_or_else(|| Error::internal("a run's go-ahead is decided with the runner at hand"))
    }

    /// Who approves what goes under `project`'s environment `environment`,
    /// where it says anybody; nothing where it says nobody or none is named.
    async fn rule_of(&self, project: &str, environment: &str) -> Result<Option<Vec<String>>> {
        if environment.is_empty() {
            return Ok(None);
        }
        let held = self.engine()?.environment_of(project, environment).await?.ok_or_else(|| {
            Error::not_found(format!(
                "`{project}` has no environment `{environment}` whose rule says who approves"
            ))
        })?;
        Ok(Some(held.rules.approve).filter(|approve| !approve.is_empty()))
    }

    /// Whether `who` may decide `ask` as things stand: why not, in a sentence.
    async fn may_go(&self, who: &Principal, ask: &Ask, asking: &Asking) -> std::result::Result<(), String> {
        let rule =
            self.rule_of(&ask.project, &asking.environment).await.map_err(|refused| refused.message)?;
        let (by, starter) = (who.actor(), ask.asker().actor());
        go::may(&Deciding {
            by: &by,
            person: who.kind == PrincipalKind::User,
            holds: who.can(go::TAKES),
            starter: &starter,
            rule: rule.as_deref(),
            environment: &asking.environment,
        })
    }

    /// The ask `doing` of `step` of `run` made, where it made one.
    async fn made_by(&self, run: &str, step: &str, doing: &str) -> Result<Option<Ask>> {
        let made = self.found("record = ?1 AND kind = 'go'", run.to_string()).await?;
        Ok(made.into_iter().find(|ask| ask.place["step"] == step && ask.place["doing"] == doing))
    }

    /// Asks for a run to be let go on: the ask this very step made, where
    /// it is done again, or one made and told to whoever may decide it.
    /// Refused where nobody here may.
    pub async fn go(&self, going: Going) -> Result<Ask> {
        if let Some(made) = self.made_by(&going.run, &going.step, &going.doing).await? {
            // Kept and not yet told where the daemon stopped between the two.
            if made.status() == Status::Waiting {
                self.tell_go(&made).await?;
            }
            return Ok(made);
        }
        let project = going.project.clone().unwrap_or_default();
        if !going.asking.environment.is_empty() && project.is_empty() {
            return Err(Error::invalid(format!(
                "this run is of no project, and so has no environment `{}` to go under",
                going.asking.environment
            )));
        }
        let rule = self.rule_of(&project, &going.asking.environment).await?;
        let starter = going.asker.actor();
        let accounts = self.studio.identity.accounts().await?;
        let could: Vec<String> = accounts
            .iter()
            .map(|account| account.principal())
            .filter(|them| {
                let by = them.actor();
                go::may(&Deciding {
                    by: &by,
                    person: them.kind == PrincipalKind::User,
                    holds: them.can(go::TAKES),
                    starter: &starter,
                    rule: rule.as_deref(),
                    environment: &going.asking.environment,
                })
                .is_ok()
            })
            .map(|them| them.actor())
            .collect();
        let to: Vec<String> = accounts
            .iter()
            .filter(|account| going.to.iter().any(|to| named_by(account, to)))
            .map(|account| format!("user:{}", account.id))
            .collect();
        let named = go::told(&could, &to);
        if named.is_empty() {
            let under = match going.asking.environment.as_str() {
                "" => String::new(),
                environment => {
                    format!(" (what goes under `{environment}` is approved by those its rule names)")
                }
            };
            return Err(Error::denied(format!(
                "nobody here may let this run go on{under}: a person who holds `{}` and did not start it",
                go::TAKES
            )));
        }
        let ask = Ask {
            id: self.journal.new_id("ask"),
            project,
            record: going.run.clone(),
            collection: String::new(),
            kind: approvals::Kind::Go.as_str().to_string(),
            cells: going.asking.kept(),
            updated: 0,
            asker: json!({
                "kind": if going.asker.kind == PrincipalKind::Agent { "agent" } else { "user" },
                "id": going.asker.id, "name": going.asker.name, "key": going.asker.key,
            }),
            why: String::new(),
            named,
            place: json!({ "run": going.run, "step": going.step, "doing": going.doing }),
            status: Status::Waiting.as_str().to_string(),
            decided: None,
            at: self.journal.now_ms(),
        };
        let (row, source) = (ask.clone(), going.source);
        let mut data = about(&ask, &going.asking);
        data["to"] = json!(ask.named);
        self.journal
            .records_of(Some(&ask.project))
            .write(move |writing| {
                writing.transaction().execute(
                    &format!("INSERT INTO asked ({COLUMNS}, owes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, NULL, ?13, 'told')"),
                    params![
                        row.id,
                        row.project,
                        row.record,
                        row.collection,
                        row.kind,
                        row.cells.to_string(),
                        row.updated,
                        row.asker.to_string(),
                        row.why,
                        json!(row.named).to_string(),
                        row.place.to_string(),
                        row.status,
                        row.at
                    ],
                )?;
                writing.say("project.approval.asked", source, data)?;
                Ok(())
            })
            .await?;
        self.tell_go(&ask).await?;
        Ok(ask)
    }

    /// Tells each of those `ask` names, and then takes off that it owed
    /// them: told twice, a line is told once (`said_by`).
    pub(crate) async fn tell_go(&self, ask: &Ask) -> Result<()> {
        let asking = Asking::read(&ask.cells);
        let says = go::line(&ask.asker().name, &asking);
        let project = if ask.project.is_empty() { Value::Null } else { json!(ask.project) };
        for owner in &ask.named {
            let tell = Tell {
                owner: owner.clone(),
                kind: Kind::Told,
                // The run that waits, whose page a line about a run opens:
                // the run and not what runs, whose latest run may be another.
                about: ask.record.clone(),
                place: ask.record.clone(),
                // From the factory's bot, as what a blueprint tells is: the
                // words say whose run it is.
                from: talk::channels::bot_actor(&self.studio.channels.speaker.name()),
                why: format!("asked {}", ask.id),
                says: Some(says.clone()),
                said_by: Some(ask.id.clone()),
                parts: Some(json!([{ "kind": "go", "project": project, "ask": ask.id }])),
            };
            self.studio.channels.inbox.tell(tell).await?;
        }
        self.owed(ask, None).await
    }

    /// Lets the run `found` waits for go on, as `by` (`approve`), or
    /// declines it with `why`. Asked again once it is decided it is
    /// answered as it was decided, and nothing changes. One that has
    /// waited its time is written as expired, and is not approved.
    pub(crate) async fn decide_go(
        &self,
        by: &Principal,
        found: Ask,
        approve: bool,
        why: &str,
    ) -> Result<Ask> {
        let keeper = self.journal.records_of(Some(&found.project));
        let read = found.id.clone();
        let ask = keeper.db().call(move |connection| Self::read_one(connection, &read)).await?;
        let Some(ask) = ask.filter(|ask| ask.status() == Status::Waiting) else {
            return self.one(&found.id).await;
        };
        let asking = Asking::read(&ask.cells);
        if go::stands(Status::Waiting, asking.until, self.journal.now_ms()) == Status::Expired {
            return self.expire_go(&ask).await;
        }
        self.may_go(by, &ask, &asking).await.map_err(Error::denied)?;
        // Who as a person beside who did it (a key is its maker), which an
        // environment's rule is held to where something is started under it.
        let decided = json!({
            "by": actor(by), "person": by.actor(), "name": by.name, "at": self.journal.now_ms(), "why": why.trim(),
        });
        let status = if approve { Status::Approved } else { Status::Declined };
        self.close_go(&ask, status, decided, Source::actor(actor(by))).await
    }

    /// Writes `ask` as having waited its time, as the factory.
    pub async fn expire_go(&self, ask: &Ask) -> Result<Ask> {
        let decided = json!({ "by": "system", "name": "", "at": self.journal.now_ms(), "why": "" });
        self.close_go(ask, Status::Expired, decided, Source::actor("system")).await
    }

    /// Writes `ask` as `status`, with what is said of it, where it waits
    /// still: one write, so that two who decide at once have one decision.
    async fn close_go(&self, ask: &Ask, status: Status, decided: Value, source: Source) -> Result<Ask> {
        let asking = Asking::read(&ask.cells);
        let mut data = about(ask, &asking);
        data["status"] = json!(status.as_str());
        data["by"] = decided["by"].clone();
        if status == Status::Declined {
            data["why"] = decided["why"].clone();
        }
        let id = ask.id.clone();
        self.journal
            .records_of(Some(&ask.project))
            .write(move |writing| {
                let changed = writing.transaction().execute(
                    "UPDATE asked SET status = ?2, decided = ?3, owes = NULL WHERE id = ?1 AND status = 'waiting'",
                    params![id, status.as_str(), decided.to_string()],
                )?;
                if changed > 0 {
                    writing.say("project.approval.decided", source, data)?;
                }
                Ok(())
            })
            .await?;
        self.one(&ask.id).await
    }

    /// What `who` is shown of a run that waits to be let go on: what it
    /// asks, how it stands, and whether they may decide it, with why not.
    pub(crate) async fn shown_go(&self, who: &Principal, ask: &Ask) -> Result<Value> {
        who.require("blueprints:read")?;
        let asking = Asking::read(&ask.cells);
        let standing = go::stands(ask.status(), asking.until, self.journal.now_ms());
        let decided = ask.decided.clone().unwrap_or(Value::Null);
        let may = match standing {
            Status::Waiting => self.may_go(who, ask, &asking).await,
            other => Err(go::decided(
                other,
                &asking.blueprint,
                decided["name"].as_str().unwrap_or_default(),
                decided["why"].as_str().unwrap_or_default(),
            )),
        };
        Ok(json!({
            "ask": ask.id, "kind": "go", "project": ask.project, "run": ask.record, "step": ask.place["step"],
            "instance": asking.instance, "blueprint": asking.blueprint, "says": asking.says,
            "environment": asking.environment, "links": asking.links, "until": asking.until,
            "asker": { "actor": ask.asker().actor(), "name": ask.asker().name },
            "status": standing.as_str(), "decided": ask.decided, "at": ask.at,
            "may": may.is_ok(), "why_not": may.err(),
        }))
    }

    /// The asks of `run` to be let go on, as `who` is shown each, the
    /// newest first: for the run's page.
    pub async fn of_run(&self, who: &Principal, run: &str) -> Result<Vec<Value>> {
        let mut shown = Vec::new();
        for ask in self.found("record = ?1 AND kind = 'go'", run.to_string()).await?.into_iter().rev() {
            shown.push(self.shown_go(who, &ask).await?);
        }
        Ok(shown)
    }
}

/// What a step that waits hears: a go ask decided.
pub const DECIDED: &str = "project.approval.decided";

/// What answers whether a run holds a go-ahead to start something under
/// an environment (`environments::held::Approvals`), installed in what the
/// runner holds of environments: an approved go ask of the run's that
/// names the environment, decided by one of those its rule names now.
/// It is read and never spent. It holds the factory's journal and nothing
/// of the runner, which holds it.
pub struct GoAheads(pub Journal);

impl environments::held::Approvals for GoAheads {
    fn approved<'a>(
        &'a self,
        project: &'a str,
        run: &'a str,
        environment: &'a str,
        by: &'a [String],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            let run = run.to_string();
            let found = self
                .0
                .records_of(Some(project))
                .db()
                .call(move |connection| {
                    let asked = format!(
                        "SELECT {COLUMNS} FROM asked WHERE record = ?1 AND kind = 'go' AND status = 'approved'"
                    );
                    let mut statement = connection.prepare(&asked)?;
                    let rows = statement.query_map([&run], Ask::read)?;
                    rows.collect::<rusqlite::Result<Vec<Ask>>>()
                })
                .await
                .unwrap_or_default();
            found.iter().any(|ask| {
                let decided = ask.decided.clone().unwrap_or_default();
                let person = decided["person"].as_str().or(decided["by"].as_str()).unwrap_or_default();
                Asking::read(&ask.cells).environment == environment && by.iter().any(|one| one == person)
            })
        })
    }
}
