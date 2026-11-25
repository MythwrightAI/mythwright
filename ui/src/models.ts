// What the model picker offers for one provider.
//
// A provider (an extension, usually) says which models it has, the same list
// its own tool shows. The picker puts the provider's usual one first, and
// choosing that means naming no model at all: the tool decides, today and
// after it changes its mind. Typing a name is always possible, for a model
// the list does not have yet.

import type { Model } from './api.ts';

export interface Choice {
  /** What the slot is given; empty for "the provider's own choice". */
  value: string;
  label: string;
  note: string;
}

/** The value the picker uses for "let me type one". It is never sent. */
export const TYPED = '\u0000typed';

/**
 * The model a provider's usual one is, where the usual one is listed by
 * a word of its own ("Default") and names in its words another model the
 * list has: the one whose name its label or its note says, written as
 * that model is called and as words of their own (a note that says "the
 * big one" names no model called "Big"), the longest where several are
 * said. Nothing where it names none.
 */
export function namedBy(usual: Model, models: Model[]): Model | null {
  const said = `${usual.label}\n${usual.note}`;
  const says = (name: string) => new RegExp(`(^|[^\\p{L}\\p{N}])${name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}($|[^\\p{L}\\p{N}])`, 'u').test(said);
  const named = models.filter((model) => model !== usual && model.label.trim() !== '' && says(model.label.trim()));
  return named.sort((one, other) => other.label.length - one.label.length)[0] ?? null;
}

/**
 * What the picker offers, in order: first the provider's usual one, which
 * means naming no model at all; then every other model, a line each.
 * Where the usual one is listed by a word of its own and names a model of
 * the list, the two are one line, called by that model's name and marked
 * as what it is (`Big (recommended)`, or `(default)` where the program
 * says nothing more), and choosing it still names none. `current` is the
 * model a slot names already: one that names that very model keeps its
 * own line, so that what it was given is what is shown.
 */
export function choices(models: Model[], current = ''): Choice[] {
  const usual = models.find((model) => model.default);
  if (!usual) return [{ value: '', label: 'Default', note: 'Whatever it is set to use' }, ...models.map(line)];
  const named = namedBy(usual, models);
  const merged = named && named.id !== current ? named : null;
  const first: Choice = merged
    ? { value: '', label: `${merged.label} (${/recommended/i.test(usual.label) ? 'recommended' : 'default'})`, note: merged.note || usual.note }
    : { value: '', label: usual.label, note: usual.note };
  return [first, ...models.filter((model) => model !== usual && model !== merged).map(line)];
}

function line(model: Model): Choice {
  return { value: model.id, label: model.label, note: model.note };
}

/** Whether a model already chosen for a slot is one the list offers. */
export function offered(models: Model[], value: string): boolean {
  return value === '' || choices(models, value).some((choice) => choice.value === value);
}
