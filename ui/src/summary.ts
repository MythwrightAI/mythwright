// A blueprint said in plain words, as the page draws it.
//
// The words are the daemon's (`blueprint::summary`): what a blueprint is,
// what it makes, what it does when it is applied, what it hears and then
// does, what it may do, and what it does as it ends. They come with the
// check's answer, beside the graph, so that the two are of one text and
// follow what is being written together. Nothing is worked out here but
// how they are drawn: which of a blueprint's views is shown first, which
// of a line's words were set apart as written, where a line leads, and
// what its runs have cost, which is read from measurements and is no
// part of the summary.

import type { Graph, Series } from './api.ts';
import type { Mode } from './applied.ts';
import { duration, amount } from './util.ts';

/** One line of a summary, with the lines under it (a handler's steps, a task's, the params). */
export interface SummaryLine {
  text: string;
  /** What the line is about, as the graph names it: `main/build`, `on.changed/@`. */
  at: string | null;
  /** What the text calls that: a step's key, a handler's or a param's name, a scope. */
  key: string | null;
  /** `command` or `secret`: what somebody deciding whether to apply it most needs to see. */
  mark: string | null;
  under: SummaryLine[];
  /** Where the line leads, for a page that can go there (`forked.ts`). */
  opens?: Opens | null;
  /** Of what is done as it ends or is changed, which ending: `success`, `fail`, `finish`, `update` or `stop`. */
  ending?: string | null;
  // The line taken apart, where the daemon hands it so (`summarylaid.ts`): each its own words, and `text` the whole.
  /** Its own words, without what holds it to when, for what or where; the value of a fact. */
  title?: string | null;
  /** A word or two for what a fact about the whole says (`Version`, `Permissions`). */
  label?: string | null;
  /** What a fact lists, each by itself. */
  items?: string[];
  /** Where a handler hears from. */
  from?: string | null;
  /** What holds it to when, for what, from whom or where, each a sentence. */
  only?: string[];
  /** A few words said of it beside its title: how a handler takes what it hears; whether what it starts is waited for, left running or attached. */
  tags?: string[];
  /** What something it starts is started under, as its text says it. */
  named?: string | null;
  /** The handler that comes to the step it is said of. */
  by?: string | null;
  /** Of a line of what it makes: what is made, and what it is called as the text says it (and as it is written out, where it is). */
  made?: { what: string; name?: string | null; called?: string | null } | null;
  /** Of a line of what it makes: what that is made of, where the action that makes it says (a table's fields). */
  model?: Model | null;
}
/** What a thing a blueprint makes is made of (`blueprint::summary::Model`): each part, what says them where they are not written out, and what is said of the whole. */
export interface Model {
  fields?: ModelField[];
  as_said?: string | null;
  said?: string[];
}
/** One part of what is made: a table's field, its kind in a few words, the table it names where it names one, and what else is said of it. */
export interface ModelField {
  name: string;
  kind: string;
  names?: string | null;
  about?: string[];
  text: string;
}
/** Where a line leads: `blueprint` (another it starts, by name), `default` (the blueprint that ships, by name), `fork` (a project's fork, by its file), `table` (a table of the project), or what the page does: `apply`, `forking`, `back`. */
export interface Opens {
  what: string;
  name?: string | null;
}
/** One part of a summary; a part with nothing to say is not there. */
export interface SummaryPart {
  /** `is`, `makes`, `does`, `hears`, `may` or `ends`; and where the text is one that ships or a fork of one, `change`, `differs`, `moved` and `back`. */
  kind: string;
  title: string;
  /** Its lines are things done one after another. */
  ordered: boolean;
  lines: SummaryLine[];
}
export interface Summary {
  name: string;
  parts: SummaryPart[];
}

/**
 * The view a blueprint's page shows when it is first opened: the one that
 * was last looked at there, if one was kept; the graph of something
 * running, which shows how its steps went; the text for whoever can
 * change it, since that is what they came to do; and for anybody else,
 * and for a blueprint that ships, what it says in plain words. One
 * that is opened at a line of its text (from what a search found, from
 * a table's column that it declares) shows that line, whatever was kept.
 */
export function firstView(kept: Mode | null, of: { running: boolean; changes: boolean; at?: boolean }): Mode {
  return of.at ? 'text' : (kept ?? startsAt(of));
}

/** The view a blueprint's page starts at where nothing was kept: what need not be kept. */
export function startsAt(of: { running: boolean; changes: boolean }): Mode {
  if (of.running) return 'node';
  return of.changes ? 'text' : 'summary';
}

/** A line's words in pieces: what was set apart as written (a name, a word of the text) and what was not. */
export interface Piece {
  text: string;
  code: boolean;
}

/** A line taken apart at the marks that set a word apart. A mark with no partner is a mark. */
export function pieces(text: string): Piece[] {
  const parts = text.split('`');
  // An even count of marks leaves an odd count of parts: every second one was set apart.
  if (parts.length % 2 === 0) return [{ text, code: false }];
  return parts.map((part, at) => ({ text: part, code: at % 2 === 1 })).filter((piece) => piece.text !== '');
}

/** The line of the text a line of a summary is about, where it is about a step or a handler the graph has. */
export function lineOf(graph: Pick<Graph, 'nodes'> | null | undefined, at: string | null): number | null {
  if (!graph || !at) return null;
  return graph.nodes.find((node) => node.id === at)?.line ?? null;
}

/** Every line of a summary, those under others among them. */
export function everyLine(summary: Summary): SummaryLine[] {
  const under = (lines: SummaryLine[]): SummaryLine[] => lines.flatMap((line) => [line, ...under(line.under)]);
  return summary.parts.flatMap((part) => under(part.lines));
}

/** What a blueprint's runs have cost, counted: never a score. */
export interface Cost {
  runs: number;
  failed: number;
  /** How long one typically took, in milliseconds. */
  typical: number;
  /** What else was measured under it that is not a length of time, added up: tokens, money. */
  spent: { name: string; unit: string; total: number; count: number }[];
}

/** The measurement of a run, as the daemon names it. */
const RUN = 'run';

/**
 * What was measured under a blueprint, as what its runs have cost: how
 * many there were, how many failed and how long one typically took, and
 * whatever else was measured under it that says what it is of (so many
 * tokens, so much money: `unit`, which the daemon says with each series,
 * so that no name is listed here). Nothing where nothing was measured.
 */
export function costOf(series: Series[]): Cost | null {
  const run = series.filter((one) => one.name === RUN);
  const runs = run.reduce((sum, one) => sum + one.total.count, 0);
  const spent = series
    .filter((one) => one.unit !== undefined && one.total.count > 0)
    .map((one) => ({ name: one.name, unit: one.unit!, total: one.total.sum, count: one.total.count }))
    .sort((a, b) => a.name.localeCompare(b.name));
  if (runs === 0 && spent.length === 0) return null;
  // Of several series (one a flow), the one with the most runs says what is typical.
  const most = [...run].sort((a, b) => b.total.count - a.total.count)[0];
  return { runs, failed: run.reduce((sum, one) => sum + one.total.errors, 0), typical: most?.total.p50 ?? 0, spent };
}

/** What its runs have cost, in sentences: counts and amounts, and no word on whether they are good. */
export function costSaid(cost: Cost, since: string): string[] {
  const said: string[] = [];
  if (cost.runs > 0) {
    const times = cost.runs === 1 ? 'once' : `${cost.runs.toLocaleString('en-US')} times`;
    const failed = cost.failed === 0 ? 'none failed' : cost.failed === 1 ? '1 failed' : `${cost.failed.toLocaleString('en-US')} failed`;
    said.push(`It ran ${times} in ${since}; ${failed}.`);
    said.push(cost.runs === 1 ? `The run took ${duration(cost.typical)}.` : `A run typically took ${duration(cost.typical)}.`);
  }
  // Money is written with its sign; anything else is so many of what the daemon says it is of.
  const written = (total: number, unit: string) => (unit === 'usd' ? amount(total, unit) : `${amount(total, unit)} ${unit}`);
  for (const one of cost.spent) said.push(`${one.name}: ${written(one.total, one.unit)} over ${one.count.toLocaleString('en-US')} ${one.count === 1 ? 'measurement' : 'measurements'}.`);
  return said;
}

/** The label that says which blueprint's measurements are asked for, and for which project where it is one's. */
export function costLabels(name: string, project: string | null): string {
  return [`blueprint:${name}`, ...(project ? [`project:${project}`] : [])].join(',');
}
