//! A factory's profile: the name it is given once and keeps.
//!
//! A factory is one daemon with its own data folder: its own database,
//! accounts, credentials and address. It may have a profile, a name
//! such as `prod` or `dev1`, and the name is not the factory: two factories
//! may be called the same, so nothing here takes a name for one of them.
//!
//! The name is stamped into the data folder the first time the factory is
//! started with one, in a small file beside the database (it has to be read
//! before the database is opened, to know where to listen). From then on
//! the folder is that profile's for good: started with no name it is
//! still what it was stamped, and started with another it is refused. That
//! refusal is one of the things that keep two profiles from sharing
//! data: a slip of the hand cannot turn production's data into
//! development's.
//!
//! The stamp also keeps where a named factory listens, so that its address
//! is the same every time it is started without one being given, and a
//! short id of the factory's own, for what has to tell one factory from
//! another where a name cannot (a browser's session cookie).

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use kernel::{Error, Result};
use serde::{Deserialize, Serialize};

/// The file in a data folder that says which profile the folder is.
pub const STAMP_FILE: &str = "factory.json";
/// The longest a name may be: it is shown in the frame of every page.
const LONGEST_NAME: usize = 32;

/// What a data folder says about the factory it belongs to.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    /// What the factory's profile is called.
    // Written as `environment`, as it always was: the file is read by
    // every program on the device, of whatever version.
    #[serde(rename = "environment")]
    pub profile: String,
    /// Where it last listened: where it listens again unless told otherwise.
    #[serde(default)]
    pub address: Option<SocketAddr>,
    /// A few letters of the factory's own, made once. Not a secret.
    #[serde(default)]
    pub id: String,
}

/// What a factory is started as, once its data folder has had its say.
pub struct Settled {
    /// The stamp as it will be kept; `None` for a factory with no name,
    /// whose folder is left as it is.
    pub stamp: Option<Stamp>,
    /// Where to listen.
    pub listen: SocketAddr,
    /// `listen` is what the folder remembered, not what was asked for.
    pub remembered: bool,
}

/// Refuses a name that cannot be a profile's: it is a lowercase word
/// of letters, digits and dashes, because it becomes part of a folder's
/// name and is typed on a command line.
pub fn check_name(name: &str) -> Result<()> {
    let word = name.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    let ends = |byte: Option<u8>| byte.is_some_and(|byte| byte != b'-');
    if !name.is_empty()
        && name.len() <= LONGEST_NAME
        && word
        && ends(name.bytes().next())
        && ends(name.bytes().last())
    {
        return Ok(());
    }
    Err(Error::invalid(format!(
        "`{name}` cannot be a profile's name: a name is a lowercase word of letters, digits and dashes, \
         such as `prod` or `dev1`, of at most {LONGEST_NAME} characters"
    )))
}

fn stamp_path(data: &Path) -> PathBuf {
    data.join(STAMP_FILE)
}

impl Stamp {
    /// The stamp of a data folder, or `None` for a folder with none: a new
    /// one, or one from before factories had profiles.
    ///
    /// A stamp that is there and cannot be read is an error, never "no
    /// stamp": taking it for none would let the folder be stamped again as
    /// something else.
    pub fn read(data: &Path) -> Result<Option<Stamp>> {
        let path = stamp_path(data);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(Error::from(err).context(path.display())),
        };
        let unreadable = |why: String| {
            Error::invalid(format!(
                "{} says which profile this data folder belongs to, and cannot be read ({why}). \
                 Nothing is started on a folder whose profile is not known; put the file right, \
                 or remove it if the folder has no profile",
                path.display()
            ))
        };
        let stamp: Stamp = serde_json::from_str(&text).map_err(|err| unreadable(err.to_string()))?;
        check_name(&stamp.profile).map_err(|err| unreadable(err.message))?;
        Ok(Some(stamp))
    }

    /// Keeps the stamp: written whole beside the old one, then put in its
    /// place, so that a start that is killed leaves one or the other.
    pub fn write(&self, data: &Path) -> Result<()> {
        let path = stamp_path(data);
        let partial = data.join(format!("{STAMP_FILE}.new"));
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(&partial, format!("{text}\n"))
            .map_err(|err| Error::from(err).context(partial.display()))?;
        std::fs::rename(&partial, &path).map_err(|err| Error::from(err).context(path.display()))
    }
}

/// Works out what a factory is started as: the name it was asked to have
/// (`asked`) against the name its data folder has, and where it listens:
/// `listen` where it was told, and where its folder remembers if it is to
/// keep its address.
pub fn settle(data: &Path, asked: Option<&str>, listen: SocketAddr, keep_address: bool) -> Result<Settled> {
    let kept = Stamp::read(data)?;
    let name = match (asked, &kept) {
        (Some(asked), Some(kept)) if *asked != kept.profile => {
            return Err(Error::invalid(format!(
                "{data} is the data of a `{was}` factory, and was asked to start as `{asked}`. A data folder \
                 belongs to one profile for good, so that one profile's data is never taken for \
                 another's. Start it as `{was}` (with no `--profile` it is `{was}` too), or give `{asked}` a data \
                 folder of its own: with `--profile {asked}` and no `--data`, it gets one",
                data = data.display(),
                was = kept.profile,
            )));
        }
        (Some(asked), _) => {
            check_name(asked)?;
            Some(asked.to_string())
        }
        (None, kept) => kept.as_ref().map(|stamp| stamp.profile.clone()),
    };
    let Some(profile) = name else {
        return Ok(Settled { stamp: None, listen, remembered: false });
    };
    let remembered = kept.as_ref().and_then(|stamp| stamp.address).filter(|_| keep_address);
    let listen = match (remembered, keep_address) {
        (Some(address), _) => address,
        // The first start of a named factory that was given no address: any
        // free port, on this device only. It is remembered from here on.
        (None, true) => SocketAddr::from(([127, 0, 0, 1], 0)),
        (None, false) => listen,
    };
    let id = kept.map(|stamp| stamp.id).filter(|id| !id.is_empty());
    let id = id.unwrap_or_else(|| kernel::id::secret()[..8].to_string());
    Ok(Settled {
        stamp: Some(Stamp { profile, address: None, id }),
        listen,
        remembered: remembered.is_some(),
    })
}
