// What is done to how a project's things are arranged in the side panel,
// from the menus of its lines (`outline.ts`, kept by the daemon for
// everybody): New… on the project's line or a folder's, which makes a
// folder there, or a table (`views/tablemade.tsx`); a folder called
// something else, moved and removed (what was in it taking its place,
// with Undo); a thing moved into a folder or out of one (Move to…); the
// arrangement forgotten, behind a question; and the project itself, a
// table, and any other thing (its board, a library, the Repo…) called
// something else for people, by a small dialog, each by what everything
// knows it by staying as it was. A table is called something else by its
// title; anything else by what the outline shows it as (`name`).
//
// Offered only to whoever holds `projects:write`, which the daemon asks of
// each: anybody else sees the tree as arranged and no line that changes
// it. Each change is sent with what the
// page drew at the top that the outline does not name (`seenOf`), so that
// where it lands is where the panel shows it.

import { signal } from '@preact/signals';
import { useState } from 'preact/hooks';
import { call } from '../api.ts';
import type { Project } from '../api.ts';
import { projectNamed, tableCalled, tableTitled } from '../called.ts';
import { namesOf } from '../envsheld.ts';
import { outlineFold, projectFold } from '../folds.ts';
import { gitListed } from '../gitpage.ts';
import { covers } from '../holds.ts';
import { folderNamed, foldersLaid, keysOf, laidOut, namesIn, placesFor, shownName, usualOf, whatIs } from '../outline.ts';
import type { Laid, Listing, Op } from '../outline.ts';
import { changeOutline, outlineOf, thingShown } from '../outlined.ts';
import { pipelinesOf } from '../pipelines.ts';
import { collections, hasBoard, librariesOf, me, mockupsOf, projects, tablesOf, toast, turned, turnedTo, used } from '../store.ts';
import { Modal, useFocus } from '../ui.tsx';
import { listed } from '../uses.ts';
import type { Offered } from './grouped.tsx';
import { signs } from './signs.tsx';
import { newTabLines } from './newtab.tsx';
import type { Tab } from '../tab.ts';
import { mayMakeTable, NewTable } from './tablemade.tsx';

/** Whether whoever reads the page may arrange a project and call things something else: what the daemon asks of each. */
export const mayArrange = (): boolean => covers(me.value?.principal.scopes ?? [], 'projects:write');

/** What a project lists under it today, in the order it always did (`keysOf`). */
export function listingOf(project: Project): Listing {
  const id = project.id;
  return {
    board: hasBoard(id),
    tables: tablesOf(id),
    libraries: librariesOf(id),
    mockups: mockupsOf(id),
    environments: namesOf(id).length > 0,
    pipelines: pipelinesOf(used.value[id]).length > 0,
    blueprints: listed(used.value[id]).length > 0,
    git: gitListed(project),
    // Where its folder is one whoever added it said; a daemon from before says nothing, and its files are listed.
    repo: project.folder !== 'own',
  };
}

/** What is drawn under a project, as its menus change it: every line it lists, in order, and those laid out. */
export interface Drawn {
  project: Project;
  listed: string[];
  laid: Laid[];
}

export function drawnOf(project: Project): Drawn {
  const keys = keysOf(listingOf(project));
  return { project, listed: keys, laid: laidOut(keys, outlineOf(project.id)) };
}

type Asking =
  | { kind: 'folder'; drawn: Drawn; into: string | null }
  | { kind: 'newTable'; drawn: Drawn; into: string | null }
  | { kind: 'refolder'; drawn: Drawn; folder: string; name: string }
  | { kind: 'project'; project: Project }
  | { kind: 'table'; project: string; name: string }
  | { kind: 'thing'; drawn: Drawn; thing: string }
  | { kind: 'forget'; drawn: Drawn };

/** What is being asked, until it is answered or put away. */
const asking = signal<Asking | null>(null);

/** What the daemon said, as a page says it: without the marks a terminal reads commands by. */
const said = (refused: unknown) => (refused instanceof Error ? refused.message : String(refused)).replaceAll('`', '');

/** Unfolds these folds (a project, a folder) where they are folded, so that what was just put there is in view. */
export function unfolded(...folds: string[]): void {
  const next = new Set(turned.value);
  for (const fold of folds) next.add(fold);
  if (next.size !== turned.value.size) turnedTo(next);
}

/** Does a change, and says in a toast why it could not be. */
async function changed(drawn: Drawn, op: Op) {
  try {
    return await changeOutline(drawn.project.id, op, drawn.listed);
  } catch (refused) {
    toast(said(refused), true);
    return null;
  }
}

/** Moves something drawn under a project (`node`) to where it is put (`outlinedrag.tsx`): what it came to, or nothing where it was refused, which is said. */
export const moveTo = (drawn: Drawn, node: string, at: { into?: string | null; before?: string | null }) =>
  changed(drawn, { op: 'put', node, into: at.into ?? null, before: at.before ?? null });

/** The lines that move something drawn under a project (`node`, a thing's key or a folder's id): where it is is marked. */
function moveLine(drawn: Drawn, node: string): Offered {
  const sub = placesFor(drawn.laid, node).map(
    (place): Offered => ({
      label: place.kind === 'top' ? drawn.project.name : place.name,
      indent: place.kind === 'top' ? 0 : place.depth + 1,
      chosen: place.here,
      closed: place.here ? 'It is here already' : undefined,
      act: () => void changed(drawn, { op: 'put', node, into: place.kind === 'top' ? null : place.id }),
    }),
  );
  return { label: 'Move to…', mark: signs.move(), sub, act: () => {} };
}

/**
 * What is made in `into` (the top, where nothing), offered beside New…: a
 * folder, and for whoever may apply what makes one, a table.
 */
const newLine = (drawn: Drawn, into: string | null): Offered => {
  const sub: Offered[] = [{ label: 'Folder…', act: () => (asking.value = { kind: 'folder', drawn, into }) }];
  if (mayMakeTable()) sub.push({ label: 'Table…', mark: signs.sheets(), act: () => (asking.value = { kind: 'newTable', drawn, into }) });
  return { label: 'New…', mark: signs.plus(), sub, act: () => {} };
};

/**
 * What a project's own menu offers of this (its row's ⋯ and the pointer's
 * other button): a folder made at the end of its lines, the project called
 * something else, and, where it is arranged, the arrangement forgotten.
 */
export function projectLines(project: Project): Offered[] {
  if (!mayArrange()) return [];
  const drawn = drawnOf(project);
  return [
    newLine(drawn, null),
    { label: 'Rename…', mark: signs.pencil(), act: () => (asking.value = { kind: 'project', project }) },
  ];
}

/** Forgetting how a project is arranged, set apart at the foot of its menu where it is arranged. */
export function forgetLine(project: Project): Offered[] {
  if (!mayArrange() || !outlineOf(project.id)) return [];
  return [{ label: 'Forget the arrangement…', mark: signs.out(), apart: true, act: () => (asking.value = { kind: 'forget', drawn: drawnOf(project) }) }];
}

/** What a folder's menu offers: a folder in it, called something else, moved, and removed with what was in it where it was. */
export function folderLines(project: Project, folder: { id: string; name: string }): Offered[] {
  if (!mayArrange()) return [];
  const drawn = drawnOf(project);
  const remove = async () => {
    // Its fold goes with it once the outline is read again (`outlineFoldsGone`): given back, it is unfolded where it was.
    const fold = outlineFold(project.id, folder.id);
    const open = turned.value.has(fold);
    const came = await changed(drawn, { op: 'remove', folder: folder.id });
    const removed = came?.done.removed as Extract<Op, { op: 'restore' }>['removed'] | undefined;
    if (!removed) return;
    const undo = async () => {
      if ((await changed(drawnOf(project), { op: 'restore', removed })) && open) unfolded(fold);
    };
    toast(`${folder.name} removed: what was in it is where it was.`, false, () => void undo());
  };
  return [
    newLine(drawn, folder.id),
    { label: 'Rename…', mark: signs.pencil(), act: () => (asking.value = { kind: 'refolder', drawn, folder: folder.id, name: folder.name }) },
    moveLine(drawn, folder.id),
    { label: 'Remove folder', mark: signs.cross(), apart: true, act: () => void remove() },
  ];
}

/** What a thing's menu offers: called something else for people, and moved. */
export function thingLines(project: Project, thing: string): Offered[] {
  if (!mayArrange()) return [];
  const lines: Offered[] = [];
  const drawn = drawnOf(project);
  if (thing.startsWith('table:')) lines.push(...tableLines(project.id, thing.slice('table:'.length)));
  else lines.push({ label: 'Rename…', mark: signs.pencil(), act: () => (asking.value = { kind: 'thing', drawn, thing }) });
  lines.push(moveLine(drawn, thing));
  const tab = thingTab(project.id, thing);
  return tab ? [...lines, ...newTabLines(tab)] : lines;
}

/** What a thing of a project's outline opens, by its key: nothing for one that opens nothing of its own (Blueprints). */
function thingTab(project: string, thing: string): Tab | null {
  const [kind, name] = [thing.slice(0, thing.indexOf(':')), thing.slice(thing.indexOf(':') + 1)];
  if (kind === 'table' || kind === 'library' || kind === 'mockups') return { kind, project, name };
  if (thing === 'board' || thing === 'environments' || thing === 'pipelines' || thing === 'git' || thing === 'repo') return { kind: thing, project };
  return null;
}

/** The line that calls a table something else for people: of a table of plain records, for whoever may. */
export function tableLines(project: string, name: string): Offered[] {
  if (!mayArrange()) return [];
  return [{ label: 'Rename…', mark: signs.pencil(), act: () => (asking.value = { kind: 'table', project, name }) }];
}

/** The line of a project's page's gear that calls it something else. */
export function projectRename(project: Project): Offered[] {
  if (!mayArrange()) return [];
  return [{ label: 'Rename…', mark: signs.pencil(), act: () => (asking.value = { kind: 'project', project }) }];
}

/** What is asked, while something is: drawn once, by the frame. */
export function OutlineAsked() {
  const now = asking.value;
  if (!now) return null;
  const close = () => (asking.value = null);
  if (now.kind === 'forget') return <Forget drawn={now.drawn} onClose={close} />;
  if (now.kind === 'newTable') {
    const where = now.into ? (foldersLaid(now.drawn.laid).find((one) => one.id === now.into)?.name ?? now.drawn.project.name) : now.drawn.project.name;
    return <NewTable key="new-table" drawn={now.drawn} into={now.into} where={where} onClose={close} />;
  }
  if (now.kind === 'folder') {
    const { drawn, into } = now;
    const beside = namesIn(drawn.laid, into);
    const where = into ? (foldersLaid(drawn.laid).find((one) => one.id === into)?.name ?? drawn.project.name) : drawn.project.name;
    return (
      <Named
        key="folder"
        title={`New folder in ${where}`}
        label="Folder name"
        typed=""
        says="Only how the side panel shows it: it is on no disk, and nothing in it moves anywhere."
        check={(typed) => {
          const named = folderNamed(typed, beside);
          return 'wrong' in named ? named.wrong : null;
        }}
        save={async (typed) => {
          const came = await changeOutline(drawn.project.id, { op: 'make', name: typed.trim(), into }, drawn.listed);
          const made = came.done.made;
          if (typeof made === 'string') unfolded(projectFold(drawn.project.id), outlineFold(drawn.project.id, made), ...(into ? [outlineFold(drawn.project.id, into)] : []));
        }}
        onClose={close}
      />
    );
  }
  if (now.kind === 'refolder') {
    const { drawn, folder, name } = now;
    // Beside it: the folders of the level it is in, but itself.
    const at = placesFor(drawn.laid, folder).find((place) => place.here);
    const beside = namesIn(drawn.laid, at?.kind === 'folder' ? at.id : null).filter((one) => one.toLowerCase() !== name.toLowerCase());
    return (
      <Named
        key={`folder-${folder}`}
        title={`Rename ${name}`}
        label="Folder name"
        typed={name}
        check={(typed) => {
          const named = folderNamed(typed, beside);
          return 'wrong' in named ? named.wrong : null;
        }}
        same={(typed) => typed.trim() === name}
        save={async (typed) => void (await changeOutline(drawn.project.id, { op: 'rename', folder, name: typed.trim() }, drawn.listed))}
        onClose={close}
      />
    );
  }
  if (now.kind === 'thing') {
    const { drawn, thing } = now;
    const usual = usualOf(thing);
    const called = thingShown(drawn.project.id, thing) ?? usual;
    return (
      <Named
        key={`thing-${drawn.project.id}-${thing}`}
        title={`Rename ${called}`}
        label="Name"
        typed={called}
        says={`What everybody reading ${drawn.project.name} sees it as. What blueprints and commands call it (${whatIs(thing)}) stays as it is; left empty, it is called ${usual} again.`}
        check={(typed) => {
          const named = shownName(typed);
          return 'wrong' in named ? named.wrong : null;
        }}
        same={(typed) => typed.trim() === called}
        save={async (typed) => {
          const named = shownName(typed);
          // Named as it always says is no name of anybody's: it is shown as it always was.
          const name = 'name' in named && named.name !== usual ? named.name : null;
          await changeOutline(drawn.project.id, { op: 'name', thing, name }, drawn.listed);
        }}
        onClose={close}
      />
    );
  }
  if (now.kind === 'project') {
    const { project } = now;
    return (
      <Named
        key={`project-${project.id}`}
        title={`Rename ${project.name}`}
        label="Project name"
        typed={project.name}
        says={`What everybody reads it as. Its id, ${project.id}, which everything knows it by, stays as it is.`}
        check={(typed) => {
          const named = projectNamed(typed, project.id, projects.value);
          return 'wrong' in named ? named.wrong : null;
        }}
        same={(typed) => typed.trim() === project.name}
        save={async (typed) => {
          const answer = await call<Project>('PATCH', `/projects/${encodeURIComponent(project.id)}`, { name: typed.trim() });
          projects.value = projects.value.map((one) => (one.id === answer.id ? { ...one, name: answer.name } : one));
        }}
        onClose={close}
      />
    );
  }
  const { project, name } = now;
  const table = (collections.value[project] ?? []).find((one) => one.name === name);
  const called = table ? tableCalled(table) : name;
  return (
    <Named
      key={`table-${project}-${name}`}
      title={`Rename ${called}`}
      label="Table name"
      typed={called}
      says={`What everybody reads it as. Its name, ${name}, which blueprints, scripts and its address know it by, stays as it is; left empty, it is called that again.`}
      check={(typed) => {
        const titled = tableTitled(typed);
        return 'wrong' in titled ? titled.wrong : null;
      }}
      same={(typed) => typed.trim() === called}
      save={async (typed) => {
        const titled = tableTitled(typed);
        const at = `/collections/${encodeURIComponent(project)}/${encodeURIComponent(name)}/title`;
        if ('title' in titled && titled.title !== null) await call('PUT', at, { title: titled.title });
        else await call('DELETE', at);
      }}
      onClose={close}
    />
  );
}

/** A small dialog that asks what something is to be called, saved on Enter; what the daemon refuses is said in its sentence, with what was typed staying. A wiki's page is renamed by it too (`wikitree.tsx`). */
export function Named(props: {
  title: string;
  label: string;
  typed: string;
  says?: string;
  check: (typed: string) => string | null;
  same?: (typed: string) => boolean;
  save: (typed: string) => Promise<void>;
  onClose: () => void;
}) {
  const [typed, setTyped] = useState(props.typed);
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const input = useFocus<HTMLInputElement>();
  const now = props.check(typed);
  const unchanged = props.same?.(typed) ?? false;
  const save = async (event: Event) => {
    event.preventDefault();
    if (now || unchanged || busy) return;
    setBusy(true);
    setWrong(null);
    try {
      await props.save(typed);
      props.onClose();
    } catch (refused) {
      setBusy(false);
      setWrong(said(refused));
    }
  };
  return (
    <Modal onClose={props.onClose}>
      <h2>{props.title}</h2>
      <form class="stack rename-form" onSubmit={save}>
        <label>
          {props.label}
          <input
            ref={input}
            value={typed}
            aria-describedby="named-says"
            onInput={(event) => {
              setTyped(event.currentTarget.value);
              setWrong(null);
            }}
          />
        </label>
        <p id="named-says" class="faint" style="margin:0;font-size:calc(12px * var(--size))">
          {(typed !== props.typed ? now : null) ?? props.says ?? ''}
        </p>
        {wrong && (
          <p role="alert" class="kept-wrong">
            {wrong}
          </p>
        )}
        <div class="row" style="justify-content:flex-end">
          <button type="button" onClick={props.onClose}>
            Cancel
          </button>
          <button class="primary" type="submit" disabled={busy || !!now || unchanged}>
            Save
          </button>
        </div>
      </form>
    </Modal>
  );
}

/** Asked before a project's arrangement is forgotten: what goes, and that nothing else does. */
function Forget({ drawn, onClose }: { drawn: Drawn; onClose: () => void }) {
  const [busy, setBusy] = useState(false);
  const folders = foldersLaid(drawn.laid).length;
  const forget = async () => {
    setBusy(true);
    const came = await changed(drawn, { op: 'forget' });
    if (came) onClose();
    else setBusy(false);
  };
  return (
    <Modal onClose={onClose}>
      <h2>Forget how {drawn.project.name} is arranged?</h2>
      <p>
        {drawn.project.name} is shown in the side panel in the usual order again, for everybody, and its {folders === 1 ? 'folder goes' : `${folders} folders go`}. Nothing else changes: its board, its tables and its files stay
        as they are, and nothing in a folder is taken away.
      </p>
      <div class="row" style="justify-content:flex-end">
        <button type="button" onClick={onClose}>
          Cancel
        </button>
        <button class="primary" type="button" disabled={busy} onClick={() => void forget()}>
          Forget it
        </button>
      </div>
    </Modal>
  );
}
