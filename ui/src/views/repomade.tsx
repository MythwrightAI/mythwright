// Making something new in a project's repo: a file, and a folder to keep
// files in. Offered as a line of the Repo page's gear and as its buttons,
// and as lines of the menu of the side panel's Repo line and of each of
// its folders, each in the folder it was pressed in; to whoever may write
// the project's files and to nobody else.
//
// What a name comes to is worked out as it is typed (`repomade.ts`), and
// what the daemon refuses all the same is said in its own sentence with
// the dialog staying, what was typed still there. A file opens in its tab
// once it is made; a folder is shown on the Repo page, where the next
// thing made is made in it.

import { signal } from '@preact/signals';
import { useState } from 'preact/hooks';
import { api } from '../api.ts';
import { firstText, folderMeant, lineOf, named, offered, saidWhere } from '../repomade.ts';
import type { Made } from '../repomade.ts';
import { me, openFolder, openTab, projects, refreshTree, trees, turn, turned } from '../store.ts';
import { Modal, useFocus } from '../ui.tsx';
import type { Offered } from './grouped.tsx';
import { signs } from './signs.tsx';

/** What is being made, and where, while the dialog that names it is open. */
const making = signal<{ made: Made; project: string; folder: string; fold?: string } | null>(null);

/** What whoever is signed in may make in a project's repo: nothing where they may not write its files. */
export function mayMake(): Made[] {
  return offered(me.value?.principal.scopes ?? []);
}

/** Asks for the name of something to be made in `folder` of a project's repo; `fold` is the side panel's fold it is made under, unfolded once it is. */
export function askToMake(made: Made, project: string, folder: string, fold?: string): void {
  making.value = { made, project, folder: folderMeant(folder, trees.value[project] ?? []), fold };
}

/** The lines of a menu that make something in a folder of a project's repo, for whoever may. */
export function makeLines(project: string, folder: string, fold?: string): Offered[] {
  return mayMake().map((made, nth) => ({
    label: lineOf(made),
    mark: made === 'file' ? signs.plus() : <span class="glyph">▸</span>,
    apart: nth === 0,
    act: () => askToMake(made, project, folder, fold),
  }));
}

/** What the daemon said, as a page says it: without the marks a terminal reads names by. */
const said = (refused: unknown) => (refused instanceof Error ? refused.message : String(refused)).replaceAll('`', '');

/** The dialog that names what is made, wherever it was asked for. */
export function AskToMake() {
  const asked = making.value;
  if (!asked) return null;
  return <Name key={`${asked.made}:${asked.project}:${asked.folder}`} {...asked} onClose={() => (making.value = null)} />;
}

function Name({ made, project, folder, fold, onClose }: { made: Made; project: string; folder: string; fold?: string; onClose: () => void }) {
  const [name, setName] = useState('');
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const input = useFocus<HTMLInputElement>();
  const came = named(name, folder, trees.value[project] ?? []);
  const called = projects.value.find((one) => one.id === project)?.name ?? project;
  const make = async (event: Event) => {
    event.preventDefault();
    if (busy || !('path' in came)) return;
    setBusy(true);
    setWrong(null);
    try {
      const entry = made === 'file' ? await api.newFile(project, came.path, firstText(came.path)) : await api.makeFolder(project, came.path);
      await refreshTree(project);
      // What it was made under is unfolded in the side panel, so that it is in view there.
      if (fold && !turned.value.has(fold)) turn(fold);
      onClose();
      if (made === 'file') openTab({ kind: 'file', project, path: entry.path });
      else openFolder(project, entry.path);
    } catch (refused) {
      setBusy(false);
      setWrong(said(refused));
    }
  };
  const typed = name.trim() !== '';
  return (
    <Modal onClose={onClose}>
      <h2>
        {made === 'file' ? 'A new file' : 'A new folder'} in {called} repo
      </h2>
      <form class="stack repo-made" onSubmit={make}>
        <label>
          Name
          <input
            ref={input}
            value={name}
            placeholder={made === 'file' ? 'notes.md' : 'notes'}
            aria-describedby="repo-made-says"
            onInput={(event) => {
              setName(event.currentTarget.value);
              setWrong(null);
            }}
          />
        </label>
        <p id="repo-made-says" class="faint repo-made-says">
          {typed && 'wrong' in came
            ? came.wrong
            : made === 'file'
              ? `It is made ${saidWhere(folder)}, ${'path' in came && firstText(came.path) ? 'beginning with a heading of its name' : 'empty'}, and opens to be written in.`
              : `It is made ${saidWhere(folder)}, empty, for files to be kept in.`}
        </p>
        {wrong && (
          <p role="alert" class="repo-made-wrong">
            {wrong}
          </p>
        )}
        <div class="row" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" type="submit" disabled={busy || !('path' in came)}>
            {busy ? 'Making it' : made === 'file' ? 'Make the file' : 'Make the folder'}
          </button>
        </div>
      </form>
    </Modal>
  );
}
