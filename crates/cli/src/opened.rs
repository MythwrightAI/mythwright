//! The client opened signed in, with nothing to register (`people::opened`).
//!
//! `open` asks the daemon for a link that signs somebody in once, within a
//! minute, and opens the browser at it (or, told `--link`, prints it and
//! opens nothing: what the desktop app and a check ask). The link is asked
//! for as whoever the command line is signed in as; where it is signed in
//! as nobody there, as the factory's owner, by proving that this machine
//! can read the factory's data folder: the one said by `--data`, the
//! profile, or the usual place, and only where that folder says it is
//! served at the address asked. `login --here` signs the command line
//! itself in the same way, and `register` asked by the owner the start
//! made gives that account an email, a name and a password.

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, url_of};
use crate::work::{given_or_ask, show, text};

/// Where a link opens the client: the address with the secret after its
/// `#`, which a browser never sends anywhere, and the page takes off the
/// address as it reads it.
fn address(client: &Client, made: &Value) -> Result<String, String> {
    let once = made["once"].as_str().ok_or("the daemon made no link")?;
    Ok(format!("{}/#once={once}", client.url))
}

/// The data folder of the factory asked for, where it is on this machine.
fn folder(args: &Args) -> Result<std::path::PathBuf, String> {
    crate::serve::place(args).map(|(_, data)| data)
}

/// A link that signs somebody in once: as whoever the command line is
/// signed in as, else as the owner by proving the data folder.
async fn made(client: &Client, data: Result<std::path::PathBuf, String>) -> Result<Value, String> {
    let mut refused = None;
    if client.signed_in() {
        match client.post("/api/auth/once", json!({})).await {
            Ok(made) => return Ok(made),
            Err(err) if Client::is_unreachable(&err) => return Err(err),
            Err(err) => refused = Some(err),
        }
    }
    match proved(client, data).await {
        Ok(made) => Ok(made),
        Err(why) => Err(refused.map_or(why.clone(), |refused| format!("{refused}; and {why}"))),
    }
}

/// The owner's link, for proving this machine can read the factory's data
/// folder: the one asked for (`--data`, the profile, the usual place) where
/// it is the one served at this address, else the one the daemon says it
/// keeps its data in, which it says to a program on this machine.
async fn proved(client: &Client, data: Result<std::path::PathBuf, String>) -> Result<Value, String> {
    let asked = client.post("/api/auth/once/asked", json!({})).await?;
    let nonce = asked["nonce"].as_str().ok_or("the daemon gave nothing to prove")?;
    let serves = |folder: &std::path::Path| {
        api::folder::served(folder).map(url_of).as_deref() == Some(client.url.as_str())
    };
    let said = asked["data"].as_str().map(std::path::PathBuf::from);
    let Some(data) = data.ok().filter(|folder| serves(folder)).or(said.filter(|folder| serves(folder)))
    else {
        return Err(format!(
            "this command line is signed in as nobody at {url}, and cannot read the data folder of the \
             factory served there, which only whoever runs that factory, on its machine, can. Sign in \
             (`{cli} login`), or say where it keeps its data (`--data <folder>`)",
            url = client.url,
            cli = brand::CLI
        ));
    };
    let proof = api::opened::proves(&data, nonce).map_err(|err| err.message)?;
    client.post("/api/auth/once/proved", json!({ "nonce": nonce, "proof": proof })).await
}

/// `open`: the client in a browser, signed in by a link; told `--link`, the
/// link printed and nothing opened.
pub async fn open(client: &Client, args: &Args) -> Result<(), String> {
    let only = args.has("link");
    let made = made(client, folder(args)).await;
    match (made, only) {
        (Ok(made), true) => println!("{}", address(client, &made)?),
        (Ok(made), false) => crate::serve::open_signed_in(&address(client, &made)?),
        (Err(why), true) => return Err(why),
        // The client all the same, which says how to be signed in.
        (Err(why), false) => {
            eprintln!("no link that signs you in: {why}");
            crate::serve::open_in_browser(&client.url);
        }
    }
    Ok(())
}

/// `login --here`: the command line signed in as the factory's owner, by
/// proving this machine can read its data folder.
pub async fn here(client: &Client, args: &Args) -> Result<Value, String> {
    let made = proved(client, folder(args)).await?;
    let once = made["once"].as_str().ok_or("the daemon made no link")?;
    client.post("/api/auth/link", json!({ "once": once })).await
}

pub async fn register(client: &mut Client, args: &Args) -> Result<(), String> {
    let body = json!({
        "email": given_or_ask(args, "email", "email", false)?,
        "name": given_or_ask(args, "name", "your name", false)?,
        "password": given_or_ask(args, "password", "password (8 characters or more)", true)?,
        "invite": args.value("invite"),
    });
    // The owner a factory made as it first started has no email and no
    // password: registering is then giving that account a way to sign in
    // from elsewhere, and a name, as whoever this terminal is signed in as,
    // or, where it is signed in as nobody, as whoever proves this machine
    // can read the factory's data folder, as `login --here` does.
    if body["invite"].is_null() {
        let mut kept = None;
        if !client.signed_in()
            && let Ok(signed) = here(client, args).await
            && let Some(token) = signed["token"].as_str()
        {
            client.ask_as(Some(token));
            kept = Some(token.to_string());
        }
        if client.signed_in() {
            let me = client.get("/api/me").await.unwrap_or_default();
            if me["account"]["email"] == "" && me["password"] == false {
                client.call(reqwest::Method::PUT, "/api/me/password", Some(body.clone())).await?;
                let named = client.call(reqwest::Method::PUT, "/api/me/name", Some(body)).await?;
                if let Some(token) = kept {
                    client.save_session(&token)?;
                }
                return welcome(client, &named);
            }
        }
        // Signed in only to be asked: the owner registered already, and
        // the session made for it is ended.
        if kept.is_some() {
            let _ = client.post("/api/auth/logout", json!({})).await;
            client.ask_as(None);
        }
    }
    let signed = client.post("/api/auth/register", body).await?;
    client.save_session(text(&signed, "token"))?;
    welcome(client, &signed)
}

fn welcome(client: &Client, signed: &Value) -> Result<(), String> {
    show(client, &signed["account"], || {
        format!(
            "Welcome, {}. You are signed in as the {}.",
            text(&signed["account"], "name"),
            text(&signed["account"], "role")
        )
    });
    Ok(())
}

pub async fn login(client: &Client, args: &Args) -> Result<(), String> {
    // With `--here`, as the owner, by proving this machine can read the
    // factory's data folder: what a terminal signed in as nobody does where
    // the owner has no password. `--data` is read whichever way it signs
    // in, as every flag is before a request.
    let _ = args.value("data");
    let signed = match args.has("here") {
        true => here(client, args).await?,
        false => {
            let body = json!({
                "email": given_or_ask(args, "email", "email", false)?,
                "password": given_or_ask(args, "password", "password", true)?,
            });
            client.post("/api/auth/login", body).await?
        }
    };
    client.save_session(text(&signed, "token"))?;
    show(client, &signed["account"], || format!("Signed in as {}.", text(&signed["account"], "name")));
    Ok(())
}
