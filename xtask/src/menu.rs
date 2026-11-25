//! The product in a desktop's menu, on Linux: an entry that names the
//! program and its icon, and the icon where a desktop looks for one.
//!
//! A desktop finds both under a folder of the person's own
//! (`~/.local/share`, or `$XDG_DATA_HOME`): the entry in `applications/`
//! and the icon, by size, under `icons/hicolor/`. An entry has to name the
//! program by its whole path, which is only known where the program has
//! been put. So an install writes the entry itself, and a package, which
//! can be unpacked anywhere, carries a small script that writes it from
//! wherever the folder is when it is run.
//!
//! This half says what those are, as functions. The pictures come from
//! `desktop/icon/`; `desktop/icon/README.md` says what each is for.

/// The pictures of the icon a menu uses, each with where it goes under the
/// person's data folder for a desktop to find it by the name `name`.
pub fn icon_places(name: &str) -> Vec<(&'static str, String)> {
    vec![
        ("icon-48.png", format!("icons/hicolor/48x48/apps/{name}.png")),
        ("icon-256.png", format!("icons/hicolor/256x256/apps/{name}.png")),
        ("icon-512.png", format!("icons/hicolor/512x512/apps/{name}.png")),
        ("icon.svg", format!("icons/hicolor/scalable/apps/{name}.svg")),
    ]
}

/// Whether a file of `desktop/icon/` is a picture of the icon, and not the
/// page about it, the script that makes the pictures, or the window's own
/// pixels (which are built into the program).
pub fn is_picture(file: &str) -> bool {
    [".svg", ".png", ".icns", ".ico"].iter().any(|kind| file.ends_with(kind))
}

/// Where the entry goes under the person's data folder.
pub fn entry_place(name: &str) -> String {
    format!("applications/{name}.desktop")
}

/// A program's path as an entry's `Exec` takes it: in quotes, with what
/// would mean something inside them made plain.
fn exec(program: &str) -> String {
    let mut quoted = String::from("\"");
    for letter in program.chars() {
        if matches!(letter, '"' | '`' | '$' | '\\') {
            quoted.push('\\');
        }
        quoted.push(letter);
    }
    quoted.push('"');
    quoted
}

/// The entry: what the product is called, the program to start, and the
/// icon by the name it was put under. `class` is what the window calls
/// itself to the desktop, which is how a running window is matched to its
/// entry: the name the program is run by.
pub fn entry(display: &str, name: &str, program: &str, class: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName={display}\nExec={}\nIcon={name}\nTerminal=false\nCategories=Development;\nStartupWMClass={class}\n",
        exec(program)
    )
}

/// The script a package carries, which puts the folder it is in into the
/// menu, or takes it out again. `program` is the window's program in that
/// folder; `name` is what the entry and the icon are filed under.
pub fn script(display: &str, name: &str, program: &str) -> String {
    let mut copies = String::new();
    let mut removes = String::new();
    for (file, place) in icon_places(name) {
        copies.push_str(&format!(
            "  if [ -f \"$here/icon/{file}\" ]; then mkdir -p \"$(dirname \"$data/{place}\")\" && cp \"$here/icon/{file}\" \"$data/{place}\"; fi\n"
        ));
        removes.push_str(&format!("  rm -f \"$data/{place}\"\n"));
    }
    let entry = entry_place(name);
    format!(
        r#"#!/bin/sh
# Puts {display} in the desktop's menu, or takes it out again.
#
#   ./menu.sh          add it: an entry that starts the program in this
#                      folder, and its icon. Run it again after moving the
#                      folder, since an entry names the program by its path.
#   ./menu.sh remove   take the entry and the icon away
#
# Both are files under your own data folder (~/.local/share, or
# $XDG_DATA_HOME); nothing else is touched.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
data=${{XDG_DATA_HOME:-$HOME/.local/share}}
entry="$data/{entry}"

if [ "${{1-}}" = remove ]; then
  rm -f "$entry"
{removes}  echo "{display} is out of the menu"
  exit 0
fi
if [ $# -gt 0 ]; then
  echo "error: \`$1\` is not something this does: run it with nothing to add {display} to the menu, or with \`remove\`" >&2
  exit 2
fi

mkdir -p "$(dirname "$entry")"
{copies}# The path is put in quotes, with what would mean something inside them
# made plain, as an entry asks.
program=$(printf '%s' "$here/{program}" | sed -e 's/[\\"`$]/\\&/g')
cat > "$entry" <<ENTRY
[Desktop Entry]
Type=Application
Name={display}
Exec="$program"
Icon={name}
Terminal=false
Categories=Development;
StartupWMClass={program}
ENTRY
echo "{display} is in the menu; it starts $here/{program}"
echo "  $entry"
"#
    )
}
