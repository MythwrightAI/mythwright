//! What this installation is to another: its secret, the keys made from it
//! and the short name of the one it shows. All of it is the `link` crate's
//! ([`link::own`]); what is here is where a device keeps the number it has
//! said to each of its hubs.

use std::path::{Path, PathBuf};

pub use link::own::{KEY, Keys, Own, SECRET_FILE, Said, fingerprint, write_private};

/// Where the number said to the hub whose key's short name is `hub` is
/// kept, in `data_dir`. There is one for each hub, named by the hub's key,
/// and it goes when the device leaves that hub.
pub fn said_file(data_dir: &Path, hub: &str) -> PathBuf {
    data_dir.join("gateway").join(format!("hub.{hub}.said"))
}

/// The number said to the hub whose public key is `hub`.
pub fn said_to(data_dir: &Path, hub: &[u8]) -> Said {
    Said::at(said_file(data_dir, &fingerprint(hub)))
}
