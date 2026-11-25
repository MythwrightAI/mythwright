// Comparisons: one blueprint started several ways, as they are shown.
//
// The daemon does the comparing, the counting and the laying out: a column
// for each text of each way, and a row for each thing measured. What is
// here says how that is written: each way with what it starts and hands,
// and the two figures a column has for a row, as `compare show` prints
// them. They are counts, times and amounts. Nothing here says which way is
// better, and nothing is to: that depends on what whoever started it
// wanted to find out.

import type { Beside, BesideRow, Comparison, Fields, Json, Way } from './api.ts';
import type { Counted, Rolled, Rule } from './comparing.ts';
import { covers } from './holds.ts';
import { amount, count, duration, percent } from './util.ts';

/** What a way starts and hands, in a few words. */
export function does(way: Pick<Way, 'blueprint' | 'version' | 'sent' | 'with' | 'prompts'>): string {
  const said: string[] = [];
  if (way.blueprint) said.push(`starts ${way.blueprint}${way.version ? ` at version ${way.version}` : way.sent ? ' as the text that was sent' : ''}`);
  for (const [param, value] of Object.entries(way.with ?? {})) said.push(`${param}=${typeof value === 'string' ? value : JSON.stringify(value)}`);
  for (const [linked, reads] of Object.entries(way.prompts ?? {})) said.push(`reads ${reads.name} for ${linked}`);
  return said.length > 0 ? said.join(', ') : 'as it is written';
}

/** Which words a column was run with, where its way reads others in place of a file that is linked: a line for each. */
export function readsOf(column: Beside['columns'][number]): string[] {
  return (column.reads ?? []).map((reads) => `reads ${reads.name}${reads.words ? ` · ${reads.words}` : ''}`);
}

/** Where a comparison stands, in a few words. */
export function stands(one: Pick<Comparison, 'ended' | 'goes'>): string {
  if (one.ended === null) return 'going';
  return one.goes ? `ended; everything goes ${one.goes}` : 'ended; everything goes as it is written';
}

/** Whether it still says how anything is started: going, or ended with a way that goes on. */
export function inForce(one: Pick<Comparison, 'ended' | 'goes'>): boolean {
  return one.ended === null || Boolean(one.goes);
}

/** What a thing is, as a comparison tells them apart. */
export function thing(one: Pick<Comparison, 'per'>): string {
  return one.per ? `each ${one.per}` : 'each place a step starts it';
}

/** How a column says which text it is of, under its way's name: the version the text says and the start of its revision; nothing where it is not known. */
export function saidOf(column: Beside['columns'][number]): string {
  if (!column.text) return '';
  return (column.version ? 'version ' + column.version + ' · ' : '') + column.text + (column.now ? ', now' : '');
}

/** What each kind of row is headed, and what the two figures a column has for it are. */
export const KINDS: Record<BesideRow['kind'], { head: string; figures: [string, string] }> = {
  came: { head: 'Came of them', figures: ['How many', 'After'] },
  chose: { head: 'Chosen', figures: ['How many', 'Average'] },
  spent: { head: 'Added up', figures: ['In all', 'A thing'] },
  accepted: { head: 'Accepted', figures: ['How many', 'Of those given'] },
  per_accepted: { head: 'For what was accepted', figures: ['In all', 'Each accepted'] },
  took: { head: 'Measured', figures: ['How many', 'Average'] },
};

/** The two figures one column has for a row, as they are written: how many and how long; what it came to in all and for each thing given; how many were accepted and that over how many were given; or what was spent in all and over how many were accepted. */
export function figures(row: Pick<BesideRow, 'kind' | 'unit'>, cell: NonNullable<BesideRow['by'][number]>): [string, string] {
  if (row.kind === 'accepted') return [count(cell.count ?? 0), cell.rate === undefined ? '-' : percent(cell.rate)];
  if (row.unit) {
    const each = row.kind === 'per_accepted' ? cell.per_accepted : cell.per_thing;
    return [amount(cell.sum ?? 0, row.unit), each === undefined ? '-' : amount(each, row.unit)];
  }
  // What came of a thing did not fail: it came. A turn or a step may have.
  const errors = cell.errors ?? 0;
  const failed = row.kind !== 'came' && errors > 0 ? ' (' + count(errors) + ' failed)' : '';
  return [count(cell.count ?? 0) + failed, duration(cell.avg ?? 0)];
}

/**
 * What a column with no figure for a row says there: of what is counted in
 * money, where the column's agents took turns (something of theirs was
 * measured), that it was not reported, since a program that says nothing of
 * what a turn cost measures nothing of it; and else a dash.
 */
export function unmeasured(rows: Pick<BesideRow, 'what' | 'unit' | 'by'>[], row: Pick<BesideRow, 'what' | 'unit'>, index: number): string {
  const turned = rows.some((other) => other.what !== row.what && other.what.startsWith('agent.') && other.by[index]);
  return row.unit === 'usd' && turned ? 'not reported' : '-';
}

/** The rows of a comparison by kind, in the order the daemon laid them: each kind once, with its rows. */
export function byKind(rows: BesideRow[]): { kind: BesideRow['kind']; rows: BesideRow[] }[] {
  const kinds: { kind: BesideRow['kind']; rows: BesideRow[] }[] = [];
  for (const row of rows) {
    const last = kinds.at(-1);
    if (last && last.kind === row.kind) last.rows.push(row);
    else kinds.push({ kind: row.kind, rows: [row] });
  }
  return kinds;
}

/** What a comparison counts of what comes of each thing, by what each count is called. */
export function counted(one: Pick<Comparison, 'counts'>): string[] {
  return [...new Set((one.counts ?? []).map((kind) => kind.name))];
}

/** What is to be counted, as somebody writes it a line each (`made=shop.made:id:how+tone`): what the count is called, the event, the field that says the thing, and the fields that say what it came to. */
export function counting(written: string): { counts: Counted[] } | { wrong: string } {
  const counts: Counted[] = [];
  const lines = written
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (const line of lines) {
    const at = line.indexOf('=');
    const [topic, thing, by, ...more] = at < 1 ? [] : line.slice(at + 1).split(':');
    if (!topic || !thing || more.length > 0) return { wrong: 'What is counted is written a line each, as name=event:field or name=event:field:field+field; "' + line + '" is not.' };
    counts.push({ name: line.slice(0, at).trim(), topic: topic.trim(), thing: thing.trim(), ...(by ? { by: by.split('+').map((field) => field.trim()) } : {}) });
  }
  return { counts };
}

/** A way as somebody writes it when starting a comparison. */
export interface Written {
  name: string;
  /** Another blueprint to start in its place, or nothing. */
  blueprint: string;
  share: string;
  /** What it hands, a line each: `param=value`. */
  hands: string;
  /** What it reads in place of a file a step links, a line each: `file=other file`, by their names in the library. */
  reads?: string;
  /** A text it starts as it is sent, in place of a blueprint by its name: one played side by side in sandboxes, by what its column was called there. */
  sent?: { called: string; source: string };
}

/** What is wrong with what was written, where something is; else the ways as the daemon is told them. */
export function asked(ways: Written[]): { ways: { name: string; share?: number; blueprint?: string; source?: string; with?: Fields; prompts?: Record<string, string> }[] } | { wrong: string } {
  const told = [];
  for (const way of ways) {
    const name = way.name.trim();
    if (!name) return { wrong: 'Each way is called something: a word, such as a or mine.' };
    const share = way.share.trim() === '' ? 1 : Number(way.share);
    if (!Number.isInteger(share) || share < 1) return { wrong: `The share of ${name} is a whole number, 1 or more.` };
    const hands: Fields = {};
    for (const line of way.hands.split('\n').map((line) => line.trim()).filter(Boolean)) {
      const at = line.indexOf('=');
      if (at < 1) return { wrong: `What ${name} hands is written a line each, as param=value; "${line}" is not.` };
      const value = line.slice(at + 1).trim();
      let read: Json = value;
      try {
        read = JSON.parse(value) as Json;
      } catch {
        // Not JSON: text as it is.
      }
      hands[line.slice(0, at).trim()] = read;
    }
    const reads: Record<string, string> = {};
    for (const line of (way.reads ?? '').split('\n').map((line) => line.trim()).filter(Boolean)) {
      const at = line.indexOf('=');
      if (at < 1 || !line.slice(at + 1).trim()) return { wrong: `What ${name} reads is written a line each, as file=other file; "${line}" is not.` };
      reads[line.slice(0, at).trim()] = line.slice(at + 1).trim();
    }
    const starts = way.sent ? { source: way.sent.source } : way.blueprint ? { blueprint: way.blueprint } : {};
    told.push({ name, share, ...starts, ...(Object.keys(hands).length > 0 ? { with: hands } : {}), ...(Object.keys(reads).length > 0 ? { prompts: reads } : {}) });
  }
  return { ways: told };
}

/** A comparison's shares as they are written under its ways: each way's part of a hundred; the rule in the daemon's sentence with what it has read since the shares were last set, or that it is done; and each time the shares were said again, the newest first, by whom and why in the daemon's words. Nothing where neither a rule nor a change was said. */
export function rolledOf(rolled: Rolled | undefined): { parts: string; rule: string | null; read: string | null; changes: { at: number; by: string; why: string }[] } | null {
  if (!rolled) return null;
  const parts = rolled.parts.map((part) => `${part.way} ${part.percent}%`).join(', ');
  const rule = rolled.rule;
  const read = !rule
    ? null
    : rule.done
      ? 'It has taken its way as far as it goes.'
      : `Since the shares were last set: ${rule.read.map((one) => `${one.way} ${one.came} of ${one.things}`).join(', ')}, of ${count(rule.over)} each it reads before it decides.`;
  const changes = [...rolled.shared].reverse().map((one) => ({ at: one.at, by: one.by === 'rule' ? 'its rule' : one.by, why: one.why }));
  return { parts, rule: rule ? rule.said + '.' : null, read, changes };
}

/** A rule as somebody writes it: each part as it was typed, and whether it falls back. */
export interface WrittenRule {
  grows: string;
  against: string;
  count: string;
  came: string;
  atLeast: boolean;
  over: string;
  step: string;
  most: string;
  /** The least it goes back to where it does not hold; nothing, and it stays. */
  least: string;
}

/** What a rule begins as, for these ways: the last grows against the first, by how often its cards are sent back from review, over twenty each, ten at a time to half. */
export function unruled(ways: string[]): WrittenRule {
  return { grows: ways.at(-1) ?? '', against: ways[0] ?? '', count: 'verdict', came: 'failed', atLeast: false, over: '20', step: '10', most: '50', least: '' };
}

/** A rule the daemon keeps, as it is written again to be changed. */
export function ruleWritten(rule: Rule): WrittenRule {
  const said = (value: number | null | undefined) => (value === null || value === undefined ? '' : String(value));
  return { grows: rule.grows, against: rule.against, count: rule.count, came: rule.came, atLeast: Boolean(rule.at_least), over: said(rule.over), step: said(rule.step), most: said(rule.most), least: said(rule.least) };
}

/** The rule as the daemon is told it, or what is wrong with what was written that the page can say before anything is asked. What is wrong with it for the comparison (a count it does not have, a share already past its most) is the daemon's to say. */
export function ruleAsked(written: WrittenRule, ways: string[]): { rule: Rule } | { wrong: string } {
  if (!ways.includes(written.grows) || !ways.includes(written.against)) return { wrong: 'Choose the way that grows and the way it is held against among its ways.' };
  if (written.grows === written.against) return { wrong: `${written.grows} cannot be held against itself: choose another way.` };
  if (!written.count.trim() || !written.came.trim()) return { wrong: 'Say the count the rule goes by and what it came to that is held, such as verdict and failed.' };
  const whole = (value: string) => (/^\d+$/.test(value.trim()) ? Number(value) : null);
  const [over, step, most] = [whole(written.over), whole(written.step), whole(written.most)];
  if (over === null || step === null || most === null) return { wrong: 'How many are read first, the step and the most are whole numbers.' };
  const least = written.least.trim() === '' ? null : whole(written.least);
  if (least === null && written.least.trim() !== '') return { wrong: 'The least it goes back to is a whole number, or nothing for it to stay where it is.' };
  return { rule: { grows: written.grows, against: written.against, count: written.count.trim(), came: written.came.trim(), at_least: written.atLeast, over, step, most, ...(least === null ? {} : { least }) } };
}

/** Whether somebody who holds `mine` may give a comparison a rule, change it or take it away, as the daemon would let them: one that shares its things out and is going, by whoever may apply and holds what its starter held. */
export function mayRule(mine: string[], one: Pick<Comparison, 'asked' | 'ended' | 'held'>): boolean {
  return !one.asked && one.ended === null && covers(mine, 'blueprints:run') && (one.held ?? []).every((scope) => covers(mine, scope));
}
