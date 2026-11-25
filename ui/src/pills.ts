// Choosing from options that are shown as tags: which there are, in what
// order, and which of them are chosen.
//
// What is chosen is given in the order of the options, so putting the
// options in another order is how the order of a list is changed. An
// option that is not chosen is kept, to be chosen again.

import type { Json } from './api.ts';

export type Item = string | number | boolean;

export interface Picked {
  /** Everything there is to choose from, in the order it is kept. */
  options: Item[];
  /** Which of them are chosen, in no particular order. */
  chosen: Item[];
}

const same = (a: Item, b: Item) => String(a) === String(b);
const has = (items: Item[], item: Item) => items.some((one) => same(one, item));

/** Whether a value can be shown as tags: a list of plain words and numbers. */
export function asItems(value: Json | undefined): Item[] | null {
  if (!Array.isArray(value)) return null;
  const plain = value.every((one) => typeof one === 'string' || typeof one === 'number' || typeof one === 'boolean');
  return plain ? (value as Item[]) : null;
}

/** Starts with these chosen. What else there is to choose from comes after them. */
export function begin(chosen: Item[], others: Item[] = []): Picked {
  return { options: [...chosen, ...others.filter((one) => !has(chosen, one))], chosen: [...chosen] };
}

/** What is chosen, in the order of the options. */
export function value(picked: Picked): Item[] {
  return picked.options.filter((one) => has(picked.chosen, one));
}

/** Chooses what is not chosen, and the other way round. */
export function toggle(picked: Picked, item: Item): Picked {
  const chosen = has(picked.chosen, item) ? picked.chosen.filter((one) => !same(one, item)) : [...picked.chosen, item];
  return { options: picked.options, chosen };
}

/** Chooses one thing and nothing else: for a choice of one. */
export function only(picked: Picked, item: Item): Picked {
  return { options: picked.options, chosen: [item] };
}

/** What typing this would make: nothing if it is empty or is an option already. */
export function making(picked: Picked, typed: string): string | null {
  const text = typed.trim();
  return text === '' || has(picked.options, text) ? null : text;
}

/**
 * Chooses what was typed. If there is no such option one is made, after
 * the others. In a list of numbers, a number that is typed is a number.
 */
export function add(picked: Picked, typed: string): Picked {
  const text = typed.trim();
  if (text === '') return picked;
  const known = picked.options.find((one) => same(one, text));
  if (known !== undefined) return has(picked.chosen, known) ? picked : toggle(picked, known);
  const numbers = picked.options.length > 0 && picked.options.every((one) => typeof one === 'number');
  const item: Item = numbers && Number.isFinite(Number(text)) ? Number(text) : text;
  return { options: [...picked.options, item], chosen: [...picked.chosen, item] };
}

/** Puts the option at one place in another, moving the ones between along. */
export function move(picked: Picked, from: number, to: number): Picked {
  const last = picked.options.length - 1;
  if (from < 0 || from > last || from === to) return picked;
  const options = [...picked.options];
  const [item] = options.splice(from, 1);
  options.splice(Math.max(0, Math.min(to, last)), 0, item!);
  return { options, chosen: picked.chosen };
}

/** The options that what was typed is found in; all of them when nothing is typed. */
export function matching(picked: Picked, typed: string): Item[] {
  const text = typed.trim().toLowerCase();
  return text === '' ? picked.options : picked.options.filter((one) => String(one).toLowerCase().includes(text));
}

/** How many colours a tag may have: one that says nothing, and eight a name comes to. */
export const COLOURS = 9;

/**
 * The colour of a word drawn as a tag (an option, a label): always the
 * same for the same words, in any case, so that a label is one colour on
 * every card and in every table, today and tomorrow, with nothing kept
 * anywhere. One of eight, never the plain one, which says nothing: a name
 * is mixed up well before it is counted off, so that names as alike as
 * two labels of one board do not come to the same colour more often than
 * eight colours must.
 */
export function colour(item: Item): number {
  let sum = 2166136261;
  for (const letter of String(item).toLowerCase()) sum = Math.imul(sum ^ letter.charCodeAt(0), 16777619) >>> 0;
  sum ^= sum >>> 15;
  sum = Math.imul(sum, 3266489917) >>> 0;
  sum ^= sum >>> 13;
  return 1 + ((sum >>> 0) % (COLOURS - 1));
}
