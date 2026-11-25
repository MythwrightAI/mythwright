// Measurements side by side: the same measure under each value of a label.
//
// A blueprint that does the same work two ways says which is which with
// `labels:`, and everything measured under each carries it. Here those
// measurements are put beside each other: a row for each thing measured,
// a group of columns for each value of the label, and where there are
// exactly two values, how the second differs from the first.
//
// A measure is a row as a whole, since that is what is the same under
// every value whatever the steps are called: all the agents' turns, all
// the runs, everything a step named `build`. Steps and runs are then also
// a row for each step and each flow, to see where a difference comes from.
//
// The daemon does the counting. This is only which series belong in which
// row and column, and how a difference is said.

import type { Series, Stats } from './api.ts';
import { duration, amount as written } from './util.ts';

/** The labels there are to compare by, each with the values it has, as the daemon lists them. */
export type Labels = Record<string, string[]>;

/** What is being compared: by which label, and narrowed to which value of another (`experiment:cart-12`), if any. */
export interface Choice {
  by: string;
  only: string;
}

export const NOTHING: Choice = { by: '', only: '' };

/** What a step or a run says of itself that makes it the same one under two values of a label. */
const SAME = ['blueprint', 'step', 'flow'];
/** The measures that are also compared part by part, and what a part of each is told by. */
const PARTS: Record<string, string> = { step: 'blueprint,step', run: 'blueprint,flow' };
/** How many parts of one measure are shown: those that took the most time. */
const MOST_PARTS = 20;

/** What a measure is counted in: a length of time, or what the daemon says its values are of (`usd`, `tokens`). */
export type Unit = string;

export interface Difference {
  /** The second average less the first. */
  by: number;
  /** That, as a fraction of the first; `null` when the first is nothing to be a fraction of. */
  ratio: number | null;
  /** The share that failed under the second, less the share under the first. */
  failed: number;
}

export interface Compared {
  /** The measure: `step`, `run`, `agent.turn`, or a name a step gave itself. */
  name: string;
  /** The measure as a heading: `Steps`, `Agent turns`. */
  title: string;
  /** Which part of it, for a row that is one step or one flow: `std/working · work`. Empty for the measure as a whole. */
  of: string;
  unit: Unit;
  /** What was measured under each value, in the order of the values; `null` where nothing was. */
  cells: (Stats | null)[];
  /** How the second value differs from the first, when there are exactly two and both were measured. */
  difference: Difference | null;
}

export interface Comparison {
  values: string[];
  rows: Compared[];
}

/** The choice as far as it can still be made: a label that is no longer there is not compared by or narrowed to. */
export function standing(labels: Labels, choice: Choice): Choice {
  if (!labels[choice.by]) return NOTHING;
  const [label, value] = split(choice.only);
  const there = label !== choice.by && (labels[label] ?? []).includes(value);
  return { by: choice.by, only: there ? choice.only : '' };
}

/** `experiment:cart-12` as the label and its value. */
export function split(only: string): [string, string] {
  const at = only.indexOf(':');
  return at < 0 ? ['', ''] : [only.slice(0, at), only.slice(at + 1)];
}

/** What a comparison by one label can be narrowed to: each other label, with its values. */
export function narrowings(labels: Labels, by: string): { label: string; values: string[] }[] {
  return Object.keys(labels)
    .filter((label) => label !== by)
    .sort()
    .map((label) => ({ label, values: [...(labels[label] ?? [])].sort(inOrder) }));
}

/** What to ask the daemon for: every measure split by the label, and steps and runs split further into their parts. */
export function asks(by: string): { name: string; by: string }[] {
  return [{ name: '*', by }, ...Object.entries(PARTS).map(([name, parts]) => ({ name, by: `${by},${parts}` }))];
}

/** What to narrow to when asking the daemon: the projects, if it is of some (a label said several times over is any of them), and the choice's other label. */
export function narrowedTo(projects: string[] | null, only: string): string | undefined {
  const parts = [...(projects ?? []).map((project) => `project:${project}`), only].filter(Boolean);
  return parts.length ? parts.join(',') : undefined;
}

/** Numbers in the order of numbers, words in the order of words. */
function inOrder(a: string, b: string): number {
  return a.localeCompare(b, undefined, { numeric: true });
}

const TITLES: Record<string, string> = {
  'agent.turn': 'Agent turns',
  'agent.cost_usd': 'Agent cost, per turn',
  run: 'Runs',
  step: 'Steps',
  poll: 'Polls',
};
/** The order the measures everyone has come in; what blueprints named themselves comes after. */
const ORDER = ['agent.turn', 'agent.cost_usd', 'run', 'step', 'poll'];

/** What a series is counted in, as the daemon says with it: a measurement says what it is of by how its name ends, which is read in one place there, and no name is listed here. */
export function unitOf(series: Pick<Series, 'unit'>): Unit {
  return series.unit ?? 'time';
}

/** A figure as it is shown: a length of time, or an amount of what it is of. */
export function amount(value: number, unit: Unit): string {
  return unit === 'time' ? duration(value) : written(value, unit);
}

/** How the second of two differs from the first, in what was measured and in how often it failed. */
export function difference(first: Stats, second: Stats): Difference {
  const by = second.avg - first.avg;
  return { by, ratio: first.avg > 0 ? by / first.avg : null, failed: second.error_rate - first.error_rate };
}

/** A difference in words: `+1m 05s (+50%)`, `−0.4s (−12%)`, `the same`. */
export function changed(difference: Difference, unit: Unit): string {
  const size = amount(Math.abs(difference.by), unit);
  // Too small to show is not a difference worth a sign.
  if (size === amount(0, unit)) return 'the same';
  const sign = difference.by > 0 ? '+' : '−';
  if (difference.ratio === null) return `${sign}${size}`;
  return `${sign}${size} (${sign}${Math.abs(difference.ratio * 100).toFixed(0)}%)`;
}

/** How much more or less often it failed, in points of a hundred; nothing when it is the same. */
export function failedChange(difference: Difference): string | null {
  const points = Math.round(Math.abs(difference.failed) * 100);
  if (points === 0) return null;
  return `${difference.failed > 0 ? '+' : '−'}${points} points failed`;
}

/**
 * Puts what the daemon answered to `asks` side by side. Series that do not
 * carry the label are someone else's and are left out.
 */
export function compare(series: Series[], by: string): Comparison {
  const carrying = series.filter((one) => one.labels[by] !== undefined);
  const values = [...new Set(carrying.map((one) => one.labels[by]!))].sort(inOrder);
  const found = new Map<string, { first: Series; under: Map<string, Stats> }>();
  for (const one of carrying) {
    const key = [one.name, ...SAME.map((label) => one.labels[label] ?? '')].join('\u0000');
    const row = found.get(key) ?? { first: one, under: new Map<string, Stats>() };
    row.under.set(one.labels[by]!, one.total);
    found.set(key, row);
  }
  const rows = [...found.values()].map(({ first, under }): Compared => {
    const cells = values.map((value) => under.get(value) ?? null);
    const [a, b] = cells;
    return {
      name: first.name,
      title: TITLES[first.name] ?? first.name,
      of: [first.labels.blueprint, first.labels.step ?? first.labels.flow].filter(Boolean).join(' · '),
      unit: unitOf(first),
      cells,
      difference: values.length === 2 && a && b ? difference(a, b) : null,
    };
  });
  const place = (row: Compared) => {
    const at = ORDER.indexOf(row.name);
    return at < 0 ? ORDER.length : at;
  };
  // A measure as a whole, then its parts: what took the most time altogether first, as in the other tables.
  const whole = (row: Compared) => (row.of === '' ? 0 : 1);
  const weight = (row: Compared) => row.cells.reduce((sum, cell) => sum + (cell ? cell.count * cell.avg : 0), 0);
  rows.sort((a, b) => place(a) - place(b) || a.name.localeCompare(b.name) || whole(a) - whole(b) || weight(b) - weight(a) || a.of.localeCompare(b.of));
  const shown = new Map<string, number>();
  const kept = rows.filter((row) => {
    if (row.of === '') return true;
    const parts = (shown.get(row.name) ?? 0) + 1;
    shown.set(row.name, parts);
    return parts <= MOST_PARTS;
  });
  return { values, rows: kept };
}
