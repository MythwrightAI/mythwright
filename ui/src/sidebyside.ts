// Sandbox tests side by side, as the page has them: which texts of a
// blueprint can be played against the same acts, what that is asked as,
// and what each column of the answer says.
//
// The daemon plays the acts in a sandbox for each text and lays what each
// came to out as a comparison's columns are (`/api/sandboxes/beside`):
// counts, never scores, and the columns in the order they were asked for.
// Nothing here works out what happened, and nothing here says which column
// did better: these are the texts offered, what is sent, and the daemon's
// rows read for one column at a time. Nothing of the page is in it, so
// that its cases can read it.

import type { Beside, Diagnostic, Fields } from './api.ts';
import { KINDS, figures, saidOf } from './compared.ts';
import type { Act, Answers, Shown } from './sandbox.ts';

/** Which text a column is: the page's as it stands (`here`), the file as saved (`saved`), the default as it ships (`default`), a project's fork of it (`fork-<project>`). Each is also the word its way is called by when it is compared on real work. */
export type Word = string;

/** A text that can be played side by side. */
export interface Text {
  word: Word;
  /** What heads its column. */
  called: string;
  /** Its text, where the page has it; what ships is read by the daemon. */
  source: string | null;
  /** What it is, in a few words, beside where it is chosen. */
  says: string;
  /** The texts offered after it that are the same to the letter, which are no column of their own: by what each would have been called. */
  same?: string[];
}

/** What the page knows of the texts of its blueprint. */
export interface Of {
  /** The text as it stands on the page, saved or not. */
  source: string;
  /** The file as it is saved, where the page is a file. */
  saved: string | null;
  /** Whether the blueprint ships: the page is the default's, or a fork of it. */
  ships: boolean;
  /** What ships under the name, where the page has read it. */
  shipped: string | null;
  /** Whether the page itself is what ships, as it ships. */
  isDefault: boolean;
  /** The forks of it the page has read: each project's own file under its name, by the project, what the project is called, and its text. */
  forks: { project: string; called: string; source: string }[];
}

/**
 * The texts offered side by side, each once: one that is the same to the
 * letter as one before it is no second column. The page's own text is
 * first, where it is not the default as it ships; then the default, and
 * the forks of it.
 */
export function offered(of: Of): Text[] {
  const all: Text[] = [];
  if (!of.isDefault) all.push({ word: 'here', called: 'As it stands here', source: of.source, says: 'the text on this page, saved or not' });
  if (of.saved !== null) all.push({ word: 'saved', called: 'As it was saved', source: of.saved, says: 'the file as it is saved' });
  if (of.ships) all.push({ word: 'default', called: 'The default', source: of.isDefault ? of.source : of.shipped, says: 'what ships under the name' });
  for (const fork of of.forks) all.push({ word: `fork-${fork.project}`, called: `The fork in ${fork.called}`, source: fork.source, says: "the project's own copy, which runs there under the name" });
  const kept: Text[] = [];
  for (const text of all) {
    const first = text.source === null ? undefined : kept.find((one) => one.source === text.source);
    if (first) first.same = [...(first.same ?? []), text.called];
    else kept.push({ ...text });
  }
  return kept;
}

/** What would make a second text where there is one: the text changed on the page (`change`), or the default forked (`fork`). */
export interface Second {
  word: 'change' | 'fork';
  /** What its press says. */
  called: string;
  /** What there is to play once it is done. */
  then: string;
}

/** Where what can be played stands: the texts are still being read, there are two or more, or there is one, with what is said of it and what would make a second. */
export type Standing = { reading: true } | { several: Text[] } | { one: Text; says: string; seconds: Second[]; otherwise: string | null };

/**
 * Where what can be played stands, once the page has read its saved file,
 * the default and the forks: before that nothing is said of how many
 * texts there are, since one read later may be a second. Where there is
 * one, what would make a second is each thing the page can do: change its
 * text (`changes`: the page is a file whoever looks may write), which is
 * then played as it stands beside as it was saved; or fork the default
 * (`forks`: the page offers Fork), which is then played beside it. Where
 * the page can do neither, it says in words what would, by whether the
 * page is a file (`file`) or what ships.
 */
export function standing(of: { read: boolean; texts: Text[]; file: boolean; changes: boolean; forks: boolean }): Standing {
  if (!of.read) return { reading: true };
  if (of.texts.length >= 2) return { several: of.texts };
  const one = of.texts[0]!;
  const lower = (called: string) => called.charAt(0).toLowerCase() + called.slice(1);
  const same = one.same?.length ? `, which is also ${one.same.map(lower).join(' and ')}` : '';
  const says = `There is one text of this blueprint to play: ${lower(one.called)}${same}. Side by side plays the same things against two texts or more, each in a sandbox of its own.`;
  const seconds: Second[] = [];
  if (of.changes) seconds.push({ word: 'change', called: 'Change the text here', then: 'it is then played as it stands here beside as it was saved' });
  if (of.forks) seconds.push({ word: 'fork', called: 'Fork', then: 'the fork is then played beside the default' });
  const otherwise =
    seconds.length > 0
      ? null
      : of.file
        ? "A second would be its text changed here, which whoever may write the project's files can do."
        : "A second would be a fork of it, which whoever may write a project's files can make.";
  return { one, says, seconds, otherwise };
}

/** How many texts are played side by side at most: each is a sandbox, and a daemon has a handful. */
export const MOST = 4;

/** What is chosen where nothing was: the first two offered. */
export function chosenFirst(texts: Text[]): Word[] {
  return texts.slice(0, 2).map((text) => text.word);
}

/** What is chosen once `word` is pressed: in or out, in the order offered, and no more than {@link MOST}. */
export function toggled(chosen: Word[], word: Word, texts: Text[]): Word[] {
  const next = chosen.includes(word) ? chosen.filter((one) => one !== word) : [...chosen, word];
  return texts.map((text) => text.word).filter((one) => next.includes(one)).slice(0, MOST);
}

/** Why what is chosen cannot be played; nothing where it can. */
export function cannotPlay(chosen: Word[]): string | null {
  return chosen.length < 2 ? 'Choose two texts, or more: each is played in a sandbox of its own, against the same things done.' : null;
}

/**
 * What an act is sent as, played in each sandbox: as a sandbox takes it,
 * but an answer given as usual, which only each sandbox can answer, since
 * each has its own stand-ins.
 */
export function played(act: Act): Fields {
  switch (act.kind) {
    case 'say':
      return { say: { topic: act.topic, data: act.data } };
    case 'do':
      return { do: { action: act.action, with: act.with } };
    case 'advance':
      return { advance: { by: act.by } };
    case 'answers':
      return { answers: { answers: act.answers } };
    case 'answer':
      if (act.how === 'fails') return { answer: { asked: act.asked, fails: act.why?.trim() || 'It was told to fail.' } };
      if (act.how === 'usual') return { answer: { asked: act.asked, usual: true } };
      return { answer: { asked: act.asked, gives: act.gives ?? {} } };
  }
}

/** What playing side by side is asked as: the texts chosen, each as its source or as what ships, and what each is made and played with. */
export function asking(of: { name: string | null; project: string | null; texts: Text[]; first: string[]; params: Fields; told: Record<string, string>; answers: Answers; acts: Act[] }): Fields {
  return {
    ...(of.name ? { name: of.name } : {}),
    ...(of.project ? { project: of.project } : {}),
    texts: of.texts.map((text): Fields => (text.source === null ? { called: text.called, ships: true } : { called: text.called, source: text.source })),
    first: of.first,
    params: of.params,
    answers: of.answers,
    acts: of.acts.map(played),
  };
}

/** One text as the daemon played it. */
export interface Column {
  called: string;
  sandbox: string;
  shown: Shown;
  /** Why its playing ended early, in the daemon's words. */
  refused: string | null;
  /** Each act passed over in it, with why. */
  skipped: string[];
}

/** What the daemon answers: each text's sandbox, and the same laid out as a comparison's columns. */
export interface SideBySide {
  ok: boolean;
  said: string;
  beside?: Beside;
  sandboxes?: Column[];
  /** Where a text does not check: what is wrong with each. */
  texts?: { called: string; diagnostics: Diagnostic[] }[];
}

/** One row of the daemon's, for one column: its head, what it is of, and the two figures, or nothing where that column has none. */
export interface Counted {
  kind: keyof typeof KINDS;
  /** The kind's heading, on the first row of each kind. */
  head: string | null;
  what: string;
  figures: [string, string] | null;
}

/** The daemon's rows for the column at `at`, in its order, every row there for every column so that columns side by side read across. */
export function countsOf(beside: Beside, at: number): Counted[] {
  return beside.rows.map((row, index) => {
    const cell = row.by[at];
    return {
      kind: row.kind,
      head: index === 0 || beside.rows[index - 1]!.kind !== row.kind ? KINDS[row.kind].head : null,
      what: row.what,
      figures: cell ? figures(row, cell) : null,
    };
  });
}

/** Which text a column ran, in a few words: its version and the start of its revision, as a comparison's column says it. */
export function ranOf(beside: Beside, at: number): string {
  const column = beside.columns[at];
  return column ? saidOf(column) : '';
}

/** Whether every column ran the same text, to the letter: then what differs is not of the texts. */
export function allTheSame(beside: Beside): boolean {
  const texts = beside.columns.map((column) => column.text);
  return texts.length > 1 && texts.every((text) => text === texts[0]);
}

/** The texts played, as ways of a comparison on real work: each by its word, and the text it was played as. */
export function asWays(texts: Text[], shipped: string | null): { word: Word; called: string; source: string }[] {
  return texts.flatMap((text) => {
    const source = text.source ?? shipped;
    return source === null ? [] : [{ word: text.word, called: text.called, source }];
  });
}

/** What is said under the texts of what is played against each: the acts of the sandbox test of this tab, or nothing but the applying; and how the stand-ins answer as each begins. */
export function playedSays(acts: Act[], answers: Answers): string {
  const done =
    acts.length === 0
      ? 'Each is applied in a sandbox of its own, and nothing else is done: things done in a sandbox test of it here are done to each.'
      : acts.length === 1
        ? 'Each is applied in a sandbox of its own, and the one thing done in the sandbox test here is done to each.'
        : `Each is applied in a sandbox of its own, and the ${acts.length} things done in the sandbox test here are done to each, in order.`;
  const standing = answers === 'default' ? ' A stand-in answers at once what it usually answers.' : ' A stand-in waits to be answered, as it did in the sandbox test.';
  return done + standing;
}

/** The forks of `name` among what every project has of its own under the names that ship (`/api/blueprints/stands`): each project's but the one whose file the page is. */
export function forksOf(own: { project: string; name: string; file: string }[], name: string | null, page: { project: string | null; path: string | null }): { project: string; file: string }[] {
  return own.filter((one) => one.name === name && !(page.path !== null && one.project === page.project && one.file === page.path)).map((one) => ({ project: one.project, file: one.file }));
}
