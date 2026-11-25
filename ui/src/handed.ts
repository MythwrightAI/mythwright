// What an applied text was handed: the params a time it was applied was
// given, as its History shows them over its text (`views/history.tsx`).
// A heading in words with how many, and under it each name with its
// value; a value that is a mapping or a list is said by how much it holds
// and opened to what is in it, a level at a time. A secret is never among
// them: the daemon keeps every one out of what it hands.
//
// Holds nothing of the page, so that its tests can read it.

/** One name and what it was handed, as a line. */
export interface Handed {
  /** What it is called: a param's name, a key of a mapping, or which of a list it is (`#1`, counted from one). */
  name: string;
  /** Where it is, from the top, by names: what keeps it open or folded. */
  path: string;
  /** A value that is a word, a number, true or false, or nothing, as it is written. */
  said?: string;
  /** A mapping or a list: what is in it, and how much that is in words. */
  under?: Handed[];
  holds?: string;
}

const counted = (count: number, one: string, many: string) => `${count} ${count === 1 ? one : many}`;

/** The heading over them: how many it was applied with. Nothing where it was handed none, and then nothing is drawn. */
export function headingOf(params: Record<string, unknown>): string | null {
  const count = Object.keys(params).length;
  return count === 0 ? null : `Applied with ${counted(count, 'param', 'params')}`;
}

/** A value that holds nothing more, as it is written: text as it is (empty text as `""`), the rest as JSON writes it. */
export function saidOf(value: unknown): string {
  if (typeof value === 'string') return value === '' ? '""' : value;
  if (value === undefined) return 'null';
  return JSON.stringify(value);
}

function lineOf(name: string, path: string, value: unknown): Handed {
  if (Array.isArray(value)) {
    return { name, path, holds: value.length === 0 ? 'an empty list' : `a list of ${value.length}`, under: value.map((one, at) => lineOf(`#${at + 1}`, `${path}/${at}`, one)) };
  }
  if (value !== null && typeof value === 'object') {
    const entries = Object.entries(value as Record<string, unknown>);
    return { name, path, holds: entries.length === 0 ? 'nothing in it' : counted(entries.length, 'name', 'names'), under: entries.map(([key, one]) => lineOf(key, `${path}/${key}`, one)) };
  }
  return { name, path, said: saidOf(value) };
}

/** Each param as a line, in the order they were handed. */
export function handedOf(params: Record<string, unknown>): Handed[] {
  return Object.entries(params).map(([name, value]) => lineOf(name, name, value));
}
