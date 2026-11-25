// Finding in a text the editor shows: whose a key is that is pressed on
// the editor's page with the keyboard outside its text, and what the bar
// says of what was found. Nothing here is the editor's or the page's, so
// that each is a function with its cases.
//
// A key pressed in the text, or in the bar, is the editor's own and is
// answered there (`find.tsx`). A text that is only read cannot hold the
// keyboard, and somebody reading one has it wherever they last pressed:
// so the page hears the keys too, and this says which of them are for
// the editor that is showing and which are left where they were.

/** What a key pressed on the page asks of the editor that is showing. */
export type Asked = 'find' | 'next' | 'before' | 'away';

/** As much of a key pressed as is gone by. */
export interface Key {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
}

/**
 * What a key asks, or nothing: the key that finds, the two that go to the
 * next match and the one before, and Escape. The key held with a letter
 * is ⌘ on a Mac and Ctrl elsewhere, as it is in the text.
 */
export function asks(event: Key, mac: boolean): Asked | null {
  const held = (mac ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey) && !event.altKey;
  const letter = event.key.toLowerCase();
  if (held && !event.shiftKey && letter === 'f') return 'find';
  if ((held && letter === 'g') || (event.key === 'F3' && !event.ctrlKey && !event.metaKey && !event.altKey)) return event.shiftKey ? 'before' : 'next';
  if (event.key === 'Escape' && !event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey) return 'away';
  return null;
}

/**
 * Where the keyboard is as a key is pressed: in this editor (its text or
 * its bar), in another editor that is showing, in a field somebody types
 * in, on something of another pane than this editor's, on something else
 * of the page (a button of this pane's head, a line of the side panel),
 * or nowhere, which is where it is once somebody has pressed on a text
 * that is only read.
 */
export type Keyboard = 'here' | 'another' | 'field' | 'pane' | 'page' | 'nowhere';

export interface Pressed {
  keyboard: Keyboard;
  /** Whether this editor is in the pane that was last used. */
  focused: boolean;
  /** Whether a dialog lies over the page. */
  over: boolean;
  /** Whether this editor's bar is open. */
  open: boolean;
  /** Whether a drawer is open beside the page. */
  drawer: boolean;
}

/**
 * Whether a key pressed on the page is this editor's to answer.
 *
 * In an editor the key is that editor's own, and is not answered twice.
 * In a field it is being typed, and under a dialog the page is not what
 * is being used: both are left to the browser. So is a key pressed on
 * something of another pane, or while another pane was the last used:
 * the page a key is for is the pane somebody is in. Otherwise the key
 * that finds is for the editor that is showing there. The keys that go
 * on to a match are answered only while the bar is open. Escape puts the bar away only
 * where nothing else can be meant by it: nothing has the keyboard, and
 * no drawer is open, which Escape puts away first.
 */
export function answers(asked: Asked, pressed: Pressed): boolean {
  const { keyboard } = pressed;
  if (keyboard === 'here' || keyboard === 'another' || keyboard === 'field' || keyboard === 'pane' || pressed.over) return false;
  if (!pressed.focused) return false;
  if (asked === 'find') return true;
  if (asked === 'away') return pressed.open && keyboard === 'nowhere' && !pressed.drawer;
  return pressed.open;
}

/** What was found of what is searched for, as the bar is told it. */
export interface Found {
  /** Whether anything is searched for. */
  asked: boolean;
  /** Whether what is searched for reads: an expression may not. */
  reads: boolean;
  /** How many matches there are, counted as far as `more` says. */
  matches: number;
  /** Whether there are more than were counted. */
  more: boolean;
  /** Which of them the caret is at, from nought, or -1 where it is at none. */
  at: number;
}

/** What the bar says of it: which match this is of how many, or why there is none. */
export function counted(found: Found): string {
  if (!found.asked) return '';
  if (!found.reads) return 'Not an expression';
  if (found.matches === 0) return 'No matches';
  const many = `${found.matches}${found.more ? '+' : ''}`;
  if (found.at >= 0) return `${found.at + 1} of ${many}`;
  return found.matches === 1 && !found.more ? '1 match' : `${many} matches`;
}

/** What the bar opens with for what is selected in the text: the selection where it is one line, and otherwise nothing. */
export function oneLine(selected: string): string {
  return /[\r\n]/.test(selected) ? '' : selected;
}

/** Which of the matches, in the order they stand in the text, is the one selected: from nought, or -1. */
export function whichOf(matches: readonly { from: number; to: number }[], from: number, to: number): number {
  let low = 0;
  let high = matches.length - 1;
  while (low <= high) {
    const mid = (low + high) >> 1;
    const one = matches[mid]!;
    if (one.from === from) return one.to === to ? mid : -1;
    if (one.from < from) low = mid + 1;
    else high = mid - 1;
  }
  return -1;
}

/** How far the bar stands from the edge of the text it lies over, and the narrowest it is put beside a drawer. */
const EDGE = 16;
const BESIDE = 8;
const NARROWEST = 280;

/**
 * Where the bar is put over an editor that is `width` wide, whose line
 * numbers take `gutters` of it at the left and of which a drawer lies
 * over `covered` at the right: how far from the editor's right edge, and
 * how wide at most. It stands clear of the line numbers and of the
 * drawer; where the drawer leaves it no room (on a phone the drawer has
 * the screen) it stays where it was, and the drawer lies over it.
 */
export function placed(at: { width: number; gutters: number; covered: number }): { right: number; widest: number } {
  const free = Math.max(0, at.width - at.gutters - BESIDE - EDGE);
  const beside = at.width - at.gutters - at.covered - 2 * BESIDE;
  if (at.covered <= 0 || beside < NARROWEST) return { right: EDGE, widest: free };
  return { right: at.covered + BESIDE, widest: beside };
}
