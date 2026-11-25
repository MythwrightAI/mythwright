// What the daemon hands of the language beyond what the client's first
// shapes of it say (`api.ts`): each thing here is in what
// `/api/blueprints/language` answers, and is read through these.
//
// They are apart from those shapes because that file is at its ceiling,
// and because nothing but what shows the language whole reads them.

import { call } from '../api.ts';
import type { ActionSpec, BlueprintLanguage, EventSpec, LanguageField, LanguageShape } from '../api.ts';

/** A whole blueprint a word or an action is shown by, where the daemon says one (`blueprint::vocabulary::shown`). */
export interface Shown {
  example?: string;
}

/** A field with what goes after it said in words, as the daemon says it (`blueprint::vocabulary::after`), and its example. */
export type ToldField = LanguageField & Shown & { after?: string };

/** An action with what it says it makes (and the argument that, said, has its rows set what something does), which of its arguments are a command it runs, the blueprints it reaches by a name of its own, and its example. */
export type ToldAction = ActionSpec & Shown & { makes?: { what: string; named: string; setting?: string }; runs?: string[]; reaches?: string[] };

/** An event with who may hear it: the scope whose holder is told of it, empty for anybody signed in. */
export type ToldEvent = EventSpec & { hears?: string | null };

/** The language as it is handed whole. */
export type ToldLanguage = Omit<BlueprintLanguage, 'shapes' | 'actions' | 'events' | 'names' | 'functions'> & {
  /** Each mapping with what it is for (`help`), from a daemon that says it, and its example. */
  shapes: (Omit<LanguageShape, 'fields'> & Shown & { help?: string; fields: ToldField[] })[];
  actions: ToldAction[];
  events?: ToldEvent[];
  names: (BlueprintLanguage['names'][number] & Shown)[];
  functions: (BlueprintLanguage['functions'][number] & { example?: string | null })[];
};

/** One place a blueprint uses something: the text by its name, the line, and where in it, as its graph names it. */
export interface UsedAt {
  blueprint: string;
  line: number;
  at: string;
}

/** The blueprints that use one thing, the shortest first, and how many more do. */
export interface Uses {
  places: UsedAt[];
  more: number;
}

/**
 * Where the blueprints the factory has use each word (`step/retry`, and
 * `step` for the mapping), each name an expression starts with and each
 * action (`blueprint::used`). What nothing uses is not there.
 */
export interface Used {
  words: Record<string, Uses>;
  names: Record<string, Uses>;
  actions: Record<string, Uses>;
}

/** Where the blueprints there are use each thing; nothing from a daemon that does not say. */
export function readUsed(): Promise<Used | null> {
  return call<Used>('GET', '/blueprints/language/used').catch(() => null);
}

/** What goes after a field, in the daemon's words; nothing from a daemon that does not say. */
export function afterOf(field: LanguageField): string {
  return (field as ToldField).after ?? '';
}

/**
 * A word shown working: an example the daemon runs in a sandbox with a few
 * things done to it, each as a sandbox's acts take it, and what to notice
 * in what came of it (`blueprint::vocabulary::SCENARIOS`). It is offered
 * beside the example it is of, wherever that is shown.
 */
export interface Scenario {
  name: string;
  example: string;
  /** Whether a stand-in waits to be answered as the sandbox is made, or answers at once. */
  answers: 'asked' | 'default';
  /** What is done, in order, as `POST /api/sandboxes/{id}/acts` takes each: applying the text is one of them. */
  acts: Record<string, unknown>[];
  notice: string;
}

/** The words shown working; none from a daemon that does not say. */
export function readScenarios(): Promise<Scenario[]> {
  return call<Scenario[]>('GET', '/blueprints/language/scenarios').catch(() => []);
}
