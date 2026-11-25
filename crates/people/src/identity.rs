//! Who is who: people's accounts and sessions, and the credentials given to
//! agents and devices.
//!
//! Every request to the daemon carries a token, and this module turns a token
//! into a [`Principal`]. Nothing is trusted because of where it came from:
//! being able to reach the daemon proves nothing about who is asking.
//!
//! Tokens are random, 256 bits, shown once and stored only as a hash. An
//! agent's credential names the slot it belongs to and carries exactly the
//! scopes that slot was given, so one agent cannot act as another and cannot
//! do what its slot was not allowed to.
//!
//! Accounts here are local to this installation. The hosted identity service
//! (sign-in with an organisation, billing) lives in its own repository; when
//! it vouches for a person, [`Identity::sign_in_external`] signs them in
//! to the account that is theirs here ([`crate::outside`]), and everything
//! after that is the same.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use journal::{Journal, optional};
use kernel::principal::{AGENT_SCOPES, MEMBER_SCOPES};
use kernel::{Error, Principal, PrincipalKind, Result, Source};
use rusqlite::params;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};

pub(crate) const SCHEMA: &[&str] = &[
    "
    CREATE TABLE accounts (
        id TEXT PRIMARY KEY,
        email TEXT NOT NULL UNIQUE,
        name TEXT NOT NULL,
        password TEXT,
        role TEXT NOT NULL,
        tier TEXT NOT NULL,
        external TEXT,
        created INTEGER NOT NULL
    );

    CREATE TABLE sessions (
        token TEXT PRIMARY KEY,
        account TEXT NOT NULL,
        created INTEGER NOT NULL,
        expires INTEGER NOT NULL
    ) WITHOUT ROWID;

    CREATE TABLE credentials (
        id TEXT PRIMARY KEY,
        token TEXT NOT NULL UNIQUE,
        kind TEXT NOT NULL,
        subject TEXT NOT NULL,
        name TEXT NOT NULL,
        scopes TEXT NOT NULL,
        created INTEGER NOT NULL,
        revoked INTEGER
    );
    CREATE INDEX credentials_subject ON credentials (kind, subject);

    CREATE TABLE invites (
        code TEXT PRIMARY KEY,
        email TEXT,
        created_by TEXT NOT NULL,
        created INTEGER NOT NULL,
        used_by TEXT
    ) WITHOUT ROWID;
",
    "
    -- For a session a browser's page holds: the digest of the key the
    -- page sends with what it asks. A browser gives a cookie to every port
    -- of the machine it came from, so a cookie alone is nobody's word.
    ALTER TABLE sessions ADD COLUMN page TEXT;
",
    "
    -- When an invitation stops working. One made before this was kept
    -- works for the usual while from when it was made.
    ALTER TABLE invites ADD COLUMN expires INTEGER;
",
    "
    -- What a person was given beyond what their role holds: a list of
    -- scopes, as text; nothing for none, which every account from before
    -- this was kept has.
    ALTER TABLE accounts ADD COLUMN given TEXT;
",
    "
    -- The key a key was made with, which withdraws it with itself
    -- (`crate::keys`): nothing for a key a person signed in made, and for
    -- every key from before this was kept.
    ALTER TABLE credentials ADD COLUMN made_with TEXT;
",
];

/// What a password is checked against where there is no account, so that
/// saying there is none takes as long as saying the password is wrong.
static NOBODYS: tokio::sync::OnceCell<String> = tokio::sync::OnceCell::const_new();

const SESSION_MS: i64 = 30 * 24 * 3600 * 1000;
pub(crate) const MIN_PASSWORD: usize = 8;
/// Wrong passwords allowed for one email within the window below.
const MAX_ATTEMPTS: usize = 5;
const ATTEMPT_WINDOW_MS: i64 = 15 * 60 * 1000;
/// The most emails wrong passwords are remembered for at once, unless an
/// installation says otherwise ([`Identity::remembering`]). Anyone who
/// can reach the daemon can try any email, so the table is held to a size:
/// past it the ones whose window has run out are dropped, and then the
/// oldest.
const MAX_REMEMBERED: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Free,
    Premium,
    Enterprise,
}

impl Tier {
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Free => "free",
            Tier::Premium => "premium",
            Tier::Enterprise => "enterprise",
        }
    }

    pub fn parse(text: &str) -> Option<Tier> {
        match text {
            "free" => Some(Tier::Free),
            "premium" => Some(Tier::Premium),
            "enterprise" => Some(Tier::Enterprise),
            _ => None,
        }
    }

    /// Whether this tier may bring other people in.
    pub fn allows_team(self) -> bool {
        self != Tier::Free
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Account {
    pub id: String,
    pub email: String,
    pub name: String,
    /// `owner` for the account that set this installation up, else `member`.
    pub role: String,
    pub tier: Tier,
    pub created: i64,
    /// What they were given beyond what their role holds, each by
    /// somebody who held it and `admin` ([`crate::given`]).
    pub given: Vec<String>,
}

impl Account {
    /// What the account holds by its role and what it was given, and
    /// nothing by the groups it is in: [`Identity::principal_of`] adds
    /// those.
    pub fn principal(&self) -> Principal {
        self.principal_with(&[])
    }

    /// What the account holds with `grouped` besides, which is what the
    /// groups it is in hold.
    pub fn principal_with(&self, grouped: &[String]) -> Principal {
        let mut scopes: Vec<String> = MEMBER_SCOPES.iter().map(|scope| scope.to_string()).collect();
        // The owner holds what nobody is given without their saying so:
        // the installation's settings, what a table keeps private, and the
        // secrets of a project's environments.
        if self.role == "owner" {
            scopes.push("admin".into());
            scopes.push("private".into());
            scopes.push("environments:secrets".into());
        }
        for given in self.given.iter().chain(grouped) {
            if !kernel::principal::covers(&scopes, given) {
                scopes.push(given.clone());
            }
        }
        Principal {
            kind: PrincipalKind::User,
            id: self.id.clone(),
            name: self.name.clone(),
            scopes,
            key: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SignedIn {
    pub account: Account,
    /// The session token. Shown once; only its hash is kept.
    pub token: String,
    /// For a session a browser's page holds: the key the page sends with
    /// what it asks ([`Identity::for_page`]). Shown once, likewise.
    pub page: Option<String>,
}

/// How a request's token came.
#[derive(Clone, Copy, Debug)]
pub enum Came<'a> {
    /// Said by whatever asks, which keeps its own token: the command
    /// line, an agent, a script with a key.
    Said,
    /// In a browser's cookie. A browser sends a cookie to every port of
    /// the machine it was set for and with every link followed there, so
    /// it counts with the key the factory's own page sends beside it
    /// (`page`). What a browser fetches by itself and only reads cannot
    /// send that: such a request counts with what its address says beside
    /// the cookie (`shown`, which the page was given with its key and is
    /// not the key: [`Identity::shows`]), so that the cookie, which
    /// whatever else listens on the machine is handed, reads nothing
    /// alone. The pages of a library and of mockups, whose own links a
    /// browser follows, are still answered for the cookie (`by_itself`).
    Cookie { page: Option<&'a str>, shown: Option<&'a str>, by_itself: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CredentialKind {
    Agent,
    Device,
    /// A long-lived token for scripts, acting as the account that made it.
    Key,
}

impl CredentialKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            CredentialKind::Agent => "agent",
            CredentialKind::Device => "device",
            CredentialKind::Key => "key",
        }
    }

    fn prefix(self) -> &'static str {
        match self {
            CredentialKind::Agent => "agt_",
            CredentialKind::Device => "dev_",
            CredentialKind::Key => "key_",
        }
    }

    fn parse(text: &str) -> CredentialKind {
        match text {
            "agent" => CredentialKind::Agent,
            "device" => CredentialKind::Device,
            _ => CredentialKind::Key,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Credential {
    pub id: String,
    pub kind: CredentialKind,
    /// The slot, device or account it speaks for.
    pub subject: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub created: i64,
    pub revoked: Option<i64>,
    /// The key it was made with, where it is a key made with a key that
    /// was kept so: it is withdrawn with that one.
    pub made_with: Option<String>,
}

/// A credential read by `crate::keys::COLUMNS` and the list here.
pub(crate) fn read_credential(row: &rusqlite::Row<'_>) -> rusqlite::Result<Credential> {
    Ok(Credential {
        id: row.get(0)?,
        kind: CredentialKind::parse(&row.get::<_, String>(1)?),
        subject: row.get(2)?,
        name: row.get(3)?,
        scopes: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
        created: row.get(5)?,
        revoked: row.get(6)?,
        made_with: row.get(7)?,
    })
}

#[derive(Clone)]
pub struct Identity {
    pub(crate) journal: Journal,
    /// Recent wrong passwords per email, to slow guessing down.
    attempts: Arc<Mutex<HashMap<String, Vec<i64>>>>,
    /// How many emails that is kept for at once.
    room: usize,
    /// Counts up each time anything that lets somebody in is withdrawn (a
    /// session ended, a credential revoked), so that whatever goes on
    /// answering somebody it let in once can ask again whether it still
    /// would ([`Identity::withdrawn`]).
    withdrawn: Arc<tokio::sync::watch::Sender<u64>>,
    /// The groups, held in memory ([`crate::groups`]).
    pub(crate) groups: crate::groups::Groups,
    /// The links that sign somebody in once, kept in memory alone ([`crate::opened`]).
    pub(crate) links: crate::opened::Links,
}

/// Whether two digests are the same, looked at to the end either way.
pub fn same(one: &str, other: &str) -> bool {
    one.len() == other.len()
        && one.bytes().zip(other.bytes()).fold(0u8, |differs, (a, b)| differs | (a ^ b)) == 0
}

/// What is stored in place of a token.
pub(crate) fn digest(token: &str) -> String {
    Sha256::digest(token.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn read_account(row: &rusqlite::Row<'_>) -> rusqlite::Result<Account> {
    Ok(Account {
        id: row.get(0)?,
        email: row.get(1)?,
        name: row.get(2)?,
        role: row.get(3)?,
        tier: Tier::parse(&row.get::<_, String>(4)?).unwrap_or(Tier::Free),
        created: row.get(5)?,
        given: crate::given::read(row.get(6)?),
    })
}

pub(crate) const ACCOUNT: &str = "id, email, name, role, tier, created, given";

impl Identity {
    pub async fn open(journal: Journal) -> Result<Identity> {
        journal.db().migrate("identity", SCHEMA).await?;
        let groups = crate::groups::Groups::open(&journal).await?;
        let withdrawn = Arc::new(tokio::sync::watch::channel(0).0);
        let (attempts, links) = (Arc::default(), Default::default());
        Ok(Identity { journal, attempts, room: MAX_REMEMBERED, withdrawn, groups, links })
    }

    /// What `account` holds as things stand: its role, what it was given
    /// and what the groups it is in hold.
    pub fn principal_of(&self, account: &Account) -> Principal {
        account.principal_with(&self.groups.held_by(&format!("user:{}", account.id)))
    }

    /// Told each time something that lets somebody in is withdrawn. What
    /// goes on answering somebody it let in once (the stream of events)
    /// follows this, and asks again whether it still would: a token is
    /// what says its bearer may be told, for as long as it is one.
    pub fn withdrawn(&self) -> tokio::sync::watch::Receiver<u64> {
        self.withdrawn.subscribe()
    }

    pub(crate) fn say_withdrawn(&self) {
        self.withdrawn.send_modify(|count| *count += 1);
    }

    /// The same, with room to remember wrong passwords for `emails` emails
    /// and no more. What happens when the room is used up is the same
    /// whatever its size, and is shown with a handful: filling a thousand
    /// places is a thousand passwords looked at, each slow on purpose.
    pub fn remembering(self, emails: usize) -> Identity {
        Identity { room: emails, ..self }
    }

    /// True until the first account exists: the client shows "set up"
    /// instead of "sign in".
    pub async fn needs_setup(&self) -> Result<bool> {
        let count: i64 = self
            .journal
            .db()
            .call(|c| c.query_row("SELECT count(*) FROM accounts", [], |row| row.get(0)))
            .await?;
        Ok(count == 0)
    }

    /// Creates an account and signs it in. The first account owns the
    /// installation; anyone after that needs an invitation.
    ///
    /// The first is made from the machine the installation runs on
    /// (`here`), and from nowhere else: until there is an owner, whoever
    /// registers becomes one, and being able to reach the daemon must not
    /// be enough for that. Which account is the first is decided by the
    /// statement that writes it, so that of two that arrive together one is
    /// the owner and the other is told there is one.
    pub async fn register(
        &self,
        email: &str,
        name: &str,
        password: &str,
        invite: Option<&str>,
        here: bool,
    ) -> Result<SignedIn> {
        let email = normalise_email(email)?;
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::invalid("a name is needed"));
        }
        if password.chars().count() < MIN_PASSWORD {
            return Err(Error::invalid(format!("the password needs at least {MIN_PASSWORD} characters")));
        }
        let taken = || Error::denied("this installation already has an owner; ask them for an invitation");
        let first = self.needs_setup().await?;
        if first && !here {
            return Err(Error::denied(
                "this installation has no owner yet, and its first account is made on the machine it runs \
                 on: open it there, at its own address, and register",
            ));
        }
        if !first {
            let Some(code) = invite else { return Err(taken()) };
            self.use_invite(code, &email).await?;
        }
        let hash = hash_password(password.to_string()).await?;
        let account = Account {
            id: self.journal.new_id("acct"),
            email: email.clone(),
            name: name.to_string(),
            role: if first { "owner" } else { "member" }.to_string(),
            tier: Tier::Free,
            created: self.journal.now_ms(),
            given: Vec::new(),
        };
        let row = account.clone();
        let written = self
            .journal
            .db()
            .call(move |connection| {
                // An owner is written only while there is no account at
                // all, in the one statement: looked for first and written
                // afterwards, two could each find none.
                let only_first = if first { "WHERE NOT EXISTS (SELECT 1 FROM accounts)" } else { "" };
                connection.execute(
                    &format!(
                        "INSERT INTO accounts (id, email, name, password, role, tier, created) \
                         SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7 {only_first}"
                    ),
                    params![row.id, row.email, row.name, hash, row.role, row.tier.as_str(), row.created],
                )
            })
            .await
            .map_err(|err| match err.kind {
                kernel::ErrorKind::Conflict => {
                    Error::conflict(format!("there is already an account for {email}"))
                }
                _ => err,
            })?;
        if written == 0 {
            return Err(taken());
        }
        let actor = account.principal().actor();
        let data = json!({ "account": account.id, "role": account.role });
        let _ = self.journal.emit("identity.registered", Source::actor(actor), data).await;
        self.start_session(account).await
    }

    pub async fn sign_in(&self, email: &str, password: &str) -> Result<SignedIn> {
        self.sign_in_from(email, password, false).await
    }

    /// Signs in, saying whether the request came from this machine. Tries
    /// from the machine are counted apart from tries from anywhere else:
    /// somebody elsewhere who guesses at an account cannot keep whoever
    /// sits at the machine out of it.
    pub async fn sign_in_from(&self, email: &str, password: &str, here: bool) -> Result<SignedIn> {
        let email = normalise_email(email)?;
        let now = self.journal.now_ms();
        let counted = format!("{}{email}", if here { "here " } else { "" });
        // Counted before the password is looked at, which takes a while:
        // tries made side by side each find the ones before them.
        self.tried(&counted, now)?;
        let lookup = email.clone();
        let found: Option<(Account, Option<String>)> = self
            .journal
            .db()
            .call(move |connection| {
                optional(connection.query_row(
                    &format!("SELECT {ACCOUNT}, password FROM accounts WHERE email = ?1"),
                    [&lookup],
                    |row| Ok((read_account(row)?, row.get(7)?)),
                ))
            })
            .await?;
        // The same answer whether the email is unknown or the password is
        // wrong, and after the same work, so that sign-in cannot be used to
        // find out who has an account: not by what it says, and not by how
        // long it takes to say it.
        let wrong = || Error::unauthenticated("that email and password do not match an account");
        let Some((account, Some(hash))) = found else {
            let made = || async { hash_password("nobody's password".to_string()).await.unwrap_or_default() };
            let nobodys = NOBODYS.get_or_init(made).await.clone();
            let _ = verify_password(password.to_string(), nobodys).await;
            return Err(wrong());
        };
        if !verify_password(password.to_string(), hash).await {
            return Err(wrong());
        }
        self.attempts.lock().expect("identity lock").remove(&counted);
        self.start_session(account).await
    }

    /// Counts a try at `counted`, and refuses it when there have been too
    /// many. What is remembered is kept to a size whoever is guessing
    /// cannot push past, and what they push out is never an account that
    /// is being guessed at: the ones tried least go first.
    fn tried(&self, counted: &str, now: i64) -> Result<()> {
        let mut attempts = self.attempts.lock().expect("identity lock");
        let recent = attempts.entry(counted.to_string()).or_default();
        recent.retain(|at| now - at < ATTEMPT_WINDOW_MS);
        if recent.len() >= MAX_ATTEMPTS {
            return Err(Error::denied(
                "too many wrong passwords for this account; wait a while and try again",
            ));
        }
        recent.push(now);
        if attempts.len() > self.room {
            attempts.retain(|_, recent| recent.last().is_some_and(|at| now - at < ATTEMPT_WINDOW_MS));
        }
        while attempts.len() > self.room {
            let least = attempts
                .iter()
                .filter(|(other, _)| other.as_str() != counted)
                .min_by_key(|(_, recent)| (recent.len(), recent.last().copied().unwrap_or_default()))
                .map(|(other, _)| other.clone());
            match least {
                Some(other) => attempts.remove(&other),
                None => break,
            };
        }
        Ok(())
    }

    /// How many emails wrong passwords are remembered for now.
    pub fn remembered_failures(&self) -> usize {
        self.attempts.lock().expect("identity lock").len()
    }

    pub(crate) async fn start_session(&self, account: Account) -> Result<SignedIn> {
        let token = format!("ses_{}", kernel::id::secret());
        let (hash, id, now) = (digest(&token), account.id.clone(), self.journal.now_ms());
        self.journal
            .db()
            .call(move |connection| {
                connection.execute(
                    "INSERT INTO sessions (token, account, created, expires) VALUES (?1, ?2, ?3, ?4)",
                    params![hash, id, now, now + SESSION_MS],
                )
            })
            .await?;
        Ok(SignedIn { account, token, page: None })
    }

    /// Makes a session just started one that a browser's page holds: from
    /// now its token is good in the browser's cookie and nowhere else, and
    /// for anything but what a browser fetches by itself only together
    /// with the key given here, which the page keeps where no other page
    /// on the machine can read it.
    ///
    /// `carried` is the cookie the browser sent as it signed in, if it sent
    /// one. Where that is this person's session already, the page is
    /// another of the same browser's (the factory reached at a second
    /// address of the machine), and the session is kept and given this
    /// page's key beside the first: a browser holds one cookie for the
    /// machine, and signing in at one address must not sign out the other.
    pub async fn for_page(&self, signed: SignedIn, carried: Option<&str>) -> Result<SignedIn> {
        let key = kernel::id::secret();
        let (made, page, account) = (digest(&signed.token), digest(&key), signed.account.id.clone());
        let (theirs, now) = (carried.map(digest), self.journal.now_ms());
        let joined = self
            .journal
            .db()
            .call(move |connection| {
                let joined = match &theirs {
                    Some(theirs) => connection.execute(
                        "UPDATE sessions SET page = page || ' ' || ?2
                         WHERE token = ?1 AND account = ?3 AND expires > ?4 AND page IS NOT NULL AND length(page) < 4000",
                        params![theirs, page, account, now],
                    )?,
                    None => 0,
                };
                match joined {
                    0 => connection.execute("UPDATE sessions SET page = ?2 WHERE token = ?1", params![made, page]),
                    _ => connection.execute("DELETE FROM sessions WHERE token = ?1", params![made]),
                }?;
                Ok(joined > 0)
            })
            .await?;
        let token = match (joined, carried) {
            (true, Some(carried)) => carried.to_string(),
            _ => signed.token,
        };
        Ok(SignedIn { page: Some(key), token, account: signed.account })
    }

    /// What a page says in the address of what its browser fetches by
    /// itself, beside the cookie: made from the page's key and not the key,
    /// so that it lets nothing be asked as the page asks it. Neither it
    /// nor the cookie reads anything alone, and an address written into a
    /// log is nobody's credential.
    pub fn shows(key: &str) -> String {
        digest(key)
    }

    /// Who a request is from, by its token and how the token came.
    pub async fn authenticate_as(&self, token: &str, came: Came<'_>) -> Result<Principal> {
        let refused = || Error::unauthenticated("that token is not valid; sign in again");
        let (hash, now) = (digest(token), self.journal.now_ms());
        let page: Option<Option<String>> = match token.starts_with("ses_") {
            true => {
                self.journal
                    .db()
                    .call(move |connection| {
                        optional(connection.query_row(
                            "SELECT page FROM sessions WHERE token = ?1 AND expires > ?2",
                            params![hash, now],
                            |row| row.get(0),
                        ))
                    })
                    .await?
            }
            // A key, or an agent's credential: never a page's.
            false => Some(None),
        };
        let fits = match (page.ok_or_else(refused)?, came) {
            (None, Came::Said) => true,
            // A page's session said as a token is a cookie somebody read.
            (Some(_), Came::Said) | (None, Came::Cookie { .. }) => false,
            (Some(_), Came::Cookie { by_itself: true, .. }) => true,
            (Some(kept), Came::Cookie { page, shown, .. }) => {
                // What is kept is the key's digest, which is also what a
                // page shows: either is compared digest against digest and
                // without stopping at the first letter that differs, so
                // that how long a refusal takes says nothing of what is kept.
                let is = |given: String| {
                    kept.split(' ').fold(false, |found, one| found | same(&digest(one), &given))
                };
                page.is_some_and(|key| is(digest(&digest(key))))
                    || shown.is_some_and(|given| !given.is_empty() && is(digest(given)))
            }
        };
        if !fits {
            return Err(refused());
        }
        self.authenticate(token).await
    }

    pub async fn sign_out(&self, token: &str) -> Result<()> {
        let hash = digest(token);
        let ended = self
            .journal
            .db()
            .call(move |c| c.execute("DELETE FROM sessions WHERE token = ?1", [&hash]))
            .await?;
        // Said only where something was withdrawn: signing out is asked
        // with any token at all, and one that is nobody's ends nothing.
        if ended > 0 {
            self.say_withdrawn();
        }
        Ok(())
    }

    /// Ends every session of the person `who` is, wherever each was
    /// begun: this browser, another, a terminal on another machine. How
    /// many there were. The keys they made are not sessions and go on
    /// until each is withdrawn (`withdraw_key`); nor is an agent's or a
    /// device's credential anybody's session.
    pub async fn sign_out_everywhere(&self, who: &Principal) -> Result<usize> {
        if who.kind != PrincipalKind::User {
            return Err(Error::denied("only a person is signed in, and only a person signs out"));
        }
        let account = who.id.clone();
        let ended = self
            .journal
            .db()
            .call(move |c| c.execute("DELETE FROM sessions WHERE account = ?1", [&account]))
            .await?;
        if ended > 0 {
            self.say_withdrawn();
        }
        let said = json!({ "account": who.id, "sessions": ended });
        let _ = self.journal.emit("identity.signed_out", Source::actor(who.actor()), said).await;
        Ok(ended)
    }

    /// Turns a token into who it speaks for.
    pub async fn authenticate(&self, token: &str) -> Result<Principal> {
        Ok(self.known(token).await?.0)
    }

    /// Who holds `token`, and which key it is when it is a key somebody
    /// made: its id, which is given by the factory and cannot be chosen, as
    /// its name can.
    pub async fn known(&self, token: &str) -> Result<(Principal, Option<String>)> {
        let (hash, now) = (digest(token), self.journal.now_ms());
        let refused = || Error::unauthenticated("that token is not valid; sign in again");
        if token.starts_with("ses_") {
            let account: Option<Account> = self
                .journal
                .db()
                .call(move |connection| {
                    optional(connection.query_row(
                        "SELECT a.id, a.email, a.name, a.role, a.tier, a.created, a.given
                         FROM sessions s JOIN accounts a ON a.id = s.account
                         WHERE s.token = ?1 AND s.expires > ?2",
                        params![hash, now],
                        read_account,
                    ))
                })
                .await?;
            return account.map(|account| (self.principal_of(&account), None)).ok_or_else(refused);
        }
        let found: Option<(String, String, String, String, String)> = self
            .journal
            .db()
            .call(move |connection| {
                optional(connection.query_row(
                    "SELECT kind, subject, name, scopes, id FROM credentials WHERE token = ?1 AND revoked IS NULL",
                    [&hash],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                ))
            })
            .await?;
        let (kind, subject, name, scopes, id) = found.ok_or_else(refused)?;
        let scopes: Vec<String> = serde_json::from_str(&scopes).unwrap_or_default();
        Ok(match CredentialKind::parse(&kind) {
            // An agent holds what its slot gives it and what its groups
            // hold, read as it asks: a group changed is held from its next
            // request, with its credential as it was.
            CredentialKind::Agent => {
                let scopes = self.grouped_agent(&subject, scopes);
                (Principal { kind: PrincipalKind::Agent, id: subject, name, scopes, key: None }, None)
            }
            // A device is known by the key it proves to its hub, and by
            // nothing it can send: a credential made for one, before that
            // was so, is nobody's.
            CredentialKind::Device => return Err(refused()),
            CredentialKind::Key => (
                Principal { kind: PrincipalKind::User, id: subject, name, scopes, key: Some(id.clone()) },
                Some(id),
            ),
        })
    }

    /// Who an event's actor is now, with what they hold now: an account
    /// (`user:<id>`), a key somebody made (`key:<id>`, which holds what it
    /// was given and nothing of its maker's besides) or an agent
    /// (`agent:<slot>`). Nobody, for one that is gone or withdrawn, and for
    /// anything else an actor can be: what a caller or the factory itself
    /// did was done by nobody who holds anything.
    pub async fn holder(&self, actor: &str) -> Result<Option<Principal>> {
        let (kind, id) = actor.split_once(':').unwrap_or((actor, ""));
        let wanted = match kind {
            "user" => return Ok(self.account(id).await.ok().map(|account| self.principal_of(&account))),
            "key" => ("id", CredentialKind::Key),
            "agent" => ("subject", CredentialKind::Agent),
            _ => return Ok(None),
        };
        let (by, id) = (wanted.0, id.to_string());
        // A key says which key it is; an agent is its slot and no key.
        let key = (wanted.1 == CredentialKind::Key).then(|| id.clone());
        let found: Option<(String, String, String)> = self
            .journal
            .db()
            .call(move |connection| {
                optional(connection.query_row(
                    &format!(
                        "SELECT subject, name, scopes FROM credentials WHERE {by} = ?1 AND kind = ?2 AND revoked IS NULL"
                    ),
                    params![id, wanted.1.as_str()],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                ))
            })
            .await?;
        Ok(found.map(|(subject, name, scopes)| {
            let scopes: Vec<String> = serde_json::from_str(&scopes).unwrap_or_default();
            let agent = wanted.1 == CredentialKind::Agent;
            Principal {
                kind: if agent { PrincipalKind::Agent } else { PrincipalKind::User },
                scopes: if agent { self.grouped_agent(&subject, scopes) } else { scopes },
                id: subject,
                name,
                key,
            }
        }))
    }

    /// What the agent `name` holds by its credential, `scopes`, and by the
    /// groups it is in.
    fn grouped_agent(&self, name: &str, mut scopes: Vec<String>) -> Vec<String> {
        for scope in self.groups.held_by(&format!("agent:{name}")) {
            if !kernel::principal::covers(&scopes, &scope) {
                scopes.push(scope);
            }
        }
        scopes
    }

    pub async fn account(&self, id: &str) -> Result<Account> {
        let id = id.to_string();
        self.journal
            .db()
            .call(move |c| {
                optional(c.query_row(
                    &format!("SELECT {ACCOUNT} FROM accounts WHERE id = ?1"),
                    [&id],
                    read_account,
                ))
            })
            .await?
            .ok_or_else(|| Error::not_found("there is no such account"))
    }

    pub async fn accounts(&self) -> Result<Vec<Account>> {
        self.journal
            .db()
            .call(|connection| {
                let mut statement =
                    connection.prepare(&format!("SELECT {ACCOUNT} FROM accounts ORDER BY created"))?;
                let rows = statement.query_map([], read_account)?;
                rows.collect()
            })
            .await
    }

    /// The tier of the installation: its owner's.
    pub async fn tier(&self) -> Result<Tier> {
        let tier: Option<String> = self
            .journal
            .db()
            .call(|c| {
                optional(c.query_row("SELECT tier FROM accounts WHERE role = 'owner' LIMIT 1", [], |row| {
                    row.get(0)
                }))
            })
            .await?;
        Ok(tier.and_then(|tier| Tier::parse(&tier)).unwrap_or(Tier::Free))
    }

    /// Sets the installation's tier. Until the hosted service is connected
    /// this is the factory owner's word; with it, the service's.
    pub async fn set_tier(&self, by: &Principal, tier: Tier) -> Result<()> {
        by.require("admin")?;
        self.journal.db().call(move |c| c.execute("UPDATE accounts SET tier = ?1", [tier.as_str()])).await?;
        let _ = self
            .journal
            .emit("identity.tier", Source::actor(by.actor()), json!({ "tier": tier.as_str() }))
            .await;
        Ok(())
    }

    /// Issues a credential and returns its token, which is shown only now.
    /// Any earlier credential of the same kind for the same subject is
    /// revoked, so a slot or device only ever has one that works.
    pub async fn issue(
        &self,
        kind: CredentialKind,
        subject: &str,
        name: &str,
        scopes: Option<Vec<String>>,
    ) -> Result<String> {
        self.issue_with(kind, subject, name, scopes, None).await
    }

    /// The same, for a key made with the key `made_with`, which withdraws
    /// it with itself (`crate::keys`).
    pub async fn issue_with(
        &self,
        kind: CredentialKind,
        subject: &str,
        name: &str,
        scopes: Option<Vec<String>>,
        made_with: Option<String>,
    ) -> Result<String> {
        let scopes = scopes.unwrap_or_else(|| match kind {
            CredentialKind::Agent => AGENT_SCOPES.iter().map(|scope| scope.to_string()).collect(),
            CredentialKind::Device => vec!["fleet".to_string()],
            CredentialKind::Key => MEMBER_SCOPES.iter().map(|scope| scope.to_string()).collect(),
        });
        if kind != CredentialKind::Key {
            self.revoke_subject(kind, subject).await?;
        }
        let token = format!("{}{}", kind.prefix(), kernel::id::secret());
        let row = (
            self.journal.new_id("cred"),
            digest(&token),
            kind.as_str(),
            subject.to_string(),
            name.to_string(),
            serde_json::to_string(&scopes).unwrap_or_default(),
            self.journal.now_ms(),
            made_with.filter(|_| kind == CredentialKind::Key),
        );
        // A key is said when it is made, as it is when it is withdrawn:
        // what can act for somebody is not made quietly.
        let made = (kind == CredentialKind::Key).then(|| {
            let mut made = json!({ "key": row.0, "name": name, "account": subject, "scopes": scopes });
            if let Some(with) = &row.7 {
                made["with"] = json!(with);
            }
            made
        });
        self.journal
            .db()
            .call(move |connection| {
                connection.execute(
                    "INSERT INTO credentials (id, token, kind, subject, name, scopes, created, made_with) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7],
                )
            })
            .await?;
        if let Some(made) = made {
            let by = Source::actor(format!("user:{subject}"));
            let _ = self.journal.emit("identity.key.made", by, made).await;
        }
        Ok(token)
    }

    pub async fn revoke_subject(&self, kind: CredentialKind, subject: &str) -> Result<usize> {
        let (subject, now) = (subject.to_string(), self.journal.now_ms());
        let revoked = self
            .journal
            .db()
            .call(move |connection| {
                connection.execute(
                    "UPDATE credentials SET revoked = ?3 WHERE kind = ?1 AND subject = ?2 AND revoked IS NULL",
                    params![kind.as_str(), subject, now],
                )
            })
            .await?;
        if revoked > 0 {
            self.say_withdrawn();
        }
        Ok(revoked)
    }

    pub async fn revoke(&self, id: &str) -> Result<()> {
        let (id, now) = (id.to_string(), self.journal.now_ms());
        if self
            .journal
            .db()
            .call(move |c| {
                c.execute(
                    "UPDATE credentials SET revoked = ?2 WHERE id = ?1 AND revoked IS NULL",
                    params![id, now],
                )
            })
            .await?
            > 0
        {
            self.say_withdrawn();
        }
        Ok(())
    }

    /// Every credential, without its token.
    pub async fn credentials(&self) -> Result<Vec<Credential>> {
        self.journal
            .db()
            .call(|connection| {
                let mut statement = connection
                    .prepare("SELECT id, kind, subject, name, scopes, created, revoked, made_with FROM credentials ORDER BY created DESC")?;
                let rows = statement.query_map([], read_credential)?;
                rows.collect()
            })
            .await
    }
}

pub(crate) fn normalise_email(email: &str) -> Result<String> {
    let email = email.trim().to_lowercase();
    let valid = email
        .split_once('@')
        .is_some_and(|(name, host)| !name.is_empty() && host.contains('.') && !host.ends_with('.'))
        && !email.contains(char::is_whitespace);
    if valid { Ok(email) } else { Err(Error::invalid("that does not look like an email address")) }
}

/// Hashing is deliberately slow, so it is kept off the async threads.
pub(crate) async fn hash_password(password: String) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|hash| hash.to_string())
            .map_err(|err| Error::internal(format!("could not hash the password: {err}")))
    })
    .await
    .map_err(|err| Error::internal(err.to_string()))?
}

async fn verify_password(password: String, hash: String) -> bool {
    tokio::task::spawn_blocking(move || {
        Argon2::default().verify_password(password.as_bytes(), hash.as_str()).is_ok()
    })
    .await
    .unwrap_or(false)
}
