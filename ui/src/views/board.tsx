// A project's board: the work, as cards in columns.
//
// Underneath, a board is a table: records with fields, which a blueprint
// declared. This page is that table, laid out as whoever is looking chose:
// as the board, or as a table, a list or a gallery of the same cards
// (`table.tsx` draws those from the fields, knowing nothing of boards).
//
// Moving a card is an event like any other, and blueprints can act on it:
// the standard fleet has a card worked when it reaches `ready` and its
// turn has come.

import type { ComponentChildren } from 'preact';
import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Board, Collection, Item } from '../api.ts';
import { coalesce } from '../events.ts';
import { listed, steps } from '../columns.ts';
import { boardLines } from '../gear.ts';
import type { Tool } from '../gear.ts';
import { banded as bandedBy } from '../groups.ts';
import { InTurn } from '../inturn.ts';
import type { Item as Option } from '../pills.ts';
import { Select } from '../select.tsx';
import { agents, attempt, drawer, hints, openTab, projectName, takeHint } from '../store.ts';
import { groupable, holdable, shown } from '../ordered.ts';
import { changed, columns as columnsOf, goesBy, keptIn, layouts as layoutsOf, offered, leavingOut, leftOut, leftOutAtFirst, nameFor } from '../table.ts';
import type { Change, Layout, Row, View } from '../table.ts';
import { Empty, Modal, useEvents, useFocus } from '../ui.tsx';
import { classes } from '../util.ts';
import { CardMarks, hasMarks } from './cardmarks.tsx';
import { Held } from './held.tsx';
import { Bands, OrderLayout } from './ordered.tsx';
import { useStages } from './sendback.tsx';
import { among } from '../cardactions.ts';
import { PRESSES, Waiting, taken } from './cardactions.tsx';
import { startLines } from './starts.tsx';
import { TableActs } from './tablehead.tsx';
import { useBeside } from './sits.tsx';
import { ColumnSetUp } from './columnset.tsx';
import { HungLayout, LayoutMark, ViewTabs } from './layouts.tsx';
import { note, pressOutside, Titled } from './pagehead.tsx';
import type { Lines as MenuLines } from './pagehead.tsx';
import { signs } from './signs.tsx';
import { CalendarLayout, GalleryLayout, ListLayout } from './table.tsx';
import { TableLayout } from './windowed.tsx';
import { useKept } from './returned.tsx';
import { aboutOf, CardForm, rowOf, useOthers } from './cardform.tsx';
import type { About } from './cardform.tsx';
import { goneBy } from '../drawerkept.ts';
import { showView, viewShown } from '../viewshown.ts';
import { arrangementOf } from '../slid.ts';
import { useCardSlide } from './cardslide.ts';
import { headWord, ThingWas } from './thingwas.tsx';

/** A choice of the reader's that is kept in this browser. */
/** How a board is looked at where nothing was saved of it: as a board, by where its cards are. */
const FIRST: View = { name: 'board', kind: 'board', by: 'status' };

function kept(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}
function keep(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* not remembered */
  }
}

export function BoardView({ project }: { project: string }) {
  // What was read is shown at once when its tab shows again, and read again behind.
  const [board, setBoard] = useKept<Board | null>('board', null);
  const [missing, setMissing] = useState('');
  // A project has no board until a blueprint makes one.
  const [none, setNone] = useState(false);
  // Where its cards are kept, which the blueprint that made it chose.
  const [store, setStore] = useKept<string>('board.store', '');
  const [dragging, setDragging] = useState<string | null>(null);
  const [over, setOver] = useState<{ column: string; before: string | null } | null>(null);
  const [adding, setAdding] = useState<string | null>(null);
  // Changing the columns: they are the blueprint's, so it is applied again.
  // `add` is for one more of them; `arrange` for the ones there are.
  const [shaping, setShaping] = useState<'add' | 'arrange' | null>(null);
  // How the view is laid out, hung under the gear while its line of the gear's menu has opened it.
  const [laying, setLaying] = useState(false);
  // The table the board is, as its blueprint declared it.
  const [table, setTable] = useKept<Collection | null>('board.table', null);
  // The same, as it was last read or a change to its views last left it,
  // which may be a moment ahead of what is drawn: a change to a view is
  // worked out from this, in its turn (`inturn.ts`).
  const views = useRef(new InTurn<Collection | null>(table)).current;
  // The views of it are kept with it, for everyone. Which of them this
  // reader is looking at is theirs, and is kept here; until they choose, it
  // is the first, which for a board is the board.
  // An address opened at a view of the board shows it here too (`viewshown.ts`).
  const viewing = viewShown(project, 'board');
  const looking = useRef(viewing);
  const seen = useRef(viewing);
  if (seen.current !== viewing) looking.current = seen.current = viewing;
  const [lines, setLines] = useState(() => kept('client.table.lines') !== 'no');
  const [wrap, setWrap] = useState(() => kept('client.table.wrap') === 'yes');

  const reload: () => void = useMemo(
    () =>
      coalesce(() => {
        const asked = views.asked();
        void api
          .boardIfAny(project)
          .then((found) => {
            setNone(found === null);
            setMissing('');
            if (found) {
              setBoard(found.board);
              setStore(found.store);
              // A reading that a change to the views overtook says how
              // they were before it: it is not kept, and is taken again.
              if (views.read(found.table, asked)) setTable(found.table);
              else reload();
            }
          })
          .catch((err) => setMissing(err instanceof Error ? err.message : String(err)));
      }, 50),
    [project],
  );
  useEffect(reload, [reload]);
  // A card changed, or the board itself was made or changed by a blueprint.
  useEvents(['board.**', 'collection.ensured', 'collection.views', 'collection.icon', 'collection.fields'], (event) => event.data?.project === project && !PRESSES.includes(event.topic) && reload(), [project]);
  // What is pressed on a card is taken in from what is said of it, with nothing read again.
  useEvents(PRESSES, (event) => event.data?.project === project && setBoard((was) => was && { ...was, items: was.items.map((item) => (item.id === event.data.id ? taken(item, event.topic, event.data, event.ts) : item)) }), [project]);

  // What the cards name, where the board's blueprint gave them a field that names a record of another collection (`cardform.tsx`).
  const others = useOthers(project, table?.fields ?? [], 'board.others');
  // What the project's fleet does with the cards of each column, where it has one: a card is sent back where its stage says.
  const stages = useStages(project);
  // What sits beside the board: a column's head says the rows that are about the column.
  const sat = useBeside(project, 'board');

  // Arriving from search, or from a line of the inbox: open the card that was named, on a board that is showing already too.
  // A line of somebody's inbox opens it at the comment that named them (`at`, its place in the card's history).
  const named = hints.value[`board:${project}`];
  useEffect(() => {
    const hint = board && stages !== undefined ? takeHint(`board:${project}`) : undefined;
    const item = hint?.item && board?.items.find((entry) => entry.id === hint.item);
    if (item && board) openCard(item, aboutOf(board, table?.fields ?? [], others, stages ?? null, among(sat)), reload, hint?.at);
    // Asked at a column (the Tools panel's list said how many more it has): that column in view.
    if (hint?.column) document.querySelector(`.board > .column[data-column="${CSS.escape(hint.column)}"]`)?.scrollIntoView({ block: 'nearest', inline: 'start' });
  }, [board === null || stages === undefined, named]);

  // A card moved to another column, by whoever moved it, slides there (`cardslide.ts`); not while one is carried, nor the one let go by hand.
  const slid = useRef<HTMLDivElement>(null);
  const byHand = useRef<string | null>(null);
  const arrangement = board ? arrangementOf(board.columns.map((column) => [column, board.items.filter((item) => item.status === column).sort((a, b) => a.position - b.position).map((item) => item.id)])) : '';
  useCardSlide(slid, arrangement, dragging !== null, byHand);

  if (none) {
    return (
      <Empty title={`${projectName(project)} has no board yet`}>
        <span>
          A board is made by a blueprint, which says what its columns are and where its cards are kept. Open it, read it, and
          apply it.
        </span>
        <button class="primary" onClick={() => openTab({ kind: 'blueprint', name: 'std/board', project })}>
          Make one with a blueprint
        </button>
      </Empty>
    );
  }
  if (missing) return <Empty title="This board cannot be shown">{missing}</Empty>;
  if (!board || !table || stages === undefined) return <div class="empty">Reading the board</div>;

  // The views that are offered: an order is not, for now (`offered`).
  const tabbed = offered(table.views);
  const ways = layoutsOf(table.fields, table.views);
  const view = tabbed.find((one) => one.name === viewing) ?? tabbed[0] ?? FIRST;
  // What the view being looked at leaves out is not a column of it; a card is still opened to the whole of it.
  const hides = leftOut(view, table.fields);
  const fields = columnsOf(table.fields).filter((field) => !hides.includes(field.name));
  // A list and a gallery say a card's fields side by side in the order the view's columns stand in; a table works its own out.
  const inOrder = bandedBy(fields, table.groups, view.columns).flatMap((band) => band.fields);
  const layout: Layout['kind'] = (ways.find((way) => way.kind === view.kind) ?? ways[0]!).kind;
  // A calendar goes by a day the cards are given where the board keeps one, and can then move them.
  const day = goesBy('calendar', table.fields, view.by) ?? 'created';
  const look = (name: string) => {
    looking.current = seen.current = name;
    setAdding(null);
    showView(project, 'board', name);
  };
  // A view is saved with the table: a new one, or the one being looked at, laid out or called something else.
  // What it leaves out goes with it when it is laid out another way or called something else. Each change
  // is made in its turn, from the views as the change before it left them and not as they are drawn here:
  // two columns switched in the time one save takes were each sent without the other.
  const viewsOf = (now: Collection): View[] => (now.views.length > 0 ? now.views : [FIRST]);
  const save = (change: Change) =>
    views.change(async (now) => {
      if (!now) return null;
      const next = changed(now.fields, viewsOf(now), looking.current, change);
      const saved = await attempt(() => api.saveView(project, 'board', next));
      if (saved) {
        setTable(saved);
        look(next.name);
      }
      return saved;
    });
  const remove = () =>
    void views.change(async (now) => {
      if (!now) return null;
      const gone = viewsOf(now).find((one) => one.name === looking.current) ?? viewsOf(now)[0]!;
      const left = await attempt(() => api.removeView(project, 'board', gone.name));
      if (left) {
        setTable(left);
        look(left.views[0]?.name ?? 'board');
      }
      return left;
    });
  const where = keptIn({ ...table, store }, 'card');
  // Every card, and those this view shows: all of them, unless it is held to some.
  const every = board.items.map(rowOf);
  const rows = shown(every, view.only, table.fields);
  const items = rows.length === every.length ? board.items : board.items.filter((item) => rows.some((row) => row.id === item.id));
  const people = agents.value.map((slot) => ({ value: `agent:${slot.name}`, name: slot.name }));
  const about = aboutOf(board, table.fields, others, stages, among(sat));
  const open = (row: Row) => {
    const item = board.items.find((one) => one.id === row.id);
    if (item) openCard(item, about, reload);
  };
  const laid = { fields, groups: table.groups, rows, people, named: { ...others, board: { fields: table.fields, rows: every } }, what: 'card', onOpen: open };
  // Where a board or an order gathers its cards in bands: by which field, and where what is folded is kept.
  const banded = layout === 'board' || layout === 'order' ? table.fields.find((field) => field.name === view.group) : undefined;
  const at = `${project}.board.${view.name}`;

  // A card added where there is no column to add it in goes to the board's first.
  const addFirst = async (title: string | null) => {
    setAdding(null);
    if (title) await attempt(() => api.addItem(project, { title, status: board.columns[0] }));
  };
  const drop = async (column: string, before: string | null) => {
    const id = dragging;
    setDragging(null);
    setOver(null);
    if (!id || before === id) return;
    byHand.current = id;
    // Show the move at once; the event that follows confirms it.
    setBoard({ ...board, items: moved(board.items, id, column, before) });
    await attempt(() => api.changeItem(id, { status: column, before: before ?? undefined }));
    reload();
  };

  // What can be done to a board is lines of its gear (`gear.ts`), each with a drawing of what it does. How the view is laid
  // out is one of them: it puts away whatever else is open over the page, and hangs where the menu was.
  const does: Record<Tool, () => void> = {
    card: () => setAdding(board.columns[0] ?? null),
    column: () => setShaping('add'),
    arrange: () => setShaping('arrange'),
    layout: () => {
      pressOutside();
      setLaying(true);
    },
  };
  const drawn: Record<Tool, () => ComponentChildren> = { card: signs.plus, column: signs.plus, arrange: signs.pencil, layout: () => <LayoutMark kind={layout} /> };
  const tools = (): MenuLines =>
    boardLines(board.columns, ways.find((way) => way.kind === layout)?.name).flatMap((line, at) => [{ label: line.label, hint: line.hint, mark: drawn[line.does](), apart: at === 0, act: does[line.does] }, ...(line.note ? [note(line.note)] : [])]);

  return (
    <div class="split" ref={slid}>
      <div class="view-head">
        <Titled>
          <h1>
            {projectName(project)} {headWord(project, 'board', 'board')}
          </h1>
          <ThingWas project={project} thing="board" />
          <span class="muted">{items.length < board.items.length ? `${items.length} of ` : ''}{board.items.length === 1 ? '1 card' : `${board.items.length} cards`}</span>
          {where && (
            <span class="faint" title={where.more}>
              {where.words}
            </span>
          )}
        </Titled>
        <span class="grow" />
        <TableActs
          tab={{ kind: 'board', project }}
          table={table}
          rows={every}
          what="card"
          menu={() => startLines(project, 'board')}
          tools={tools}
          last
          hung={(gear) =>
            laying && (
              <HungLayout
                from={gear}
                onClose={() => setLaying(false)}
                layouts={ways}
                layout={layout}
                name={view.name}
                onLayout={(kind) => save((view) => ({ name: view.name, kind }))}
                onRename={(name) => save((view) => ({ name, kind: view.kind, was: view.name }))}
                onRemove={tabbed.length > 1 ? remove : undefined}
                options={[
                  ['Show vertical lines', lines, (on) => (setLines(on), keep('client.table.lines', on ? 'yes' : 'no'))],
                  ['Wrap all content', wrap, (on) => (setWrap(on), keep('client.table.wrap', on ? 'yes' : 'no'))],
                ]}
                columns={listed({ fields: columnsOf(table.fields), groups: table.groups, order: view.columns }, hides)}
                onMove={(name, by) => save(steps(table.groups, name, by, true))}
                onColumn={(name, shown) => save((view, fields) => ({ name: view.name, kind: view.kind, hides: leavingOut(view, fields, name, shown) }))}
                groups={groupable(table.fields, 'board', 'status', banded?.name, every)}
                holdings={holdable(table.fields, layout === 'board' ? 'status' : null, every, view.only)}
                only={view.only}
                onOnly={(only) => save((view) => ({ name: view.name, kind: view.kind, only }))}
                group={banded?.name ?? null}
                onGroup={(name) => save((view) => ({ name: view.name, kind: view.kind, group: name }))}
              />
            )
          }
        />
        {/* A new view leaves out what the board says is left out to begin with, whatever the one being looked at shows. */}
        <ViewTabs
          views={tabbed}
          current={view.name}
          ways={ways}
          onSelect={look}
          onAdd={(kind) => save((_, fields, all) => ({ name: nameFor(kind, all.map((one) => one.name)), kind, hides: leftOutAtFirst(fields) }))}
        />
        <Held only={view.only} fields={table.fields} what="card" shown={items.length} of={board.items.length} onClear={() => save((view) => ({ name: view.name, kind: view.kind, only: null }))} />
      </div>
      {layout === 'calendar' && (
        <CalendarLayout
          {...laid}
          by={day}
          onMove={
            day === 'due'
              ? async (row, to) => {
                  await attempt(() => api.changeItem(row.id, { due: to }));
                  reload();
                }
              : undefined
          }
        />
      )}
      {layout === 'table' && (
        <TableLayout
          {...laid}
          of={{ project, name: 'board', then: reload }}
          heads={{ view, fields: columnsOf(table.fields), groups: table.groups, name: 'board', owner: table.owner, at, rows: every, named: laid.named, change: save }}
          lines={lines}
          wrap={wrap}
          adding={adding !== null}
          onChange={async (row, field, value) => {
            await attempt(() => api.changeItem(row.id, { [field.name]: value }));
            reload();
          }}
          onAdd={addFirst}
        />
      )}
      {(layout === 'list' || layout === 'gallery') && (
        <div class="rows-page">
          {adding !== null && <NewCard onDone={addFirst} />}
          {layout === 'list' ? <ListLayout {...laid} fields={inOrder} /> : <GalleryLayout {...laid} fields={inOrder} />}
        </div>
      )}
      {layout === 'order' && (
        <OrderLayout fields={table.fields} rows={rows} every={every} named={laid.named} by={view.by} doneBy={view.done ?? null} group={banded?.name ?? null} at={at} what="card" onOpen={open} above={adding !== null && <NewCard onDone={addFirst} />} marks={(row) => <CardMarks item={board.items.find((item) => item.id === row.id)!} />} />
      )}
      {layout === 'board' && <Bands rows={rows} by={banded} named={laid.named} at={at} each={(only, first) => <div class="board">
        {board.columns.map((column) => {
          const cards = items.filter((item) => item.status === column && (!only || only.some((row) => row.id === item.id))).sort((a, b) => a.position - b.position);
          return (
            <section
              key={column}
              data-column={column}
              class={classes('column', over?.column === column && 'over')}
              onDragOver={(event) => {
                if (!dragging) return;
                event.preventDefault();
                const before = cardBelow(event);
                if (over?.column !== column || over.before !== before) setOver({ column, before });
              }}
              onDragLeave={(event) => {
                if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setOver(null);
              }}
              onDrop={(event) => {
                event.preventDefault();
                void drop(column, cardBelow(event));
              }}
            >
              <div class="column-head">
                <span class="grow column-name">
                  <span>{column}</span>
                  <ColumnSetUp project={project} sat={sat} column={column} columns={board.columns} />
                </span>
                <span class="faint">{cards.length}</span>
                <button class="ghost icon" aria-label={`Add a card to ${column}`} onClick={() => setAdding(column)}>
                  +
                </button>
              </div>
              <div class="cards">
                {adding === column && first && (
                  <NewCard
                    onDone={async (title) => {
                      setAdding(null);
                      if (title) await attempt(() => api.addItem(project, { title, status: column }));
                    }}
                  />
                )}
                {cards.map((item) => (
                  <article
                    key={item.id}
                    class={classes('card', dragging === item.id && 'dragging')}
                    data-card={item.id}
                    data-slides={item.id}
                    draggable
                    tabIndex={0}
                    style={over?.column === column && over.before === item.id ? 'box-shadow:0 -2px 0 var(--accent)' : ''}
                    onDragStart={(event) => {
                      event.dataTransfer?.setData('text/plain', item.id);
                      if (event.dataTransfer) event.dataTransfer.effectAllowed = 'move';
                      setDragging(item.id);
                    }}
                    onDragEnd={() => {
                      setDragging(null);
                      setOver(null);
                    }}
                    onClick={() => openCard(item, about, reload)}
                    onKeyDown={(event) => event.key === 'Enter' && openCard(item, about, reload)}
                  >
                    <span class="id">{item.id}</span>
                    <span class="card-name">{item.title}</span>
                    <Waiting item={item} actions={about.actions} />
                    {hasMarks(item) && (
                      <span class="row">
                        <CardMarks item={item} />
                      </span>
                    )}
                  </article>
                ))}
              </div>
            </section>
          );
        })}
      </div>} />}
      {shaping && <Columns project={project} board={board} adding={shaping === 'add'} onClose={() => setShaping(null)} />}
    </div>
  );
}

/**
 * Changing a board's columns. They are arranged by the board itself
 * (`POST /board/<project>/columns`), which keeps them as the order
 * somebody chose, with any column added here: the board's blueprint
 * applied again, or a new version of it, keeps them where they are and
 * puts only a column of its own that is new after the one it follows.
 * Applied again with other `columns` the board follows the blueprint
 * as before; arranging says the whole of the board's columns.
 */
function Columns({ project, board, adding, onClose }: { project: string; board: Board; adding: boolean; onClose: () => void }) {
  const [columns, setColumns] = useState<Option[]>(board.columns);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  // The instance that holds the board, once it is known; `null` for a board from before boards were made by blueprints.
  const [holder, setHolder] = useState<string | null | undefined>(undefined);
  useEffect(() => {
    void api.collections(project).then(
      (all) => setHolder(all.find((one) => one.name === 'board')?.owner ?? null),
      () => setHolder(null),
    );
  }, [project]);
  const same = columns.length === board.columns.length && columns.every((column, index) => String(column) === board.columns[index]);

  const apply = async () => {
    setBusy(true);
    setError('');
    try {
      await api.boardColumns(project, columns.map(String));
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal onClose={onClose}>
      <h2>The board's columns</h2>
      <div class="field-of-pills">
        <span>Left to right. Type to add one; drag one by its grip to move it.</span>
        <Select label="columns" chosen={board.columns} many makes ordered focused={adding} onChange={setColumns} />
        <span class="faint">They are the board's: applying its blueprint again keeps them where they are. A column with cards still in it cannot be taken away.</span>
      </div>
      {error && <p class="error-text">{error}</p>}
      <div class="row">
        {holder && (
          <button
            type="button"
            class="ghost link"
            onClick={() => {
              onClose();
              openTab({ kind: 'instance', id: holder });
            }}
          >
            Its blueprint and history
          </button>
        )}
        <span class="grow" />
        <button type="button" onClick={onClose}>
          Cancel
        </button>
        <button class="primary" disabled={busy || same || columns.length === 0} onClick={() => void apply()}>
          {busy ? 'Applying' : 'Apply'}
        </button>
      </div>
    </Modal>
  );
}

/** The card the pointer is above, so a dropped card lands before it. */
function cardBelow(event: DragEvent): string | null {
  const column = event.currentTarget as HTMLElement;
  for (const card of column.querySelectorAll<HTMLElement>('[data-card]')) {
    const box = card.getBoundingClientRect();
    if (event.clientY < box.top + box.height / 2) return card.dataset.card ?? null;
  }
  return null;
}

/** The items as they will be once `id` is in `column`, before `before`. */
export function moved(items: Item[], id: string, column: string, before: string | null): Item[] {
  const others = items.filter((item) => item.status === column && item.id !== id).sort((a, b) => a.position - b.position);
  const index = before ? others.findIndex((item) => item.id === before) : -1;
  const at = index < 0 ? others.length : index;
  const above = others[at - 1]?.position;
  const below = others[at]?.position;
  const position = above === undefined ? (below ?? 1) - 1 : below === undefined ? above + 1 : (above + below) / 2;
  return items.map((item) => (item.id === id ? { ...item, status: column, position } : item));
}

function NewCard({ onDone }: { onDone: (title: string) => void }) {
  const [title, setTitle] = useState('');
  const input = useFocus<HTMLInputElement>();
  // Pressing Enter also takes the cursor out of the field as it goes away;
  // whichever comes first is the answer, and there is only one.
  const answered = useRef(false);
  const done = (text: string) => {
    if (answered.current) return;
    answered.current = true;
    onDone(text);
  };
  return (
    <form
      onSubmit={(event) => {
        event.preventDefault();
        done(title.trim());
      }}
    >
      <input
        ref={input}
        value={title}
        placeholder="What needs doing? Enter to add"
        onInput={(event) => setTitle(event.currentTarget.value)}
        onBlur={() => done(title.trim())}
        onKeyDown={(event) => event.key === 'Escape' && done('')}
      />
    </form>
  );
}

function openCard(item: Item, about: About, changed: () => void, at?: number): void {
  // Each card's form is its own: one opened while another is showing does not take over what was typed for that one.
  drawer.value = { title: item.id, body: () => <CardForm key={item.id} item={item} about={about} at={at} onChanged={changed} onDone={() => (drawer.value = null)} />, gone: goneBy(() => api.record(item.id)) };
}
