//! Records kept by a store service.
//!
//! A store service is a process that some blueprint made and hosts, and
//! that keeps a collection's records in a database of its own. The
//! application names no database. It speaks one small protocol, HTTP with
//! JSON, to whatever answers at the address the collection was declared
//! with, and this module is the application's half of it:
//!
//! ```text
//! GET    /records?project=<p>&collection=<c>   { "records": [ ... ] }
//! PUT    /records/<id>                         a record, new or replacing
//! DELETE /records/<id>                         gone, whether or not it was there
//! ```
//!
//! A program that answers these is a store, whatever it keeps the records
//! in. The whole of it, with what a refusal looks like, is in
//! `docs/stores.md`.

use std::time::{Duration, Instant};

use journal::Journal;
use kernel::{Error, ErrorKind, Result};
use reqwest::{Method, StatusCode, Url};
use serde::Deserialize;

use crate::collections::{Collection, Record, Store};

/// How long a store has to take the connection. It is nearly always on
/// this device, so one that has not by now is not there.
const CONNECT: Duration = Duration::from_secs(2);
/// How long it has to answer. Whoever asked for a board is waiting.
const ANSWER: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub(crate) struct StoreService {
    client: reqwest::Client,
    journal: Journal,
}

#[derive(Deserialize)]
struct Listing {
    records: Vec<Record>,
}

/// What a store says when it will not do something.
#[derive(Deserialize)]
struct Refusal {
    error: String,
}

/// The address of a store service as it is kept: `http://host:port`, with
/// no `/` after it.
pub(crate) fn address(written: &str) -> Result<String> {
    let trimmed = written.trim().trim_end_matches('/');
    let plain = Url::parse(trimmed).is_ok_and(|url| url.scheme() == "http" && url.has_host());
    if !plain {
        return Err(Error::invalid(format!(
            "`{written}` is not the address of a store service. One is written like http://127.0.0.1:7801, and is what the blueprint that hosts the store gives out as `url`"
        )));
    }
    Ok(trimmed.to_string())
}

/// The address a collection called `name` is to have: the one `written`
/// for it, or for one already kept by a store service and declared again
/// without one, the address it had. A collection kept anywhere else has
/// none, and is refused one.
pub(crate) fn place(
    name: &str,
    store: Store,
    written: Option<&str>,
    was: Option<&Collection>,
) -> Result<Option<String>> {
    match (store, written) {
        (Store::Service, Some(at)) => address(at).map(Some),
        (Store::Service, None) => was.and_then(|was| was.at.clone()).map(Some).ok_or_else(|| {
            Error::invalid(format!(
                "`{name}` is to be kept by a store service and needs `at`: the address the service answers at, which the blueprint that hosts it gives out as `url`"
            ))
        }),
        (Store::Documents | Store::Mockups, written) => crate::documents::place(name, written, was).map(Some),
        (kind, Some(at)) => Err(Error::invalid(format!(
            "`at: {at}` is the address of a store service, and `{name}` is kept in `{}`; say `store: service`, or leave `at` out",
            kind.as_str()
        ))),
        (_, None) => Ok(None),
    }
}

impl StoreService {
    pub fn new(journal: Journal) -> StoreService {
        // Straight to the address, whatever proxy this device is set up
        // with: the store is where its blueprint put it, usually here, and
        // a request for it must not be handed to anything else. And on a
        // connection of its own each time: a store is restarted and
        // replaced behind its address, and a request must not be sent down
        // a connection to the one that was there before.
        let client = reqwest::Client::builder()
            .no_proxy()
            .pool_max_idle_per_host(0)
            .connect_timeout(CONNECT)
            .timeout(ANSWER)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        StoreService { client, journal }
    }

    /// Every record of a collection, in the order they were made.
    pub async fn records(&self, collection: &Collection) -> Result<Vec<Record>> {
        let mut url = self.url(collection, None)?;
        url.query_pairs_mut()
            .append_pair("project", &collection.project)
            .append_pair("collection", &collection.name);
        let answer = self.ask(collection, "list", "list its records", Method::GET, url, None).await?;
        let mut listing: Listing = answer.json().await.map_err(|err| {
            Error::unavailable(format!(
                "the store service that keeps `{}` at {} answered with something that is not a list of records ({err}); is that address a store?",
                collection.name,
                collection.at.as_deref().unwrap_or_default()
            ))
        })?;
        listing.records.sort_by_key(|record| record.number);
        Ok(listing.records)
    }

    /// What the store at `at` holds of a collection that is about to be
    /// declared there, which is asked to find out that it answers.
    pub async fn answers(&self, project: &str, name: &str, at: Option<String>) -> Result<Vec<Record>> {
        let asked = Collection {
            project: project.to_string(),
            name: name.to_string(),
            model: None,
            fields: Vec::new(),
            views: Vec::new(),
            history: Vec::new(),
            store: Store::Service,
            at,
            owner: None,
            takes: None,
            reads: None,
            steers: None,
            shown: Default::default(),
            created: 0,
            updated: 0,
        };
        self.records(&asked).await
    }

    /// Stores a record, new or in place of the one with its id.
    pub async fn put(&self, collection: &Collection, record: &Record) -> Result<()> {
        let url = self.url(collection, Some(&record.id))?;
        self.ask(collection, "put", "keep a record", Method::PUT, url, Some(record)).await.map(|_| ())
    }

    /// Removes a record. One that is not there is already removed.
    pub async fn remove(&self, collection: &Collection, id: &str) -> Result<()> {
        let url = self.url(collection, Some(id))?;
        self.ask(collection, "remove", "remove a record", Method::DELETE, url, None).await.map(|_| ())
    }

    /// `<address>/records`, or `<address>/records/<id>`.
    fn url(&self, collection: &Collection, id: Option<&str>) -> Result<Url> {
        let broken = || {
            Error::internal(format!(
                "`{}` is kept by a store service and has no usable address; apply the blueprint that declared it again, with `at`",
                collection.name
            ))
        };
        // The one way a table kept by a store comes to have no address: it
        // is in a factory started from a copy of another, and a copy does
        // not point at the store of the factory it was made from.
        let nowhere = || {
            Error::unavailable(format!(
                "`{name}` of `{project}` is kept by a store service, and has no store here: this factory was started from a copy of another, and a copy does not reach into the store of the factory it was made from. Host a store here and say where it is by applying the blueprint that declared `{name}` again with its address (for a board: `{cli} apply std/store --project {project}`, then `{cli} apply std/board --project {project} --param store=service --param database=<its url>`)",
                name = collection.name,
                project = collection.project,
                cli = brand::CLI,
            ))
        };
        let at = collection.at.as_deref().ok_or_else(nowhere)?;
        let mut url = Url::parse(&format!("{at}/records")).map_err(|_| broken())?;
        if let Some(id) = id {
            // As one part of the path, whatever is in it.
            url.path_segments_mut().map_err(|()| broken())?.push(id);
        }
        Ok(url)
    }

    /// Asks the store for one thing, and measures how long it took to
    /// answer: a store is a process of its own, and a slow or absent one
    /// should show as that and not as a slow board.
    async fn ask(
        &self,
        collection: &Collection,
        op: &str,
        what: &str,
        method: Method,
        url: Url,
        body: Option<&Record>,
    ) -> Result<reqwest::Response> {
        let started = Instant::now();
        let mut request = self.client.request(method, url);
        if let Some(record) = body {
            request = request.json(record);
        }
        let answered = match request.send().await {
            Ok(answer) if answer.status().is_success() => Ok(answer),
            Ok(answer) => Err(refused(collection, what, answer).await),
            Err(err) => Err(silent(collection, &err)),
        };
        let labels = [("op", op), ("project", collection.project.as_str()), ("collection", &collection.name)];
        self.journal.measure(
            "store.request",
            &labels,
            started.elapsed().as_secs_f64() * 1000.0,
            answered.is_ok(),
        );
        answered
    }
}

/// The store did not answer at all.
fn silent(collection: &Collection, err: &reqwest::Error) -> Error {
    let why = if err.is_connect() {
        "nothing is answering there"
    } else if err.is_timeout() {
        "it did not answer in time"
    } else {
        "it could not be reached"
    };
    let cli = brand::CLI;
    Error::unavailable(format!(
        "`{}` of `{}` is kept by a store service at {}, and {why}. Start the blueprint that hosts the store, or look at why it stopped: `{cli} services` and `{cli} instances` show what is running",
        collection.name,
        collection.project,
        collection.at.as_deref().unwrap_or_default()
    ))
}

/// The store answered, and said no.
async fn refused(collection: &Collection, what: &str, answer: reqwest::Response) -> Error {
    let status = answer.status();
    let kind = match status {
        StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => ErrorKind::Invalid,
        StatusCode::NOT_FOUND => ErrorKind::NotFound,
        StatusCode::CONFLICT => ErrorKind::Conflict,
        _ => ErrorKind::Unavailable,
    };
    let said = match answer.json::<Refusal>().await {
        Ok(refusal) => refusal.error,
        Err(_) => format!("it answered {status} and did not say why"),
    };
    Error::new(
        kind,
        format!(
            "the store service that keeps `{}` at {} would not {what}: {said}",
            collection.name,
            collection.at.as_deref().unwrap_or_default()
        ),
    )
}
