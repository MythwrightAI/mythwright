// How long the detail of runs' steps is kept, and where their output is
// written: the factory's two settings, as the daemon hands them
// (`/api/system/logs`), and what is so now said in sentences. Read and
// changed here and not in `api.ts`, which is at its ceiling.

import { call } from './api.ts';

/** What was let go of at once, as the daemon says it. */
export interface LetGo {
  before: number;
  days: number;
  events: number;
  files: number;
  bytes: number;
  held?: string;
}

/** The two settings, as the daemon hands them. */
export interface LogsKept {
  /** For how many days; for ever where nothing is said. */
  days: number | null;
  /** Where runs' steps write, whole; the data folder where nothing is said. */
  folder: string | null;
  /** The folders said before, the latest first, where what was written then is still read. */
  before: string[];
  /** The data folder's, where they are written where nothing is said. */
  usual: string;
  by: string | null;
  at: number | null;
  let_go?: LetGo | null;
}

export const logsKept = () => call<LogsKept>('GET', '/system/logs');

/** Says how long (a number of days, or `always`), or where (a folder, or `usual`), or both. */
export const keepLogs = (asked: { days?: number | 'always'; folder?: string }) => call<LogsKept>('POST', '/system/logs', asked);

/** How long, in a sentence. */
export function howLong(now: Pick<LogsKept, 'days'>): string {
  if (now.days === null) return "What runs' steps printed is kept for ever.";
  const days = now.days === 1 ? 'a day' : `${now.days} days`;
  return `What runs' steps printed, and what was said of each as it went, is let go of after ${days}. What each run came to is kept for ever.`;
}

/** Where, in a sentence, with where what was written before is read. */
export function where(now: Pick<LogsKept, 'folder' | 'usual' | 'before'>): string {
  const now_ = now.folder ? `It is written in ${now.folder}.` : `It is written in the data folder, ${now.usual}.`;
  if (now.before.length === 0) return now_;
  return `${now_} What was written before is read where it is: ${now.before.join(', ')}.`;
}

/** What went at once, in a sentence; nothing where nothing did. */
export function wentNow(went: LetGo | null | undefined): string | null {
  if (!went || (went.events === 0 && went.files === 0)) return null;
  const files = went.files === 1 ? 'one file' : `${went.files} files`;
  const events = went.events === 1 ? 'one event' : `${went.events} events`;
  return `Let go of now: ${files} and ${events}.`;
}
