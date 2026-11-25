//! What this installation is to another: a secret of its own, the keys made
//! from it, and the short name of the one it shows.
//!
//! The secret is made the first time the daemon starts and is never shown:
//! not in an event, a log, a measurement or an error, and it is not in the
//! database, so it is in no copy of a factory. A factory started from a
//! snapshot has a data folder of its own and makes a secret of its own there:
//! it is another installation, and nothing that held the first to its key
//! takes the copy for it.
//!
//! The secret is a seed and is used for nothing directly. Each key the
//! installation has is made from it for one purpose, and the purpose is part
//! of what makes it ([`Own::derive`]), so that a key made for one thing is
//! never the key for another. There are two purposes today: the link between
//! this installation and another ([`Own::link`]), and sealing what it keeps
//! secret where it is kept ([`Own::at_rest`], [`crate::kept`]).
//!
//! The file is made readable by its owner alone as it is made, never
//! afterwards, and is whole and on the disk before it has its name: it is
//! written beside where it goes and given its name only if nothing has it,
//! so that a start cut short leaves no empty file, and of two daemons
//! started at once on one data folder both come to hold the one secret. One
//! that is there and that others may read is not used: the daemon says so
//! and does not start. On Windows there are no such modes: the file has the
//! permissions of the folder it is in, which is the data folder under the
//! profile of whoever runs the daemon.
//!
//! The secret is held as bytes and wiped where it is done with, as far as
//! that can be asked of the compiler; it is never in a `String`, and what
//! prints or copies the types here prints and copies no secret. That is
//! what is in this crate's hands. The handshake is the `snow` crate's, and
//! for the length of each handshake it keeps a copy of the private half of
//! the link's key that it is not built here to wipe as it lets go; and
//! nothing here keeps the system from writing memory out. Whoever can read
//! this process's memory can read the key; nobody else can.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};
use snow::params::DHChoice;
use snow::resolvers::{CryptoResolver, DefaultResolver};

use crate::{Error, Result};

/// The file in the data folder that holds the secret.
pub const SECRET_FILE: &str = "installation.secret";

/// What the key for links is made for. Part of the key: change a letter and
/// it is another key.
const LINK: &str = "the key a device and its hub prove to each other (Noise, X25519)";

/// How long a key is, private or public.
pub const KEY: usize = 32;

/// This installation's secret.
pub struct Own {
    seed: [u8; KEY],
}

/// Never printed, whatever is asked to print it.
impl fmt::Debug for Own {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str("Own(a secret)")
    }
}

/// Overwrites `bytes`, in a way the compiler is asked not to leave out.
fn wipe(bytes: &mut [u8]) {
    bytes.fill(0);
    std::hint::black_box(&*bytes);
}

impl Drop for Own {
    fn drop(&mut self) {
        wipe(&mut self.seed);
    }
}

/// The half of a pair that is kept. Wiped when the last holder lets go.
struct Private([u8; KEY]);

impl Drop for Private {
    fn drop(&mut self) {
        wipe(&mut self.0);
    }
}

/// A pair of keys: the half that is kept and the half that is shown. A
/// copy of this holds the same private half, and does not copy it.
#[derive(Clone)]
pub struct Keys {
    private: Arc<Private>,
    pub public: [u8; KEY],
}

impl fmt::Debug for Keys {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "Keys({})", fingerprint(&self.public))
    }
}

impl Keys {
    /// The pair whose private half is `private`.
    fn of(private: [u8; KEY]) -> Result<Keys> {
        let private = Arc::new(Private(private));
        let mut pair = DefaultResolver
            .resolve_dh(&DHChoice::Curve25519)
            .ok_or_else(|| Error::new("this build has no way to make a key for links"))?;
        pair.set(&private.0);
        let public: [u8; KEY] =
            pair.pubkey().try_into().map_err(|_| Error::new("a key for links came out the wrong length"))?;
        Ok(Keys { private, public })
    }

    pub(crate) fn private(&self) -> &[u8; KEY] {
        &self.private.0
    }
}

/// A key's short name, for two people to compare what each of them sees:
/// the first eighty bits of its hash, in groups of four.
pub fn fingerprint(public: &[u8]) -> String {
    let hash = Sha256::digest(public);
    let hex: String = hash.iter().take(10).map(|byte| format!("{byte:02x}")).collect();
    hex.as_bytes()
        .chunks(4)
        .map(|group| String::from_utf8_lossy(group).into_owned())
        .collect::<Vec<_>>()
        .join("-")
}

fn private_options() -> std::fs::OpenOptions {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}

/// Makes a file that only its owner can read from the moment it exists,
/// with all of `value` in it and on the disk, replacing one that was there.
pub fn write_private(path: &Path, value: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // Written beside it and given its name when it is whole: never there
    // with half of what it should hold, or with another's leave to read it.
    // Beside it under its whole name, so that two files kept in one folder
    // are never written through the same one.
    let beside = beside(path, "writing");
    let _ = std::fs::remove_file(&beside);
    let mut file = private_options().open(&beside)?;
    file.write_all(value)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&beside, path)
}

/// `path` with `.word` after all of its name: `hub.json.writing`.
fn beside(path: &Path, word: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".");
    name.push(word);
    path.with_file_name(name)
}

/// Sixteen letters that nobody else making a file in the same moment has.
fn nobody_elses() -> std::io::Result<String> {
    let mut random = [0u8; 8];
    getrandom::fill(&mut random).map_err(|err| std::io::Error::other(err.to_string()))?;
    Ok(random.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// The same, for a file that must not replace one that is there: says
/// whether this one was kept, or one that was there before it stands.
fn write_private_once(path: &Path, value: &[u8]) -> std::io::Result<bool> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // Under a name nobody else is using, whoever else is making one now.
    let beside = beside(path, &format!("{}.making", nobody_elses()?));
    let mut file = private_options().open(&beside)?;
    file.write_all(value)?;
    file.sync_all()?;
    drop(file);
    // A second name for it, which is given only where nothing has that
    // name: whole from the moment it is there, and never over another's.
    let kept = match std::fs::hard_link(&beside, path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        // Where a second name cannot be given (some file systems), it is
        // made in place: still never over one that is there.
        // One that could not be written whole is taken away again, so that
        // what is left is a whole secret or none, and the next start makes
        // one.
        Err(_) => match private_options().open(path) {
            Ok(mut file) => match file.write_all(value).and_then(|()| file.sync_all()) {
                Ok(()) => Ok(true),
                Err(err) => {
                    drop(file);
                    let _ = std::fs::remove_file(path);
                    Err(err)
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
            Err(err) => Err(err),
        },
    };
    let _ = std::fs::remove_file(&beside);
    kept
}

/// Why a secret's file that others may read is not used, where the system
/// has such a thing to say.
#[cfg(unix)]
fn open_to_others(file: &Path) -> Option<String> {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(file).ok()?.permissions().mode() & 0o777;
    (mode & 0o077 != 0).then(|| {
        format!(
            "{0} holds this installation's secret and can be read by others on this machine (its mode is {mode:o}). \
             Make it its owner's alone, `chmod 600 {0}`, and start again. If anyone else may have read it, remove it instead: \
             this becomes a new installation, to be joined again wherever the old one was",
            file.display()
        )
    })
}

#[cfg(not(unix))]
fn open_to_others(_: &Path) -> Option<String> {
    None
}

impl Own {
    fn file(data_dir: &Path) -> PathBuf {
        data_dir.join(SECRET_FILE)
    }

    /// Reads the secret that is kept in `file`.
    fn read(file: &Path) -> Result<Own> {
        if let Some(why) = open_to_others(file) {
            return Err(Error::new(why));
        }
        let mut written = std::fs::read(file).map_err(|err| {
            Error::new(format!("could not read this installation's secret from {}: {err}", file.display()))
        })?;
        let seed = unhex(written.trim_ascii());
        wipe(&mut written);
        let seed = seed.ok_or_else(|| {
            Error::new(format!(
                "{} does not hold this installation's secret as it was written. Put back the file as it was; \
                 or remove it, and this becomes a new installation that every hub and device joined to the old one must be joined to again",
                file.display()
            ))
        })?;
        Ok(Own { seed })
    }

    /// This installation's secret: the one in its data folder, or, where
    /// there is none yet, one made now and kept there.
    ///
    /// One that is there and cannot be read is an error and is never made
    /// again unasked: a new secret is a new installation, which every hub
    /// and member of the old one would rightly refuse.
    pub fn open(data_dir: &Path) -> Result<Own> {
        let file = Self::file(data_dir);
        if file.exists() {
            return Self::read(&file);
        }
        let mut seed = [0u8; KEY];
        getrandom::fill(&mut seed).map_err(|err| {
            Error::new(format!("the system gave nothing random to make a secret of: {err}"))
        })?;
        // Written out a letter at a time into what is wiped below: nothing
        // of it is ever in a string that is left lying.
        const LETTERS: &[u8; 16] = b"0123456789abcdef";
        let mut hex = Vec::with_capacity(KEY * 2 + 1);
        for byte in seed {
            hex.push(LETTERS[usize::from(byte >> 4)]);
            hex.push(LETTERS[usize::from(byte & 15)]);
        }
        hex.push(b'\n');
        let kept = write_private_once(&file, &hex);
        wipe(&mut hex);
        match kept {
            Ok(true) => Ok(Own { seed }),
            // Another start of this installation made one in the same
            // moment: that is the secret, and this one is let go.
            Ok(false) => {
                wipe(&mut seed);
                Self::read(&file)
            }
            Err(err) => Err(Error::new(format!(
                "could not keep this installation's secret in {}: {err}",
                file.display()
            ))),
        }
    }

    /// Thirty-two bytes that are this installation's for `purpose` and no
    /// other: the hash of the purpose and the seed, each where the other
    /// cannot be taken for part of it (the purpose is said with its length).
    ///
    /// This is the one derivation here that is not the handshake's own. The
    /// seed is thirty-two bytes from the system's random source, so there is
    /// nothing in it to stretch: what is needed is that two purposes never
    /// give the same bytes, and that the bytes say nothing of the seed.
    pub(crate) fn derive(&self, purpose: &str) -> [u8; KEY] {
        let mut hash = Sha256::new();
        hash.update(b"a key of an installation, version 1\0");
        hash.update((purpose.len() as u64).to_be_bytes());
        hash.update(purpose.as_bytes());
        hash.update(self.seed);
        hash.finalize().into()
    }

    /// The keys this installation proves itself with on a link.
    pub fn link(&self) -> Result<Keys> {
        Keys::of(self.derive(LINK))
    }

    /// The secret kept in `data_dir`, where one is: never made here. Read
    /// by the command line, on this machine, as whoever may read the
    /// factory's data folder: that is what it proves with [`Own::proves`].
    pub fn kept(data_dir: &Path) -> Result<Own> {
        let file = Self::file(data_dir);
        if !file.exists() {
            return Err(Error::new(format!("{} holds no installation's secret", data_dir.display())));
        }
        Self::read(&file)
    }

    /// What says that whoever has it could read this installation's
    /// secret, for `nonce`, which the daemon gave and takes back once:
    /// the hash of a key made for this purpose alone and the nonce. The
    /// key is never sent, and what is sent says nothing of it and is
    /// good for nothing but that nonce.
    pub fn proves(&self, nonce: &str) -> String {
        let mut hash = Sha256::new();
        hash.update(self.derive(OPENS));
        hash.update(nonce.as_bytes());
        hash.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

/// What the key that proves the data folder is made for (`Own::proves`).
const OPENS: &str =
    "the command line proves it can read this factory's data folder, for a link that signs its owner in";

fn unhex(text: &[u8]) -> Option<[u8; KEY]> {
    if text.len() != KEY * 2 {
        return None;
    }
    let digit = |letter: u8| (letter as char).to_digit(16).map(|digit| digit as u8);
    let mut bytes = [0u8; KEY];
    for (byte, pair) in bytes.iter_mut().zip(text.chunks(2)) {
        *byte = digit(pair[0])? << 4 | digit(pair[1])?;
    }
    Some(bytes)
}

/// The number the first message of a handshake carries: one that only goes
/// up, so that a first message sent again by whoever saw it cross is known
/// for one that was said before.
///
/// It is kept in a file, and is on the disk before it is used, so that it
/// is not used again after a restart. It is never below the time, so that
/// an installation that has lost the file is above whatever it said before
/// all the same.
///
/// There is one for each other side it is said to, kept where whoever
/// shakes hands says ([`Said::at`]): what one of them tells this
/// installation of its number ([`Said::above`]) is nothing to any other.
pub struct Said {
    file: PathBuf,
    last: Mutex<u64>,
}

/// The furthest above the time the other side may say this installation's
/// number has got to. A number is the time of a handshake or a little over;
/// one that no clock will reach for a hundred years was not counted up to
/// by anybody.
const FAR: u64 = 100 * 366 * 24 * 3600 * 1000;

impl Said {
    /// The number kept in `file`: nothing said yet, where there is none.
    pub fn at(file: PathBuf) -> Said {
        let last = std::fs::read_to_string(&file).ok().and_then(|kept| kept.trim().parse().ok()).unwrap_or(0);
        Said { file, last: Mutex::new(last) }
    }

    /// Whether `floor` is somewhere a count of handshakes could have got to
    /// by `now` (milliseconds).
    pub fn sensible(floor: u64, now: i64) -> bool {
        floor <= u64::try_from(now).unwrap_or(0).saturating_add(FAR)
    }

    /// The next number, at `now` (milliseconds): above every one before.
    pub fn next(&self, now: i64) -> std::io::Result<u64> {
        self.above(0, now)
    }

    /// The next number above `floor` as well: the other side has said what
    /// it has taken from this installation already.
    pub fn above(&self, floor: u64, now: i64) -> std::io::Result<u64> {
        let mut last = self.last.lock().expect("said lock");
        let next = (*last).max(floor).saturating_add(1).max(u64::try_from(now).unwrap_or(0));
        write_private(&self.file, next.to_string().as_bytes())?;
        *last = next;
        Ok(next)
    }
}
