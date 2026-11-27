// Sending a card back: where a card that is being looked over is opened,
// a box for what is wrong and one press that puts it where a card that
// did not pass goes. Where that is, is a row of a table that sits beside
// the board and is about its columns (`sentback.ts`); this reads the rows
// and draws the box.

import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Item } from '../api.ts';
import { coalesce } from '../events.ts';
import { asStage, backTo, outOfRounds, sendingBack } from '../sentback.ts';
import { attempt, draftOf, keepDraft } from '../store.ts';
import { RENAMING, renames } from '../table.ts';
import type { Row } from '../table.ts';
import { useEvents } from '../ui.tsx';
import { useKept } from './returned.tsx';

/**
 * The rows that say where a card of each column is sent back to
 * (`sendingBack`), read as any table is and read again when one of them
 * is added, changed or taken away, or a table is declared: `undefined`
 * until they have been read, and nothing where the project has no such
 * table or it may not be read.
 */
export function useStages(project: string): Row[] | null | undefined {
  const [stages, setStages] = useKept<Row[] | null | undefined>(`stages:${project}`, undefined);
  // Which tables they were last read from: what is followed for a row of them changing.
  const from = useRef<string[]>([]);
  const read = useMemo(
    () =>
      coalesce(() => {
        void api
          .collections(project)
          .then(async (all) => {
            const tables = sendingBack(all);
            from.current = tables.map((one) => one.name);
            if (tables.length === 0) return null;
            const rows = await Promise.all(tables.map(async (table) => (await api.records(project, table.name)).map((row) => asStage(table, row))));
            return rows.flat();
          })
          .then(setStages, () => setStages(null));
      }, 50),
    [project],
  );
  useEffect(read, [read]);
  // A table declared may be one that now says so: that is read whichever table it was.
  useEvents(RENAMING, (event) => (event.topic === 'collection.ensured' ? event.data?.project === project : renames(event, project, from.current)) && read(), [project, read]);
  return stages;
}

/**
 * What a card that can be sent back is opened with: a box for what is
 * wrong, and the press that sends it. Nothing, for a card that has
 * nowhere to be sent back to. What is typed is kept in the store until
 * it is sent: the drawer closes on a key, and opens for another card on
 * a click.
 */
export function SendBack({ item, columns, stages, onSent }: { item: Item; columns: string[]; stages: Row[] | null; onSent: () => void }) {
  const to = backTo(item.status, columns, stages);
  const unsent = `send-back:${item.id}`;
  const [wrong, setWrong] = useState(to ? (draftOf(unsent)?.text ?? '') : '');
  if (!to) return null;
  const say = (text: string) => {
    setWrong(text);
    keepDraft(unsent, text.trim() ? { text, base: '' } : null);
  };
  // Sent back as often as its stage allows, sending it back as it is would
  // have it left again as it came back: one more round is offered with it.
  const spent = outOfRounds(item.status, stages, item.more?.rounds, item.given);
  const send = async (give: number) => {
    const note = wrong.trim();
    if (await attempt(() => api.changeItem(item.id, { status: to, ...(note ? { note } : {}), ...(give ? { give } : {}) }))) {
      keepDraft(unsent, null);
      onSent();
    }
  };
  return (
    <div class="send-back">
      <label>
        Not right yet? Say what is wrong
        <textarea
          rows={3}
          value={wrong}
          placeholder="What should be different. Whoever worked on it is told this, and takes it up again."
          onInput={(event) => say(event.currentTarget.value)}
        />
      </label>
      <div class="row">
        <span class="faint grow">
          It goes back to {to}, where it is taken up again.
          {spent && ' It has been sent back as often as its stage allows: given one more round, it is told what it was last sent back with and looked at once more.'}
        </span>
        {spent && (
          <button type="button" onClick={() => send(1)}>
            One more round
          </button>
        )}
        <button type="button" onClick={() => send(0)}>
          Send back
        </button>
      </div>
    </div>
  );
}
