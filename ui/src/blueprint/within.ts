// What is written inside a step's `with:`, and what its action says of it.
//
// The language knows a step's arguments by name and no further: what is
// inside one (the fields of a table, each with its `type`; a health
// check's `tcp`) is the action's to read. An action may say what it reads
// there (`within` of an argument, as the daemon hands it out), and this
// finds, for a place in a text, which argument it is inside, by which keys,
// and what was said of that.
//
// It describes and never judges. A key the action does not name is found
// with nothing said of it: the action is what takes or refuses it when it
// runs, and a text may be older than what the action now says.

import type { ActionSpec, BlueprintLanguage } from '../api.ts';
import { entriesOf, placeAt } from './read.ts';
import type { Reading } from './read.ts';
import { shapeOf } from './scan.ts';
import type { Frame } from './scan.ts';

/** A word that may be written, and what it means where that is worth saying. */
export interface WithinWord {
  name: string;
  help: string;
}
/** One key of a mapping an action reads. */
export interface WithinKey {
  name: string;
  type: string;
  help: string;
  /** Where it is one of a few words: those words. */
  words?: WithinWord[];
  /** What it holds in turn, where that is described. */
  within?: Within;
}
/**
 * What is written inside an argument, as its action says it: the keys a
 * mapping takes, or with `each` the keys each thing it holds by a name of
 * its writer's choosing takes. It describes and decides nothing: a key it
 * does not name is one it says nothing of.
 */
export interface Within {
  /** What each thing held by a name is, as it reads after `is`. */
  each?: string;
  keys?: WithinKey[];
  /** Where each such thing is a word and no mapping: the few it is one of. */
  words?: WithinWord[];
}
/** An argument of an action, as it is handed out. */
type ArgSpec = ActionSpec['args'][number];

/** A place inside a step's `with:`. */
export interface Inside {
  /** What the mapping that has the `with:` is known by: a step's key; empty for one with none, such as a hook. */
  step: string;
  /** The action the arguments are of. */
  action: string;
  /** The argument the place is inside. */
  argument: string;
  /** The keys from the argument down to the mapping the place is in, names of the writer's choosing among them. */
  path: string[];
  /** Whether what is written there is a key of that mapping, or the value of one. */
  at: 'key' | 'value';
  /** The key: the one being written or pointed at, or the one the value is of. */
  key: string;
  /**
   * The keys the action says, from the argument down to the mapping the place is in: `path` without the names of
   * the writer's own, which is how what is said of the place is named (`reference.ts`). Nothing where nothing is said.
   */
  said: string[] | null;
  /** What the mapping is, as the action says it: `a field of the table`, or the key it is the value of. Empty where nothing is said. */
  of: string;
  /** The keys the mapping takes, as the action says them: none where it says none, or where its keys are names of their writer's choosing. */
  keys: WithinKey[];
  /** What is said of `key`, where the action names it. */
  described: WithinKey | null;
  /** At a value: the few words it may be, where it is one of a few. */
  words: WithinWord[];
}

/**
 * What a `with` is the arguments of, by name: the action or the blueprint
 * named beside it, in the mapping it belongs to.
 */
export function namedBeside(lines: string[], frames: Frame[], at: number, language: BlueprintLanguage): { action?: string; blueprint?: string } {
  const frame = frames[at];
  if (!frame || frame.holds.is !== 'arguments') return {};
  if (frame.holds.of) return { action: frame.holds.of };
  const owner = frames[at - 1];
  if (!owner) return {};
  const shape = owner.holds.is === 'shape' || owner.holds.is === 'list_of' ? shapeOf(language, owner.holds.shape) : undefined;
  for (const entry of entriesOf(lines, owner)) {
    const kind = shape?.fields.find((field) => field.name === entry.key)?.value.kind;
    if (kind === 'action' || kind === 'decision') return { action: entry.value };
    if (kind === 'blueprint') return { blueprint: entry.value };
  }
  return {};
}

/**
 * Where the mappings a place is in say it is: which of them is the
 * nearest `with:`, and the keys from there down to the innermost. Nothing
 * for a place in no `with:`.
 */
export function pathTo(frames: Frame[]): { at: number; keys: string[] } | null {
  let at = -1;
  frames.forEach((frame, index) => {
    if (frame.holds.is === 'arguments') at = index;
  });
  if (at < 0) return null;
  const keys: string[] = [];
  for (let index = at + 1; index < frames.length; index++) {
    const frame = frames[index]!;
    // An item of a list is said as its list is. On lines of its own it
    // has no key; inside `[ ]` it is given the list's, a second time.
    if (frame.key === '' || frames[index - 1]!.flow === '[') continue;
    keys.push(frame.key);
  }
  return { at, keys };
}

/** What an action says a mapping holds: the mapping, and whether its keys are names of their writer's choosing. */
interface Held {
  argument: ArgSpec;
  within: Within;
  named: boolean;
  of: string;
  /** The keys that were gone through, the argument first: the names of things are no part of it. */
  said: string[];
}

/**
 * What an action says is held at the end of a path of keys, the first of
 * them an argument's name. Nothing where it says nothing of it.
 */
export function heldAt(action: ActionSpec | undefined, keys: string[]): Held | null {
  const argument = action?.args.find((arg) => arg.name === keys[0]);
  if (!argument?.within) return null;
  let held: Held = { argument, within: argument.within, named: argument.within.each !== undefined, of: `\`${argument.name}\``, said: [argument.name] };
  for (const key of keys.slice(1)) {
    if (held.named) {
      // A name of the writer's choosing: inside it is one of the things.
      held = { ...held, named: false, of: held.within.each ?? '' };
      continue;
    }
    const within = held.within.keys?.find((one) => one.name === key)?.within;
    if (!within) return null;
    held = { argument, within, named: within.each !== undefined, of: `\`${key}\``, said: [...held.said, key] };
  }
  return held;
}

/** The place a reading is at, where that is inside a step's `with:`. */
export function inside(reading: Reading, language: BlueprintLanguage): Inside | null {
  const place = reading.place;
  if (place.at !== 'key' && place.at !== 'value') return null;
  const found = pathTo(place.frames);
  // Directly in the `with:` is an argument, which the language knows.
  if (!found || found.keys.length === 0) return null;
  const { action } = namedBeside(reading.lines, place.frames, found.at, language);
  if (!action) return null;
  const key = place.at === 'key' ? place.typed.trim() : place.key;
  const held = heldAt(
    language.actions.find((one) => one.name === action),
    found.keys,
  );
  const keys = held && !held.named ? (held.within.keys ?? []) : [];
  const described = keys.find((one) => one.name === key) ?? null;
  // The value of a name is one of the things; the value of a key is the key's.
  const words = place.at === 'key' ? [] : held?.named ? (held.within.words ?? []) : (described?.words ?? []);
  return {
    step: place.frames[found.at - 1]?.key ?? '',
    action,
    argument: found.keys[0]!,
    path: found.keys.slice(1),
    at: place.at,
    key,
    said: held?.said ?? null,
    of: held ? (held.named ? (held.within.each ?? '') : held.of) : '',
    keys,
    described,
    words,
  };
}

/**
 * What is at an offset of a blueprint's text, where that is inside a
 * step's `with:`: read as if what is written up to there had just been
 * written, which for a word is asked at its end.
 */
export function insideAt(text: string, offset: number, language: BlueprintLanguage | null): Inside | null {
  return language ? inside(placeAt(text, offset, language), language) : null;
}
