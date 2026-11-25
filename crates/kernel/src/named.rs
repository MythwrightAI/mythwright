//! What a file's path says of it by its name alone: the folders it is
//! in and what kind of file it is. Functions of the text of a path, said
//! with a file's change (`studio::projects`, which offers them under the
//! names they always had), so that whatever follows a folder or a kind
//! of file takes no path apart to know. They came out of the studio when
//! it was at its ceiling, as functions of plain values do.

/// Every folder a file is in, the outermost first, each from the top of
/// its project: `docs/guides/install.md` is in `docs` and in `docs/guides`.
/// A file at the top is in none.
///
/// Said with a file's change, so that whatever follows a folder asks
/// whether the folder is among them and takes no path apart to know.
pub fn folders(path: &str) -> Vec<&str> {
    path.match_indices('/').map(|(at, _)| &path[..at]).filter(|folder| !folder.is_empty()).collect()
}

/// What kind of file `path` is, by its name: what follows the last dot of
/// it (`md` for `docs/plans/cart.md`), and nothing for a name with no dot
/// in it or one that only begins with one (`.gitignore`).
///
/// Said with a file's change beside its folders, so that whatever follows
/// one kind of file asks which it is and takes no name apart to know.
pub fn kind(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rsplit_once('.') {
        Some((before, kind)) if !before.is_empty() => kind,
        _ => "",
    }
}
