// A table made from the side panel: New… and then Table on a project's
// line or a folder's. The dialog asks what it is called and how
// it is first laid out, refuses before anything is asked a name the
// project has a table of, and applies `std/table` for the project as
// whoever asked (`tablemade.ts`); the daemon refuses again any name a
// table is there by, which is said in its sentence with the dialog
// staying. Made in a folder, it is then put in that folder: a stop in
// between leaves it at the end of the project's list, where a table
// nobody placed is drawn. It is opened once it is made.
//
// Offered to whoever may write the project and apply (`mayMakeTable`);
// where the project runs under environments, which one is asked, as for
// anything applied for it.

import { useState } from 'preact/hooks';
import { api } from '../api.ts';
import { namesOf } from '../envsheld.ts';
import { outlineFold, projectFold } from '../folds.ts';
import { covers } from '../holds.ts';
import { changeOutline } from '../outlined.ts';
import { collections, me, openTab } from '../store.ts';
import { applied, LAYOUTS, tableNamed } from '../tablemade.ts';
import { Modal, useFocus } from '../ui.tsx';
import { drawnOf, mayArrange, unfolded } from './outline.tsx';
import type { Drawn } from './outline.tsx';

/** Whether whoever reads the page may make a table from the side panel: write the project, and apply what makes it. */
export const mayMakeTable = (): boolean => mayArrange() && covers(me.value?.principal.scopes ?? [], 'blueprints:run');

/** What the daemon said, as a page says it: its own sentence, without the step it failed in and the marks a terminal reads commands by. */
const said = (refused: unknown) =>
  (refused instanceof Error ? refused.message : String(refused)).replace(/^(step \S+ failed: )+/, '').replaceAll('`', '');

/** The dialog that makes a table in `into` (the project's top, where nothing), called `where`. */
export function NewTable({ drawn, into, where, onClose }: { drawn: Drawn; into: string | null; where: string; onClose: () => void }) {
  const project = drawn.project;
  const [typed, setTyped] = useState('');
  const [layout, setLayout] = useState('table');
  const environments = namesOf(project.id);
  const [environment, setEnvironment] = useState('');
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const input = useFocus<HTMLInputElement>();
  const names = (collections.value[project.id] ?? []).map((one) => one.name);
  const named = tableNamed(typed, names, project.name);
  const unchosen = environments.length > 0 && environment === '';
  const closed = typed.trim() === '' || 'wrong' in named || unchosen || busy;
  const make = async (event: Event) => {
    event.preventDefault();
    if (closed || 'wrong' in named) return;
    setBusy(true);
    setWrong(null);
    try {
      const answer = await api.apply(applied(project.id, named, layout, environment || null));
      if (answer.finished && answer.finished.status !== 'ok') throw new Error(answer.finished.error ?? `${named.name} could not be made.`);
      if (into) {
        await changeOutline(project.id, { op: 'put', node: `table:${named.name}`, into }, drawnOf(project).listed);
        unfolded(projectFold(project.id), outlineFold(project.id, into));
      } else {
        unfolded(projectFold(project.id));
      }
      onClose();
      openTab({ kind: 'table', project: project.id, name: named.name });
    } catch (refused) {
      setBusy(false);
      setWrong(said(refused));
    }
  };
  const says =
    typed.trim() === ''
      ? 'Its rows begin with a title; more columns are added from its head.'
      : 'wrong' in named
        ? named.wrong
        : `Blueprints, scripts and its address know it as ${named.name}; more columns are added from its head.`;
  return (
    <Modal onClose={onClose}>
      <h2>New table in {where}</h2>
      <form class="stack rename-form table-made" onSubmit={make}>
        <label>
          Table name
          <input
            ref={input}
            value={typed}
            aria-describedby="table-made-says"
            onInput={(event) => {
              setTyped(event.currentTarget.value);
              setWrong(null);
            }}
          />
        </label>
        <p id="table-made-says" class="faint" style="margin:0;font-size:calc(12px * var(--size))">
          {says}
        </p>
        <fieldset class="table-layouts">
          <legend>Laid out as</legend>
          {LAYOUTS.map((one) => (
            <label key={one.kind}>
              <input type="radio" name="table-layout" value={one.kind} checked={layout === one.kind} onChange={() => setLayout(one.kind)} />
              {one.called}
            </label>
          ))}
        </fieldset>
        {environments.length > 0 && (
          <label>
            Environment
            <select value={environment} onChange={(event) => setEnvironment(event.currentTarget.value)}>
              <option value="">Choose one</option>
              {environments.map((one) => (
                <option key={one} value={one}>
                  {one}
                </option>
              ))}
            </select>
          </label>
        )}
        {wrong && (
          <p role="alert" class="kept-wrong">
            {wrong}
          </p>
        )}
        <div class="row" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" type="submit" disabled={closed}>
            {busy ? 'Making it…' : 'Make it'}
          </button>
        </div>
      </form>
    </Modal>
  );
}
