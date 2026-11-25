// Running a sandbox test for a tab, and doing things to it.
//
// A sandbox's events are on no stream, on purpose: nothing of one is
// said where the factory's own things are said. So the page asks, one
// thing at a time, and reads what came of it after each: an act is
// answered when everything that came of it is at rest, and what the
// daemon then hands is drawn. While something is being answered the tab
// says so (`busy`), and whatever is asked meanwhile waits its turn.
//
// Running it again ends the sandbox the tab had and makes another for
// the text as it is now, with what it was run with before, and plays
// each act it had in the order it was done. What came of the run before
// is kept beside what comes of this one, so that the two can be told
// apart. An act the daemon refuses is said and ends the playing; an
// answer for a stand-in that does not wait this time is passed over,
// and said.

import { ApiError, call } from './api.ts';
import type { Diagnostic, Fields } from './api.ts';
import { actSays, making, sent } from './sandbox.ts';
import type { Act, Answers, Plan, Shown, Standing } from './sandbox.ts';
import { endSandbox, keepSandbox, sandboxOf } from './sandboxkept.ts';

const at = (id: string) => `/sandboxes/${encodeURIComponent(id)}`;

/** What is asked of each tab's sandbox, one after another. */
const turns = new Map<string, Promise<unknown>>();

function inTurn<T>(tab: string, work: () => Promise<T>): Promise<T> {
  // However it went, it was answered: whoever waits for it goes on.
  const answered = () => void (sandboxOf(tab) && keepSandbox(tab, { busy: null, answered: (sandboxOf(tab)?.answered ?? 0) + 1 }));
  const done = (turns.get(tab) ?? Promise.resolve()).then(work).finally(answered);
  turns.set(
    tab,
    done.catch(() => {}),
  );
  return done;
}

/** The daemon has no such sandbox: it was ended, for having been asked nothing for a while, or the daemon was started since. */
const isGone = (err: unknown): err is ApiError => err instanceof ApiError && err.status === 404;

const why = (err: unknown): string => (err instanceof Error ? err.message : String(err));

/** What a text that is to be run is, and what it is told. */
export interface Running {
  source: string;
  /** The project whose own blueprints it may call, where it is of one. */
  project: string | null;
  first: string[];
  /** What it is told as values, and as it was written in its boxes. */
  params: Fields;
  told: Record<string, string>;
  /** How its stand-ins answer as it begins. */
  answers: Answers;
}

/**
 * Runs the text in a sandbox for a tab: ends the one the tab had, makes
 * another, and plays what was done to the last. A text that does not
 * check makes nothing, and what is wrong with it is kept to be said.
 */
export function runSandbox(tab: string, running: Running): Promise<void> {
  return inTurn(tab, async () => {
    const had = sandboxOf(tab);
    const plan: Plan = { first: running.first, told: running.told, answers: running.answers, acts: had?.plan.acts ?? [] };
    keepSandbox(tab, { plan, busy: 'Running it in a sandbox', refused: null, skipped: [], problems: null, gone: null });
    if (had?.id) await endSandbox(had.id);
    let made: Standing & { ok: boolean; diagnostics?: Diagnostic[] };
    try {
      made = await call('POST', '/sandboxes', making(running.source, running.project, running.first, running.params, running.answers));
    } catch (err) {
      keepSandbox(tab, { id: null, busy: null, refused: why(err) });
      return;
    }
    if (!made.ok) {
      keepSandbox(tab, { id: null, busy: null, problems: made.diagnostics ?? [] });
      return;
    }
    const id = made.sandbox;
    keepSandbox(tab, { id });
    let standing: Standing = made;
    const skipped: string[] = [];
    let refused: string | null = null;
    for (const act of plan.acts) {
      const as = sent(act, standing.waiting);
      if ('wrong' in as) {
        skipped.push(as.wrong);
        continue;
      }
      keepSandbox(tab, { busy: `${actSays(act)}, again` });
      try {
        standing = await call<Standing>('POST', `${at(id)}/acts`, as.body);
      } catch (err) {
        refused = `${actSays(act)} was refused this time, and what was done after it was not done: ${why(err)}`;
        break;
      }
    }
    await read(tab, id, { before: had?.shown ?? null, skipped, refused });
  });
}

/** Reads everything that came of a tab's sandbox, and keeps it. */
async function read(tab: string, id: string, more: Partial<{ before: Shown | null; skipped: string[]; refused: string | null }> = {}): Promise<void> {
  try {
    const shown = await call<Shown>('GET', at(id));
    keepSandbox(tab, { shown, busy: null, ...more });
  } catch (err) {
    keepSandbox(tab, isGone(err) ? { id: null, busy: null, gone: why(err), ...more } : { busy: null, ...more, refused: why(err) });
  }
}

/**
 * Does one thing to a tab's sandbox, reads what came of it, and keeps
 * the act with those done before it once the daemon has done it.
 * Whether it was done.
 */
export function actOn(tab: string, act: Act): Promise<boolean> {
  return inTurn(tab, async () => {
    const had = sandboxOf(tab);
    if (!had?.id) return false;
    const as = sent(act, had.shown?.waiting ?? []);
    if ('wrong' in as) {
      keepSandbox(tab, { refused: as.wrong });
      return false;
    }
    keepSandbox(tab, { busy: actSays(act), refused: null });
    try {
      await call<Standing>('POST', `${at(had.id)}/acts`, as.body);
    } catch (err) {
      if (isGone(err)) keepSandbox(tab, { id: null, busy: null, gone: why(err) });
      else {
        // What it had come to before it was refused is read all the same.
        await read(tab, had.id, { refused: why(err) });
      }
      return false;
    }
    keepSandbox(tab, { plan: { ...had.plan, acts: [...had.plan.acts, act] } });
    await read(tab, had.id);
    return true;
  });
}

/** Takes one act out of what a tab's sandbox test is played with: the next run is without it. */
export function dropAct(tab: string, index: number): void {
  const had = sandboxOf(tab);
  if (had) keepSandbox(tab, { plan: { ...had.plan, acts: had.plan.acts.filter((_, at) => at !== index) } });
}
