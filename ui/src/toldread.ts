// What a told line shows beside its words that the daemon reads for
// whoever reads it (`kernel::parts`, `told::read`): rows of a project's
// table, runs, and measurements, each as the daemon says it comes to
// now; and which tab of the line's page each part is in. Nothing here
// asks anything or holds anything of the page, so that its tests can
// read it: what is listed is what the daemon handed, and what is kept
// from the reader is its sentence.

import { amount, unitOf } from './compare.ts';
import type { Field } from './table.ts';
import { duration } from './util.ts';

/** A column a part of rows shows: the field as its table declares it, and whether it holds seconds, drawn as a length of time. */
export interface RowsColumn extends Field {
  seconds?: boolean;
}

/** A row a part of rows lists: its cells by column, or that no row of the table has its id now. */
export interface ListedRow {
  id: string;
  cells?: Record<string, unknown>;
  gone?: boolean;
}

export interface RowsPart {
  kind: 'rows';
  project: string;
  table: string;
  ids?: string[] | null;
  saying?: Record<string, unknown> | null;
  columns?: RowsColumn[];
  rows?: ListedRow[];
  /** How many more rows were chosen than are listed. */
  more?: number;
  /** The columns it names that the table has not got. */
  unknown?: string[];
  /** Why the reader is shown none of it. */
  refused?: string;
}

/** A run a part of runs lists. */
export interface ListedRun {
  id: string;
  /** What it is a run of, in words (`kernel::parts::run_called`). */
  called: string;
  status: string;
  started: number;
  finished: number | null;
}

export interface RunsPart {
  kind: 'runs';
  project: string;
  instance?: string;
  key?: string;
  environment?: string;
  most: number;
  runs?: ListedRun[];
  refused?: string;
}

/** A measurement a part of measures lists: its latest value beside the median of its last few, where anything was measured. */
export interface Measure {
  name: string;
  /** What it is of, where its name says (`usd`, `tokens`, ...); a length of time, in milliseconds, where it says nothing. */
  unit: string | null;
  measured: { latest: number; at: number; median: number; of: number } | null;
  count?: number;
}

export interface MeasuresPart {
  kind: 'measures';
  last: number;
  measures?: Measure[];
  labels?: Record<string, string>;
  refused?: string;
}

/** What the first tab is called where there are others: its parts say no tab, as a card's drawer calls its first tab. */
export const FIRST = 'Details';

/** One tab of a line's page: its name (empty for the first, which has none), what it is called, and its parts by their places among them. */
export interface Tab {
  name: string;
  called: string;
  parts: number[];
}

/**
 * The tabs of a line's page (`kernel::parts::tabs`): the parts that say
 * no tab on a first, and then each tab as it is first said, with the
 * parts in it. A page with one tab draws no row of tabs.
 */
export function tabsOf(parts: { tab?: unknown }[]): Tab[] {
  const tabs: Tab[] = [];
  const named = (part: { tab?: unknown }) => (typeof part.tab === 'string' ? part.tab : '');
  if (parts.some((part) => named(part) === '')) tabs.push({ name: '', called: FIRST, parts: [] });
  parts.forEach((part, at) => {
    const name = named(part);
    const tab = tabs.find((one) => one.name === name);
    if (tab) tab.parts.push(at);
    else tabs.push({ name, called: name, parts: [at] });
  });
  return tabs;
}

/** The tab showing: the one chosen while it is there, and the first otherwise. */
export function showing(tabs: Tab[], chosen: string | null): Tab | null {
  return tabs.find((tab) => tab.name === chosen) ?? tabs[0] ?? null;
}

/** What a part of rows is headed with, and what is said under it. */
export function rowsHead(part: RowsPart): { title: string; note: string | null } {
  const notes: string[] = [];
  if (part.more) notes.push(`${part.more} more ${part.more === 1 ? 'row says' : 'rows say'} the same and ${part.more === 1 ? 'is' : 'are'} not listed: the table has every one.`);
  if (part.unknown?.length) notes.push(`${part.table} has no column called ${part.unknown.join(' or ')}.`);
  return { title: `Rows of ${part.table}`, note: notes.length ? notes.join(' ') : null };
}

/** What a part of rows that lists no row says. */
export function noRows(part: RowsPart): string {
  return part.ids ? 'None of these rows is there now.' : 'No row says that now.';
}

/** A cell of seconds as a length of time; nothing where it says no number. */
export function secondsSaid(value: unknown): string | null {
  const seconds = typeof value === 'number' ? value : typeof value === 'string' && value.trim() !== '' ? Number(value) : NaN;
  return Number.isFinite(seconds) ? duration(seconds * 1000) : null;
}

/** What a part of runs is headed with. */
export function runsHead(part: RunsPart): string {
  if (part.environment) return `Runs under ${part.environment}`;
  if (part.key) return `Runs of ${part.key}`;
  return 'Its runs';
}

/** How a run went, in words. */
export function wentSaid(status: string): string {
  switch (status) {
    case 'ok':
      return 'went well';
    case 'failed':
      return 'failed';
    case 'cancelled':
      return 'was cancelled';
    case 'running':
      return 'running';
    default:
      return status;
  }
}

/** How long a run took, or that it is still running. */
export function tookSaid(run: ListedRun): string {
  return run.finished === null || run.finished === undefined ? 'still running' : duration(run.finished - run.started);
}

/** A run's page, by its address: what pressing it opens as a tab here. */
export const runAddress = (run: string): string => `/run/${encodeURIComponent(run)}`;

/** A measurement in words, as the measurements page writes what it is of: its latest, and the median of its last few. Counted, and never scored. */
export function measureSaid(measure: Measure): { latest: string; median: string } | null {
  if (!measure.measured) return null;
  const unit = unitOf({ unit: measure.unit ?? undefined });
  const { latest, median, of } = measure.measured;
  return { latest: amount(latest, unit), median: `${amount(median, unit)}, the median of its last ${of === 1 ? 'one' : of}` };
}

/** The labels a part of measures reads by, in words: `blueprint deploy, project shop`, or nothing where it says none. */
export function labelsSaid(labels: Record<string, string> | undefined): string | null {
  const said = Object.entries(labels ?? {}).map(([label, value]) => `${label} ${value}`);
  return said.length ? `Measured with ${said.join(', ')}` : null;
}
