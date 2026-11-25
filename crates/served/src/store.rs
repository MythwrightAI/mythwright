//! The store program: a store service that keeps records in one database
//! file.
//!
//! A table's records may be kept by a store service, a process that speaks
//! a small protocol (`docs/stores.md`) and that a blueprint makes and
//! hosts. This is the one that ships, and the reference for writing
//! another. It is not part of the daemon: it is run as a program of its
//! own (the `store serve` command), by a blueprint (`std/store`), like
//! anything else that is hosted. The daemon does not start it and cannot
//! tell that this is what answers at the address it was given.
//!
//! It keeps what it is handed and nothing more: what a record's fields
//! are, and whether a record may be put, is the business of whoever
//! declared the table.

use std::net::SocketAddr;
use std::path::Path;

use axum::extract::{Path as Named, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use axum::{Json, Router};
use journal::Db;
use kernel::{Error, ErrorKind, Result};
use rusqlite::params;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use windowed::Record;

const SCHEMA: &[&str] = &["
    CREATE TABLE records (
        id TEXT PRIMARY KEY,
        project TEXT NOT NULL,
        collection TEXT NOT NULL,
        number INTEGER NOT NULL,
        data TEXT NOT NULL,
        position REAL NOT NULL,
        created_by TEXT NOT NULL,
        created INTEGER NOT NULL,
        updated INTEGER NOT NULL
    );
    CREATE INDEX records_collection ON records (project, collection);
"];

/// How long a store that stops waits for its database's file to be free:
/// as long as a daemon's own stop waits for its own (`api`'s `PUT_AWAY`).
const PUT_AWAY: std::time::Duration = std::time::Duration::from_secs(60);

const RECORD: &str = "id, project, collection, number, data, position, created_by, created, updated";

/// A store that is being served.
pub struct StoreServer {
    pub addr: SocketAddr,
    stop: oneshot::Sender<()>,
    served: JoinHandle<()>,
    listening: Listening,
    /// The database it serves, kept to be closed when it stops.
    db: Db,
}

/// A listening socket, kept so that it can be told to stop listening.
///
/// Closing a listener does not always end it. A program that this process
/// is starting at that moment has, until it becomes the program it was
/// started as, a copy of everything the process has open, the listener
/// among it. For that instant the socket is still listening: a connection
/// to it is taken, and never answered, and its address cannot be listened
/// on again. Shutting the socket down ends it for everyone who holds it.
struct Listening {
    #[cfg(unix)]
    socket: Option<std::os::fd::OwnedFd>,
}

impl Listening {
    fn of(listener: &TcpListener) -> Listening {
        #[cfg(unix)]
        {
            use std::os::fd::AsFd;
            Listening { socket: listener.as_fd().try_clone_to_owned().ok() }
        }
        #[cfg(not(unix))]
        {
            let _ = listener;
            Listening {}
        }
    }

    /// Stops it listening. Connections it has already taken are sockets of
    /// their own and carry on.
    fn end(self) {
        // There is no asking a listener to shut down, only a connection;
        // the system makes no such distinction, and is asked through one.
        #[cfg(unix)]
        if let Some(socket) = self.socket {
            let _ = std::net::TcpStream::from(socket).shutdown(std::net::Shutdown::Both);
        }
    }
}

/// What the store says when it will not do something: a sentence, as
/// `{ "error": ... }`, under a status that fits.
struct Refusal(StatusCode, String);

impl IntoResponse for Refusal {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl From<Error> for Refusal {
    fn from(err: Error) -> Self {
        let status = match err.kind {
            ErrorKind::Invalid => StatusCode::BAD_REQUEST,
            ErrorKind::NotFound => StatusCode::NOT_FOUND,
            ErrorKind::Conflict => StatusCode::CONFLICT,
            ErrorKind::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Refusal(status, err.message)
    }
}

type Answer = std::result::Result<Json<Value>, Refusal>;

#[derive(Deserialize)]
struct Which {
    project: Option<String>,
    collection: Option<String>,
}

/// The database at `file`, made if it is not there, with the table a store
/// keeps its records in.
async fn open(file: &Path) -> Result<Db> {
    let db = Db::open(file)?;
    db.migrate("store", SCHEMA).await?;
    Ok(db)
}

/// Makes the database at `file` if it is not there, and says how many
/// records it holds: none, for one that was just made. What is in one that
/// was there is left as it is.
pub async fn create(file: &Path) -> Result<i64> {
    let db = open(file).await?;
    let held = db
        .call(|connection| connection.query_row("SELECT count(*) FROM records", [], |row| row.get(0)))
        .await;
    put_away(&db).await;
    held
}

/// Closes a store's database and waits until its file is free. Letting go
/// of a database asks its thread to end and does not wait for it, and a
/// store started on the same file in the moment between was told the
/// database was locked, by the one before it putting the file away: once
/// in eleven runs of the tests that drive the whole product, held to
/// four processors. Bounded, as a daemon's own stop is, and by as much:
/// closing brings the database's log back and holds the file to the disk.
async fn put_away(db: &Db) {
    db.close();
    let _ = tokio::time::timeout(PUT_AWAY, db.closed()).await;
}

impl StoreServer {
    /// Serves the records in the database at `file`, which is made if it
    /// is not there, at `listen`. Port `0` picks a free one; `addr` says
    /// which.
    pub async fn start(file: &Path, listen: SocketAddr) -> Result<StoreServer> {
        // Where it is to listen is taken before the database is opened:
        // a store that cannot listen has opened nothing, and leaves no
        // file to be put away behind a refusal.
        let listener = TcpListener::bind(listen)
            .await
            .map_err(|err| Error::unavailable(format!("cannot listen on {listen}: {err}")))?;
        let addr = listener.local_addr()?;
        let db = open(file).await?;
        let listening = Listening::of(&listener);
        let router = Router::new()
            .route("/health", get(health))
            .route("/records", get(list))
            .route("/records/{id}", put(keep).delete(remove))
            .fallback(nothing_there)
            .method_not_allowed_fallback(not_done)
            // A record that is more than is taken is read to its end
            // before it is refused, as the daemon's own routes do.
            .layer(axum::middleware::from_fn(crate::sent::read_through))
            .with_state(db.clone());
        let (stop, stopped) = oneshot::channel::<()>();
        let served = tokio::spawn(async move {
            // Told to stop, it closes the connections it has as well as
            // taking no more: a store that has stopped must not go on
            // answering whoever was already talking to it.
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    let _ = stopped.await;
                })
                .await;
        });
        Ok(StoreServer { addr, stop, served, listening, db })
    }

    /// The address a table kept here is declared with.
    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Stops serving, and returns when nothing is answering any more: a
    /// connection made after this is refused, the address is free, and so
    /// is the database's file, for a store that is started straight after
    /// on the same one.
    pub async fn stop(self) {
        let _ = self.stop.send(());
        self.listening.end();
        let _ = self.served.await;
        put_away(&self.db).await;
    }
}

/// Whether it can serve: the database is asked something that costs the
/// same however much it holds, since this is asked every second or two.
async fn health(State(db): State<Db>) -> Answer {
    db.call(|connection| {
        connection
            .query_row("SELECT count(*) FROM (SELECT 1 FROM records LIMIT 1)", [], |row| row.get::<_, i64>(0))
    })
    .await
    .map_err(|err| Refusal(StatusCode::SERVICE_UNAVAILABLE, err.message))?;
    Ok(Json(json!({ "ok": true })))
}

async fn list(State(db): State<Db>, Query(which): Query<Which>) -> Answer {
    let (Some(project), Some(collection)) = (which.project, which.collection) else {
        return Err(Refusal(
            StatusCode::BAD_REQUEST,
            "say which records: `project` and `collection`, as in /records?project=shop&collection=board"
                .into(),
        ));
    };
    let records: Vec<Value> = db
        .call(move |connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT {RECORD} FROM records WHERE project = ?1 AND collection = ?2 ORDER BY number"
            ))?;
            let rows = statement.query_map([&project, &collection], |row| {
                let data: String = row.get(4)?;
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "project": row.get::<_, String>(1)?,
                    "collection": row.get::<_, String>(2)?,
                    "number": row.get::<_, i64>(3)?,
                    "data": serde_json::from_str::<Value>(&data).unwrap_or_else(|_| json!({})),
                    "position": row.get::<_, f64>(5)?,
                    "created_by": row.get::<_, String>(6)?,
                    "created": row.get::<_, i64>(7)?,
                    "updated": row.get::<_, i64>(8)?,
                }))
            })?;
            rows.collect()
        })
        .await?;
    Ok(Json(json!({ "records": records })))
}

/// The body is read as text and not as a record, so that one that is not
/// a record is refused in a sentence like everything else here.
async fn keep(State(db): State<Db>, Named(id): Named<String>, body: String) -> Answer {
    let record: Record = serde_json::from_str(&body).map_err(|err| {
        Refusal(StatusCode::BAD_REQUEST, format!("what was sent for `{id}` is not a record: {err}"))
    })?;
    if record.id != id {
        return Err(Refusal(
            StatusCode::BAD_REQUEST,
            format!(
                "the record sent to /records/{id} says its id is `{}`; the two must be the same",
                record.id
            ),
        ));
    }
    db.call(move |connection| {
        connection.execute(
            &format!("INSERT OR REPLACE INTO records ({RECORD}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"),
            params![
                record.id,
                record.project,
                record.collection,
                record.number,
                Value::Object(record.data).to_string(),
                record.position,
                record.created_by,
                record.created,
                record.updated
            ],
        )
    })
    .await?;
    Ok(Json(json!({ "id": id })))
}

async fn remove(State(db): State<Db>, Named(id): Named<String>) -> Answer {
    let gone = id.clone();
    db.call(move |connection| connection.execute("DELETE FROM records WHERE id = ?1", [&gone])).await?;
    Ok(Json(json!({ "id": id })))
}

async fn not_done() -> Refusal {
    Refusal(
        StatusCode::METHOD_NOT_ALLOWED,
        "a store's records are listed with GET, kept with PUT and removed with DELETE, and nothing else is done to them".into(),
    )
}

async fn nothing_there() -> Refusal {
    Refusal(
        StatusCode::NOT_FOUND,
        "a store answers /health, /records?project=..&collection=.. and /records/<id>, and nothing else"
            .into(),
    )
}
