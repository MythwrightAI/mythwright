//! The factory inside a sandbox: made, acted on and ended.
//!
//! Everything here is made new from what [`crate::Made`] hands over: a
//! journal in memory with its projects' beside it, a studio over that
//! journal which reaches nothing but its tables and one folder, a
//! made-up person who owns it, one made-up project, and a conductor
//! with the sandbox's own registry. Nothing here is found, opened or
//! asked of anything that was there before.

use std::sync::Arc;
use std::time::Duration;

use conductor::{Applied, Apply, Engine, Layers, MemoryLibrary};
use journal::{Db, Journal, Logs, Order};
use kernel::{Clock, Error, Fields, ManualClock, Principal, Result, Source};
use serde_json::{Value, json};
use studio::board::{Changes, New};
use studio::{Place, Studio};
use tokio::task::JoinHandle;

use crate::at::At;
use crate::offered::{Doing, Kind, Working, offered};
use crate::outside::Chosen;
use crate::rest::Parked;
use crate::stood::{Record, Stood};
use crate::{Act, Limits, Made, PROJECT};

/// What whoever runs a sandbox test may do to the tables as the made-up
/// person, each called what the action that does the same is called and
/// taking what that takes.
pub const DOES: &[&str] = &[
    "board.add",
    "board.move",
    "board.update",
    "record.add",
    "record.update",
    "record.remove",
    "record.comment",
    "channel.ensure",
    "channel.post",
];

/// What is written in the folder of a sandbox's project, so that it is a
/// folder with something in it: one that is taken up as it is, with
/// nothing made in it.
const MARKER: &str = "SANDBOX.md";

pub(crate) struct Inside {
    pub id: String,
    pub name: String,
    pub source: String,
    pub clock: ManualClock,
    /// What its clock said when it was made: how long after that
    /// something came about is read from it.
    pub began: i64,
    pub journal: Journal,
    pub order: Arc<Order>,
    /// Where the order of what is said began, and where it stood when
    /// the sandbox was made: what was said in the making is not shown.
    pub begins: u64,
    pub made_at: u64,
    /// Told when the sandbox's runtime has nothing left to run, and how
    /// many of its own actions are in the middle of their work: what
    /// says whether it is at rest (`crate::rest`).
    pub parked: Parked,
    pub working: Working,
    /// What it is at, for whoever waits on it.
    pub at: At,
    /// Which run each action of the sandbox's own was done for.
    pub doing: Arc<Doing>,
    /// The logs of its steps, which count toward what it may keep.
    pub logs: Logs,
    pub studio: Studio,
    pub engine: Engine,
    pub owner: Principal,
    pub stood: Arc<Stood>,
    pub record: Arc<Record>,
    pub kinds: std::collections::BTreeMap<String, Kind>,
    pub limits: Limits,
    /// The database what is said would be searched in, which nothing
    /// here searches: a studio is opened with one.
    pub search: Db,
    /// Why it was stopped, where it came to more than a sandbox may.
    pub over: Option<String>,
    /// How many steps it had run and how many bytes it kept when it
    /// last came to rest.
    pub ran: u64,
    pub keeps: u64,
    /// An apply that has not answered: its steps wait for a stand-in.
    applying: Option<JoinHandle<Result<Applied>>>,
    /// What each apply that answered came to.
    pub applied: Vec<Value>,
}

fn text(args: &Fields, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string).filter(|text| !text.is_empty())
}

fn needed(args: &Fields, action: &str, key: &str) -> Result<String> {
    text(args, key).ok_or_else(|| Error::invalid(format!("`{action}` needs `{key}`")))
}

fn fields(args: &Fields, key: &str) -> Fields {
    args.get(key).and_then(Value::as_object).cloned().unwrap_or_default()
}

fn strings(args: &Fields, key: &str) -> Option<Vec<String>> {
    args.get(key).and_then(Value::as_array).map(|items| items.iter().map(blueprint::expr::display).collect())
}

/// `params` with `project` said, where the text `source` asks for one:
/// a sandbox has one project, and a text that is told which project it
/// is for is for that one.
fn in_its_project(source: &str, mut params: Fields) -> Result<Fields> {
    let asks = blueprint::check(source, &blueprint::NoCatalog)
        .blueprint
        .is_some_and(|read| read.params.iter().any(|param| param.name == "project"));
    match params.get("project").and_then(Value::as_str) {
        Some(said) if asks && said != PROJECT => Err(Error::invalid(format!(
            "a sandbox has one project, a made-up one called `{PROJECT}`, and this was told `project` is \
             `{said}`. Leave it out, or say `{PROJECT}`: nothing of `{said}` is in a sandbox but its own \
             blueprints, where it was named as the project they are read from"
        ))),
        None if asks => {
            params.insert("project".to_string(), json!(PROJECT));
            Ok(params)
        }
        _ => Ok(params),
    }
}

impl Inside {
    pub async fn make(made: Made, parked: Parked, at: At) -> Result<Inside> {
        let folder = made.folder.clone();
        let project = folder.join("projects").join(PROJECT);
        std::fs::create_dir_all(&project)?;
        std::fs::write(
            project.join(MARKER),
            "This folder is a sandbox's own: a blueprint is being run in a factory made for that, and this is \
             the folder of its one made-up project. It is removed when the sandbox ends, and by the daemon as \
             it next starts.\n",
        )?;
        let clock = ManualClock::new();
        let ahead = made.now_ms.saturating_sub(clock.now_ms());
        if ahead > 0 {
            clock.advance(Duration::from_millis(ahead as u64));
        }
        at.now("opening its journal");
        let journal = Journal::open(Db::memory()?, Arc::new(clock.clone()), "sandbox").await?;
        at.keeps_in(journal.dbs());
        // The order things are said in is written down: a clock that
        // stands still says nothing of it, and what is shown is in it.
        let order = Arc::new(Order::default());
        let begins = order.so_far();
        journal.orders_by(order.clone());
        journal.keeps_projects_in_memory().await?;
        let search = Db::memory()?;
        at.now("opening its studio");
        let studio = Studio::open_within(journal.clone(), search.clone(), &folder).await?;
        // A made-up person, who is the sandbox's owner and nobody anywhere
        // else: the account is a row of the sandbox's own journal.
        at.now("registering its made-up person");
        let signed = studio
            .identity
            .register("somebody@sandbox.invalid", "Somebody trying this", &kernel::id::secret(), None, true)
            .await?;
        let owner = Principal { scopes: vec!["*".to_string()], ..signed.account.principal() };
        studio.settle().await?;
        at.now("making its project");
        studio.projects.create(PROJECT, Some(project), &owner).await?;
        let mut dbs = journal.dbs();
        dbs.push(search.clone());
        at.keeps_in(dbs);

        let own = MemoryLibrary::new();
        for (name, text) in &made.own {
            own.put(name, text);
        }
        own.put(&made.name, &made.source);
        let library = Layers::new().then(Arc::new(own)).then(made.ships.clone());
        let chosen = Chosen::of(&made);
        let (working, doing) = (at.working.clone(), Doing::new(order.clone()));
        let offered =
            offered(&studio, &made.actions, &made.events, made.knows_events, &chosen, (&working, &doing));
        let logs = Logs::new(folder.join("logs"));
        logs.say_what_is_handed(true);
        let engine = Engine::new(journal.clone(), logs.clone(), offered.registry, Arc::new(library)).await?;
        studio.attach(&engine);
        at.now("starting its conductor");
        engine.resume().await?;
        Ok(Inside {
            at,
            begins,
            made_at: order.so_far(),
            parked,
            working,
            doing,
            logs,
            id: made.id,
            name: made.name,
            source: made.source,
            began: clock.now_ms(),
            clock,
            journal,
            order,
            studio,
            engine,
            owner,
            stood: chosen.stood,
            record: chosen.record,
            kinds: offered.kinds,
            limits: made.limits,
            search,
            over: None,
            ran: 0,
            keeps: 0,
            applying: None,
            applied: Vec::new(),
        })
    }

    /// Does `act`, waits for what comes of it to be at rest, and says
    /// what the act itself came to.
    pub async fn act(&mut self, act: Act) -> Result<Value> {
        if let Some(over) = &self.over {
            return Err(Error::conflict(over.clone()));
        }
        let did = match act {
            Act::Apply { blueprint, params, key } => {
                if self.applying.is_some() {
                    return Err(Error::conflict(
                        "a text was applied and its steps have not ended: a stand-in waits to be answered \
                         (`waiting` lists them). Answer it first",
                    ));
                }
                let (name, source) = match blueprint.filter(|name| *name != self.name) {
                    None => (self.name.clone(), self.source.clone()),
                    Some(name) => match self.engine.library().load(&name, Some(PROJECT)) {
                        Some(source) => (name, source),
                        None => {
                            return Err(Error::not_found(format!(
                                "there is no blueprint called `{name}` for a sandbox to apply: it has the \
                                 text it was made for, the blueprints that ship, and those of the project \
                                 it was told to read"
                            )));
                        }
                    },
                };
                let request = Apply {
                    params: in_its_project(&source, params)?,
                    source: Some(source),
                    key,
                    project: Some(PROJECT.to_string()),
                    ..Apply::new(name, self.owner.clone())
                };
                let engine = self.engine.clone();
                self.applying = Some(tokio::spawn(async move { engine.apply(request).await }));
                json!({})
            }
            Act::Say { topic, data } => self.said_by_hand(&topic, data).await?,
            Act::Do { action, with } => self.does(&action, with).await?,
            Act::Advance { by } => {
                let ms = blueprint::duration::parse(&by).map_err(Error::invalid)?;
                self.clock.advance(Duration::from_millis(ms));
                json!({ "advanced_ms": ms })
            }
            Act::Answer { asked, gives, fails } => {
                self.stood.answer(asked, gives, fails)?;
                json!({ "answered": asked })
            }
            Act::Answers { answers } => {
                self.stood.answers(answers);
                json!({ "answers": answers })
            }
        };
        self.settle().await;
        let mut did = did;
        if let Some(came) = self.applied_now().await {
            did["applied"] = came?;
        }
        Ok(did)
    }

    /// What an apply that has answered since it was last asked came to:
    /// nothing while none has, the refusal where it was refused.
    async fn applied_now(&mut self) -> Option<Result<Value>> {
        if !self.applying.as_ref().is_some_and(JoinHandle::is_finished) {
            return None;
        }
        let came = match self.applying.take()?.await {
            Ok(Ok(applied)) => Ok(json!(applied)),
            Ok(Err(refused)) => Err(refused),
            Err(crashed) => Err(Error::internal(format!("applying the text crashed: {crashed}"))),
        };
        if let Ok(applied) = &came {
            self.applied.push(applied.clone());
        }
        Some(came)
    }

    /// Whether an apply has not answered yet.
    pub fn is_applying(&self) -> bool {
        self.applying.is_some()
    }

    /// Says an event as the made-up person: held to what its topic is
    /// described as carrying, where it is one that is described.
    async fn said_by_hand(&self, topic: &str, data: Value) -> Result<Value> {
        let data = match data {
            Value::Null => json!({}),
            Value::Object(_) => data,
            other => {
                return Err(Error::invalid(format!(
                    "what an event carries is named fields, and `{topic}` was given {other}"
                )));
            }
        };
        if topic.trim().is_empty() {
            return Err(Error::invalid("say which event: its `topic`"));
        }
        let known = self.engine.registry().events();
        if known.get(topic).is_some()
            && let Some(fault) = known.fault(topic, &data)
        {
            return Err(Error::invalid(format!(
                "{fault}. Nothing was said: say it with what `{topic}` carries"
            )));
        }
        self.journal.emit(topic, Source::actor(self.owner.actor()), data).await?;
        Ok(json!({ "said": topic }))
    }

    /// Does one of [`DOES`] to the sandbox's tables as the made-up person.
    async fn does(&self, action: &str, with: Fields) -> Result<Value> {
        let (studio, by) = (&self.studio, &self.owner);
        let project = text(&with, "project").unwrap_or_else(|| PROJECT.to_string());
        Ok(match action {
            "board.add" => {
                let card = New {
                    title: text(&with, "title").unwrap_or_default(),
                    body: text(&with, "body").unwrap_or_default(),
                    status: text(&with, "status"),
                    labels: strings(&with, "labels").unwrap_or_default(),
                    more: fields(&with, "fields"),
                };
                json!(studio.board.add_new(&project, card, by).await?)
            }
            "board.move" => {
                let (id, to) = (needed(&with, action, "id")?, needed(&with, action, "to")?);
                let note = text(&with, "note");
                json!(studio.board.move_through(&id, &to, None, note.as_deref(), by, None).await?)
            }
            "board.update" => {
                let cleared = |key: &str| {
                    with.get(key)
                        .and_then(Value::as_str)
                        .map(|said| (!said.is_empty()).then(|| said.to_string()))
                };
                let changes = Changes {
                    more: fields(&with, "fields"),
                    title: text(&with, "title"),
                    body: with.get("body").and_then(Value::as_str).map(str::to_string),
                    assignee: cleared("assignee"),
                    labels: strings(&with, "labels"),
                    due: cleared("due"),
                };
                json!(studio.board.update(&needed(&with, action, "id")?, changes, by).await?)
            }
            "record.add" => {
                let collection = needed(&with, action, "collection")?;
                json!(studio.records.add(&project, &collection, fields(&with, "fields"), by, None).await?)
            }
            "record.update" => {
                let id = needed(&with, action, "id")?;
                json!(studio.records.update(&id, fields(&with, "fields"), by, None).await?)
            }
            "record.remove" => {
                studio.records.remove(&needed(&with, action, "id")?, by, None).await?;
                json!({})
            }
            "record.comment" => {
                let (id, body) = (needed(&with, action, "id")?, needed(&with, action, "body")?);
                let public = with.get("public").and_then(Value::as_bool).unwrap_or(false);
                json!(studio.records.comment(&id, &body, public, by, None).await?)
            }
            "channel.ensure" => {
                // A channel for the text to say things in, as somebody would make one: by its name, open.
                let name = needed(&with, action, "name")?;
                let topic = text(&with, "topic").unwrap_or_default();
                json!(studio.channels.ensure(&name, Some(&project), &topic, by).await?)
            }
            "channel.post" => {
                let (channel, body) = (needed(&with, action, "channel")?, needed(&with, action, "body")?);
                let (reply_to, thread) = (text(&with, "reply_to"), text(&with, "thread"));
                let place =
                    Place { reply_to: reply_to.as_deref(), thread: thread.as_deref(), ..Place::default() };
                json!(studio.channels.post_at(&channel, by, &body, place).await?)
            }
            other => {
                return Err(Error::invalid(format!(
                    "`{other}` is not something done to a sandbox's tables by hand. What is: {}. Anything else a \
                     blueprint does is done by applying one",
                    DOES.join(", ")
                )));
            }
        })
    }

    /// Every database the sandbox keeps anything in.
    pub fn dbs(&self) -> Vec<Db> {
        let mut dbs = self.journal.dbs();
        dbs.push(self.search.clone());
        dbs
    }

    /// Ends everything the sandbox runs and lets go of what it keeps: its
    /// conductors halted, and each database closed and its thread ended.
    pub async fn end(mut self) {
        if let Some(applying) = self.applying.take() {
            applying.abort();
        }
        self.at.now("halting its conductors");
        self.engine.halt();
        self.engine.logs().close();
        let dbs = self.dbs();
        for db in &dbs {
            db.close();
        }
        self.at.now("waiting for its databases to have closed");
        for db in &dbs {
            db.closed().await;
        }
    }
}
