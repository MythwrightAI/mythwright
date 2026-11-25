// The one row of tabs at the top of a record's form, for a card and for a
// row of any table:
// Details, the form itself, with what takes the record away; Comments,
// what was said on it with the box something is said in; History, what
// happened to it; Attachments, with how many; and after those whatever
// tabs whoever draws it hands (a card's tables about cards,
// `cardtabs.tsx`). A card's form and a row's form are this one drawing.
// Which tab shows is kept by the tab the form is drawn beside (`useKept`).

import type { ComponentChildren } from 'preact';
import { useEffect, useRef, useState } from 'preact/hooks';
import type { HistoryLine } from '../api.ts';
import { COMMENTS as ASKED_COMMENTS, tabAsked } from '../cardasked.ts';
import { tabCalled } from '../recordfiles.ts';
import { HappenedPanel, SaidPanel } from './journey.tsx';
import { OfRecord, WordsOf } from './pathlink.tsx';
import { AttachedPanel, useAttached } from './recordfiles.tsx';
import { useKept } from './returned.tsx';

/** The tabs every record has, by what is kept of which shows: a tab handed in is kept by its own word, which none of these is. */
export const DETAILS = '';
export const COMMENTS = ASKED_COMMENTS;
export const HISTORY = '@history';
export const ATTACHMENTS = '@attachments';

/** A tab more than a record always has: what is kept of it showing, what it is called, and what it draws. */
export interface MoreTab {
  which: string;
  label: string;
  /** A table's name, said on the tab for whoever looks for it. */
  table?: string;
  panel: () => ComponentChildren;
}

/**
 * A record's form (`children`) under the one row of its tabs. `lines` is
 * its history, `box` what something is said on it with, and `at` a comment
 * a line of somebody's inbox opened, by its place in the history: Comments
 * is shown at it, marked and brought into view, until another tab is
 * chosen. `kept` is what the tab showing is kept under; `of` what the row
 * of tabs is said to be of, for whoever hears it read out. `project` says
 * which project the words said on it are of, where whoever draws it knows;
 * otherwise it is read from the record's id. `empty` is what History says
 * where nothing has been kept of it.
 */
export function RecordTabs({ record, kept, of, lines, box, at, may = false, project, empty, more = [], class: also, children }: { record: string; kept: string; of: string; lines: HistoryLine[] | null; box?: ComponentChildren; at?: number | null; may?: boolean; project?: string; empty?: string; more?: MoreTab[]; class?: string; children: ComponentChildren }) {
  const [shown, show] = useKept<string>(kept, DETAILS);
  // The comment asked for is shown until another tab is chosen.
  const [held, setHeld] = useState<number | null>(at ?? null);
  useEffect(() => setHeld(at ?? null), [at]);
  const [attached, readAttached] = useAttached(record);
  const chosen = shown === COMMENTS || shown === HISTORY || shown === ATTACHMENTS || more.some((one) => one.which === shown) ? shown : DETAILS;
  const current = held ? COMMENTS : chosen;
  const drawnIn = useRef<HTMLDivElement>(null);
  // A tab asked for from elsewhere in the form: Comments from Details, with the box given the keyboard once it is
  // drawn, where what it waits on is answered; a tab handed in from a question in Comments.
  const asked = tabAsked.value?.card === record ? tabAsked.value.tab : null;
  useEffect(() => {
    if (asked === null) return;
    tabAsked.value = null;
    setHeld(null);
    show(asked);
    if (asked === COMMENTS) requestAnimationFrame(() => drawnIn.current?.querySelector<HTMLTextAreaElement>('.card-say textarea')?.focus());
  }, [asked]);
  // Called by what they are and nothing more: a count that came with the reading would make the same tab two names.
  const tab = (which: string, label: string, table?: string) => (
    <button
      key={which || 'details'}
      type="button"
      role="tab"
      class="view-tab record-tab"
      data-table={table}
      data-tab={which || 'details'}
      aria-selected={current === which}
      onClick={() => {
        setHeld(null);
        show(which);
      }}
    >
      {label}
    </button>
  );
  const said = (panel: ComponentChildren) => (project ? <WordsOf.Provider value={project}>{panel}</WordsOf.Provider> : <OfRecord record={record}>{panel}</OfRecord>);
  const other = more.find((one) => one.which === current);
  return (
    <div class={also ? `record-tabs ${also}` : 'record-tabs'} ref={drawnIn}>
      <div class="view-tabs tabs-ruled">
        <div role="tablist" aria-label={of} class="view-tabs-list">
          {tab(DETAILS, 'Details')}
          {tab(COMMENTS, 'Comments')}
          {tab(HISTORY, 'History')}
          {tab(ATTACHMENTS, tabCalled(attached?.length ?? null))}
          {more.map((one) => tab(one.which, one.label, one.table))}
        </div>
      </div>
      {/* The form stays on the page while another tab shows, so that nothing typed in it is lost. */}
      <div hidden={current !== DETAILS} role="tabpanel" aria-label="Details">
        {children}
      </div>
      {current === COMMENTS && <div class="journey talked">{said(<SaidPanel lines={lines} box={box} at={held} />)}</div>}
      {current === HISTORY && (
        <div class="journey talked">
          <HappenedPanel lines={lines} empty={empty} />
        </div>
      )}
      {current === ATTACHMENTS && <AttachedPanel record={record} list={attached} may={may} read={readAttached} />}
      {other?.panel()}
    </div>
  );
}
