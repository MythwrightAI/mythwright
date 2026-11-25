// The folder a new project is made of, where whoever adds it says one.
//
// A project is not assumed to be a repository. Added with nothing said of
// a folder, the factory keeps what the project has (its documents, its
// blueprints) in a folder of its own making, and no Repo is listed
// under the project: nobody asked for any. Somebody who has a repository,
// or files that are already somewhere, says where, and the project is
// that folder: its files are then listed under it.
//
// Saying a folder is taking one up, which puts everything in it within
// reach of whoever may read the project, so the daemon asks more for it
// than for adding a project (`admin`), and the field is drawn only for
// whoever holds that. What a folder is refused for (one that is another
// project's, one that holds the factory's own data, a path that is not
// whole) is the daemon's to say, in its own sentence.

import { covers } from '../holds.ts';
import { me } from '../store.ts';

/** Whether whoever is signed in may say the folder a project is made of, as the daemon asks. */
export function mayGiveFolder(): boolean {
  return covers(me.value?.principal.scopes ?? [], 'admin');
}

/** Where a new project's folder is said, by whoever may say one; nothing is drawn for anybody else. */
export function FolderField(props: { value: string; onChange: (folder: string) => void }) {
  if (!mayGiveFolder()) return null;
  return (
    <div class="ask-folder stack" style="gap:4px;min-width:0">
      <label>
        Folder
        <input
          value={props.value}
          placeholder="A folder on this device, for a repository or files already there"
          aria-describedby="ask-folder-says"
          onInput={(event) => props.onChange(event.currentTarget.value)}
        />
      </label>
      {/* Beside the field and not part of what it is called: a field is found by its label. */}
      <span id="ask-folder-says" class="faint" style="font-size:calc(12px * var(--size));line-height:1.4">
        Its whole path; one that is not there yet is made. Left empty, the factory keeps the project's documents and blueprints itself, and no Repo is listed under it.
      </span>
    </div>
  );
}
