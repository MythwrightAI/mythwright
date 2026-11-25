// What a table read a window at a time is searched by, and gone about by:
// a box that searches its rows for words, which the
// daemon looks for row by row and counts as it goes (`find` of
// `rowsread.ts`), and one that goes to a row by its id or its number,
// brought into view and marked, or said to be kept out of the view and
// by what, with the way to open it all the same.
//
// Ctrl and F with the keyboard in the table puts the caret in the search,
// and Ctrl and G with the table showing in the box that goes to a row.

import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { coalesce } from '../events.ts';
import type { Read } from '../rowsapi.ts';
import { lookedSaid, outSaid } from '../rowslive.ts';
import type { Reading } from '../rowsread.ts';
import type { Field, Row } from '../table.ts';
import { useKept } from './returned.tsx';

/** How long after the last letter typed a search is asked. */
const TYPED = 250;

export function RowsFind(props: { find: string; onFind: (find: string) => void; reading: Reading; fields: Field[]; onOpen: (row: Row) => void }) {
  const { reading } = props;
  const [typed, setTyped] = useKept<string>('rows.find.typed', props.find);
  const [going, setGoing] = useKept<string>('rows.goto.typed', '');
  const [said, setSaid] = useState<{ words: string; row?: Read } | null>(null);
  const box = useRef<HTMLInputElement>(null);
  const go = useRef<HTMLInputElement>(null);
  // Asked a moment after the letters typed, with what is typed then.
  const latest = useRef({ typed, onFind: props.onFind });
  latest.current = { typed, onFind: props.onFind };
  const asked = useMemo(() => coalesce(() => latest.current.onFind(latest.current.typed.trim()), TYPED), []);
  // Ctrl and G with the table showing, and Ctrl and F with the keyboard in it.
  useEffect(() => {
    const heard = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
      const key = event.key.toLowerCase();
      const page = box.current?.closest('.split');
      if (key === 'g') {
        event.preventDefault();
        go.current?.focus();
        go.current?.select();
      } else if (key === 'f' && !event.shiftKey && page?.contains(document.activeElement)) {
        event.preventDefault();
        box.current?.focus();
        box.current?.select();
      }
    };
    window.addEventListener('keydown', heard);
    return () => window.removeEventListener('keydown', heard);
  }, []);
  const goTo = async () => {
    const to = going.trim();
    if (!to) return;
    setSaid(null);
    const gone = await reading.goTo(to);
    if ('missing' in gone) setSaid({ words: gone.missing });
    else if (!gone.in) setSaid({ words: outSaid(gone.out, props.fields), row: gone.row ?? undefined });
  };
  const looked = lookedSaid(reading.counted, props.find);
  return (
    <div class="rows-find">
      <input
        ref={box}
        type="search"
        class="rows-find-box"
        aria-label="Search this table"
        placeholder="Search"
        value={typed}
        onInput={(event) => {
          setTyped(event.currentTarget.value);
          latest.current.typed = event.currentTarget.value;
          asked();
        }}
        onKeyDown={(event) => {
          if (event.key === 'Escape' && typed) {
            event.stopPropagation();
            setTyped('');
            props.onFind('');
          }
        }}
      />
      <input
        ref={go}
        class="rows-find-go"
        aria-label="Go to a row, by its id or its number"
        placeholder="Go to row"
        value={going}
        onInput={(event) => {
          setGoing(event.currentTarget.value);
          setSaid(null);
        }}
        onKeyDown={(event) => {
          if (event.key === 'Enter') void goTo();
        }}
      />
      {looked && (
        <span class="rows-find-said faint" data-looked="">
          {looked}
        </span>
      )}
      {said && (
        <span class="rows-find-said faint" role="status">
          {said.words}
          {said.row && (
            <button type="button" class="link" onClick={() => props.onOpen(said.row!)}>
              Open it
            </button>
          )}
        </span>
      )}
    </div>
  );
}
