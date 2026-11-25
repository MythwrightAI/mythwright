// What somebody has written of a blueprint they have not applied yet.
//
// What a blueprint is applied with is asked in the drawer, beside its
// text, and the drawer is put away when another tab is shown: what was
// typed is kept here, by the blueprint and the project it is of, and is
// there again when the drawer is opened. It is let go when the blueprint
// is applied, and not kept across a reload of the page.
//
// And what is a function of a param and what was typed for it, with no
// page in it: what its box starts with, and what the text in it comes to.

import type { Fields, GraphParam, Json } from './api.ts';

/** What is asked before a blueprint is applied, as it is written: each param's box, and the project where that is asked. */
export interface Applying {
  where: string;
  values: Record<string, string>;
  /** The environment it is to run under, where one is asked. */
  under?: string;
}

const kept = new Map<string, Applying>();

/** What stands for one blueprint of one project. */
export const applyingKey = (of: string, project: string | null): string => `${project ?? ''}\n${of}`;

/** What was written of it and not applied; nothing where nothing was. */
export function applyingOf(key: string): Applying | null {
  return kept.get(key) ?? null;
}

/** Keeps what was written, or with `null` lets it go. */
export function keepApplying(key: string, applying: Applying | null): void {
  if (applying) kept.set(key, applying);
  else kept.delete(key);
}

/** Lets go of everything written: whoever was signed in is no longer. */
export function forgetApplying(): void {
  kept.clear();
}

/** What a param is before anybody says: what it was last applied with, else what its blueprint says, else nothing. */
export function startOf(param: GraphParam, known: Fields): Json {
  return known[param.name] ?? param.default ?? null;
}

/** A value as the text its box holds: words as they are, anything else as JSON, nothing as nothing. */
export function written(value: Json): string {
  return value === null ? '' : typeof value === 'string' ? value : JSON.stringify(value);
}

/** What the text in a param's box is, by the param's type. It throws for text that is not one. */
export function valueOf(param: GraphParam, text: string): Json {
  switch (param.type) {
    case 'string':
    case 'duration':
      return text;
    case 'int':
    case 'number': {
      const value = Number(text);
      if (text.trim() === '' || !Number.isFinite(value)) throw new Error('not a number');
      return value;
    }
    case 'bool':
      return text === 'true';
    case 'any':
      try {
        return JSON.parse(text);
      } catch {
        return text;
      }
    default:
      return JSON.parse(text);
  }
}

/**
 * What a blueprint is applied with, from what is written in each box: a
 * box left empty for a param that is not needed says nothing of it. Or
 * what is wrong with the first box that cannot be read, in a sentence.
 */
export function answered(params: GraphParam[], values: Record<string, string>): { values: Fields } | { wrong: string } {
  const out: Fields = {};
  for (const param of params) {
    const text = values[param.name] ?? '';
    if (text === '' && !param.required) continue;
    try {
      out[param.name] = valueOf(param, text);
    } catch {
      return { wrong: `\`${param.name}\` should be ${param.type === 'int' || param.type === 'number' ? 'a number' : `a ${param.type} written as JSON`}.` };
    }
  }
  return { values: out };
}
