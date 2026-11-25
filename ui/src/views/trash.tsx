// The Trash, a page of the factory's: what was deleted that whoever looks
// could read, by kind, with who deleted it and when it goes for good;
// given back, or deleted for good behind a question, each offered only to
// whoever the daemon says may. And what deleting anywhere says, with Undo;
// and, in Settings, how long what is deleted is kept.

import { useEffect, useState } from 'preact/hooks';
import { ApiError, type Project } from '../api.ts';
import { attempt, openTab, projects, toast } from '../store.ts';
import { byKind, forget, forgetProject, goesSaid, keepTrash, keptSaid, movedSaid, restore, type Trashed, trashed, trashKept, whereSaid } from '../trash.ts';
import { Empty, Modal, useEvents } from '../ui.tsx';
import { ago } from '../util.ts';
import { useKept } from './returned.tsx';

/** What every page that deletes says once the daemon has answered: moved to the Trash, with Undo, or deleted at once. */
export function saidDeleted(answer: { trash?: string | null; kept?: number | null; with?: number } | undefined, called: string): void {
  if (!answer) return;
  if (!answer.trash) return toast(`${called}: ${movedSaid(0, answer.with)}`);
  const id = answer.trash;
  toast(`${called}: ${movedSaid(answer.kept ?? null, answer.with)}`, false, () => {
    // Where it came back is said where it is not where it was: a page whose page above it is gone.
    void attempt(() => restore(id)).then((back) => back && toast(back.said ?? `${called} is back.`));
  });
}

const projectName = (id: string) => projects.value.find((project) => project.id === id)?.name ?? id;

export function TrashView() {
  // As it was last read, while it is read again.
  const [all, setAll] = useKept<Trashed[] | null>('trash.all', null);
  const [days, setDays] = useState<number | null>(30);
  const [asking, setAsking] = useState<Trashed | null>(null);
  const [beside, setBeside] = useState<{ id: string; said: string } | null>(null);
  const read = () =>
    void trashed()
      .then((read) => {
        setAll(read.trash);
        setDays(read.days);
      })
      .catch(() => setAll([]));
  useEffect(read, []);
  // What goes in, comes out or goes for good, wherever it was done.
  useEvents(['trash.put', 'trash.restored', 'trash.gone'], read, []);
  const give = async (one: Trashed, besideIt = false) => {
    try {
      const back = await restore(one.id, besideIt);
      setBeside(null);
      toast(back.said ?? (one.kind === 'file' ? `${one.called} is back, at ${back.at}.` : `${one.called} is back.`));
      read();
    } catch (err) {
      // Something is at a file's place again: said in the daemon's sentence, with the way that puts it beside.
      if (err instanceof ApiError && err.status === 409 && one.kind === 'file') return setBeside({ id: one.id, said: err.message });
      toast(err instanceof Error ? err.message : String(err), true);
    }
  };
  const gone = async (one: Trashed) => {
    if ((await attempt(() => forget(one.id))) === undefined) return;
    setAsking(null);
    read();
  };
  const now = Date.now();
  return (
    <div class="trash-page">
      <div class="view-head">
        <h1 class="grow">Trash</h1>
      </div>
      <div class="view-body stack" style="gap:20px">
        <p class="muted trash-kept">{keptSaid(days)}</p>
        {all && all.length === 0 && (
          <Empty title="The Trash is empty">
            <p class="muted">What is deleted (a row, a card, a file, a project, a channel) comes here, and can be given back from here.</p>
          </Empty>
        )}
        {all &&
          byKind(all).map((group) => (
            <div key={group.kind} class="panel trash-kind" data-kind={group.kind}>
              <div class="panel-head">
                <h2 class="grow">{group.heading}</h2>
                <span class="muted">{group.things.length}</span>
              </div>
              {group.things.map((one) => (
                <div key={one.id} class="trash-thing" data-trash={one.id} data-thing={one.thing}>
                  <div class="trash-what">
                    <strong class="trash-called">{one.called}</strong>
                    <span class="muted trash-where">{whereSaid(one, projectName)}</span>
                    <span class="faint trash-when">
                      deleted by {one.who || one.by} {ago(one.at, now)} · {goesSaid(one.goes, now)}
                    </span>
                    {beside?.id === one.id && (
                      <span class="trash-taken">
                        <span class="muted">{beside.said}</span>{' '}
                        <button type="button" onClick={() => void give(one, true)}>
                          Restore beside
                        </button>
                      </span>
                    )}
                  </div>
                  {one.may && (
                    <div class="trash-acts">
                      <button type="button" onClick={() => void give(one)}>
                        Restore
                      </button>
                      <button type="button" class="danger" onClick={() => setAsking(one)}>
                        Delete forever
                      </button>
                    </div>
                  )}
                </div>
              ))}
            </div>
          ))}
      </div>
      {asking && (
        <Modal onClose={() => setAsking(null)}>
          <h2>Delete {asking.called} forever?</h2>
          <div class="stack">
            <p style="margin:0">
              {whereSaid(asking, projectName)} goes for good, for everyone, and cannot be given back.
              {asking.kind === 'project' && ' Its files stay where they are on the disk.'}
            </p>
            <div class="row" style="justify-content:flex-end">
              <button type="button" onClick={() => setAsking(null)}>
                Cancel
              </button>
              <button class="danger" type="button" onClick={() => void gone(asking)}>
                Delete forever
              </button>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}

/** Asked before a project is moved to the Trash, from its row's menu: what happens, and for how long it is kept. */
export function TrashProject({ project, onClose }: { project: Project; onClose: () => void }) {
  const [days, setDays] = useState<number | null | undefined>(undefined);
  const [busy, setBusy] = useState(false);
  useEffect(() => void trashed().then((read) => setDays(read.days)).catch(() => setDays(30)), []);
  const away = async () => {
    setBusy(true);
    const gone = await attempt(() => forgetProject(project.id));
    setBusy(false);
    if (gone === undefined) return;
    onClose();
    saidDeleted(gone, project.name);
  };
  const kept = days === undefined ? '' : days === 0 ? ' It is not kept: it cannot be given back.' : days === null ? ' It is kept in the Trash until somebody deletes it for good, and given back from there into its workspace.' : ` It is kept in the Trash for ${days === 1 ? 'a day' : `${days} days`}, and given back from there into its workspace.`;
  return (
    <Modal onClose={onClose}>
      <h2>Move {project.name} to the Trash?</h2>
      <div class="stack">
        <p style="margin:0" class="trash-asked">
          It leaves the factory's lists, for everyone. Nothing it runs is stopped, and its files stay where they are on the disk.{kept}
        </p>
        <div class="row" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="danger" type="button" disabled={busy || days === undefined} onClick={() => void away()}>
            Move to Trash
          </button>
        </div>
      </div>
    </Modal>
  );
}

/** How many days each choice keeps for; `always` keeps until deleted for good, and nought deletes at once. */
const DAYS = [0, 1, 7, 14, 30, 90, 365];

/** In Settings, beside the logs of runs: how long what is deleted is kept. The owner's. */
export function TrashPanel() {
  const [days, setDays] = useState<number | null | undefined>(undefined);
  const read = () => void trashKept().then((kept) => setDays(kept.days)).catch(() => {});
  useEffect(read, []);
  if (days === undefined) return null;
  const set = async (asked: number | 'always') => {
    const said = await attempt(() => keepTrash(asked));
    if (!said) return;
    setDays(said.days);
    toast(said.gone > 0 ? `Said. ${said.gone === 1 ? 'One thing' : `${said.gone} things`} went for good now.` : 'Said.');
  };
  const value = days === null ? 'always' : String(days);
  const choices = days === null || DAYS.includes(days) ? DAYS : [...DAYS, days].sort((a, b) => a - b);
  return (
    <div class="panel trash-setting">
      <div class="panel-head">
        <h2>Trash</h2>
        <span class="muted grow">how long what is deleted is kept before it goes for good</span>
        <button type="button" onClick={() => openTab({ kind: 'trash' })}>
          Open the Trash
        </button>
      </div>
      <div class="stack" style="gap:10px;padding:12px 16px">
        <label class="stack" style="gap:4px">
          <span class="muted">Kept</span>
          <select aria-label="What is deleted is kept" value={value} onChange={(event) => void set(event.currentTarget.value === 'always' ? 'always' : Number(event.currentTarget.value))}>
            <option value="always">Until deleted for good</option>
            {choices.map((one) => (
              <option key={one} value={String(one)}>
                {one === 0 ? 'Not at all: deleted at once' : `For ${one === 1 ? 'a day' : `${one} days`}`}
              </option>
            ))}
          </select>
        </label>
        <span class="muted trash-how-long">{keptSaid(days)}</span>
      </div>
    </div>
  );
}
