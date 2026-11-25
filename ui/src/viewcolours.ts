// The reader's colours, as a page attached to a message is told them.
//
// A page is written by somebody who has not seen the client's own names
// for its colours, and reads each name for what it usually means. So each
// is told under the name its author would look for it by, which is not
// always the client's: here `card` is the yellow of a piece of work on a
// board, and to anyone writing a page a card is a box with something in it.

/** The name a page knows each colour by, and the client's own name for it. */
export const VIEW_COLOURS: Record<string, string> = {
  text: 'text',
  dim: 'dim',
  faint: 'faint',
  line: 'line',
  accent: 'accent',
  ok: 'ok',
  bad: 'bad',
  warn: 'card',
  info: 'info',
  agent: 'agent',
  bg: 'bg0',
  panel: 'bg1',
  card: 'bg2',
};

/** The colours a page is told, given how the client's own are read. */
export function viewColours(read: (name: string) => string): Record<string, string> {
  return Object.fromEntries(Object.entries(VIEW_COLOURS).map(([told, own]) => [told, read(own)]));
}
