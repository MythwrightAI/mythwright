// What a file's page hands the right of its head (`pagehead.tsx`).
//
// When it was last changed is the file's own time, as the project's list
// of files says it. Who changed it is said only for a document of a
// library, from its history, and only where a line was written as the
// file was: what another program does to a file is not kept, since nobody
// saw who did it.
//
// The gear holds what can be done with the file through the daemon as it
// is: a copy made beside it, a copy saved from the browser, its path, and,
// for whoever may write the project's files, the file taken off the disk
// behind a question. Nothing moves or renames a file here: a document is
// known by where it is, and what was said on it would be left behind.
// A document is shared by its own mark and its page (`shared.tsx`).

import { useEffect, useState } from 'preact/hooks';
import { api, shown } from '../api.ts';
import type { Collection } from '../api.ts';
import { edited as editedOf, lastOf } from '../edited.ts';
import type { Edited, Last } from '../edited.ts';
import { covers } from '../holds.ts';
import { forgetThing } from '../latelykept.ts';
import { forgetTab } from '../tabsof.ts';
import { attempt, closeTab, me, openTab, tabKey, trees } from '../store.ts';
import type { Tab } from '../tab.ts';
import { Modal, useEvents } from '../ui.tsx';
import { saidDeleted } from './trash.tsx';
import { copyOf } from '../util.ts';
import { copy, PageActs } from './pagehead.tsx';
import type { Lines, Tool } from './pagehead.tsx';
import { documentShare, marksPublic } from './shared.tsx';
import { signs } from './signs.tsx';

/** When a file was last written, and by whom where it is the document `record` and its history says. */
export function useFileEdited(project: string, path: string, record: string | null): Edited | null {
  const [read, setRead] = useState<Last | null>(null);
  const [heard, setHeard] = useState<Last | null>(null);
  useEffect(() => {
    setRead(null);
    setHeard(null);
    if (!record) return;
    let stale = false;
    // Whoever may not read its history is told nothing of who: when is still said.
    void api.history(record).then(
      (lines) => !stale && setRead(lastOf(lines)),
      () => {},
    );
    return () => {
      stale = true;
    };
  }, [record]);
  useEvents(
    ['collection.history'],
    (event) => {
      if (record && event.data?.record === record && typeof event.data?.who === 'string') setHeard({ who: event.data.who, ts: event.ts });
    },
    [record],
  );
  const entry = (trees.value[project] ?? []).find((one) => one.path === path);
  return editedOf([entry?.modified], [read, heard]);
}

interface Props {
  project: string;
  path: string;
  /** The document it is, and the library it is of, where it is one. */
  of?: { library: Collection; record: string } | null;
  /** It is text: what it says can be read here, and so copied. */
  text?: boolean;
  /** It holds changes nobody saved. */
  dirty?: boolean;
  /** Shows its history, where it keeps one. */
  onHistory?: () => void;
  /** What opens the panel beside it, where it is a document: its comments, what it is and its history. */
  tool?: Tool;
}

export function FileActs({ project, path, of = null, text = false, dirty = false, onHistory, tool }: Props) {
  const tab: Tab = { kind: 'file', project, path };
  const edited = useFileEdited(project, path, of?.record ?? null);
  const [asking, setAsking] = useState(false);
  const [busy, setBusy] = useState(false);
  const name = path.split('/').pop() ?? path;
  // What the daemon asks of whoever writes or deletes a project's file.
  const may = covers(me.value?.principal.scopes ?? [], 'projects:write');
  const duplicate = async () => {
    // What is there now, asked for and not what the page last listed: a copy never goes where a file is.
    const [content, there] = [await attempt(() => api.readFile(project, path)), await attempt(() => api.tree(project))];
    if (content === undefined || there === undefined) return;
    const made = copyOf(path, there.map((one) => one.path));
    if ((await attempt(() => api.writeFile(project, made, content))) !== undefined) openTab({ kind: 'file', project, path: made });
  };
  const save = () => {
    const link = document.createElement('a');
    link.href = shown(`/api/projects/${encodeURIComponent(project)}/raw?path=${encodeURIComponent(path)}`);
    link.download = name;
    link.click();
  };
  const remove = async () => {
    setBusy(true);
    const gone = await attempt(() => api.removeFile(project, path));
    setBusy(false);
    if (gone === undefined) return;
    setAsking(false);
    // Into the Trash, with Undo, which puts it back at its path.
    saidDeleted(gone as { trash?: string; kept?: number | null }, name);
    // It is known to be gone here and now: what this browser kept of it goes with it, and its tab, here and among the tabs of every other workspace.
    forgetThing(tab);
    forgetTab(tabKey(tab));
    closeTab(tabKey(tab), true);
  };
  const lines = (): Lines => {
    const all: Lines = [];
    if (text && may) all.push({ label: 'Duplicate', mark: signs.copy(), closed: dirty ? 'Save it first: a copy is made of what is saved.' : undefined, act: () => void duplicate() });
    if (onHistory) all.push({ label: 'History', mark: signs.clock(), act: onHistory });
    all.push({ label: of ? 'Export as Markdown' : 'Save a copy', mark: signs.down(), act: save }, { label: 'Copy path', mark: signs.path(), act: () => void copy(path, `Copied ${path}`) });
    if (may) all.push({ label: 'Delete this file…', mark: signs.bin(), look: 'grave', apart: true, act: () => setAsking(true) });
    return all;
  };
  const share = of && marksPublic(of.library) ? () => documentShare(project, of.library, of.record, path, dirty) : undefined;
  return (
    <>
      <PageActs tab={tab} edited={edited} share={share} menu={lines} tool={tool} last />
      {asking && (
        <Modal onClose={() => setAsking(false)}>
          <h2>Move {name} to the Trash?</h2>
          <div class="stack">
            <p style="margin:0">
              <code>{path}</code> leaves the project's folder, for everyone, and is kept in the Trash until it goes for good. It is given back from there, to
              where it was.
              {of && ' What was said on it is kept with it.'}
            </p>
            <div class="row" style="justify-content:flex-end">
              <button type="button" onClick={() => setAsking(false)}>
                Cancel
              </button>
              <button class="danger" type="button" disabled={busy} onClick={() => void remove()}>
                Move to Trash
              </button>
            </div>
          </div>
        </Modal>
      )}
    </>
  );
}
