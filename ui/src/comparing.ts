// What the daemon says of comparing and of what moved between two texts:
// the shapes of its answers, apart from the calls that fetch them.

import type { Fields } from './api.ts';

/** One text that ships, as a running tree went on with it and as another one is. */
export interface Moved {
  name: string;
  /** The version of the one it went on with, where it says one. */
  was: string | null;
  now: string | null;
  changed: string[];
}
/** One of the ways a comparison has a blueprint started. */
export interface Way {
  name: string;
  share: number;
  /** The blueprint started in place of the one compared, where it says one. */
  blueprint?: string;
  /** The version of it the way is held to, where one was said. */
  version?: string;
  /** Its text was sent as it was (one played side by side in sandboxes), and it is held to that text. */
  sent?: boolean;
  /** What it is handed, over what the step hands. */
  with?: Fields;
  /** The files it reads in place of ones a step links: for each, by its name in the library, the file read instead. */
  prompts?: Record<string, { name: string }>;
  /** How many things have gone this way. */
  went: number;
  /** The texts it has run, the one it runs now last. */
  texts?: { text: string; revision: string; version: string | null; since: number }[];
}
/** One blueprint started several ways, side by side. */
export interface Comparison {
  name: string;
  project: string | null;
  /** The blueprint whose starting it is about. */
  of: string;
  /** The param that says which thing a start is for. */
  per?: string;
  also?: string[];
  /** What is counted of what comes of each thing: each what it is called, the event, the field that says the thing and the fields it is counted by. */
  counts?: Counted[];
  ways: Way[];
  by: string;
  started: number;
  ended: number | null;
  /** Once ended: the way everything goes from then on, where one was said. */
  goes: string | null;
  /** Its ways are named by the start: it shares nothing out, and has no shares to change. */
  asked?: boolean;
  /** What whoever started it held: changing its shares or its rule takes holding all of it. */
  held?: string[];
  /** The rule its shares change by, where one was given. */
  rule?: Rule | null;
}

/** A rule a comparison's shares change by, as the daemon keeps it: the way that grows, the one it is held against, a count and what it came to that is held, held to at least as often where `at_least`, how many things of each are read first, and the step, the most and (where it falls back) the least of a hundred. */
export interface Rule {
  grows: string;
  against: string;
  count: string;
  came: string;
  at_least?: boolean;
  over: number;
  step: number;
  most: number;
  least?: number | null;
}

/** One kind of event counted for each thing: what the count is called, the event, the field that says the thing, and the fields that say what it came to. */
export interface Counted {
  name: string;
  topic: string;
  thing: string;
  by?: string[];
}
/** What a comparison is asked to count: each kind said, or the usual ones for a board's cards by their word. */
export type Counts = Counted[] | 'cards';

/** A comparison's figures laid beside each other by the daemon: a column for each text of each way, and a row for each thing measured. */
export interface Beside {
  columns: {
    way: string;
    /** What measurements of it carry as `text`; empty where it is not known. */
    text: string;
    version: string | null;
    /** Whether it is the text the way runs now, said where the way ran several. */
    now: boolean;
    /** How many things it was given, where that is known. */
    given: number | null;
    /** Where the way reads other words in place of a file that is linked: the file, the one read instead, and the start of the revision of the words this column was run with. */
    reads?: { for: string; name: string; words: string | null }[];
  }[];
  rows: BesideRow[];
  /** Where its shares change by a rule or were said again by somebody: each way's part, the rule with what it has read, and each time they were said again with why. */
  rolled?: Rolled;
}
/** A comparison's shares as the daemon lays them out: each way's share and part of a hundred, the rule in a sentence with what it has read of its two ways since the shares were last set, and each time the shares were said again, the oldest first. */
export interface Rolled {
  parts: { way: string; share: number; percent: number }[];
  rule: { said: string; since: number; done: boolean; over: number; read: { way: string; things: number; came: number }[] } | null;
  shared: { at: number; by: string; was: Record<string, number>; now: Record<string, number>; why: string }[];
}
export interface BesideRow {
  /** What came of the things, what was chosen, what is added up, how many were accepted, what was added up over those, and what else was measured: in that order. */
  kind: 'came' | 'chose' | 'spent' | 'accepted' | 'per_accepted' | 'took';
  what: string;
  /** What its values are of where they are not a length of time. */
  unit?: string;
  /** For each column in order: its figures, or nothing. Of `accepted`, how many and over how many given (`rate`); of `per_accepted`, what was spent in all over how many were accepted. */
  by: ({ count?: number; errors?: number; avg?: number; sum?: number; per_thing?: number; given?: number; rate?: number; accepted?: number; per_accepted?: number } | null)[];
}

/** How what is running comes by new texts, and how a taken one lands: the factory's setting. */
export interface TextsSetting {
  /** `usual` where nothing was said. */
  come: 'usual' | 'held' | 'follow';
  land: 'in_place' | 'again';
  /** What holds where nothing was said: `held` for an installed factory, `read` for one run from its checkout. */
  usual: 'held' | 'read';
  by: string | null;
  at: number | null;
  /** What having texts followed gives up, in words. */
  gives_up: string;
  /** After a change: each text that was not taken because it does not check. */
  not_taken?: { instance: string; key: string; blueprint: string; error: string }[];
}
