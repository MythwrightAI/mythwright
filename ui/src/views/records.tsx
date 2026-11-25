// A table of a project that is neither its board, a library nor its
// mockups: whatever else a blueprint declared, such as who is in its
// fleet and what they do with a card.
//
// It is the table and nothing more: a row a record, a cell changed where
// it is, a row added by what it is called, and one opened to be read
// whole or taken away (where its rows are written only by what keeps it,
// `byhand.ts`, a row is read and taken away and nothing else); and the same records as a list, a gallery, on a
// calendar or in columns by a choice, where somebody saved such a view. What changing a row does is whoever hears of it
// to say (`record.updated`): this page writes the field and draws what is
// then there.
//
// A table that sits beside another (`sits.ts`) is drawn the same way in
// the drawer, beside the page of the table it sits beside (`aside`): the
// same rows, cells and views, with a way to a tab of its own; and one row
// of any table is opened into its form from wherever it is named.

import { tableLines } from './outline.tsx';
import { Fragment } from 'preact';
import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import { closedWhy, keptSaid, offered } from '../byhand.ts';
import { coalesce } from '../events.ts';
import { banded } from '../groups.ts';
import { rowWords } from '../sits.ts';
import { agents, attempt, collections, draftOf, drawer, keepDraft, openTab, projectName } from '../store.ts';
import { columns as columnsOf, filled, flagColumn, keptIn, knownBy, titleField } from '../table.ts';
import type { Field, Group, Named, Row } from '../table.ts';
import { Empty, useEvents } from '../ui.tsx';
import { saidDeleted } from './trash.tsx';
import { useJourney } from './journey.tsx';
import { RecordTabs } from './recordtabs.tsx';
import { TagBox } from './tagbox.tsx';
import { Laid, ViewsHead, useViews } from './laid.tsx';
import { TableActs } from './tablehead.tsx';
import { Titled, copy } from './pagehead.tsx';
import { signs } from './signs.tsx';
import { rowLink } from '../address.ts';
import { FieldInput } from './table.tsx';
import type { Person } from './table.tsx';
import { opensInto, rowsOf, Within } from './within.tsx';
import { useNamed } from './others.ts';
import { useKept } from './returned.tsx';
import { useSort } from './headmenu.tsx';
import { useRowsRead, WHOLE } from '../rowsread.ts';
import { SMALL } from '../window.ts';
import { goneBy } from '../drawerkept.ts';
import { RowsFind } from './rowsfind.tsx';

/** Shows a table in the drawer, beside whatever page is showing. */
export function openAside(project: string, name: string): void {
  drawer.value = { title: `${projectName(project)} ${name}`, body: () => <RecordsView key={`${project}/${name}`} project={project} name={name} aside /> };
}

/**
 * Opens one row of a table into its form in the drawer, as its own page
 * opens it. Where it was opened from a table shown in the drawer, `back`
 * shows that table again: when the form is left, and from its head.
 */
export function openRow(project: string, name: string, row: Row, back?: () => void, at?: number): void {
  const table = (collections.value[project] ?? []).find((each) => each.name === name);
  const called = titleField(table?.fields ?? []);
  drawer.value = {
    title: String((called && row.data[called.name]) || row.id),
    body: () => <One key={row.id} project={project} name={name} row={row} back={back} at={at} />,
    gone: goneBy(() => api.record(row.id)),
  };
}

/**
 * Shows one row of a table in its form, over its table, as an address
 * that names it asks (`address.ts`). One that is not there, or is not the
 * reader's to read, is said so in the drawer, in the same words.
 */
export function showRow(project: string, name: string, id: string): void {
  openTab({ kind: 'table', project, name });
  const none = () => {
    drawer.value = {
      title: id,
      body: () => (
        <Empty title="There is no such row here">
          {id} is not a row of {name} that you may read: it was taken away, or was never there.
        </Empty>
      ),
    };
  };
  void api.record(id).then((row) => (row.id === id ? openRow(project, name, row) : none()), none);
}

/** Copies the address that opens a row's form over its table (`address.ts`), from the form's head. */
function RowLink({ project, name, id }: { project: string; name: string; id: string }) {
  return (
    <button type="button" class="ghost icon row-link" aria-label="Copy link" title="Copy an address that opens this row" onClick={() => void copy(rowLink(project, name, id), `Copied. It opens ${id} for whoever is signed in to this factory here and may read it.`)}>
      {signs.link()}
    </button>
  );
}

/**
 * One row's form, of the table as it is now, for wherever it is opened
 * from. `closed` says why nothing of it is the reader's to change, and
 * `closedFor` why one field is not, where whoever draws it says so (an
 * agent's setup, `agentedit.tsx`).
 */
export function One({ project, name, row, back, closed, closedFor, at }: { project: string; name: string; row: Row; back?: () => void; closed?: string | null; closedFor?: (field: Field) => string | null; at?: number }) {
  const table = (collections.value[project] ?? []).find((each) => each.name === name);
  const others = useNamed(project, table?.fields ?? [], name);
  // A field that names a record of this table is chosen from its own rows, as they were last read.
  const rows = rowsOf(project, name).rows.value;
  if (!table) return <p class="faint">There is no such table here any more.</p>;
  const people: Person[] = agents.value.map((slot) => ({ value: `agent:${slot.name}`, name: slot.name }));
  const named: Named = { ...others, [name]: { fields: table.fields, rows: rows ?? [row] } };
  return (
    <>
      <div class="row-acts">
        {back && (
          <button type="button" class="ghost link sits-back" onClick={back}>
            ‹ All {rowWords(table).many}
          </button>
        )}
        {/* A document and a mockup have addresses of their own, as files; a card is its board's. */}
        {table.store !== 'documents' && table.store !== 'mockups' && name !== 'board' && <RowLink project={project} name={name} id={row.id} />}
      </div>
      <Whole row={row} fields={columnsOf(table.fields)} groups={table.groups} people={people} named={named} what={rowWords(table).one} onDone={back ?? (() => (drawer.value = null))} closed={closed} closedFor={closedFor} kept={closedWhy(table)} at={at} />
    </>
  );
}

/**
 * Escape, while a table is shown in the drawer, is first for what is
 * open in the table: a cell being filled in, or what a button of its head
 * opened (its layout, a menu, the menu of a column's head, the icons of a column). The frame puts the
 * drawer away on Escape whatever was meant by it, so the key is kept from
 * the frame while anything is open, wherever the caret is, and what was
 * opened is told as a press outside it would tell it. With nothing open,
 * Escape is the drawer's, as it always was.
 *
 * Whether anything is open is asked as the key comes down (`opens`, at
 * the window, before anything else hears it): asked on the way back up, a
 * cell that had just closed itself was no longer there. Where the caret
 * is in the table the key goes on to whatever has it, a cell's own way of
 * being given up, and is stopped as it leaves the table (`firstHere`);
 * where the caret is elsewhere it goes no further.
 */
const OPEN = '.cell-edit, .menu, .layout-menu';
const taken = new WeakSet<Event>();
function closeOpen(): void {
  document.dispatchEvent(new Event('pointerdown', { bubbles: true }));
}
function opens(table: HTMLElement | null, event: KeyboardEvent): void {
  if (event.key !== 'Escape' || !table || !(table.querySelector(OPEN) || document.querySelector('.iconpick, .headmenu'))) return;
  if (table.contains(event.target as Node)) return void taken.add(event);
  event.stopPropagation();
  closeOpen();
}
function firstHere(event: KeyboardEvent): void {
  if (!taken.has(event)) return;
  event.stopPropagation();
  closeOpen();
}

export function RecordsView({ project, name, aside }: { project: string; name: string; aside?: boolean }) {
  const table = (collections.value[project] ?? []).find((each) => each.name === name);
  // `null` while they are first being read.
  // What was read is shown at once when its tab shows again, and read again behind.
  const [rows, setRows] = useKept<Row[] | null>(`rows:${project}/${name}`, null);
  const [adding, setAdding] = useState(false);
  // Its own look is a table; whoever looks at it saves other views beside that.
  const views = useViews(project, table, name, 'table');
  // Laid out as a table it is read a window at a time, sorted, held to what the view is held to and counted by the
  // daemon (`rowsread.ts`), whatever it holds; the reader's sort is sent with each reading and kept in their browser.
  const [sort, setSort] = useSort(views.heads.at);
  const keptView = (table?.views ?? []).some((one) => one.name === views.view.name);
  const says = { only: views.view.only ?? null, group: views.view.group ?? null, figures: views.view.figures ?? null };
  // What is searched for among its rows, kept by the tab: the daemon looks for it row by row (`rowsfind.tsx`).
  const [find, setFind] = useKept<string>('rows.find', '');
  const reading = useRowsRead(project, name, { view: keptView ? views.view.name : null, sort, says, find }, table !== undefined);
  const counted = reading.counted;
  // Every row is read as well only while the table is small enough (`WHOLE`), for what still goes by all of them: the
  // other layouts, what a view can be held to, a CSV written out, a row naming another of the same table. Past that
  // nothing reads it whole.
  const whole = counted !== null && counted.of <= WHOLE;
  const reload = useMemo(() => coalesce(() => void api.records(project, name).then(setRows, () => setRows([])), 80), [project, name]);
  useEffect(() => {
    if (whole) reload();
    else if (counted !== null) setRows(null);
  }, [reload, whole]);
  // Read once, and again when a record of it is added, changed or taken
  // away by anyone, or the table itself is declared again.
  useEvents(
    ['record.created', 'record.updated', 'record.removed', 'collection.ensured', 'collection.fields'],
    (event) => whole && event.data?.project === project && (event.data?.collection ?? event.data?.name) === name && reload(),
    [project, name, reload, whole],
  );

  // What its records name, where a field names records of another table.
  const others = useNamed(project, table?.fields ?? [], name);

  // In the drawer, Escape is first for what is open in the table (`opens`).
  const shown = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!aside) return;
    const first = (event: KeyboardEvent) => opens(shown.current, event);
    window.addEventListener('keydown', first, true);
    return () => window.removeEventListener('keydown', first, true);
  }, [aside]);

  if (!table) {
    return (
      <Empty title="There is no such table here">
        A project has a table once a blueprint declares it: <code>std/fleet</code> declares <code>agents</code> and <code>stages</code>.
      </Empty>
    );
  }
  const fields = columnsOf(table.fields);
  const people: Person[] = agents.value.map((slot) => ({ value: `agent:${slot.name}`, name: slot.name }));
  // What one of its rows is called and what several are: in the words it is called for people where it is (`A/B attempts`).
  const words = rowWords(table);
  const what = words.one;
  const where = keptIn(table);
  // A table whose rows are written only by what keeps it is read here: nothing adds a row or changes one (`byhand.ts`).
  const hand = offered(table);
  const kept = keptSaid(table);
  const change = async (row: Row, field: Field, value: unknown) => {
    // The reader's own change is drawn where the row is as the daemon says it, and is counted as nothing that changed under them.
    reading.mine(row.id);
    await attempt(() => api.setRecord(row.id, { [field.name]: value }));
    if (whole) reload();
  };
  const called = titleField(table.fields);
  // A field that names a record of this table is chosen from its own rows.
  const named: Named = { ...others, [name]: { fields: table.fields, rows: rows ?? [] } };
  const open = (row: Row) => {
    // From the drawer, its form takes the table's place there, and gives it back.
    if (aside) return openRow(project, name, row, () => openAside(project, name));
    drawer.value = {
      title: knownBy(row, table.fields),
      body: () => (
        <>
          <div class="row-acts">
            <RowLink project={project} name={name} id={row.id} />
          </div>
          <Whole
            key={row.id}
            row={row}
            fields={fields}
            groups={table.groups}
            people={people}
            named={named}
            what={what}
            kept={closedWhy(table)}
            onDone={() => {
              drawer.value = null;
              reading.mine(row.id);
              if (whole) reload();
            }}
          />
        </>
      ),
      gone: goneBy(() => api.record(row.id)),
    };
  };
  return (
    <div class={aside ? 'split sits-table' : 'split'} ref={shown} onKeyDown={aside ? firstHere : undefined}>
      <div class="view-head">
        <Titled>
          {/* In the drawer its name is the drawer's own head. */}
          {!aside && (
            <h1>
              {projectName(project)} {table.title_chosen || table.title || name}
            </h1>
          )}
          {(counted || rows) && (
            <span class="muted">
              {(counted?.of ?? rows!.length).toLocaleString()} {(counted?.of ?? rows!.length) === 1 ? words.one : words.many}
            </span>
          )}
          {where && (
            <span class="faint" title={where.more}>
              {where.words}
            </span>
          )}
          {kept && (
            <span class="faint table-kept" title={kept.more}>
              {kept.words}
            </span>
          )}
        </Titled>
        <span class="grow" />
        {!aside && (
          <TableActs
            tab={{ kind: 'table', project, name }}
            table={table}
            rows={rows}
            what={what}
            menu={() => tableLines(project, name)}
            // Written out and read in against every record as it is then, read as it is asked for: the rows drawn are a window of them.
            every={counted ? { read: () => api.records(project, name), count: counted.of, again: () => reading.again() } : undefined}
          />
        )}
        {aside && (
          <button type="button" title="This table in a tab of its own, as wide as the page" onClick={() => {
              // What was in the drawer goes to a tab of its own: it is put away, and kept for no tab to have back.
              drawer.value = null;
              openTab({ kind: 'table', project, name });
            }}
          >
            Open as a tab
          </button>
        )}
        {hand.adds && (
          <button type="button" class="primary" onClick={() => setAdding(true)}>
            Add {/^[aeiou]/.test(what) ? 'an' : 'a'} {what}
          </button>
        )}
        {views.layout === 'table' && <RowsFind find={find} onFind={setFind} reading={reading} fields={fields} onOpen={open} />}
        <ViewsHead views={views} rows={rows} counted={counted} what={what} aside={aside} />
        {/* What it is for, as its blueprint says, on a line of its own under the rest of its head. */}
        {table.description && <p class="table-said">{table.description}</p>}
      </div>
      {views.layout !== 'table' && counted !== null && !whole ? (
        // Laid out another way it would read every row: past `WHOLE` it is read a window at a time, as a table alone, until the other layouts are.
        <div class="empty">
          This table has {counted.of.toLocaleString()} {words.many}: laid out as a {views.layout} it would read every one of them. Lay this view out as a table, which reads them a window at a time.
        </div>
      ) : (views.layout === 'table' ? counted === null && rows === null : rows === null) ? (
        <div class="empty">Reading the table</div>
      ) : (
        <Laid
          views={views}
          fields={fields}
          groups={table.groups}
          rows={rows ?? []}
          reads={{ reading, sort, setSort, large: (counted?.of ?? 0) > SMALL }}
          people={people}
          named={named}
          what={what}
          adding={hand.adds && adding}
          byHand={hand.changes}
          onOpen={open}
          within={opensInto(project, name) ? (row) => <Within project={project} name={name} id={row.id} /> : undefined}
          onChange={hand.changes ? change : undefined}
          onAdd={
            hand.adds
              ? async (title) => {
                  setAdding(false);
                  if (title) await attempt(() => api.addRecord(project, name, { [called?.name ?? 'title']: title }));
                  if (whole) reload();
                  reading.again();
                }
              : undefined
          }
        />
      )}
    </div>
  );
}

/**
 * One record read whole: each field where it can be changed, and the
 * record taken away. What is changed is written when it is saved, as one
 * change and not a letter at a time: something may be following this
 * table, and would follow every letter. What is shown once it is saved
 * or taken away is whoever opened it to say (`onDone`).
 */
function Whole(props: { row: Row; fields: Field[]; groups?: Group[] | null; people: Person[]; named: Named; what: string; onDone: () => void; closed?: string | null; closedFor?: (field: Field) => string | null; kept?: string | null; at?: number }) {
  const { row, fields, groups, people, named, what, onDone, closed, closedFor } = props;
  // Where its table's rows are written only by what keeps it (`byhand.ts`), every field is read and none is changed
  // here, and it is still taken away by whoever may: that is tidying, not writing.
  const kept = props.kept ?? null;
  // What is changed in the form is kept by the tab it was opened beside, with the drawer: given back, it is as it was left.
  const [changed, setChanged] = useKept<Record<string, unknown>>(`row.changed:${row.id}`, {});
  // What has happened to it and what was said on it, where its table keeps
  // that; and somewhere to say more, which whatever follows the table hears.
  // What is being said is kept in the store until it is sent: the drawer
  // closes on a key, and the page is put away when another tab is shown.
  const lines = useJourney(row.id);
  const unsaid = `say:${row.id}`;
  const [saying, setSaying] = useState(draftOf(unsaid)?.text ?? '');
  const write = (text: string) => {
    setSaying(text);
    keepDraft(unsaid, text.trim() ? { text, base: '' } : null);
  };
  const say = async () => {
    const words = saying.trim();
    if (words && (await attempt(() => api.say(row.id, words)))) write('');
  };
  const dirty = Object.keys(changed).length > 0;
  const save = async () => {
    // Kept as written if it was refused, with why said, to be put right.
    if ((await attempt(() => api.setRecord(row.id, changed))) === undefined) return;
    onDone();
  };
  const box = (
    <div class="card-say">
      <TagBox rows={2} value={saying} label={`Say something on this ${what}`} placeholder={`Say something on this ${what}. Name somebody with @ and they are told in their inbox.`} onInput={write} />
      <div class="row">
        <span class="grow" />
        <button type="button" disabled={!saying.trim()} onClick={() => void say()}>
          Say
        </button>
      </div>
    </div>
  );
  return (
    // One row of tabs at the top, as a card's form has them: Details, which is the form and what takes the row away;
    // what was said on it, with the box to say more, apart from what happened to it; and what is attached to it.
    <RecordTabs record={row.id} kept={`row-tab:${row.id}`} of={`What is kept of this ${what}`} lines={lines} box={box} at={props.at} may={!closed} empty="Nothing has been kept of it yet.">
      <div class="form" style={flagColumn(fields) ? { '--flag-column': flagColumn(fields) } : undefined}>
        {/* What its table says is kept on a record for whatever looks after it is nobody's to fill in. Where the
            table says its columns in groups, the form has a section under each heading, in the same order. */}
        {banded(filled(fields), groups).map((band) => (
          <Fragment key={band.name ?? ''}>
            {band.name && <h3 class="form-band">{band.name}</h3>}
            {band.fields.map((field) => {
              const why = closed ?? kept ?? closedFor?.(field) ?? null;
              const input = (
                <FieldInput
                  key={field.name}
                  field={field}
                  value={field.name in changed ? changed[field.name] : row.data[field.name]}
                  people={people}
                  others={named}
                  self={row.id}
                  onChange={(next) => setChanged({ ...changed, [field.name]: next })}
                />
              );
              // Closed, it is shown as it is and says why where it is pointed at.
              return why ? (
                <fieldset key={field.name} class="field-closed" disabled title={why}>
                  {input}
                </fieldset>
              ) : (
                input
              );
            })}
          </Fragment>
        ))}
        <p class="faint">{row.id}</p>
        {kept && !closed && <p class="faint row-closed">{kept}</p>}
        {closed ? (
          <p class="faint row-closed">{closed}</p>
        ) : (
          <div class="row">
            <button
              type="button"
              class="danger"
              onClick={async () => {
                const gone = await attempt(() => api.removeRecord(row.id));
                if (gone === undefined) return;
                // Into the Trash, with Undo, which gives it back where it was.
                saidDeleted(gone as { trash?: string; kept?: number | null }, knownBy(row, fields) || `This ${what}`);
                onDone();
              }}
            >
              Remove this {what}
            </button>
            <span class="grow" />
            {!kept && (
              <button type="button" class="primary" disabled={!dirty} onClick={() => void save()}>
                Save
              </button>
            )}
          </div>
        )}
      </div>
    </RecordTabs>
  );
}
