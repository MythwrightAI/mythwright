// What to offer at a place in a blueprint, and what to say about a word in
// one.
//
// Nothing here knows the language. The fields come from the daemon's
// description of it, the actions and their arguments from the device, the
// names of steps and params from the text itself, and blueprints, agents
// and devices from what the client already knows about (`World`).

import type { ActionSpec, BlueprintLanguage, Described, EventSpec, LanguageField, LanguageShape, ValueKind, Within, WithinKey } from '../api.ts';
import { declared, entriesOf, entriesUnder, placeAt, stepsAround, tokenAt } from './read.ts';
import type { Entry, Reading } from './read.ts';
import { shapeOf } from './scan.ts';
import type { Frame, Place } from './scan.ts';
import { actionId, eventId, functionId, nameId, wordId } from '../reference.ts';
import { afterOf } from './told.ts';
import type { ToldLanguage } from './told.ts';
import { inside, namedBeside } from './within.ts';
import type { Inside } from './within.ts';

/** What the client knows that is in neither the language nor the text. */
export interface World {
  blueprints: Described[];
  agents: string[];
  devices: string[];
}

/** What something is, said to the person choosing or pointing at it. */
export interface Note {
  says: string;
  /** Where it is written, for a word that means what it does only there: a key inside an argument. */
  where?: string;
  /** Things it has, each by name with what it is and, where it has one, its type. */
  lists?: { title: string; rows: Row[] }[];
  /** A whole blueprint written with it, as the daemon says it is shown by. */
  example?: string;
}

export interface Row {
  name: string;
  type?: string;
  says: string;
}

export interface Suggestion {
  label: string;
  /** A few words beside it. */
  detail?: string;
  note?: Note;
  /** What is put in, when it is not the label. */
  insert?: string;
  /** Where the caret goes within what was put in; after it when not said. */
  caret?: number;
  /** What comes next has suggestions of its own: offer again once this is in. */
  again?: boolean;
  kind: 'field' | 'argument' | 'action' | 'blueprint' | 'name' | 'word' | 'function' | 'member';
}

export interface Suggestions {
  /** Where what is being replaced starts. */
  from: number;
  options: Suggestion[];
  /** What may go on being typed without asking again. */
  still: RegExp;
  /**
   * Worth offering unasked. An empty line that could take a field is not:
   * it is offered its fields when they are asked for, or once a letter is
   * typed.
   */
  ready: boolean;
}

export interface Explained {
  from: number;
  to: number;
  title: string;
  note: Note;
  /** The entry of the Reference that says more of it, by its name (`reference.ts`), where it has one. */
  entry?: string;
}

const WORD = /^[\w.\/@-]*$/;
const NAME = /^\w*$/;

function pad(columns: number): string {
  return ' '.repeat(Math.max(0, columns));
}

function actionNote(spec: ActionSpec): Note {
  const lists: NonNullable<Note['lists']> = [];
  if (spec.args.length > 0) {
    lists.push({
      title: spec.open ? 'Takes, among others' : 'Takes',
      rows: spec.args.map((arg) => ({ name: arg.name, type: arg.required ? `${arg.type}, needed` : arg.type, says: arg.help })),
    });
  }
  if (spec.outputs.length > 0) lists.push({ title: 'Gives back', rows: spec.outputs.map(([name, says]) => ({ name, says })) });
  if (spec.capability) lists.push({ title: 'Needs', rows: [{ name: spec.capability, says: 'in `permissions`, when any are listed' }] });
  return { says: spec.summary, lists };
}

function eventNote(spec: EventSpec): Note {
  const lists: NonNullable<Note['lists']> = [];
  if (spec.fields.length > 0) {
    lists.push({
      title: spec.open ? 'Carries, among others' : 'Carries',
      rows: spec.fields.map((field) => ({ name: field.name, type: field.always ? field.type : `${field.type}, sometimes`, says: field.help })),
    });
  }
  return { says: spec.summary, lists };
}

/** Whether `topic` is one `pattern` takes: `*` is one word, `**` any number. */
export function takes(pattern: string, topic: string): boolean {
  const at = (pattern: string[], topic: string[]): boolean => {
    if (pattern.length === 0) return topic.length === 0;
    const [word, ...rest] = pattern;
    if (word === '**') return topic.some((_, skip) => at(rest, topic.slice(skip))) || at(rest, []);
    return topic.length > 0 && (word === '*' || word === topic[0]) && at(rest, topic.slice(1));
  };
  return at(pattern.split('.'), topic.split('.'));
}

/**
 * The events the handler around a place hears: the ones its `event:`
 * takes, of those the daemon describes. None for a handler on a timer, on
 * a topic blueprints say to each other, or outside any handler.
 */
function heard(reading: Reading, frames: Frame[], language: BlueprintLanguage): EventSpec[] {
  for (let at = frames.length - 1; at >= 0; at--) {
    const frame = frames[at]!;
    if (shapeAt(frame, language)?.fields.find((field) => field.name === 'event')?.value.kind !== 'topic') continue;
    const topic = entriesOf(reading.lines, frame).find((entry) => entry.key === 'event')?.value;
    if (!topic) return [];
    return (language.events ?? []).filter((spec) => spec.kept && takes(topic, spec.topic));
  }
  return [];
}

/** What the events a handler hears carry, each field once, with which of them carry it when not all do. */
function carried(events: EventSpec[]): { name: string; type: string; says: string }[] {
  const found = new Map<string, { name: string; type: string; says: string; by: string[] }>();
  for (const spec of events) {
    for (const field of spec.fields) {
      const one = found.get(field.name) ?? { name: field.name, type: field.always ? field.type : `${field.type}, sometimes`, says: field.help, by: [] };
      one.by.push(spec.topic);
      found.set(field.name, one);
    }
  }
  return [...found.values()].map((one) => ({
    name: one.name,
    type: one.type,
    says: one.by.length === events.length ? sentence(one.says) : `${sentence(one.says)} Carried by ${one.by.join(', ')}.`,
  }));
}

function blueprintNote(one: Described): Note {
  const lists: NonNullable<Note['lists']> = [];
  if (one.params.length > 0) {
    lists.push({
      title: 'Started with',
      rows: one.params.map((param) => ({ name: param.name, type: param.required ? `${param.type}, needed` : param.type, says: param.description ?? '' })),
    });
  }
  if (one.outputs.length > 0) lists.push({ title: 'Gives back', rows: one.outputs.map((name) => ({ name, says: '' })) });
  return { says: one.description ?? 'A blueprint.', lists };
}

function fieldNote(field: LanguageField): Note {
  const lists = field.value.kind === 'one_of' ? [{ title: 'One of', rows: field.value.of.map((choice) => ({ name: choice.name, says: choice.help })) }] : undefined;
  return { says: field.help, lists };
}

/** The shape whose fields a mapping has, if it has a closed set of them. */
function shapeAt(frame: Frame, language: BlueprintLanguage): LanguageShape | undefined {
  return frame.holds.is === 'shape' || frame.holds.is === 'list_of' ? shapeOf(language, frame.holds.shape) : undefined;
}

/**
 * What a `with` is the arguments of: the action or blueprint named beside
 * it, in the mapping it belongs to.
 */
function target(reading: Reading, frames: Frame[], at: number, language: BlueprintLanguage, world: World): { action?: ActionSpec; blueprint?: Described } {
  const named = namedBeside(reading.lines, frames, at, language);
  if (named.action) return { action: language.actions.find((action) => action.name === named.action) };
  if (named.blueprint) return { blueprint: world.blueprints.find((one) => one.name === named.blueprint) };
  return {};
}

/** What an action says is written inside something, as lists under what is said of it. */
function withinLists(within: Within | undefined): NonNullable<Note['lists']> {
  if (!within) return [];
  const lists: NonNullable<Note['lists']> = [];
  const keys = within.keys ?? [];
  if (keys.length > 0) lists.push({ title: within.each ? 'Each takes' : 'Takes', rows: keys.map((key) => ({ name: key.name, type: key.type, says: key.help })) });
  if (within.words?.length) lists.push({ title: 'Each one of', rows: within.words.map((word) => ({ name: word.name, says: word.help })) });
  return lists;
}

/** What is said of a key written inside an argument. */
function keyNote(key: WithinKey, where?: string): Note {
  const lists: NonNullable<Note['lists']> = [];
  if (key.words?.length) lists.push({ title: 'One of', rows: key.words.map((word) => ({ name: word.name, says: word.help })) });
  return { says: sentence(key.help), where, lists: [...lists, ...withinLists(key.within)] };
}

/** Where a place inside an argument is, in a sentence: the argument, its action, and what the mapping there is. */
function whereOf(at: Inside): string {
  const inArgument = `In \`${at.argument}\` of \`${at.action}\``;
  // A mapping that is the argument itself, or a key of it, is said by that already.
  const what = at.of && at.of !== `\`${at.argument}\`` ? `: ${at.of.startsWith('`') ? `under ${at.of}` : at.of}` : '';
  return `${inArgument}${what}.`;
}

/** Kinds whose values can be offered, so that a field of one is worth following with its suggestions. */
const OFFERED = new Set(['action', 'decision', 'blueprint', 'agent', 'device', 'one_of', 'flag', 'topic']);

/** What to put in for a field, so that the caret lands where its value goes. */
function opening(field: LanguageField, column: number, inline: boolean, language: BlueprintLanguage): Pick<Suggestion, 'insert' | 'caret' | 'again'> {
  const name = field.name;
  const kind = field.value.kind;
  const beside = { insert: `${name}: `, again: OFFERED.has(kind) };
  const below = `${name}:\n${pad(column + 2)}`;
  // Inside `{ }` everything stays on the line.
  if (inline) return beside;
  switch (field.value.kind) {
    case 'shape':
      // `retry: 3` is commoner than the mapping it stands for.
      return shapeOf(language, field.value.of)?.short ? beside : { insert: below, again: true };
    case 'map_of':
    case 'data':
      return { insert: below };
    case 'arguments':
      return { insert: below, again: true };
    case 'list_of':
      return { insert: `${below}- `, again: true };
    case 'steps':
    case 'capabilities':
      return { insert: `${name}: []`, caret: name.length + 3, again: true };
    case 'expression':
      return { insert: `${name}: \${  }`, caret: name.length + 5, again: true };
    default:
      return beside;
  }
}

/** The fields a mapping may still be given, each ready to be put in. */
function fields(reading: Reading, place: Extract<Place, { at: 'key' }>, language: BlueprintLanguage, world: World): Suggestion[] {
  const frames = place.frames;
  const top = frames[frames.length - 1]!;
  const inline = top.flow === '{';
  const column = place.from + (place.dash ? 2 : 0);
  const lead = place.dash ? '- ' : '';
  // What the mapping has already, not counting the line being written. A
  // list that is waiting for its dash has no item yet, so nothing is had.
  const has = place.dash ? [] : entriesOf(reading.lines, top).filter((entry) => inline || entry.line !== reading.line).map((entry) => entry.key);
  const withLead = (made: Pick<Suggestion, 'insert' | 'caret' | 'again'>, label: string): Pick<Suggestion, 'insert' | 'caret' | 'again'> => ({
    ...made,
    insert: `${lead}${made.insert ?? label}`,
    caret: made.caret === undefined ? undefined : made.caret + lead.length,
  });

  const shape = shapeAt(top, language);
  if (shape) {
    const taken = (name: string) => has.includes(name) || shape.alternatives.some((group) => group.includes(name) && group.some((other) => has.includes(other)));
    return shape.fields
      .filter((field) => !taken(field.name))
      .map((field) => ({
        label: field.name,
        detail: afterOf(field),
        note: fieldNote(field),
        kind: 'field' as const,
        ...withLead(opening(field, column, inline, language), field.name),
      }));
  }
  // Under a handler's `for:`, what its events carry is what one may be addressed by.
  if (top.holds.is === 'data' && top.key === 'for') {
    return carried(heard(reading, frames.slice(0, -1), language))
      .filter((field) => !has.includes(field.name))
      .map((field) => ({ label: field.name, detail: field.type, note: { says: field.says }, insert: `${field.name}: `, kind: 'argument' as const }));
  }
  if (top.holds.is === 'arguments') {
    const { action, blueprint } = target(reading, frames, frames.length - 1, language, world);
    const args = action
      ? action.args.map((arg) => ({ name: arg.name, type: arg.type, required: arg.required, help: arg.help }))
      : (blueprint?.params ?? []).map((param) => ({ name: param.name, type: param.type, required: param.required, help: param.description ?? '' }));
    return args
      .filter((arg) => !has.includes(arg.name))
      .sort((a, b) => Number(b.required) - Number(a.required))
      .map((arg) => ({
        label: arg.name,
        detail: arg.required ? `${arg.type}, needed` : arg.type,
        note: arg.help ? { says: sentence(arg.help) } : undefined,
        insert: `${arg.name}: `,
        again: arg.type === 'bool',
        kind: 'argument' as const,
      }));
  }
  // Inside an argument: the keys its action says are taken there. A key
  // it does not name is never taken for a mistake; nothing is offered
  // where nothing is said.
  const at = inside(reading, language);
  if (at) {
    return at.keys
      .filter((key) => !has.includes(key.name))
      .map((key) => ({
        label: key.name,
        detail: key.type,
        note: keyNote(key),
        insert: `${key.name}: `,
        again: (key.words?.length ?? 0) > 0 || key.type === 'bool',
        kind: 'argument' as const,
      }));
  }
  return [];
}

/** The capabilities there are: each area, then each thing that may be done in it. */
function capabilities(language: BlueprintLanguage): Suggestion[] {
  const needed = [...new Set(language.actions.map((action) => action.capability).filter((one): one is string => !!one))].sort();
  const areas = [...new Set(needed.map((one) => one.split(':')[0]!))];
  const uses = (grant: string) =>
    language.actions.filter((action) => action.capability === grant || action.capability?.split(':')[0] === grant).map((action) => action.name);
  const row = (grant: string, says: string): Suggestion => {
    const actions = uses(grant);
    const shown = actions.slice(0, 6).join(', ') + (actions.length > 6 ? ', and more' : '');
    return { label: grant, note: { says: `${says} Lets it use: ${shown}.` }, kind: 'word' };
  };
  return [
    ...areas.map((area) => row(area, `Everything under \`${area}\`.`)),
    ...needed.filter((one) => one.includes(':')).map((one) => row(one, `Only \`${one}\`.`)),
  ];
}

const DECIDES = 'Decides what happens to the step that failed.';

/** What may be written as a value of a kind. */
function words(reading: Reading, kind: ValueKind, key: string, frames: Frame[], language: BlueprintLanguage, world: World): Suggestion[] {
  switch (kind.kind) {
    case 'action':
    case 'decision': {
      const actions = language.actions.map(
        (spec): Suggestion => ({ label: spec.name, detail: spec.capability ?? undefined, note: actionNote(spec), kind: 'action' }),
      );
      const decisions = kind.kind === 'decision' ? language.decisions.map((word): Suggestion => ({ label: word, detail: 'decides', note: { says: DECIDES }, kind: 'word' })) : [];
      return [...decisions, ...actions];
    }
    case 'blueprint':
      return world.blueprints.map((one) => ({ label: one.name, detail: one.params.some((param) => param.required) ? 'needs params' : undefined, note: blueprintNote(one), kind: 'blueprint' }));
    case 'agent':
      return world.agents.map((name) => ({ label: name, note: { says: 'An agent here.' }, kind: 'name' }));
    case 'device':
      return ['local', ...world.devices].map((name) => ({ label: name, note: { says: name === 'local' ? 'This device.' : 'A device of this fleet.' }, kind: 'name' }));
    case 'one_of':
      return kind.of.map((choice) => ({ label: choice.name, note: { says: choice.help }, kind: 'word' }));
    case 'flag':
      return ['true', 'false'].map((word) => ({ label: word, kind: 'word' }));
    case 'steps': {
      const { steps, own } = stepsAround(reading.lines, frames);
      const top = frames[frames.length - 1]!;
      const listed = top.flow === '[' ? (top.entries ?? []).map(([, value]) => value) : [];
      return steps
        .filter((step) => step.key !== own && !listed.includes(step.key))
        .map((step) => ({ label: step.key, detail: doing(reading.lines, step), kind: 'name' }));
    }
    case 'capabilities':
      return capabilities(language);
    case 'topic':
      // What a handler can hear: an event that is only passed on reaches none.
      return (language.events ?? [])
        .filter((spec) => spec.kept)
        .map((spec): Suggestion => ({ label: spec.topic, note: eventNote(spec), kind: 'name' }));
    case 'expression':
      return [{ label: '${ }', note: { says: 'An expression, worked out when the step runs.' }, insert: '${  }', caret: 3, again: true, kind: 'word' }];
    default: {
      // The value of an argument that is true or false.
      const top = frames[frames.length - 1]!;
      const { action, blueprint } = target(reading, frames, frames.length - 1, language, world);
      const type = top.holds.is === 'arguments' ? (action?.args.find((arg) => arg.name === key)?.type ?? blueprint?.params.find((param) => param.name === key)?.type) : undefined;
      if (type === 'bool') return ['true', 'false'].map((word) => ({ label: word, kind: 'word' }));
      // The value of a key inside an argument: the few words its action says it is one of.
      const at = reading.place.at === 'value' ? inside(reading, language) : null;
      if (at && at.words.length > 0) return at.words.map((word) => ({ label: word.name, note: word.help ? { says: sentence(word.help) } : undefined, kind: 'word' }));
      return at?.described?.type === 'bool' ? ['true', 'false'].map((word) => ({ label: word, kind: 'word' })) : [];
    }
  }
}

/** What a step does, in a few words: the action or blueprint it names. */
function doing(lines: string[], step: Entry): string | undefined {
  const inside = entriesUnder(lines, step);
  const verb = inside.find((entry) => entry.key === 'do' || entry.key === 'call' || entry.key === 'spawn');
  return verb ? `${verb.key}: ${verb.value}` : undefined;
}

/** What a step gives back, as far as can be known without running it. */
function outputs(lines: string[], step: Entry | undefined, language: BlueprintLanguage, world: World): [string, string][] {
  if (!step) return [];
  for (const entry of entriesUnder(lines, step)) {
    if (entry.key === 'do') return language.actions.find((action) => action.name === entry.value)?.outputs ?? [];
    if (entry.key === 'call') return (world.blueprints.find((one) => one.name === entry.value)?.outputs ?? []).map((name) => [name, '']);
  }
  return [];
}

/** The path being written at the end of an expression: `steps.build.ou` is `['steps', 'build']` and `ou`. */
function path(typed: string): { before: string[]; partial: string } | null {
  // Inside a string in the expression nothing is a name.
  let quote = '';
  for (let at = 0; at < typed.length; at++) {
    const ch = typed[at]!;
    if (quote) {
      if (ch === '\\') at++;
      else if (ch === quote) quote = '';
    } else if (ch === '"' || ch === "'") quote = ch;
  }
  if (quote) return null;
  const tail = /[A-Za-z0-9_.]*$/.exec(typed)![0];
  if (/^[0-9.]/.test(tail)) return null;
  const parts = tail.split('.');
  return { before: parts.slice(0, -1), partial: parts[parts.length - 1]! };
}

/** What may follow a path in an expression. */
function members(reading: Reading, before: string[], frames: Frame[], language: BlueprintLanguage, world: World): Suggestion[] {
  const [root, second, third] = before;
  const member = (name: string, says: string): Suggestion => ({ label: name, note: says ? { says } : undefined, kind: 'member' });
  if (root === undefined) {
    return [
      ...language.names.map((name): Suggestion => ({ label: name.name, note: { says: sentence(name.meaning) }, kind: 'name' })),
      ...language.functions.map((one): Suggestion => ({ label: one.name, detail: 'function', note: { says: one.usage }, insert: `${one.name}(`, kind: 'function' })),
    ];
  }
  const { steps, own } = stepsAround(reading.lines, frames);
  if (root === 'params' || root === 'state') {
    if (before.length > 1) return [];
    return declared(reading.lines, root).map((entry) => ({ label: entry.key, detail: entry.value || undefined, kind: 'name' }));
  }
  if (root === 'steps') {
    if (before.length === 1) {
      // A step may read the steps above it, which are the ones it follows.
      const at = steps.findIndex((step) => step.key === own);
      return (at < 0 ? steps : steps.slice(0, at)).map((step) => ({ label: step.key, detail: doing(reading.lines, step), kind: 'name' }));
    }
    if (before.length === 2) return language.step.map((one) => member(one.name, sentence(one.help)));
    if (before.length === 3 && third === 'out') return outputs(reading.lines, steps.find((step) => step.key === second), language, world).map(([name, says]) => member(name, sentence(says)));
    return [];
  }
  if (root === 'self' && before.length === 2 && second === 'out') {
    return outputs(reading.lines, steps.find((step) => step.key === own), language, world).map(([name, says]) => member(name, sentence(says)));
  }
  // What the event that fired this handler carries.
  if (root === 'event' && before.length === 2 && second === 'data') {
    return carried(heard(reading, frames, language)).map((field) => ({ label: field.name, detail: field.type, note: { says: field.says }, kind: 'member' as const }));
  }
  if (before.length === 1) return (language.names.find((name) => name.name === root)?.members ?? []).map((one) => member(one.name, sentence(one.help)));
  return [];
}

function sentence(text: string): string {
  if (!text) return '';
  const said = text[0]!.toUpperCase() + text.slice(1);
  return /[.!?]$/.test(said) ? said : `${said}.`;
}

/** What to offer at an offset in a blueprint's text, if anything. */
export function suggest(text: string, offset: number, language: BlueprintLanguage | null, world: World): Suggestions | null {
  if (!language) return null;
  const reading = placeAt(text, offset, language);
  const place = reading.place;
  const at = (column: number) => reading.lineStart + column;
  const some = (from: number, options: Suggestion[], still: RegExp, ready = true): Suggestions | null =>
    options.length > 0 ? { from, options, still, ready } : null;
  switch (place.at) {
    case 'nowhere':
      return null;
    case 'key': {
      // Where a bare word may stand for the mapping, it is offered beside the fields.
      const bare = place.orValue ? words(reading, place.orValue, '', place.frames, language, world) : [];
      // A list of plain names waiting for its dash: the names, each as an item.
      const top = place.frames[place.frames.length - 1]!;
      const items = place.dash && top.holds.is === 'list' ? words(reading, top.holds.of, top.key, place.frames, language, world).map((one) => ({ ...one, insert: `- ${one.label}` })) : [];
      const ready = place.typed !== '' || place.orValue !== null || top.flow === '{';
      return some(at(place.from), [...fields(reading, place, language, world), ...bare, ...items], WORD, ready);
    }
    case 'value': {
      const options = words(reading, place.kind, place.key, place.frames, language, world);
      // Right after the colon, what is put in brings its own space.
      const spaced = place.tight ? options.map((one) => ({ ...one, insert: ` ${one.insert ?? one.label}`, caret: one.caret === undefined ? undefined : one.caret + 1 })) : options;
      return some(at(place.from), spaced, WORD);
    }
    case 'expression': {
      const written = path(place.typed);
      if (!written) return null;
      const from = at(place.from + place.typed.length - written.partial.length);
      return some(from, members(reading, written.before, place.frames, language, world), NAME);
    }
  }
}

/** What the word at an offset is, for somebody pointing at it. */
export function explain(text: string, offset: number, language: BlueprintLanguage | null, world: World): Explained | null {
  if (!language) return null;
  const found = tokenAt(text, offset, language);
  if (!found) return null;
  const { token, lineStart } = found;
  // The place is read as if the word had just been written.
  const reading = placeAt(text, lineStart + token.to, language);
  const place = reading.place;
  const word = reading.lines[reading.line]!.slice(token.from, token.to);
  const told = (title: string, note: Note | undefined, entry?: string): Explained | null => {
    if (!note) return null;
    const example = exampleAt(entry, language);
    return { from: lineStart + token.from, to: lineStart + token.to, title, note: example ? { ...note, example } : note, ...(entry ? { entry } : {}) };
  };
  const known = (name: string) => (language.actions.some((action) => action.name === name) ? actionId(name) : undefined);

  if (place.at === 'key') {
    const top = place.frames[place.frames.length - 1]!;
    const field = shapeAt(top, language)?.fields.find((one) => one.name === word);
    if (field) return told(`${word}: ${afterOf(field)}`, fieldNote(field), wordId(shapeAt(top, language)!.name, word));
    if (place.orValue?.kind === 'action' || place.orValue?.kind === 'decision') return told(word, describeAction(word, language), known(word));
    if (top.holds.is === 'data' && top.key === 'for') {
      const field = carried(heard(reading, place.frames.slice(0, -1), language)).find((one) => one.name === word);
      if (field) return told(`${word}: ${field.type}`, { says: field.says });
    }
    if (top.holds.is === 'arguments') {
      const { action, blueprint } = target(reading, place.frames, place.frames.length - 1, language, world);
      const arg = action?.args.find((one) => one.name === word);
      if (arg) return told(`${word}: ${arg.type}${arg.required ? ', needed' : ''}`, { says: sentence(arg.help), lists: withinLists(arg.within) }, actionId(action!.name, word));
      const param = blueprint?.params.find((one) => one.name === word);
      if (param) return told(`${word}: ${param.type}${param.required ? ', needed' : ''}`, { says: param.description ?? `A param of ${blueprint!.name}.` });
    }
    // A key inside an argument, where its action says what it is; one it does not name is said nothing of.
    const at = inside(reading, language);
    if (at?.described) return told(`${word}: ${at.described.type}`, keyNote(at.described, whereOf(at)), actionId(at.action, ...(at.said ?? []), word));
    return null;
  }
  if (place.at === 'value') {
    if (place.kind.kind === 'action' || place.kind.kind === 'decision') return told(word, describeAction(word, language), known(word));
    if (place.kind.kind === 'blueprint') {
      const one = world.blueprints.find((known) => known.name === word);
      return told(word, one && blueprintNote(one));
    }
    if (place.kind.kind === 'one_of') {
      const choice = place.kind.of.find((one) => one.name === word);
      // The word is said where the field it is one of is.
      const shape = shapeAt(place.frames[place.frames.length - 1]!, language);
      return told(word, choice && { says: sentence(choice.help) }, shape && wordId(shape.name, place.key));
    }
    if (place.kind.kind === 'topic') {
      const spec = (language.events ?? []).find((one) => one.topic === word);
      return told(word, spec && eventNote(spec), eventId(word));
    }
    // One of the few words a key inside an argument may be, where it has a sentence.
    const at = inside(reading, language);
    const one = at?.words.find((known) => known.name === word);
    if (at && one?.help) {
      const of = at.described ? `What \`${at.key}\` may be` : 'What it may be';
      return told(word, { says: sentence(one.help), where: `${of}. ${whereOf(at)}` }, at.said ? actionId(at.action, ...at.said, ...(at.described ? [at.key] : [])) : undefined);
    }
    return null;
  }
  if (place.at === 'expression') {
    const written = path(place.typed);
    if (!written || written.partial !== word) return null;
    if (written.before.length === 0) {
      const usage = language.functions.find((one) => one.name === word)?.usage;
      if (usage && token.type === 'function') return told(word, { says: usage }, functionId(word));
    }
    const one = members(reading, written.before, place.frames, language, world).find((option) => option.label === word);
    const says = one?.note?.says ?? one?.detail;
    // A name an expression starts with, and what follows one, are said where the name is.
    const root = written.before[0] ?? word;
    return told([...written.before, word].join('.'), says ? { says } : undefined, language.names.some((name) => name.name === root) ? nameId(root) : undefined);
  }
  return null;
}

/**
 * The example of what an entry of the Reference names, where the daemon
 * handed one: a word, an action, a name an expression starts with, a
 * function. An argument, a key inside one and an event have none.
 */
export function exampleAt(entry: string | undefined, language: BlueprintLanguage): string | undefined {
  const told = language as ToldLanguage;
  const [kind, first = '', second, ...more] = (entry ?? '').split('/');
  if (more.length > 0) return undefined;
  if (kind === 'word' && second !== undefined) return told.shapes.find((shape) => shape.name === first)?.fields.find((field) => field.name === second)?.example;
  if (kind === 'word') return told.shapes.find((shape) => shape.name === first)?.example;
  if (second !== undefined) return undefined;
  if (kind === 'action') return told.actions.find((action) => action.name === first)?.example;
  if (kind === 'name') return told.names.find((name) => name.name === first)?.example;
  if (kind === 'function') return told.functions.find((one) => one.name === first)?.example ?? undefined;
  return undefined;
}

function describeAction(name: string, language: BlueprintLanguage): Note | undefined {
  if (language.decisions.includes(name)) return { says: DECIDES };
  const spec = language.actions.find((action) => action.name === name);
  return spec && actionNote(spec);
}
