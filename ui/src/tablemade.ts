// A table made from the side panel (New… and then Table): what
// it is to be called, worked out as the daemon will work it out, and what
// it is applied with (`std/table`, as whoever asks, under the key
// `table-<name>`). Holds nothing of the page, so that its tests can read it.

import { tableTitled } from './called.ts';

/** How a table made from the side panel may first be laid out, with what each is called. Only those its first column allows. */
export const LAYOUTS: { kind: string; called: string }[] = [
  { kind: 'table', called: 'Table' },
  { kind: 'list', called: 'List' },
  { kind: 'gallery', called: 'Gallery' },
];

/** What a name comes to as everything knows a table by it: lowercase letters and digits, words joined by one dash (`blueprint::expr::slug`). */
export function slugOf(text: string): string {
  let out = '';
  for (const ch of text) {
    if (/^[A-Za-z0-9]$/.test(ch)) out += ch.toLowerCase();
    else if (out !== '' && !out.endsWith('-')) out += '-';
  }
  return out.replace(/-+$/, '');
}

/** What a table is to be made as: its name, what it is called for people where that is other, and the key it is applied under. */
export interface Named {
  name: string;
  title: string;
  key: string;
}

/**
 * What a table typed as `typed` is made as, or why it is refused before
 * anything is asked: a name with no letter or digit in it, one too long to
 * be read, or one the project has a table of (`names`, every table it has,
 * the board and the tables beside another among them).
 */
export function tableNamed(typed: string, names: string[], project: string): Named | { wrong: string } {
  const name = slugOf(typed);
  if (name === '') return { wrong: 'A table is called something with a letter or a digit in it.' };
  const titled = tableTitled(typed);
  if ('wrong' in titled) return titled;
  if (names.includes(name)) return { wrong: `${project} has a table called ${name} already: call this one something else.` };
  const title = titled.title ?? '';
  return { name, title: title === name ? '' : title, key: `table-${name}` };
}

/** What `std/table` is applied with for a table made so. */
export function applied(project: string, named: Named, layout: string, environment: string | null) {
  return {
    blueprint: 'std/table',
    project,
    key: named.key,
    params: { name: named.name, title: named.title, layout },
    ...(environment ? { environment } : {}),
    wait: true,
  };
}
