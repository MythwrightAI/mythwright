// How a summary is laid out on a page, from what the daemon hands.
//
// The words are all the daemon's (`blueprint::summary`): each line's
// `text` is the whole of it, which a terminal prints, and a line that is
// a handler, a step or a fact about the whole is handed taken apart as
// well (`title`, `label`, `items`, `from`, `only`, `takes`). What is
// worked out here is only where each goes: the head, what is said once
// for a part's handlers, the order the parts are read in, and what is
// folded. Nothing here is a sentence.

import { pieces } from './summary.ts';
import type { Model, SummaryLine, SummaryPart } from './summary.ts';

/** The head of a summary: its part `is`, and the facts about the whole from the rest. */
export interface Head {
  /** Where its text stands and what it is a part of: the lines of `is` that lead somewhere, first. */
  standing: SummaryLine[];
  /** What it says it is: the first line of `is` that is its own words and nothing more. */
  lead: SummaryLine | null;
  /** Facts about the whole, each a label with a value or a list: of what it is, of what it may do, then of the rest; those of one label together, where the first of them stands. */
  facts: Facts[];
  /** The rest of `is` as lines: what whoever applies it may tell it, and whatever else. */
  rest: SummaryLine[];
  /** Each kind of mark a line of it has, once, in the order first met: what somebody deciding most needs to see. */
  marks: string[];
}

/** The facts of one label: the permissions it uses, say, each its own line. */
export interface Facts {
  label: string;
  lines: SummaryLine[];
}

/** Whether facts are said in words of their own and not as names alone: such are a list, a line each. */
export function wordy(facts: Facts): boolean {
  return facts.lines.some((line) => !line.items?.length && pieces(line.title ?? line.text).some((piece) => !piece.code && piece.text.trim().includes(' ')));
}

/** Whether a line is a fact about the whole: a label with a value or a list. */
export function isFact(line: SummaryLine): boolean {
  return !!line.label;
}

/** Every line of some lines, those under others among them. */
function every(lines: SummaryLine[]): SummaryLine[] {
  return lines.flatMap((line) => [line, ...every(line.under)]);
}

/** What the head of a summary holds. A summary with no part `is` has a head with only its facts and marks. */
export function headOf(parts: SummaryPart[]): Head {
  const is = parts.find((part) => part.kind === 'is')?.lines ?? [];
  const standing = is.filter((line) => !!line.opens);
  const plain = (line: SummaryLine) => !line.opens && !isFact(line) && line.under.length === 0 && !line.key;
  const lead = is.find(plain) ?? null;
  const facts: Facts[] = [];
  // What it is, then what it may do.
  const first = (part: SummaryPart) => (part.kind === 'is' ? 0 : part.kind === 'may' ? 1 : 2);
  const byFirst = [...parts].sort((a, b) => first(a) - first(b));
  for (const line of byFirst.flatMap((part) => (FACTS.includes(part.kind) ? part.lines.filter(isFact) : []))) {
    const same = facts.find((one) => one.label === line.label);
    if (same) same.lines.push(line);
    else facts.push({ label: line.label!, lines: [line] });
  }
  const rest = is.filter((line) => !line.opens && line !== lead && !isFact(line));
  const marks: string[] = [];
  for (const line of parts.flatMap((part) => every(part.lines))) if (line.mark && !marks.includes(line.mark)) marks.push(line.mark);
  return { standing, lead, facts, rest, marks };
}

/**
 * The parts whose facts are facts of the whole, drawn in the head: what it
 * is and what it may do. A line with a label anywhere else (a way to
 * change it) is drawn where its part is; what it makes is a table of its
 * own, a row for each thing made.
 */
const FACTS = ['is', 'may'];

/** The lines of a part that are drawn where the part is: all but the facts, which are drawn in the head. */
export function ownLines(part: SummaryPart): SummaryLine[] {
  if (part.kind === 'is') return [];
  return FACTS.includes(part.kind) ? part.lines.filter((line) => !isFact(line)) : part.lines;
}

/** Lines that say the same thing started the same way, as one row: the first of them, and each of them, for what starts it. */
export interface Row {
  line: SummaryLine;
  lines: SummaryLine[];
}

/**
 * What a blueprint makes and starts, as rows: lines that start the same
 * blueprint under the same name the same way (`std/due` as `look`,
 * waited for, from `first`, `staffed` and `listening`) are one row, in
 * the order the first of them is said; any other line is a row alone.
 */
export function rows(lines: SummaryLine[]): Row[] {
  const found = new Map<string, Row>();
  const all: Row[] = [];
  for (const line of lines) {
    const same = line.title ? JSON.stringify([line.title, line.named ?? null, line.tags ?? [], line.only ?? [], line.opens ?? null, line.mark]) : null;
    const row = same === null ? undefined : found.get(same);
    if (row) row.lines.push(line);
    else {
      const made = { line, lines: [line] };
      all.push(made);
      if (same !== null) found.set(same, made);
    }
  }
  return all;
}

/**
 * Where a line of what it makes is started from, in a word: the handler
 * that comes to it, else the step of what it does when it is applied;
 * nothing for a blueprint attached to it, which no step starts.
 */
export function startedFrom(line: Pick<SummaryLine, 'by' | 'at' | 'key'>): string {
  return line.by ?? (line.at?.startsWith('uses/') ? '' : (line.key ?? ''));
}

/** How many rows of what it makes are shown before the rest is asked for. */
export const MOST_ROWS = 5;

/** Whether the rows of what it makes are folded to their first few: only where that puts away more than a couple, or the press would show one or two. */
export function foldsRows(count: number): boolean {
  return count > MOST_ROWS + 2;
}

/** Where each kind of part is read: what it does and hears before what it makes and may do. A kind not named comes after those, before how to change it. */
const READ: Record<string, number> = { is: 0, differs: 1, moved: 2, does: 3, hears: 4, ends: 5, makes: 6, may: 7, change: 8, back: 9 };

/** The parts in the order a reader needs them: what it is, where a fork differs, what it does and hears and does as it ends, then the rest. */
export function readOrder(parts: SummaryPart[]): SummaryPart[] {
  const rank = (part: SummaryPart, at: number) => [READ[part.kind] ?? 7.5, at] as const;
  return parts
    .map((part, at) => ({ part, rank: rank(part, at) }))
    .sort((a, b) => a.rank[0] - b.rank[0] || a.rank[1] - b.rank[1])
    .map((one) => one.part);
}

/** Whether a line is a handler: what something hears, with its steps under it. */
export function isHandler(line: Pick<SummaryLine, 'at'>): boolean {
  return !!line.at?.endsWith('/@');
}

/** How many handlers a part has. */
export function handlers(part: SummaryPart): number {
  return part.lines.filter(isHandler).length;
}

/**
 * Where most of a part's handlers hear from, said once at the part's
 * head: the `from` that more of them say than any other, where two or
 * more say it. A handler that says it is not told it again; one that
 * says another is. Of two said as often, the first met.
 */
export function sharedFrom(lines: SummaryLine[]): string | null {
  const counts = new Map<string, number>();
  for (const line of lines) if (isHandler(line) && line.from) counts.set(line.from, (counts.get(line.from) ?? 0) + 1);
  let best: string | null = null;
  for (const [from, count] of counts) if (count >= 2 && (best === null || count > counts.get(best)!)) best = from;
  return best;
}

/** What a handler's card says of where it hears from: nothing where the part says it once for it. */
export function fromOf(line: SummaryLine, shared: string | null): string | null {
  return line.from && line.from !== shared ? line.from : null;
}

/** What a fold of a summary is known by, within its tab: a handler by what the graph calls it, any other line by its part and place. */
export function foldKey(kind: string, line: SummaryLine, at: number): string {
  return line.at ?? `${kind}:${at}`;
}

/** How many lines may stand under a line about the whole and still be shown without being asked for. */
export const UNFOLDED = 4;

/**
 * Whether what is under a line is folded away until it is asked for: a
 * handler's steps, so that what a blueprint hears is read down as a list
 * of what it hears; and a long list under a line about the whole (the
 * dozen things a blueprint may be told). A step's own steps (a task's),
 * and what is done as it ends, are short and are what the line is.
 */
export function folds(line: Pick<SummaryLine, 'at' | 'under'>): boolean {
  if (line.under.length === 0) return false;
  return isHandler(line) || (line.at === null && line.under.length > UNFOLDED);
}

/** A row of what it makes, as its columns say it: what is made and what it is called; a line from a daemon that does not say is called by its own words. */
export function madeOf(line: SummaryLine): { what: string; name: string } {
  if (line.made) return { what: line.made.what, name: line.made.name ?? '' };
  return { what: '', name: line.title ?? line.text };
}

/** How many parts a model has, in words: `13 fields`, `1 field`; what says them where they are not written out; nothing for none. */
export function fieldsSaid(model: Model | null | undefined): string {
  if (!model) return '';
  const many = model.fields?.length ?? 0;
  if (many > 0) return many === 1 ? '1 field' : `${many} fields`;
  return model.as_said ? 'as said' : '';
}

/** What a page folds a row's model under, within its tab. */
export const modelFold = (at: string | null, index: number): string => `makes:model:${at ?? index}`;

/** A table among those a blueprint makes, as the Data model lists it: what it is called, by its name as written where it is, and its model. */
export interface Modelled {
  name: string;
  called: string | null;
  model: Model;
}

/**
 * What the part Data model lists: each table it makes with fields, in the
 * order they are made, a table made in several places once. Only where
 * there are two or more, since one is read under its own row.
 */
export function dataModel(lines: SummaryLine[]): Modelled[] {
  const all: Modelled[] = [];
  for (const line of lines) {
    if (!line.model?.fields?.length) continue;
    const called = line.made?.called ?? null;
    if (called !== null && all.some((one) => one.called === called)) continue;
    all.push({ name: madeOf(line).name, called, model: line.model });
  }
  return all.length >= 2 ? all : [];
}

/** The table among those listed that a field names, where it names one of them. */
export function namedBy(names: string | null | undefined, tables: Modelled[]): Modelled | null {
  return names ? (tables.find((one) => one.called === names) ?? null) : null;
}
