// What is said of a blueprint's text short of refusing it. The check says
// most of it as the text is written; an apply says the rest, which
// depends on where the text was applied: a handler that says no `scope`
// hears its own project's events and the factory's own, so one applied
// for no project that listens for a project's cards will hear nothing.

import type { Diagnostic } from './api.ts';

/** What a check says of a text, and after it what an apply of that text said that the check does not: each thing once, by what it is and where. */
export function withSaid(checked: Diagnostic[], said: Diagnostic[]): Diagnostic[] {
  const key = (one: Diagnostic) => `${one.code} ${one.span.start.line}:${one.span.start.col} ${one.message}`;
  const there = new Set(checked.map(key));
  return [...checked, ...said.filter((one) => !there.has(key(one)))];
}
