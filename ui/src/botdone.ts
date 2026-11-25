// What the factory's bot did lately, for whoever reads about it: what a
// blueprint's step did to records, which is the bot's doing, and what it
// said in channels (`/api/system/speaker/history`). The daemon hands each
// line held to what the reader may read; this only says each in a few
// words, in the words the records' histories use, with the blueprint it
// was done through as detail after it.

import { call } from './api.ts';

/** A thing the bot did, as the daemon hands it. */
export interface Done {
  kind: 'record' | 'said';
  ts: number;
  /** The history's sentence (`conductor moved it from ready to doing`), or what was said. */
  said: string;
  /** The blueprint it was done through, where one is known. */
  via: string | null;
  record?: string;
  project?: string;
  collection?: string;
  channel?: string;
  message?: string;
}

export interface Lately {
  /** What the bot is called now, which every sentence says. */
  name: string;
  done: Done[];
}

/** How many lines the page reads: the latest, which it says it is once it is as long. */
export const LATEST = 50;

export const doneLately = (limit = LATEST): Promise<Lately> => call<Lately>('GET', `/system/speaker/history?limit=${limit}`);

/**
 * What a line says, with the bot left out since the page is about it: of
 * a record, the history's sentence with the record in the place of `it`
 * (`moved shop-17 from ready to doing`), or the usual words after the
 * record (`shop-notes-2: created`); of a message, where it was said and
 * how it begins. `called` says what a channel is called, by its id.
 */
export function doneWords(line: Done, bot: string, called: (channel: string) => string): string {
  if (line.kind === 'said') {
    const first = line.said.trim().split('\n')[0] ?? '';
    return `said in ${called(line.channel ?? '')}: ${first || 'something attached'}`;
  }
  const record = line.record ?? 'a record';
  if (line.said.startsWith(`${bot}: `)) return `${record}: ${line.said.slice(bot.length + 2)}`;
  if (line.said.startsWith(`${bot} `)) {
    const rest = line.said.slice(bot.length + 1);
    return /\bit\b/.test(rest) ? rest.replace(/\bit\b/, record) : `${rest} (${record})`;
  }
  return `${line.said} (${record})`;
}
