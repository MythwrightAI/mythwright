//! What somebody with an account is named by after an `@`.
//!
//! An agent and a bot are named by what they take part as; a person is
//! named by this, a function of what their account is called: lowercase,
//! every run of anything but a plain letter, a digit or `_` a dash, and
//! no dash at either end. The client offers the same word as a name is
//! typed (`handleOf` in `ui/src/mentions.ts`), and the two are held to
//! one table of cases (`ui/src/handles.json`), each by its own tests, so
//! that what a page offers is what the daemon takes. It is nothing an
//! account keeps and nothing is unique by it: two accounts called the
//! same are both named by one word, and a name with no plain letter or
//! digit in it is named by nothing.
//!
//! Both this and what somebody holds now ([`held`]) are read on a
//! connection whoever asks hands over, so that something written with
//! what they answer (a line of somebody's inbox, how many wait in it)
//! is written as that write finds the accounts.

use kernel::Principal;

use crate::identity::{ACCOUNT, read_account};

/// The word `name` is named by after an `@`; empty where nothing of it
/// can be written there.
pub fn handle(name: &str) -> String {
    let mut handle = String::with_capacity(name.len());
    for letter in name.to_lowercase().chars() {
        match letter {
            'a'..='z' | '0'..='9' | '_' => handle.push(letter),
            _ if handle.ends_with('-') || handle.is_empty() => {}
            _ => handle.push('-'),
        }
    }
    if handle.ends_with('-') {
        handle.pop();
    }
    handle
}

/// The people with an account here whom one of `names` names, each as
/// they are now, with what they hold: read where it is asked, so in a
/// write that is under way as that write finds the accounts. Nobody,
/// where no name is anybody's; everybody a name is shared by.
pub fn named(connection: &rusqlite::Connection, names: &[String]) -> rusqlite::Result<Vec<Principal>> {
    if names.is_empty() {
        return Ok(Vec::new());
    }
    let mut accounts = connection.prepare(&format!("SELECT {ACCOUNT} FROM accounts ORDER BY rowid"))?;
    let all = accounts.query_map([], read_account)?.collect::<rusqlite::Result<Vec<_>>>()?;
    let is_named = |account: &&crate::Account| {
        let handle = handle(&account.name);
        !handle.is_empty() && names.contains(&handle)
    };
    let principal = |account: &crate::Account| {
        let grouped = crate::groups::held_through(connection, &format!("user:{}", account.id))?;
        Ok(account.principal_with(&grouped))
    };
    all.iter().filter(is_named).map(principal).collect()
}

/// What whoever is `actor` holds now: an account (`user:<id>`) what it
/// is given by what it is, an agent (`agent:<name>`) what its
/// credential holds, and anybody else, or one that is gone or has no
/// credential, nothing. Read where it is asked, as [`named`] is.
pub fn held(connection: &rusqlite::Connection, actor: &str) -> rusqlite::Result<Vec<String>> {
    let found: Option<Vec<String>> = match actor.split_once(':') {
        Some(("user", id)) => journal::optional(connection.query_row(
            &format!("SELECT {ACCOUNT} FROM accounts WHERE id = ?1"),
            [id],
            read_account,
        ))?
        .map(|account| account.principal().scopes),
        Some(("agent", name)) => {
            let kept: Option<String> = journal::optional(connection.query_row(
                "SELECT scopes FROM credentials WHERE subject = ?1 AND kind = 'agent' AND revoked IS NULL",
                [name],
                |row| row.get(0),
            ))?;
            kept.map(|scopes| serde_json::from_str(&scopes).unwrap_or_default())
        }
        _ => None,
    };
    // And what the groups it is in hold, as this write finds them.
    let Some(mut held) = found else { return Ok(Vec::new()) };
    for scope in crate::groups::held_through(connection, actor)? {
        if !kernel::principal::covers(&held, &scope) {
            held.push(scope);
        }
    }
    Ok(held)
}

/// Whom a word given as a card's assignee is: an actor as it is written
/// (`user:<id>`, `agent:<name>`), an agent by its name, or a person by
/// their handle ([`handle`]) or their email. A word that is both an
/// agent's name and a person's, or more than one person's, is refused
/// saying how to say which; one that is nobody's is refused in a
/// sentence that lists who it could be. Nothing is asked here: the
/// agents' names and the accounts are handed in as they are.
pub fn assignee(word: &str, agents: &[String], accounts: &[crate::Account]) -> Result<String, String> {
    let word = word.trim();
    if word.contains(':') {
        return Ok(word.to_string());
    }
    let lower = word.to_lowercase();
    let agent = agents.iter().find(|name| **name == word).map(|name| format!("agent:{name}"));
    let people: Vec<&crate::Account> = accounts
        .iter()
        .filter(|account| {
            account.email.to_lowercase() == lower
                || (!handle(&account.name).is_empty() && handle(&account.name) == lower)
        })
        .collect();
    let said = |account: &crate::Account| format!("{} ({})", handle(&account.name), account.email);
    match (agent, &people[..]) {
        (Some(agent), []) => Ok(agent),
        (None, [one]) => Ok(format!("user:{}", one.id)),
        (Some(agent), people) => Err(format!(
            "`{word}` is the name of an agent and of {}; say which: `{agent}` or {}",
            people.iter().map(|one| said(one)).collect::<Vec<_>>().join(", "),
            people.iter().map(|one| format!("`user:{}`", one.id)).collect::<Vec<_>>().join(", ")
        )),
        (None, people) if !people.is_empty() => Err(format!(
            "`{word}` names {} people: {}; give it by email, or as `user:<id>`",
            people.len(),
            people.iter().map(|one| said(one)).collect::<Vec<_>>().join(", ")
        )),
        (None, _) => {
            let mut could: Vec<String> = agents.to_vec();
            could.extend(accounts.iter().map(said));
            let could = match could.is_empty() {
                true => "there is nobody yet".to_string(),
                false => format!("it could be {}", could.join(", ")),
            };
            Err(format!(
                "`{word}` is no agent's name and no person's handle or email here, so the card is given to \
                 nobody; {could}"
            ))
        }
    }
}
