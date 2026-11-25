// What one version of what an instance was applied with changed from the
// one before it, in words: for the history of a blueprint as it was
// applied, where "version 4" says nothing and "columns: added blocked"
// says what happened. And what each of those is called, which is not a
// version: that word is the blueprint's own, for what its text says.

import type { Behind, Fields, Instance, Json, Version } from './api.ts';

function short(value: Json | undefined): string {
  if (value === undefined) return 'nothing';
  const text = typeof value === 'string' ? value : JSON.stringify(value);
  return text.length > 48 ? `${text.slice(0, 47)}…` : text;
}

function plain(value: Json | undefined): string[] | null {
  if (!Array.isArray(value)) return null;
  return value.every((one) => ['string', 'number', 'boolean'].includes(typeof one)) ? value.map(String) : null;
}

/** How one param changed, or `null` if it did not. */
function changed(name: string, before: Json | undefined, after: Json | undefined): string | null {
  if (JSON.stringify(before) === JSON.stringify(after)) return null;
  const was = plain(before);
  const is = plain(after);
  if (was && is) {
    // A list of words: which came, which went, or only their order.
    const added = is.filter((one) => !was.includes(one));
    const gone = was.filter((one) => !is.includes(one));
    const parts = [added.length > 0 && `added ${added.join(', ')}`, gone.length > 0 && `took out ${gone.join(', ')}`].filter(Boolean);
    return `${name}: ${parts.length > 0 ? parts.join('; ') : 'put in another order'}`;
  }
  if (before === undefined) return `${name}: ${short(after)}`;
  if (after === undefined) return `${name}: no longer given`;
  return `${name}: ${short(before)} → ${short(after)}`;
}

/** What a version changed from the one before it. The first one changed nothing: it is what there was to start with. */
export function changes(before: Version | undefined, after: Version): string[] {
  if (!before) return ['what it was started with'];
  const said: string[] = [];
  if (before.revision !== after.revision) said.push('its text changed');
  const was: Fields = before.params;
  const is: Fields = after.params;
  for (const name of [...new Set([...Object.keys(was), ...Object.keys(is)])].sort()) {
    const one = changed(name, was[name], is[name]);
    if (one) said.push(one);
  }
  return said.length > 0 ? said : ['the same again'];
}

/**
 * The version a blueprint's text says of itself (`version:`, a word of
 * its writer's choosing), read from the text as it was applied; nothing
 * where it says none.
 */
export function saidVersion(source: string): string | null {
  // A key of the text itself is written at the start of a line; one further in is a param's or a step's.
  const found = /^version:[ \t]*(.*)$/m.exec(source);
  if (!found) return null;
  const written = found[1]!.trim();
  const quoted = /^(["'])(.*?)\1/.exec(written);
  // Not in quotes, it ends where a comment begins.
  const said = quoted ? quoted[2]! : written.replace(/\s+#.*$/, '').trim();
  return said === '' ? null : said;
}

const TIMES = ['first', 'second', 'third', 'fourth', 'fifth', 'sixth', 'seventh', 'eighth', 'ninth', 'tenth', 'eleventh', 'twelfth'];

/** Which of the times something was applied one is, counted from 1: in a word while there is one, then by its number. */
export function nth(count: number): string {
  const word = TIMES[count - 1];
  if (word) return word;
  const tens = count % 100;
  const ending = tens >= 11 && tens <= 13 ? 'th' : (['th', 'st', 'nd', 'rd'][count % 10] ?? 'th');
  return `${count}${ending}`;
}

/**
 * What one of the definitions an instance was given is called. Two
 * numbers are true of it, and were both written as "version": which of
 * the times the instance was applied it is, counted for the instance, and
 * the version its text says of itself, which is the blueprint's and may
 * be the same for many of those times or say nothing. So the word
 * version is the blueprint's alone, said first where the text says one,
 * and the other is said as which time it was applied.
 */
export interface Called {
  /** `Blueprint version 3`; nothing where the text says no version. */
  version: string | null;
  /** `second time applied`. */
  time: string;
  /** The two as one line's heading, the version first: `Blueprint version 3`, or `Second time applied` where there is none. */
  head: string;
  /** What is said under that: `second time applied` where the heading is the version, and nothing where it says so itself. */
  under: string | null;
  /** The whole in a sentence's middle: `blueprint version 3, the second time it was applied`. */
  whole: string;
}

export function called(one: Pick<Version, 'version' | 'source'>): Called {
  const said = saidVersion(one.source);
  const time = `${nth(one.version)} time applied`;
  const version = said === null ? null : `Blueprint version ${said}`;
  const capital = time.charAt(0).toUpperCase() + time.slice(1);
  return {
    version,
    time,
    head: version ?? capital,
    under: version ? time : null,
    whole: `${said === null ? '' : `blueprint version ${said}, `}the ${nth(one.version)} time it was applied`,
  };
}

/** The text an instance runs, in a few characters: the start of its revision, and the version the text gives itself where it gives one. */
export function running(instance: Pick<Instance, 'revision' | 'version'>): string {
  const short = instance.revision.slice(0, 8);
  return instance.version ? `${short} · version ${instance.version}` : short;
}

/** What is said of an instance that is behind the text now there for its blueprint, and what can be done from here. */
export interface Lag {
  /** The one word that marks it. */
  mark: string;
  /** The text it is behind, said as its own is. */
  there: string;
  why: string;
  /** What moved between the two, by the version each says first: `std/lane-round 3 to 4: the step work is changed`. Nothing where nothing was said of it. */
  moved: string | null;
  /** What leaving it comes to, said beside how to take it. */
  leave: string;
  /** What the button that applies it again says, where applying it again brings it up to date. It says what that does: its steps run again, it is carried over and they do not, or it takes the text where it is and nothing runs. */
  again: string | null;
}

/** What applying it again does, said on the button that does it; nothing where applying it again is not what brings it up to date. */
const AGAIN: Record<Behind['takes'], string | null> = {
  again: 'Apply it again (its steps run again)',
  carried: 'Apply it again (it is carried over: its steps do not run again)',
  in_place: 'Apply it again (it takes the text where it is: nothing runs again)',
  unfit: null,
};

/** What is said once an instance has been applied again, by how it came to the text now there. */
export function applied(blueprint: string, key: string, took: 'again' | 'carried' | 'in_place' | undefined, changed: boolean): string {
  if (!changed) return `${key} is already running that.`;
  const how = { again: 'its steps ran again', carried: 'it was carried over, and its steps did not run again', in_place: 'it took the text where it is, and nothing ran again' };
  return `Applied ${blueprint} again: ${key} runs the text now there${took ? ` (${how[took]})` : ''}.`;
}

/** One text as it moved, by version first: `std/lane-round 3 to 4: the step work is changed`. */
export function moved(one: { name: string; was?: string | null; now?: string | null; changed?: string[] }): string {
  const lead =
    one.was && one.now
      ? `${one.name} ${one.was} to ${one.now}`
      : one.now
        ? `${one.name}, now version ${one.now}`
        : one.was
          ? `${one.name}, which was version ${one.was} and now says none`
          : one.name;
  return one.changed && one.changed.length > 0 ? `${lead}: ${one.changed.join('; ')}` : lead;
}

/** What leaving an instance behind comes to: nothing is taken for it. */
const LEAVE = 'Or leave it: it goes on with the text it has, whatever is installed, until somebody applies it.';

/** How an instance that is behind is shown; `null` for one that runs the text now there. */
export function lag(instance: Pick<Instance, 'behind'> & Partial<Pick<Instance, 'blueprint' | 'version'>>): Lag | null {
  const behind = instance.behind;
  if (!behind) return null;
  const changed = behind.changed ?? [];
  return {
    mark: behind.takes === 'unfit' ? 'behind: the text there does not check' : 'behind',
    there: running(behind),
    why: behind.why,
    moved: changed.length > 0 ? moved({ name: instance.blueprint ?? 'it', was: instance.version, now: behind.version, changed }) : null,
    leave: LEAVE,
    again: AGAIN[behind.takes],
  };
}
