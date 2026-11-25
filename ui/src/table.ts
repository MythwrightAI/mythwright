// A table: records with fields, and the ways it is laid out.
//
// What a project keeps (its board's cards, and whatever else a blueprint
// declares) is a table underneath: a row a record, a column a field. A
// board, a list and a gallery are the same rows laid out another way, and
// which is looked at is the reader's choice, not a different thing. This
// is the part of that with no page in it: which columns there are and in
// what order, which layouts a table can be given, and the order of rows.

/** A field as its collection declares it. */
export interface Field {
  name: string;
  /** `text`, `choice`, `actor`, `list`, `number`, `flag`, `date`, `record` or `records`. */
  kind: string;
  /** For a choice: what can be chosen, in order. For a record, or records: the one collection whose records it names. */
  of: string[];
  required: boolean;
  /** Its table says it is left out of a view until somebody shows it there. It is no more private for that. */
  hidden?: boolean;
  /** Its table says a change of it alone is no news of the record: what is kept on a record beside what happened to it. */
  quiet?: boolean;
  /** The words its table offers whoever fills it, for a text that takes any word. */
  suggests?: string[];
  /** Its table says its words are a time of day with the zone it is in after it: a time written with none is given the reader's own (`zoned.ts`). */
  zoned?: boolean;
  /** The icon its column is drawn with, by name: what its table declared, and what somebody chose, which stands over that (`icons.ts`). */
  icon?: string | null;
  icon_chosen?: string | null;
  /** Its table says it is a list of scopes that a record gives somebody (`takes: each`), and what such a list stands for where a record leaves it unsaid (`scopes.ts`). */
  takes_each?: boolean;
  /** Of such a list, what it stands for; of a choice, the one word nothing chosen stands for (a card's priority, `normal`: `nothingSaid`). */
  unsaid?: string[];
  /** What giving it a value takes beside what changing the record does, and what giving it one of its values takes, as its table says (`added.ts` says both to whoever asks): one scope as a word, several, every one of which is asked, as a list. */
  takes?: string | string[] | null;
  takes_of?: Record<string, string | string[]>;
  /** Somebody who looks at the table added it: it is the table's and no blueprint's (`added.ts`). */
  added?: boolean;
  /** What somebody called its column, which stands over its name where the column is headed and nowhere else. */
  label_chosen?: string | null;
  /** Where it is among the fields a record is called by, from 1, as its table says (`knownBy`). */
  calls?: number | null;
}

/** What a table says of how it is shown, as its blueprint declared it (`sits.ts`). */
export interface Sits {
  /** The table of the same project it sits beside. */
  beside?: string | null;
  /** The field of its own that says what of that table a row is about. */
  about?: string | null;
  /** Its columns in groups: headings in the order they are drawn, each with the fields under it in order (`groups.ts`). */
  groups?: Group[] | null;
  /** `false` where its rows are written only by the commands and blueprints that keep it (`byhand.ts`). */
  by_hand?: boolean | null;
  /** What it is called for people, as its blueprint declared it, and what somebody called it, which stands over that (`called.ts`). */
  title?: string | null;
  title_chosen?: string | null;
  /** What it is for, in a sentence, said under its head. */
  description?: string | null;
}

/** What the daemon says of a table beside what it holds: when the table itself was last written, and what reading or changing it takes beyond the usual. */
export interface Guarded {
  updated?: number;
  reads?: string | null;
  takes?: string | null;
  /** The table of the same project whose rows its own steer: a row here is written by whoever could write each of those. */
  steers?: string | null;
}

/** A heading, and the fields drawn under it. */
export interface Group {
  name: string;
  fields: string[];
}

/** The words a field offers whoever fills it: those its table says, for a text; none for anything else. */
export function offeredBy(field: Field): string[] {
  return field.kind === 'text' ? (field.suggests ?? []) : [];
}

/**
 * What a list field offers to be chosen: what its table declared, in its order, then every other word the
 * rows already hold in it, alphabetically. A card's labels are picked from those already on its board's
 * cards, not typed again for each.
 */
export function inUse(fields: Field[], name: string, rows: Row[]): string[] {
  const declared = fields.find((field) => field.name === name)?.of ?? [];
  const known = new Set(declared);
  const held = new Set<string>();
  for (const row of rows) {
    const value = row.data[name];
    for (const one of Array.isArray(value) ? value : []) {
      const word = String(one).trim();
      if (word !== '' && !known.has(word)) held.add(word);
    }
  }
  return [...declared, ...[...held].sort((a, b) => a.localeCompare(b, undefined, { sensitivity: 'base' }) || (a < b ? -1 : a > b ? 1 : 0))];
}

/** A record, whatever it is a record of. */
export interface Row {
  id: string;
  number: number;
  data: Record<string, unknown>;
  position: number;
  created: number;
  updated: number;
}

export interface Layout {
  kind: 'table' | 'board' | 'list' | 'gallery' | 'calendar' | 'order';
  name: string;
  says: string;
}

/**
 * Where a table is kept, said in a few words at the head of its page, with
 * a sentence for whoever points at them. Nothing is said of one kept in
 * the installation's own database: that is where anything is, unless its
 * blueprint chose otherwise.
 */
export function keptIn(table: { name: string; store: string; at?: string | null }, what = 'record'): { words: string; more: string } | null {
  if (table.store === 'files') {
    return { words: `kept as files in ${table.name}/`, more: `Each ${what} is a file in the project's ${table.name} folder` };
  }
  if (table.store === 'service') {
    const at = table.at ?? '';
    return {
      words: `kept in a database at ${at.replace(/^https?:\/\//, '') || 'an address that is not known'}`,
      more: `The ${what}s are kept by a store service: a database that a blueprint made and hosts${at ? `, at ${at}` : ''}`,
    };
  }
  return null;
}

/** Every way a table can be laid out, in the order they are offered. */
export const LAYOUTS: Layout[] = [
  { kind: 'table', name: 'Table', says: 'A row a record, a column a field.' },
  { kind: 'board', name: 'Board', says: 'In columns, by one of their choices.' },
  { kind: 'list', name: 'List', says: 'A line a record.' },
  { kind: 'gallery', name: 'Gallery', says: 'A card a record.' },
  { kind: 'calendar', name: 'Calendar', says: 'On the days one of their dates says.' },
  { kind: 'order', name: 'Order', says: 'What can be begun now, and what waits on what.' },
];

/** The field a record is known by: the one called `title`, else its first text. */
export function titleField(fields: Field[]): Field | undefined {
  return fields.find((field) => field.name === 'title') ?? fields.find((field) => field.kind === 'text');
}

// What is glanced at comes before what is read: a choice, who, what it
// names, the labels, a day, a number, a tick, and long text last.
const ORDER = ['choice', 'actor', 'record', 'records', 'list', 'date', 'number', 'flag', 'text'];

/**
 * The columns of a table: what a record is known by first, then the rest
 * by kind. A blueprint's fields are a map, which has no order of its own.
 */
export function columns(fields: Field[]): Field[] {
  const title = titleField(fields);
  const rank = (field: Field) => (ORDER.indexOf(field.kind) + ORDER.length + 1) % (ORDER.length + 1);
  const rest = fields.filter((field) => field !== title).sort((a, b) => rank(a) - rank(b) || a.name.localeCompare(b.name));
  return title ? [title, ...rest] : rest;
}

/**
 * The layouts a table with these fields can be given, always in the one
 * order they are offered in: where one stands is nothing to do with which
 * a view is laid out as now, so the one chosen is found where it was. (The
 * ways a table's views were laid out came first once, and choosing another
 * for a view moved it to the front under the pointer.) A board needs a
 * choice to make columns of, and an order a field in which a record says
 * which others come before it: one that names records of some table, or
 * the one a view of the table already goes by. The rest can be drawn of
 * anything.
 */
export function layouts(fields: Field[], views: { kind: string }[] = [], order: boolean = orderOffered()): Layout[] {
  const ordered = order && (views.some((view) => view.kind === 'order') || fields.some((field) => field.kind === 'records'));
  return LAYOUTS.filter((layout) => (layout.kind !== 'board' || fields.some((field) => field.kind === 'choice')) && (layout.kind !== 'order' || ordered));
}

/**
 * Whether a table's records by what comes first are offered: the order's
 * tab, and Order among the ways a view is laid out. They are not, for now.
 * On a board whose cards name nothing they wait on, which is every board
 * until its work is broken down into cards that do, the order is every
 * card side by side, and says less than the board beside it. The view is
 * still declared, kept and drawn as it was, so that it comes back by this
 * one answer once cards are grouped and broken down; a browser told so
 * (`client.order` is `on`) is offered it meanwhile, which is how the check
 * of it goes on holding it.
 */
export function orderOffered(): boolean {
  try {
    return typeof localStorage !== 'undefined' && localStorage.getItem('client.order') === 'on';
  } catch {
    return false;
  }
}

/** The views of a table that are offered: all of them, but for an order while that is not ([`orderOffered`]). */
export function offered<Of extends { kind: string }>(views: Of[], order: boolean = orderOffered()): Of[] {
  return order ? views : views.filter((view) => view.kind !== 'order');
}

/** A card's own fields, which its form has a place for whatever its board is. */
export const CARD = ['title', 'body', 'status', 'assignee', 'labels', 'due'];
/**
 * The field what is asked on the card and not yet answered is kept in, by
 * what looks after the board's cards. It is shown as the question it is, where it will
 * be seen, and not as a field among the others in the card's form.
 */
export const QUESTION = 'question';
/**
 * The field what the card waits for is kept in, while it waits its turn
 * where its fleet works a limited number of cards at once. The card says it; nobody writes it, so its form has no place
 * for it.
 */
export const WAITING = 'waiting';
/**
 * The field how many times the card was sent back from review is kept in.
 * It is counted for the card; nobody writes it.
 */
export const ROUNDS = 'rounds';

/**
 * What the blueprints that look after a card keep on it for themselves:
 * who asked what is asked on it, the column it was taken from to be
 * worked, and what its agent is next to be told. Nobody writes them, and a
 * card's form has no place for them.
 */
export const KEPT = ['asker', 'taken', 'again'];

/**
 * The field a card's plan is kept in, where a fleet has somebody say how a
 * card is to be done before anybody does it. Whoever planned it writes it;
 * it is read on the card, and has no place in the card's form.
 */
export const PLAN = 'plan';

/** How a card is to be done, as whoever planned it said; nothing for one that was not planned. */
export function planned(more: Record<string, unknown> | undefined): string {
  const plan = more?.[PLAN];
  return typeof plan === 'string' ? plan.trim() : '';
}

/**
 * Whether a field is one that whatever looks after the records keeps
 * there for itself, as its table says it: left out of a view until
 * somebody shows it (`hidden`), and no news of the record when it changes
 * (`quiet`). How a card came to be where it is, which its board writes
 * with each move, is one. Nobody fills such a field in, so a form has no
 * place for it: what it says happened is in the record's history.
 */
export function keptForItself(field: Field): boolean {
  return field.hidden === true && field.quiet === true;
}

/** The fields of a record that a form of it has a place for: all but what is kept there for whatever looks after it. */
export function filled(fields: Field[]): Field[] {
  return fields.filter((field) => !keptForItself(field));
}

/**
 * What a board keeps of a card beyond a card's own, in the order a table
 * shows them: the fields its blueprint declared besides, each of which the
 * card's form has a place for, by its kind. What its table says is kept
 * for whatever looks after the cards has none, whatever it is called; the
 * names are for a board declared before a table could say so.
 */
export function extras(fields: Field[]): Field[] {
  const itsOwn = [QUESTION, WAITING, ROUNDS, PLAN, ...KEPT];
  return filled(columns(fields)).filter((field) => !CARD.includes(field.name) && !itsOwn.includes(field.name));
}

/** How many times a card was sent back from review, as the card says it; none for one that never was. */
export function sentBack(more: Record<string, unknown> | undefined): number {
  const rounds = more?.[ROUNDS];
  return typeof rounds === 'number' && rounds > 0 ? rounds : 0;
}

/** What a card waits its turn for, as the card says it; nothing for one that is not waiting its turn. */
export function waitsFor(more: Record<string, unknown> | undefined): string {
  const waits = more?.[WAITING];
  return typeof waits === 'string' ? waits : '';
}

/** What is asked on a card and not yet answered, as the card says it; nothing for one that is not waiting. */
export function asks(more: Record<string, unknown> | undefined): string {
  const asked = more?.[QUESTION];
  return typeof asked === 'string' ? asked : '';
}

/** What a column is headed by: what somebody called it, else its field's name as it reads. */
export function heading(field: Partial<Field> & { name: string }): string {
  return field.label_chosen || field.name.charAt(0).toUpperCase() + field.name.slice(1).replace(/[-_]/g, ' ');
}

/** The widest a form's column of flag names is, in letters: a name longer than that wraps in it, and the switches stay near what they name. */
export const FLAG_COLUMN_MOST = 24;

/**
 * How wide the column of flag names is in a record's form, so that every
 * flag's switch stands at one place, a little past the longest of their
 * names, and never at the form's far edge: in letters of the
 * names' own size (`ch`), one more than the longest, at most
 * `FLAG_COLUMN_MOST`. Nothing where the form has no flag.
 */
export function flagColumn(fields: Pick<Field, 'name' | 'kind' | 'label_chosen'>[]): string | null {
  const longest = Math.max(0, ...fields.filter((field) => field.kind === 'flag').map((field) => heading(field).length));
  return longest === 0 ? null : `${Math.min(longest + 1, FLAG_COLUMN_MOST)}ch`;
}

/** What choosing nothing of a choice is called where it is offered: the word nothing stands for, where its table says one (`unsaid`). */
export function nothingSaid(field: Pick<Field, 'kind' | 'unsaid'>): string {
  const usual = field.kind === 'choice' ? field.unsaid?.[0] : undefined;
  return usual ? `nothing (${usual})` : 'nothing';
}

/** What is in a cell, in words: for sorting, and where nothing better can be drawn. */
export function said(value: unknown): string {
  if (value === null || value === undefined) return '';
  if (Array.isArray(value)) return value.join(', ');
  if (typeof value === 'boolean') return value ? 'yes' : '';
  return String(value);
}

/**
 * What `record` fields can name, by the collection each names records of:
 * that collection's fields, and the records it has. One that has not been
 * read yet is not here.
 */
export type Named = Record<string, { fields: Field[]; rows: Row[] }>;

/**
 * What a record is known by where it is named: what the fields its table
 * says it is called by hold (`calls`, in their order: an attempt of a
 * card's A/B test is `shop-7 · quick`), else its `title`, else the first
 * text it must have, else its id.
 */
export function knownBy(row: Row, fields: Field[]): string {
  const calling = fields.filter((one) => (one.calls ?? 0) > 0).sort((a, b) => a.calls! - b.calls!);
  const called = calling.map((one) => said(row.data[one.name]).trim()).filter(Boolean);
  if (called.length > 0) return called.join(' · ');
  const field = fields.find((one) => one.name === 'title') ?? fields.find((one) => one.kind === 'text' && one.required);
  return (field && said(row.data[field.name]).trim()) || row.id;
}

/**
 * What a `record` field holds, as it is read: what the record it names is
 * known by, or its id and that it is `gone` when there is no such record
 * any more. `null` when it names nothing. Until what it can name has been
 * read, it is its id and nothing is said to be gone.
 */
export function named(field: Field, value: unknown, others: Named): { id: string; words: string; gone: boolean } | null {
  const id = said(value);
  if (id === '') return null;
  const of = others[field.of[0] ?? ''];
  if (!of) return { id, words: id, gone: false };
  const row = of.rows.find((one) => one.id === id);
  return row ? { id, words: knownBy(row, of.fields), gone: false } : { id, words: id, gone: true };
}

/** Whether a field names records of another table: one (`record`) or several (`records`). */
export function names(field: Field): boolean {
  return field.kind === 'record' || field.kind === 'records';
}

/** The ids a field that names records holds: the one, or each of the several. */
export function idsOf(value: unknown): string[] {
  if (Array.isArray(value)) return value.map(String).filter((id) => id !== '');
  const id = said(value);
  return id === '' ? [] : [id];
}

/** What a field that names several records holds, each as [`named`] says one. */
export function namedAll(field: Field, value: unknown, others: Named): { id: string; words: string; gone: boolean }[] {
  return idsOf(value).flatMap((id) => named(field, id, others) ?? []);
}

/**
 * What can be chosen for a field that names several, each under words no
 * other is under: two records called the same are told apart by their ids.
 */
export function choosable(field: Field, value: unknown, others: Named, self?: string): { id: string; name: string }[] {
  const now = namedAll(field, value, others).filter((one) => one.gone);
  const all = [...now.map((one) => ({ value: one.id, name: `${one.id}, which is no longer there` })), ...nameable(field, null, others, self)];
  const times = new Map<string, number>();
  for (const one of all) times.set(one.name, (times.get(one.name) ?? 0) + 1);
  return all.map((one) => ({ id: one.value, name: (times.get(one.name) ?? 0) > 1 ? `${one.name} (${one.value})` : one.name }));
}

/**
 * What a `record` field of the record `self` can be changed to: every
 * other record of the collection it names, in the order they were made;
 * and before them the one it names now, when that one is gone, so that
 * leaving the choice as it is does not change it.
 */
export function nameable(field: Field, value: unknown, others: Named, self?: string): { value: string; name: string }[] {
  const of = others[field.of[0] ?? ''];
  const rows = [...(of?.rows ?? [])].sort((a, b) => a.number - b.number).filter((row) => row.id !== self);
  const now = named(field, value, others);
  const kept = now && !rows.some((row) => row.id === now.id) ? [{ value: now.id, name: of ? `${now.id}, which is no longer there` : now.id }] : [];
  return [...kept, ...rows.map((row) => ({ value: row.id, name: knownBy(row, of!.fields) }))];
}

/** A table whose records name records of another: which, and by which of its fields. */
export interface Naming {
  table: string;
  field: string;
}

/**
 * The tables whose records name records of the table `name`: what a row of
 * it can be opened into. A quarter names its year, so a year is opened
 * into its quarters; nothing is said anywhere but in the field that names.
 * A table that names itself is left out: its rows are all there already.
 */
export function namedBy(all: { name: string; fields: Field[] }[], name: string): Naming[] {
  return all
    .filter((table) => table.name !== name)
    .flatMap((table) => table.fields.filter((field) => names(field) && field.of[0] === name).map((field) => ({ table: table.name, field: field.name })))
    .sort((a, b) => a.table.localeCompare(b.table) || a.field.localeCompare(b.field));
}

/** The rows that name the record `id` by their field `field`, alone or among others, in the order they were made. */
export function naming(rows: Row[], field: string, id: string): Row[] {
  return rows.filter((row) => idsOf(row.data[field]).includes(id)).sort((a, b) => a.position - b.position || a.number - b.number || a.id.localeCompare(b.id));
}

/** What is heard that may change what the records other records name are known by. */
export const RENAMING = ['record.created', 'record.updated', 'record.removed', 'collection.ensured', 'project.file.changed', 'project.rescanned'];

/**
 * Whether something that happened may have changed the records of one of
 * a project's collections `names`: one of them added, changed or taken
 * away; one of the collections declared again; or a file written where
 * one of them is kept as files. A record of some other collection, or a
 * file elsewhere in the project, is nothing these name.
 */
export function renames(event: { topic: string; data?: Record<string, unknown> }, project: string, names: string[]): boolean {
  if (event.data?.project !== project) return false;
  if (event.topic.startsWith('record.')) return names.includes(String(event.data?.collection ?? ''));
  if (event.topic !== 'project.file.changed') return true;
  const path = String(event.data?.path ?? '');
  return names.some((name) => path.startsWith(`${name}/`));
}

export interface Sort {
  field: string;
  down: boolean;
}

/** One press on a column's heading: up, then down, then as they were. */
export function nextSort(now: Sort | null, field: string): Sort | null {
  if (now?.field !== field) return { field, down: false };
  return now.down ? null : { field, down: true };
}

/**
 * The rows in order: by a field when one is asked for, a choice in the
 * order of what can be chosen and anything else by what it says, with the
 * empty ones last either way; otherwise in the order they were added.
 */
export function sorted(rows: Row[], fields: Field[], sort: Sort | null): Row[] {
  const field = sort && fields.find((one) => one.name === sort.field);
  if (!sort || !field) return [...rows].sort((a, b) => a.number - b.number);
  const key = (row: Row): [number, number | string] => {
    const value = row.data[field.name];
    const words = said(value);
    if (words === '') return [1, ''];
    if (field.kind === 'choice') return [0, field.of.indexOf(words)];
    if (field.kind === 'number') return [0, Number(value)];
    return [0, words.toLowerCase()];
  };
  return [...rows].sort((a, b) => {
    const [emptyA, valueA] = key(a);
    const [emptyB, valueB] = key(b);
    if (emptyA !== emptyB) return emptyA - emptyB;
    const order = typeof valueA === 'number' && typeof valueB === 'number' ? valueA - valueB : String(valueA).localeCompare(String(valueB));
    return (sort.down ? -order : order) || a.number - b.number;
  });
}

/** A view of a table as it is kept with it: a way of laying it out, under a name. */
export interface View {
  name: string;
  kind: string;
  by: string | null;
  saved?: boolean;
  /** The fields it leaves out, by name, where it says which. */
  hides?: string[] | null;
  /** The field its records are gathered by, a band for each thing it names: for a board and an order. */
  group?: string | null;
  /** For an order: the choice whose last value says a record is done. */
  done?: string | null;
  /** What it is held to: for each field named, the things a record is to hold one of there to be shown. */
  only?: Record<string, string[]> | null;
  /** The order its columns stand in, by name: one it does not name stands after those it names, as columns always stood (`groups.ts`). */
  columns?: string[] | null;
  /** The column it is frozen up to: that one and those before it stay while the rest are scrolled sideways (`columns.ts`). */
  freezes?: string | null;
  /** The figure at the foot of a column: for each field named, which (`figures.ts`). */
  figures?: Record<string, string> | null;
  /** How wide a column is: for each field named, whole pixels at the usual size of text (`widths.ts`). */
  widths?: Record<string, number> | null;
}

/** The fields a table says are left out of a view to begin with: what a new view of it leaves out. */
export function leftOutAtFirst(fields: Field[]): string[] {
  const known = titleField(fields);
  return fields.filter((field) => field.hidden && field !== known).map((field) => field.name);
}

/**
 * The fields a view leaves out, by name. It says which, or it says
 * nothing: then it leaves out what the table's fields say is left out to
 * begin with, whether it is the blueprint's own view or one somebody
 * saved before a view could say. Nobody who saved such a view chose to
 * see what a table has since come to keep for itself; one who wants it
 * turns it on, and the view then says so. What a record is known by is
 * never left out: it is what a row is opened by. Nothing here goes by
 * what a field is called.
 */
export function leftOut(view: View, fields: Field[]): string[] {
  const known = titleField(fields);
  const said = view.hides ?? leftOutAtFirst(fields);
  return said.filter((name) => name !== known?.name && fields.some((field) => field.name === name));
}

/** The fields a view shows, in the order they were given. */
export function shownIn(view: View, fields: Field[]): Field[] {
  const out = leftOut(view, fields);
  return fields.filter((field) => !out.includes(field.name));
}

/** What a view leaves out once `name` is shown in it, or is not: what is saved when a column is turned on or off. */
export function leavingOut(view: View, fields: Field[], name: string, shown: boolean): string[] {
  const out = leftOut(view, fields).filter((one) => one !== name);
  return shown ? out : [...out, name];
}

/** A view's name as it is written on its tab. */
export function titled(name: string): string {
  return name.charAt(0).toUpperCase() + name.slice(1);
}

/** What a new view of this kind is called: the kind, with a number where that is taken. */
export function nameFor(kind: Layout['kind'], taken: string[]): string {
  const base = LAYOUTS.find((layout) => layout.kind === kind)?.name ?? titled(kind);
  const has = (name: string) => taken.some((one) => one.toLowerCase() === name.toLowerCase());
  for (let nth = 1; ; nth++) {
    const name = nth === 1 ? base : `${base} ${nth}`;
    if (!has(name)) return name;
  }
}

/** What a view of this kind goes by, for a table with these fields: a board by a choice, a calendar by a day. */
export function goesBy(kind: string, fields: Field[], asked?: string | null): string | null {
  if (kind === 'board') return fields.find((field) => field.kind === 'choice' && field.name === asked)?.name ?? fields.find((field) => field.kind === 'choice')?.name ?? null;
  if (kind !== 'calendar') return null;
  if (asked === 'created' || asked === 'updated') return asked;
  // A day its records are given, before the day they happened to be made.
  return fields.find((field) => field.kind === 'date' && field.name === asked)?.name ?? fields.find((field) => field.kind === 'date')?.name ?? 'created';
}

/**
 * What a view of this kind is saved with beside its name: what it goes
 * by, what its records are gathered by (a board, an order, and a table,
 * whose rows are then under a line for each thing), for an order which
 * choice says one is done, and what it says of its columns: the order
 * they stand in (a table, a list and a gallery, which draw a record's
 * fields side by side), which it is frozen up to, the figures at their
 * feet and how wide each is (a table alone). Each is carried over from the view being
 * changed where the kind still has a use for it; an order that says
 * nothing of its own takes what another order of the table goes by.
 */
export function goes(kind: string, fields: Field[], view: View, all: View[] = []): Carried {
  const has = (name: string | null | undefined, ...kinds: string[]) => fields.find((field) => field.name === name && kinds.includes(field.kind))?.name ?? null;
  const banded = kind === 'board' || kind === 'order' || kind === 'table';
  const group = banded ? has(view.group, 'record', 'records', 'choice', 'actor', 'text', 'list') : null;
  // What it is held to goes with it however it is laid out, less what names a field that is gone.
  const held = Object.entries(view.only ?? {}).filter(([name, any]) => any.length > 0 && has(name, 'record', 'records', 'choice', 'actor', 'text', 'list'));
  const only = held.length > 0 ? Object.fromEntries(held) : null;
  // The order of its columns goes where a record's fields are drawn side by side in that order: a table, a list and a gallery.
  const there = (name: string) => fields.some((field) => field.name === name);
  const order = ['table', 'list', 'gallery'].includes(kind) ? (view.columns ?? []).filter(there) : [];
  // What it is frozen up to, the figures at its columns' feet and how wide each is are a table's alone.
  const figures = Object.entries((kind === 'table' && view.figures) || {}).filter(([name]) => there(name));
  const widths = Object.entries((kind === 'table' && view.widths) || {}).filter(([name]) => there(name));
  const shown = { columns: order.length > 0 ? order : null, freezes: kind === 'table' && view.freezes && there(view.freezes) ? view.freezes : null, figures: figures.length > 0 ? Object.fromEntries(figures) : null, widths: widths.length > 0 ? Object.fromEntries(widths) : null };
  if (kind !== 'order') return { by: goesBy(kind, fields, view.by), group, done: null, only, ...shown };
  const other = all.find((one) => one.kind === 'order');
  const by = has(view.by, 'list', 'records') ?? has(other?.by, 'list', 'records') ?? fields.find((field) => field.kind === 'records')?.name ?? null;
  return { by, group, done: has(view.done, 'choice') ?? has(other?.done, 'choice'), only, ...shown };
}

/** What is carried over to a view as it is saved (`goes`). */
export interface Carried {
  by: string | null;
  group: string | null;
  done: string | null;
  only: Record<string, string[]> | null;
  columns: string[] | null;
  freezes: string | null;
  figures: Record<string, string> | null;
  widths: Record<string, number> | null;
}

/** What a view is saved as, beside what is carried over to it. */
export interface Saved {
  name: string;
  kind: string;
  /** What it was called, where it is called something else now. */
  was?: string;
  hides?: string[];
  group?: string | null;
  only?: Record<string, string[]> | null;
  columns?: string[] | null;
  freezes?: string | null;
  figures?: Record<string, string> | null;
  widths?: Record<string, number> | null;
}

/**
 * A change to a table's views, as a function of how they stand when the
 * change is made: the view being looked at, the table's fields, and every
 * view it has. Not what the view is to be, worked out from how it was
 * drawn: two of those made in the time one takes each leave the other out
 * (`inturn.ts`).
 */
export type Change = (view: View, fields: Field[], views: View[]) => Saved;

/**
 * What is sent to save `change`, for a table with these fields and views
 * of which the one called `looking` is being looked at (the first, where
 * none is called that): what the view leaves out and what it goes by are
 * carried over, and what the change says is said over them.
 */
export function changed(fields: Field[], views: View[], looking: string | null, change: Change): Saved & Carried {
  const view = views.find((one) => one.name === looking) ?? views[0]!;
  const next = change(view, fields, views);
  return { hides: leftOut(view, fields), ...goes(next.kind, fields, view, views), ...next };
}

const two = (number: number) => String(number).padStart(2, '0');

/** A day as a date field keeps one: `2026-11-30`, where the reader is. */
export function dayString(date: Date): string {
  return `${date.getFullYear()}-${two(date.getMonth() + 1)}-${two(date.getDate())}`;
}

/** The day a record is on a calendar that goes by `by`, or `null` when it has none. */
export function dayOf(row: Row, by: string): string | null {
  if (by === 'created' || by === 'updated') return row[by] > 0 ? dayString(new Date(row[by])) : null;
  const day = row.data[by];
  return typeof day === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(day) ? day : null;
}

/**
 * A month as a calendar draws it: whole weeks, Monday first, each day as
 * `2026-11-30`, with the days of the months either side that fill the
 * first week and the last. `month` counts from 0.
 */
export function weeksOf(year: number, month: number): string[][] {
  const first = new Date(year, month, 1);
  const before = (first.getDay() + 6) % 7;
  const days = new Date(year, month + 1, 0).getDate();
  const weeks: string[][] = [];
  for (let at = -before; at < days; at += 7) {
    weeks.push(Array.from({ length: 7 }, (_, nth) => dayString(new Date(year, month, 1 + at + nth))));
  }
  return weeks;
}
