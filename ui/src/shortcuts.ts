// The keys the client answers to, in one place, for the panel that lists
// them. Each is answered where it belongs (the frame, the composer, an
// editor); this says what there is, so a key added there is added here.

import { CHANNELS_CALLED } from './sidepanel.ts';

export interface Shortcut {
  does: string;
  /** The keys held together. `Mod` is Ctrl, or ⌘ on a Mac. */
  keys: string[];
  /** Listed only in the desktop app: a browser keeps these keys for itself, and the page never hears them; or they turn through tabs, which only the window has (`inplace.ts`). */
  desktop?: boolean;
}

export interface Section {
  title: string;
  rows: Shortcut[];
}

export const SHORTCUTS: Section[] = [
  {
    title: 'Anywhere',
    rows: [
      { does: 'Search everything', keys: ['Mod', 'K'] },
      { does: 'Search everything, as an editor searches its files', keys: ['Mod', 'Shift', 'F'] },
      { does: 'Collapse or expand the side panel', keys: ['Mod', '\\'] },
      { does: `${CHANNELS_CALLED}, Resources, the inbox, Extensions or Topics in the side panel`, keys: ['Alt', '1 2 3 4 5'] },
      { does: 'The next tab', keys: ['Ctrl', '.'], desktop: true },
      { does: 'The tab before', keys: ['Ctrl', ','], desktop: true },
      { does: 'The next tab, as a browser turns its own', keys: ['Ctrl', 'Tab'], desktop: true },
      { does: 'The tab before, the same way', keys: ['Ctrl', 'Shift', 'Tab'], desktop: true },
      { does: 'Keyboard shortcuts', keys: ['Mod', 'Shift', '/'] },
      { does: 'Close what is open over the page', keys: ['Esc'] },
    ],
  },
  {
    title: 'In a channel',
    rows: [
      { does: 'Write a message', keys: ['/'] },
      { does: 'Send', keys: ['Enter'] },
      { does: 'A new line', keys: ['Shift', 'Enter'] },
      { does: 'Mention someone', keys: ['@'] },
      { does: 'Mark another of the names offered', keys: ['↑ ↓'] },
      { does: 'Take the name that is marked', keys: ['Tab'] },
    ],
  },
  {
    title: 'Writing a message',
    rows: [
      { does: 'Bold', keys: ['Mod', 'B'] },
      { does: 'Italic', keys: ['Mod', 'I'] },
      { does: 'Underline', keys: ['Mod', 'U'] },
      { does: 'Strikethrough', keys: ['Mod', 'Shift', 'X'] },
      { does: 'Link', keys: ['Mod', 'Shift', 'U'] },
      { does: 'Numbered list', keys: ['Mod', 'Shift', '7'] },
      { does: 'Bulleted list', keys: ['Mod', 'Shift', '8'] },
      { does: 'Quotation', keys: ['Mod', 'Shift', '9'] },
      { does: 'Code', keys: ['Mod', 'Shift', 'C'] },
      { does: 'Block of code', keys: ['Mod', 'Alt', 'Shift', 'C'] },
      { does: 'The next item of a list', keys: ['Shift', 'Enter'] },
    ],
  },
  {
    title: 'A blueprint or a file, as text',
    rows: [
      { does: 'Save, which lays a blueprint out the one way', keys: ['Mod', 'S'] },
      { does: 'Lay a blueprint out without saving it', keys: ['Shift', 'Alt', 'F'] },
      { does: 'Find in the text', keys: ['Mod', 'F'] },
      { does: 'The next match, and with Shift the one before', keys: ['F3'] },
      { does: 'Find and replace, where the text may be changed', keys: ['Mod', 'Alt', 'F'] },
      { does: 'What can go here', keys: ['Ctrl', 'Space'] },
      { does: 'Take the suggestion that is marked', keys: ['Enter'] },
      { does: 'Undo', keys: ['Mod', 'Z'] },
      { does: 'Redo', keys: ['Mod', 'Shift', 'Z'] },
      { does: 'Further in', keys: ['Tab'] },
      { does: 'Back out', keys: ['Shift', 'Tab'] },
    ],
  },
  {
    title: 'A document, as blocks',
    rows: [
      { does: 'Save', keys: ['Mod', 'S'] },
      { does: 'Bold', keys: ['Mod', 'B'] },
      { does: 'Italic', keys: ['Mod', 'I'] },
      { does: 'Code', keys: ['Mod', 'E'] },
      { does: 'Undo', keys: ['Mod', 'Z'] },
      { does: 'Redo', keys: ['Mod', 'Shift', 'Z'] },
      { does: 'Another kind of block', keys: ['/'] },
      { does: 'A new line in the same block', keys: ['Shift', 'Enter'] },
      { does: 'A list item further in', keys: ['Tab'] },
      { does: 'A list item back out', keys: ['Shift', 'Tab'] },
    ],
  },
  {
    title: 'Choosing from options',
    rows: [
      { does: 'Mark another option', keys: ['↑ ↓'] },
      { does: 'Choose the one marked, or make what was typed', keys: ['Enter'] },
      { does: 'Move the one marked in the order', keys: ['Alt', '↑ ↓'] },
      { does: 'Take out the last one chosen', keys: ['Backspace'] },
    ],
  },
];

/** A key as it is written on the keyboard in front of whoever is reading. */
export function shown(key: string, mac: boolean): string {
  if (key === 'Mod') return mac ? '⌘' : 'Ctrl';
  if (mac && key === 'Alt') return '⌥';
  return key;
}

interface Pressed {
  key: string;
  /** Which key it is on the keyboard, whatever it types: `Digit1`. */
  code?: string;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  altKey?: boolean;
}

/** Whether a key press asks for the list of shortcuts: Ctrl (or ⌘), Shift and `/`, which with Shift is `?`. */
export function asksForShortcuts(event: Pressed): boolean {
  return (event.ctrlKey || event.metaKey) && event.shiftKey && (event.key === '?' || event.key === '/');
}

/**
 * What a key press asks of the search of everything: Ctrl (or ⌘) and K
 * opens it or closes it again; Ctrl, Shift and F opens it, as an editor's
 * search of all its files is opened, and leaves it open. Neither is a key
 * a browser keeps for its page (Ctrl and F alone is its own find).
 */
export function asksForSearch(event: Pressed): 'toggle' | 'open' | null {
  if (!(event.ctrlKey || event.metaKey) || event.altKey) return null;
  const key = event.key.toLowerCase();
  if (key === 'k' && !event.shiftKey) return 'toggle';
  if (key === 'f' && event.shiftKey) return 'open';
  return null;
}

/** Whether a key press folds the side panel away or brings it back: Ctrl (or ⌘) and `\`. */
export function foldsSide(event: Pressed): boolean {
  return (event.ctrlKey || event.metaKey) && !event.shiftKey && event.key === '\\';
}

/**
 * The views on the rail, by their number: Alt and a view's number shows
 * it. The numbers are what they were before Topics had a place of its own,
 * which is given the next; so they are not the order of the
 * rail, where Topics is just after the channels.
 */
export const RAIL = ['channels', 'resources', 'inbox', 'extensions', 'topics'] as const;

/** The view on the rail a key press asks for, if it asks for one: Alt and 1 to 5. */
export function railView(event: Pressed): (typeof RAIL)[number] | null {
  if (!event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return null;
  // By where the key is: with Alt held a number key types something else on some keyboards.
  const found = new RegExp(`^Digit([1-${RAIL.length}])$`).exec(event.code ?? '');
  return found ? RAIL[Number(found[1]) - 1]! : null;
}

/**
 * Whether a key press asks for the box a message is written in: `/` by
 * itself, pressed while nothing is being typed anywhere. Where something
 * is, a slash is a slash.
 */
export function asksForComposer(event: Pressed, typing: boolean): boolean {
  return event.key === '/' && !typing && !event.ctrlKey && !event.metaKey && !event.altKey;
}

/**
 * Which way a key press turns through the tabs; 0 when it is not that.
 * Ctrl with the full stop to the next and with the comma to the one before:
 * the keys with `>` and `<` on them, which a browser leaves to the page.
 * Ctrl and Tab too, with Shift to go back, where the page is told of them:
 * a browser turns its own tabs with those, the desktop app has none.
 */
export function turnsTabs(event: Pressed): 1 | -1 | 0 {
  if (!event.ctrlKey || event.metaKey || event.altKey) return 0;
  if (event.key === 'Tab') return event.shiftKey ? -1 : 1;
  // With Shift held the keyboard names the keys by what they then type.
  if (event.key === '.' || event.key === '>') return 1;
  if (event.key === ',' || event.key === '<') return -1;
  return 0;
}

/** The tab after the one showing, or the one before it, going round at the ends. `null` where there is no other. */
export function tabAfter(tabs: string[], showing: string, step: 1 | -1): string | null {
  if (tabs.length < 2) return null;
  const at = tabs.indexOf(showing);
  // With none of them showing, the first going forward and the last going back.
  if (at === -1) return step === 1 ? tabs[0]! : tabs.at(-1)!;
  return tabs[(at + step + tabs.length) % tabs.length]!;
}
