# Moving between builds

What to do when you take another build, and what an earlier program left
behind becomes. [The README](../README.md#which-build) says which builds
there are.

**Moving to another build**, by hand: stop the daemon (`mw down`), copy
its data folder somewhere while it is stopped (the package's `README.md`
says where it is: `~/.local/share/mythwright` on Linux, and the same with
`-<profile>` after it for a factory with a profile), unpack the other
package anywhere and start it (`./mw up`). The package's folder holds no
data, so the new program takes up the factory where it was, as an
install does (`cargo xtask install` from source is the same move). Then
`mw extension list` says whether the packages you had installed differ
from the ones that came with it. To try a nightly without touching the
factory you use, give it a profile of its own (`./mw up --profile
nightly`): its data is apart, and the two run side by side.

**Moving back** is not the same move the other way. The first start of a
newer program on a factory changes how its journal is kept, and an
earlier program refuses a journal whose tables are newer than it knows.
A nightly is usually ahead of every pinned release, and so is anything it
has started. So going from a nightly back to a pinned version, or from
one version back to an earlier one, needs the copy of the data folder
you made *before* the newer program first ran: stop the daemon, put that
copy back in place of the data folder, and start the earlier program.
What was done in the factory since that copy is not in it. Without such
a copy there is no going back; keep one each time you move forward.

**A factory an earlier program made** is taken up by this one as it
starts, and that is one way: how its journal is kept changes, and the
earlier program refuses it afterwards. To be able to go back, take a copy
first: the data folder copied while the factory is stopped, or
`mw apply std/snapshot --param to=<folder>`, which writes what a new
factory is started from, with nobody signed in and nothing running
([docs/profiles.md](profiles.md)). Its projects go on as they were, all kept in the
factory's one file. Each can be moved to a journal of its own, whole
(what is run for it and what was said of it, its cards and their
history, what its agents were asked), which is what lets one project
with a great deal to do not hold up another:
stop the factory (`mw down`), `mw apart --look` to read what would be
moved and what is in the way, `mw apart`, and start it again
(`mw help apart`). A project a program between moved, or made with a
journal that keeps less, is taken the rest of the way by the same. That
cannot be gone back on either.

**A package an earlier program installed** goes on as the copy it was
installed as: what a newer program's package says (what its program
needs behind a wall, which of what it keeps is its sign-in) is not said
here until you take the one that came. After an upgrade
`mw extension list` says which installed packages are not the ones that
came with this program and where each differs, and so does each
package's line under **Extensions** in the client;
`mw extension update <name>` (or `--all`, or **Update** there) takes the
one that came in place of the copy, whatever was changed in the copy,
and keeps the copy beside it (`mw help extensions`). Nothing takes it
unasked, since the copy may be one you changed, and nothing tells such a
copy from an earlier one (`agents`: `came.rs`; `api`: `updated.rs`;
`cli`: `updated.rs`).

**A fleet an earlier program started** goes on with the blueprints it
had until you say otherwise (`mw apply fleet --project <project>
--take`). One from before the fleet had a blueprint for each kind of
column (`std/working`, `std/reviewing`, `std/planning`) had one for
every column, `std/stage`, which was taken out before anything was
released: it is not brought to the new ones by taking them, and is
stopped and applied again.
