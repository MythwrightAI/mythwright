//! Something attached to a record that was taken away into the Trash
//! (`studio::attaching`, `Kind::Attachment`): given back to its record,
//! its file whole, and its file taken away where it goes for good.

use std::path::PathBuf;

use journal::Journal;
use kernel::{Error, Principal, Result};
use serde_json::json;
use studio::attaching::{self, Attached};
use studio::trash::{self, Trashed};
use studio::{Collections, Records};

/// Gives back something attached that was taken away into the Trash, to
/// its record where that is there: its file moved back from the entry's
/// folder (or found where it was, where a stop came before it was moved),
/// and its row put back in one write with what is said of it and the
/// entry taken out.
pub(crate) async fn restore(
    collections: &Collections,
    journal: &Journal,
    entry: &Trashed,
    folder: Option<PathBuf>,
    by: &Principal,
) -> Result<()> {
    let attached: Attached = serde_json::from_value(entry.kept.clone())
        .map_err(|_| Error::internal(format!("what is kept of `{}` cannot be read", entry.called)))?;
    let (collection, record) = collections.record(&attached.record).await.map_err(|_| {
        Error::conflict(format!(
            "`{}` was attached to `{}`, which is not there: it is given back once its record is",
            attached.name, attached.record
        ))
    })?;
    let file = collections.attached_in()?.join(&attached.id);
    let held = folder.map(|folder| folder.join(&entry.id));
    if let Some(held) = held.as_ref().map(|held| held.join(trash::KEPT)).filter(|kept| kept.exists()) {
        std::fs::rename(held, &file)?;
    }
    if !file.exists() {
        return Err(Error::not_found(format!("the file of `{}` is no longer kept", attached.name)));
    }
    let mut say = Records::said(&collection, &attached, by, None).carrying(&collection, &record.data);
    say.data["restored"] = json!(true);
    let (row, out, actor, project) =
        (attached.clone(), entry.clone(), by.actor(), collection.project.clone());
    let written = journal.write(move |writing| {
        attaching::put(writing, &project, &row)?;
        writing.say("record.attached", say.source, say.data)?;
        trash::take_out(writing, &out, "trash.restored", &actor, json!({}))
    });
    written.await?;
    if let Some(held) = held {
        let _ = std::fs::remove_dir_all(held);
    }
    Ok(())
}

/// Takes away the file of something attached that went from the Trash for
/// good where it is still where it was, a stop having come before it was
/// moved into the entry's folder, which goes by itself.
pub(crate) fn forget_left(collections: &Collections, entry: &Trashed) {
    let id = entry.kept.get("id").and_then(serde_json::Value::as_str).unwrap_or_default();
    if let (Ok(dir), false) = (collections.attached_in(), id.is_empty() || id.contains(['/', '\\'])) {
        let _ = std::fs::remove_file(dir.join(id));
    }
}
