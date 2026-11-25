// The lines of a project's gear that say where it keeps blueprints of its
// own, and change that for whoever the daemon lets (`blueprinted.ts`):
// kept by the factory, the usual; a folder of its repository read too; or
// none; and the blueprints of that folder moved into the factory; and a
// new one of its own made, for whoever may write its files. The one
// that is so is chosen. A folder is asked in a dialog, and what the daemon
// refuses is said there in its own sentence, the dialog staying with what
// was written. Anybody else is told where they are, and offered nothing.

import { signal } from '@preact/signals';
import { useEffect, useState } from 'preact/hooks';
import { ApiError, type Project } from '../api.ts';
import { type Asked, folderAsked, type KeptIn, keepIn, keptIn, keptLines, keptSaid, mayKeep, readKept } from '../blueprinted.ts';
import { me, toast } from '../store.ts';
import { Modal, useEvents, useFocus } from '../ui.tsx';
import { newBlueprintLine } from './ownblueprints.tsx';
import type { Lines } from './pagehead.tsx';

/** The project whose other folder is being asked for, with where it keeps them now, while the dialog is open. */
const asking = signal<{ project: Project; kept: KeptIn } | null>(null);

/** Where `project` keeps its own blueprints: read once, and again whenever that is said of it. Nothing until it is read. */
export function useKeptIn(project: string): KeptIn | null {
  const [kept, setKept] = useState<KeptIn | null>(null);
  const [said, setSaid] = useState(0);
  useEffect(() => {
    let stale = false;
    void readKept(project).then(
      (answer) => !stale && setKept(keptIn(answer)),
      () => !stale && setKept(null),
    );
    return () => {
      stale = true;
    };
  }, [project, said]);
  useEvents('project.blueprints', (event) => event.data?.project === project && setSaid((now) => now + 1), [project]);
  return kept;
}

const said = (asked: { blueprints: KeptIn; moved?: unknown[] }, name: string) => {
  if (asked.moved) return `${asked.moved.length} of ${name}'s own blueprints are kept by the factory now; its repository's files are as they were, for you to take away`;
  const kept = keptIn(asked);
  return kept.said === 'none' ? `${name} keeps no blueprints of its own now` : `${keptSaid(kept).replace(/^Its/, `${name}'s`)} now`;
};

async function keep(project: Project, asked: Asked): Promise<void> {
  try {
    const answer = await keepIn(project.id, asked);
    if (answer.changed !== false) toast(said(answer, project.name));
  } catch (refused) {
    toast(refused instanceof Error ? refused.message : String(refused), true);
  }
}

/** The lines for `project`, first among its gear's, once where it keeps them is read. */
export function keptMenu(project: Project | undefined, kept: KeptIn | null): Lines {
  if (!project || !kept) return [];
  const lines: Lines = [{ said: <span class="menu-note">{keptSaid(kept)}</span> }];
  // Whoever may write the project's files makes one of its own here, where it keeps any, whatever else they may.
  const made = kept.said === 'none' ? null : newBlueprintLine(project.id);
  if (made) lines.push(made);
  if (!mayKeep(me.value?.principal.scopes ?? [])) return lines;
  for (const line of keptLines(kept)) {
    lines.push({
      label: line.label,
      // Moving them in is something done, and no choice of where they are.
      chosen: typeof line.asks === 'object' && 'move_in' in line.asks ? undefined : line.chosen,
      act: () => {
        if (line.asks === 'folder') asking.value = { project, kept };
        else if (!line.chosen) void keep(project, line.asks);
      },
    });
  }
  return lines;
}

/** The dialog that asks for another folder, where one is being asked for. */
export function KeepFolder() {
  const asked = asking.value;
  return asked ? <Asking key={asked.project.id} project={asked.project} kept={asked.kept} /> : null;
}

function Asking({ project, kept }: { project: Project; kept: KeptIn }) {
  const [folder, setFolder] = useState(kept.said === 'folder' ? (kept.folder ?? '') : '');
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const input = useFocus<HTMLInputElement>();
  const close = () => (asking.value = null);
  const submit = async (event: Event) => {
    event.preventDefault();
    const asked = folderAsked(folder);
    if (!asked) return;
    setBusy(true);
    setWrong(null);
    try {
      const answer = await keepIn(project.id, asked);
      if (answer.changed !== false) toast(said(answer, project.name));
      close();
    } catch (refused) {
      setBusy(false);
      setWrong(refused instanceof ApiError || refused instanceof Error ? refused.message : String(refused));
    }
  };
  return (
    <Modal onClose={close}>
      <h2>A folder of {project.name}'s repository for its blueprints</h2>
      <form class="stack" onSubmit={(event) => void submit(event)}>
        <label>
          Folder
          <input ref={input} required value={folder} placeholder="tools/blueprints" onInput={(event) => setFolder(event.currentTarget.value)} />
        </label>
        <p class="faint" style="margin:0;font-size:calc(12px * var(--size))">
          A folder within the project, from its top, for blueprints kept with its code. Its blueprints are looked for there and then where the factory
          keeps them, and one name in both is refused; a fork is written where the factory keeps them. What is running goes on as it was applied,
          unless texts are followed.
        </p>
        {wrong && (
          <p role="alert" class="faint" style="margin:0">
            {wrong}
          </p>
        )}
        <div class="row" style="justify-content:flex-end">
          <button type="button" onClick={close}>
            Cancel
          </button>
          <button class="primary" type="submit" disabled={busy}>
            Read it
          </button>
        </div>
      </form>
    </Modal>
  );
}
