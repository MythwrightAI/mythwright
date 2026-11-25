// A sandbox test of a blueprint, as the page has it: what is done to it,
// and what is made of what the daemon hands back.
//
// The daemon runs the text in a factory of its own and hands everything
// that came of it (`/api/sandboxes`). Nothing of what happened is worked
// out here: these are the acts somebody asks for as plain values, what
// each is sent as, the timeline the daemon hands put under the run each
// line is of, which tiles of the text's graph are lit how, and what
// differs between one run of the acts and the next. Nothing of the page
// is in it, so that its cases can read it.

import type { ActionSpec, Fields, GraphParam, Instance, Json } from './api.ts';
import type { Start } from './starts.ts';
import { baseStep } from './util.ts';

/** A stand-in that was asked: what it was asked, and what it answered once it has. */
export interface Asking {
  /** Which asking of the sandbox this is, from 1: what it is answered by. */
  id: number;
  action: string;
  /** What the action says it does, in the daemon's words. */
  summary: string;
  instance: string;
  blueprint: string;
  run: string;
  step: string;
  asked: Json;
  /** What a stand-in for this action answers where nobody says. */
  usual: Json;
  waiting: boolean;
  answered?: Json;
  failed?: string | null;
  by?: string;
}

/** One line of what happened, as the daemon hands it, in the order it came about. */
export interface Line {
  what: 'instance' | 'run' | 'step' | 'waits' | 'said';
  did?: string;
  instance?: string | null;
  run?: string | null;
  flow?: string | null;
  step?: string | null;
  blueprint?: string | null;
  key?: string | null;
  status?: string | null;
  error?: string | null;
  heard?: string | null;
  says?: string;
  does?: string;
  tolerated?: boolean;
  handed?: Json;
  gave?: Json;
  stood?: number[];
  topic?: string;
  by?: string | null;
  data?: Json;
  event?: string | null;
  until?: Json;
}

/** A table of the sandbox, with its rows. */
export interface Tabled {
  name: string;
  records: { id: string; data: Fields }[];
  fields?: Record<string, unknown> | { name: string }[];
}

/** A channel of the sandbox, with what was said in it. */
export interface Channelled {
  id: string;
  name: string;
  messages: { id: string; author_name: string; body: string }[];
}

/** A line of the made-up person's inbox. */
export interface Told {
  id: string;
  owner: string;
  why: string;
  about: string;
  says?: string;
  count: number;
}

/** How a sandbox stands: what every answer of the daemon's says. */
export interface Standing {
  sandbox: string;
  /** What this was, in the daemon's own sentence. */
  said: string;
  blueprint: string;
  project: string;
  clock: number;
  applying: boolean;
  waiting: Asking[];
  /** Why it was stopped, where it came to more than a sandbox may. */
  over: string | null;
  /** Each action that was stood in, with how many times, and the daemon's sentence for it. */
  stood: { action: string; times: number }[];
  stood_said: string;
  /** What can be done to its tables by hand, each called what the action that does it is called. */
  does: string[];
}

/** Everything that came of a sandbox. */
export interface Shown extends Standing {
  instances: Instance[];
  timeline: Line[];
  tables: Tabled[];
  channels: Channelled[];
  inbox: Told[];
  asked: Asking[];
  /** The made-up person who applied it. */
  somebody: string;
}

/** Something done to a sandbox once its text is applied, as somebody asked for it. */
export type Act =
  | { kind: 'say'; topic: string; data: Fields }
  | { kind: 'do'; action: string; with: Fields }
  | { kind: 'advance'; by: string }
  | { kind: 'answer'; asked: number; how: 'usual' | 'fails' | 'gives'; gives?: Json; why?: string }
  /** How stand-ins answer from here on: each waits to be asked, or answers at once what it usually answers (one that waits is then answered so). */
  | { kind: 'answers'; answers: Answers };

/** Whether a stand-in waits to be answered (`asked`), or answers at once what a stand-in for its action usually answers (`default`). */
export type Answers = 'asked' | 'default';

/** What a sandbox test is run with: what goes first, what the text is told, and what is done after. */
export interface Plan {
  /** Blueprints applied before the text, by name: what it needs to be there. */
  first: string[];
  /** What the text is told, each as it was written in its box. */
  told: Record<string, string>;
  /** How its stand-ins answer as it begins. */
  answers: Answers;
  acts: Act[];
}

/**
 * What the daemon says a blueprint needs to be there before it: for a
 * blueprint that a project is given as a start, the blueprints of what
 * that start cannot be had without (a fleet needs a board). A start's
 * blueprint may be a project's own text that calls the one that ships
 * under the same name (a project's `fleet` calls `std/fleet`), so the
 * text that ships is taken for that start too: the daemon says no more
 * of which text a start comes to.
 */
export function needsFirst(name: string | null, starts: Start[]): string[] {
  const last = name?.split('/').at(-1) ?? null;
  const start = starts.find((one) => one.blueprint === name) ?? starts.find((one) => !one.blueprint.includes('/') && one.blueprint === last);
  if (!start) return [];
  return start.needs.flatMap((need) => starts.filter((one) => one.name === need).map((one) => one.blueprint));
}

/** What making a sandbox for a text is asked as. */
export function making(source: string, project: string | null, first: string[], params: Fields, answers: Answers): Fields {
  return { source, ...(project ? { project } : {}), first, params, answers };
}

/**
 * What an act is sent as, as things stand; or why it cannot be sent. An
 * answer names a stand-in by which asking it was, and is for one that
 * waits: played again against a text that asks other things, the one it
 * was for may not be there.
 */
export function sent(act: Act, waiting: Asking[]): { body: Fields } | { wrong: string } {
  switch (act.kind) {
    case 'say':
      return { body: { say: { topic: act.topic, data: act.data } } };
    case 'do':
      return { body: { do: { action: act.action, with: act.with } } };
    case 'advance':
      return { body: { advance: { by: act.by } } };
    case 'answers':
      return { body: { answers: { answers: act.answers } } };
    case 'answer': {
      const asking = waiting.find((one) => one.id === act.asked);
      if (!asking) return { wrong: `No stand-in waits as the ${nth(act.asked)} thing asked in this run: the text asks other things now, so this answer was not given.` };
      if (act.how === 'fails') return { body: { answer: { asked: act.asked, fails: act.why?.trim() || 'It was told to fail.' } } };
      return { body: { answer: { asked: act.asked, gives: act.how === 'usual' ? asking.usual : (act.gives ?? {}) } } };
    }
  }
}

/** `1st`, `2nd`, `3rd`, `4th`. */
export function nth(count: number): string {
  const tens = count % 100;
  const ending = tens >= 11 && tens <= 13 ? 'th' : (['th', 'st', 'nd', 'rd'][count % 10] ?? 'th');
  return `${count}${ending}`;
}

/** An act in a line of words, for the list of what was done. */
export function actSays(act: Act): string {
  switch (act.kind) {
    case 'say':
      return `Said ${act.topic}`;
    case 'do':
      return `Did ${act.action}`;
    case 'advance':
      return `Moved the clock on ${act.by}`;
    case 'answers':
      return act.answers === 'default' ? 'Had every stand-in answer as usual from here on' : 'Had every stand-in wait to be answered from here on';
    case 'answer':
      return act.how === 'fails' ? `Answered the ${nth(act.asked)} stand-in: it fails` : act.how === 'usual' ? `Answered the ${nth(act.asked)} stand-in as usual` : `Answered the ${nth(act.asked)} stand-in`;
  }
}

/** An answer written out, read as JSON before it is sent; or why it does not read. */
export function answerRead(text: string): { value: Json } | { wrong: string } {
  if (text.trim() === '') return { value: {} };
  try {
    return { value: JSON.parse(text) as Json };
  } catch (err) {
    return { wrong: `That does not read as JSON, so it was not sent: ${(err as Error).message}` };
  }
}

/** One thing asked in a form made from what an action or an event says of itself. */
export interface Asked {
  name: string;
  /** How its box is drawn and read: words, a number, a switch, or JSON for a list or a mapping. */
  box: 'text' | 'number' | 'bool' | 'json';
  needed: boolean;
  help: string;
}

/** A project is not asked: a sandbox has one, and the daemon tells an action, and a text that asks, which. */
export const UNASKED = ['project'];

const boxOf = (type: string): Asked['box'] => (type === 'int' || type === 'number' ? 'number' : type === 'bool' ? 'bool' : type === 'list' || type === 'object' || type === 'any' ? 'json' : 'text');

/** What is asked for something done by hand, from what its action says it takes. */
export function asksOf(action: Pick<ActionSpec, 'args'>): Asked[] {
  return action.args.filter((arg) => !UNASKED.includes(arg.name)).map((arg) => ({ name: arg.name, box: boxOf(arg.type), needed: arg.required, help: arg.help }));
}

/** What is asked for an event said by hand, from what it says it carries. */
export function carries(fields: { name: string; type: string; always: boolean; help: string }[]): Asked[] {
  return fields.map((field) => ({ name: field.name, box: boxOf(field.type), needed: false, help: field.help }));
}

/**
 * What was written in a form's boxes as values: a box left empty says
 * nothing, a number is a number, and a list or a mapping is read as
 * JSON, or as the words it is where it does not read (a card's title).
 * Or what is wrong: something needed that is not there.
 */
export function valuesOf(asks: Asked[], written: Record<string, string>): { values: Fields } | { wrong: string } {
  const values: Fields = {};
  for (const ask of asks) {
    const text = (written[ask.name] ?? '').trim();
    if (text === '') {
      if (ask.needed) return { wrong: `\`${ask.name}\` is needed.` };
      continue;
    }
    if (ask.box === 'number') {
      const number = Number(text);
      if (!Number.isFinite(number)) return { wrong: `\`${ask.name}\` is a number, and \`${text}\` is not one.` };
      values[ask.name] = number;
    } else if (ask.box === 'bool') values[ask.name] = text === 'true';
    else if (ask.box === 'json') {
      try {
        values[ask.name] = JSON.parse(text) as Json;
      } catch {
        values[ask.name] = text;
      }
    } else values[ask.name] = text;
  }
  return { values };
}

/** What an instance of the sandbox is called: its blueprint, and its key where that is something else. */
export function called(instances: Pick<Instance, 'id' | 'blueprint' | 'key'>[], id: string | null | undefined): string {
  const one = instances.find((instance) => instance.id === id);
  if (!one) return '';
  return one.key && one.key !== one.blueprint ? `${one.blueprint} (${one.key})` : one.blueprint;
}

/** The lines of one run, one after another, under what it is a run of; or what was said for no run. */
export interface Group {
  /** The run, or nothing for what somebody said by hand and what is no run's. */
  run: string | null;
  instance: string | null;
  /** Its text's own steps (`main`), or a handler's (`on.<name>`). */
  flow: string | null;
  lines: Line[];
}

/**
 * The timeline under the run each line is of, in the order it came
 * about: a run is a group again each time something of it comes after
 * something of another. What says that an instance began or ended, and
 * what the factory says of itself, stays where it came; what somebody
 * said for no run stands apart.
 */
export function grouped(timeline: Line[]): Group[] {
  const groups: Group[] = [];
  for (const line of timeline) {
    const last = groups.at(-1);
    const passing = line.what === 'instance' || (line.what === 'said' && !line.run && line.by === 'system');
    if (passing && last) {
      last.lines.push(line);
      continue;
    }
    const run = passing ? null : (line.run ?? null);
    if (last && last.run === run) last.lines.push(line);
    else groups.push({ run, instance: run ? (line.instance ?? null) : null, flow: run ? (line.flow ?? null) : null, lines: [line] });
  }
  return groups;
}

/** How a step of the text came out, for its tile: said in a word as well as drawn. */
export type Lit = 'ok' | 'failed' | 'let' | 'skipped' | 'running' | 'stood';

/** What each way a tile is lit means, in the order they are listed under the graph. */
export const LIT: [Lit, string][] = [
  ['ok', 'ran'],
  ['stood', 'stood in'],
  ['skipped', 'passed over'],
  ['let', 'failed, and its run went on'],
  ['failed', 'failed'],
  ['running', 'under way'],
];

/** The instance of the sandbox that is the text itself: the last applied of its name with nothing above it. */
export function itself(shown: Pick<Shown, 'instances' | 'blueprint'>): Instance | null {
  return shown.instances.filter((one) => one.blueprint === shown.blueprint && !one.parent).at(-1) ?? null;
}

/**
 * Which tiles of the text's graph are lit, and how, by what its own
 * instance did: each step as it last came out, one a stand-in answered
 * for told apart from one that ran, and a handler that heard something.
 */
export function lit(timeline: Line[], instance: string | null): Record<string, Lit> {
  const tiles: Record<string, Lit> = {};
  if (!instance) return tiles;
  for (const line of timeline) {
    if (line.instance !== instance || !line.flow) continue;
    if (line.what === 'run' && line.did === 'started' && line.heard) tiles[`${line.flow}/@`] = 'ok';
    if (line.what !== 'step' || !line.step) continue;
    const stood = (line.stood ?? []).length > 0;
    const how: Lit =
      line.status === 'failed' ? (line.tolerated ? 'let' : 'failed') : line.status === 'skipped' ? 'skipped' : line.status === 'ok' ? (stood ? 'stood' : 'ok') : line.status === 'cancelled' ? 'skipped' : 'running';
    tiles[`${line.flow}/${baseStep(line.step)}`] = how;
  }
  return tiles;
}

/** How the test stands, in a sentence, with how it is to be read (`good`, `bad`, or waiting on somebody). */
export function outcome(shown: Shown): { how: 'good' | 'bad' | 'waits'; says: string } {
  if (shown.over) return { how: 'bad', says: shown.over };
  const failed = shown.instances.filter((one) => one.status === 'failed');
  if (failed.length > 0) {
    const first = failed[0]!;
    return { how: 'bad', says: `${called(shown.instances, first.id)} failed${first.error ? `: ${first.error}` : '.'}` };
  }
  if (shown.waiting.length > 0) {
    const count = shown.waiting.length;
    return { how: 'waits', says: count === 1 ? 'It waits: a stand-in was asked something and waits to be answered.' : `It waits: ${count} stand-ins were asked something and wait to be answered.` };
  }
  const own = itself(shown);
  if (!own) return { how: 'waits', says: 'The text has not been applied in the sandbox yet.' };
  if (own.status === 'active') return { how: 'good', says: 'Its steps went well and it is up, listening for what it hears.' };
  if (own.status === 'starting') return { how: 'waits', says: 'Its steps are under way, and wait for something: the clock, or an event.' };
  return { how: 'good', says: 'It ran to its end.' };
}

const many = (count: number, one: string, more: string) => (count === 1 ? `1 ${one}` : `${count} ${more}`);

/** How each step of each instance came out, by what the instance is called, its flow and the step. */
function steps(shown: Shown): Map<string, string> {
  const came = new Map<string, string>();
  for (const line of shown.timeline) {
    if (line.what !== 'step' || !line.step) continue;
    const stood = (line.stood ?? []).length > 0 && line.status === 'ok';
    const flow = line.flow === 'main' ? '' : ` (${(line.flow ?? '').replace('on.', 'when ')})`;
    came.set(`\`${line.step}\` of ${called(shown.instances, line.instance)}${flow}`, stood ? 'stood in' : line.status === 'skipped' ? 'passed over' : (line.status ?? ''));
  }
  return came;
}

/**
 * What differs between one run of the acts and the next, in sentences:
 * each step that came out another way, ran now and not before or the
 * other way round; each table with another count of rows; what was stood
 * in more or less often; and how each stands. One sentence where nothing
 * does.
 */
export function differs(before: Shown, now: Shown): string[] {
  const lines: string[] = [];
  const [was, is] = [steps(before), steps(now)];
  for (const [step, how] of is) {
    const then = was.get(step);
    if (then === undefined) lines.push(`${step} ran this time (${how}), and did not the time before.`);
    else if (then !== how) lines.push(`${step} was ${then} the time before, and is ${how} now.`);
  }
  for (const step of was.keys()) if (!is.has(step)) lines.push(`${step} ran the time before, and did not this time.`);
  const rows = (shown: Shown) => new Map(shown.tables.map((table) => [table.name, table.records.length]));
  const [had, has] = [rows(before), rows(now)];
  for (const name of new Set([...had.keys(), ...has.keys()])) {
    const [then, here] = [had.get(name), has.get(name)];
    if (then === undefined) lines.push(`The table \`${name}\` is there now, with ${many(here!, 'row', 'rows')}, and was not before.`);
    else if (here === undefined) lines.push(`The table \`${name}\` was there before, and is not now.`);
    else if (then !== here) lines.push(`The table \`${name}\` had ${many(then, 'row', 'rows')} before, and has ${here} now.`);
  }
  const times = (shown: Shown) => new Map(shown.stood.map((one) => [one.action, one.times]));
  const [stoodBefore, stoodNow] = [times(before), times(now)];
  for (const action of new Set([...stoodBefore.keys(), ...stoodNow.keys()])) {
    const [then, here] = [stoodBefore.get(action) ?? 0, stoodNow.get(action) ?? 0];
    if (then !== here) lines.push(`\`${action}\` was stood in ${many(then, 'time', 'times')} before, and ${many(here, 'time', 'times')} now.`);
  }
  const [then, here] = [outcome(before).says, outcome(now).says];
  if (then !== here) lines.push(`Before: ${then} Now: ${here}`);
  return lines.length > 0 ? lines : ['Nothing differs from the run before: every step came out as it did, and the tables hold as many rows.'];
}

/**
 * What a text is told, as values, from what is written in its boxes:
 * nothing for a box left empty, a switch as a switch, words as words, and
 * anything else read as JSON. Which project it is for is the daemon's to
 * say. Or what is wrong: something needed and not said, or not of its kind.
 */
export function paramsOf(params: Pick<GraphParam, 'name' | 'type' | 'required'>[], told: Record<string, string>): { params: Fields } | { wrong: string } {
  const values: Fields = {};
  for (const param of params) {
    if (UNASKED.includes(param.name)) continue;
    const text = told[param.name] ?? '';
    if (text === '') {
      if (param.required) return { wrong: `\`${param.name}\` is needed: say it under What it is told.` };
      continue;
    }
    try {
      values[param.name] = param.type === 'string' || param.type === 'duration' ? text : param.type === 'bool' ? text === 'true' : (JSON.parse(text) as Json);
    } catch {
      if (param.type === 'any') values[param.name] = text;
      else return { wrong: `\`${param.name}\` should be a ${param.type}.` };
    }
  }
  return { params: values };
}
