// The small fixed drawings of the side panel's foot, of the right of a
// page's head, and of the menus they open: each a few strokes on a grid
// of sixteen, written out here.
//
// They are the client's own and are part of the page as it comes. The set
// of icons somebody chooses a column's from is a part of its own, asked
// for only where one of those is drawn (`icons.ts`), and nothing here asks
// for it.

import { letter } from '../help.ts';

/** One drawing, in the colour of the words beside it. */
function Sign({ d, size = 15, filled = false }: { d: string; size?: number; filled?: boolean }) {
  return (
    <svg class="sign" viewBox="0 0 16 16" width={size} height={size} fill={filled ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d={d} />
    </svg>
  );
}

const GEAR =
  'M6.6 1.8h2.8l.4 1.8 1.1.6 1.7-.6 1.4 2.4-1.3 1.2v1.6l1.3 1.2-1.4 2.4-1.7-.6-1.1.6-.4 1.8H6.6l-.4-1.8-1.1-.6-1.7.6L2 10l1.3-1.2V7.2L2 6l1.4-2.4 1.7.6 1.1-.6zM8 6a2 2 0 1 0 0 4a2 2 0 1 0 0-4z';
const ENVELOPE = 'M2 3.8h12v8.4H2zM2.3 4.2L8 8.8l5.7-4.6';
const PLUS = 'M8 3.2v9.6M3.2 8h9.6';
const PENCIL = 'M2.6 13.4l.7-3 7.4-7.4 2.3 2.3-7.4 7.4zM9.4 4.3l2.3 2.3';
// A robot's face: a feeler, a head with ears, two eyes and a mouth; few enough strokes to read at 15px.
const ROBOT =
  'M8 4V2.2M5 4h6a2.5 2.5 0 0 1 2.5 2.5v4A2.5 2.5 0 0 1 11 13H5a2.5 2.5 0 0 1-2.5-2.5v-4A2.5 2.5 0 0 1 5 4zM6 7.4v.6M10 7.4v.6M6.2 10.2h3.6M1.2 7.2v2.4M14.8 7.2v2.4';
const BIN = 'M3 4.4h10M6.2 4.4V2.8h3.6v1.6M4.2 4.4l.6 8.8h6.4l.6-8.8M6.8 6.8v4M9.2 6.8v4';
const DOOR = 'M9.6 2.6H3.4v10.8h6.2M7 8h6.6M11.2 5.6L13.6 8l-2.4 2.4';
const QUESTION = 'M5.9 6.1a2.2 2.2 0 1 1 3.3 1.9c-.8.5-1.2.9-1.2 1.8M8 12.1v.1';
const OUT = 'M9.2 2.8h4v4M13.2 2.8L7.6 8.4M6.4 4H2.8v9.2H12V9.6';
const COPY = 'M5.6 5.6h7.6v7.6H5.6zM3 10.4V2.8h7.6';
const SHEETS = 'M3.4 2.6h9.2v10.8H3.4zM5.6 5.6h4.8M5.6 8h4.8M5.6 10.4h2.8';
// Of a page's head and its menus: an address, a pin, what something is shared by, what happened to it, a file
// saved from the page and one read into it, where a file is, and what anybody may read.
const LINK = 'M6.9 9.1a2.5 2.5 0 0 0 3.5 0l2.3-2.3a2.5 2.5 0 0 0-3.5-3.5l-.9.9M9.1 6.9a2.5 2.5 0 0 0-3.5 0L3.3 9.2a2.5 2.5 0 0 0 3.5 3.5l.9-.9';
const PIN = 'M9.6 2.4l4 4-1.2.5-2.2 2.2-.3 2.5-5.5-5.5 2.5-.3 2.2-2.2zM7.1 8.9L2.8 13.2';
const SHARE = 'M8 10V2.6M5.3 5.2L8 2.5l2.7 2.7M3.5 8.6v4.6h9V8.6';
// A bell: what is followed tells whoever follows it.
const BELL = 'M4.2 11.2V7.4a3.8 3.8 0 0 1 7.6 0v3.8l1.2 1.2H3zM6.6 13.4a1.5 1.5 0 0 0 2.8 0';
const CLOCK = 'M8 2.4a5.6 5.6 0 1 0 0 11.2a5.6 5.6 0 1 0 0-11.2zM8 5v3.2l2.2 1.3';
const DOWN = 'M8 2.6V10M5.3 7.3L8 10l2.7-2.7M3.2 13.2h9.6';
const UP = 'M8 10V2.6M5.3 5.3L8 2.6l2.7 2.7M3.2 13.2h9.6';
const PATH = 'M2.8 11h3M10.2 5h3M6.6 13.2L9.4 2.8';
const GLOBE = 'M8 2.4a5.6 5.6 0 1 0 0 11.2a5.6 5.6 0 1 0 0-11.2zM2.4 8h11.2M8 2.4c-2.4 2.3-2.4 8.9 0 11.2M8 2.4c2.4 2.3 2.4 8.9 0 11.2';
// Of the bar that finds in a text: the match after this one, the one before, and the bar put away.
const BELOW = 'M8 3v10M4.4 9.4L8 13l3.6-3.6';
const ABOVE = 'M8 13V3M4.4 6.6L8 3l3.6 3.6';
const CROSS = 'M4.2 4.2l7.6 7.6M11.8 4.2l-7.6 7.6';
// A padlock: what is kept to its members.
const LOCK = 'M4.2 7.4h7.6v5.8H4.2zM5.9 7.4V5.3a2.1 2.1 0 0 1 4.2 0v2.1';
// A box with its lid: what is archived, read and written in by nobody.
const ARCHIVE = 'M2.4 3h11.2v2.8H2.4zM3.4 5.8v7.2h9.2V5.8M6.4 8.4h3.2';
// A shield: what a project made of a folder that was already there is, once it is trusted.
// Of an agent's line: one stopped (a square in a ring), and one moved to somewhere else (an arrow into a place).
const STOP = 'M8 2.4a5.6 5.6 0 1 0 0 11.2a5.6 5.6 0 1 0 0-11.2zM6.2 6.2h3.6v3.6H6.2z';
const MOVE = 'M2.6 8h7.2M7.4 5.4L10 8l-2.6 2.6M8.8 3.2h4.6v9.6H8.8';
const SHIELD = 'M8 1.9l5.2 1.9v3.9c0 3.1-2.2 5.4-5.2 6.4-3-1-5.2-3.3-5.2-6.4V3.8z';
// A list with its lines ticked off: the guide of first steps.
const GUIDE = 'M2.6 4.4l1.2 1.2 2-2.4M8 4.4h5.4M2.6 9.4l1.2 1.2 2-2.4M8 9.4h5.4M8 13h3.4';
// Of the head of a panel whose lists fold: everything folded, and everything unfolded.
const FOLD_ALL = 'M4.4 2.6L8 6.2l3.6-3.6M4.4 13.4L8 9.8l3.6 3.6';
const UNFOLD_ALL = 'M4.4 6L8 2.4 11.6 6M4.4 10L8 13.6l3.6-3.6';
// A speech bubble: what a channel's agents talk over between them.
const CHAT = 'M3.4 3h9.2a1.4 1.4 0 0 1 1.4 1.4v5.2a1.4 1.4 0 0 1-1.4 1.4H7.2l-3.2 2.4V11h-.6A1.4 1.4 0 0 1 2 9.6V4.4A1.4 1.4 0 0 1 3.4 3z';
// Somebody, a head and shoulders: who somebody is (a member's Details).
const PERSON = 'M8 2.6a2.6 2.6 0 1 0 0 5.2a2.6 2.6 0 1 0 0-5.2zM3 13.4c.4-2.6 2.4-4 5-4s4.6 1.4 5 4';
// A key: what somebody may ask of the factory.
const KEY = 'M5.4 6.8a2.8 2.8 0 1 0 0 5.6a2.8 2.8 0 1 0 0-5.6zM7.4 7.6l5.4-5.4M10.6 4.4l1.6 1.6M12 3l1.4 1.4';
// A list with a small cog at its foot: what is set up for a column of a board (the rows about it of the tables beside it).
const SET_UP = 'M2.4 3.6h8.2M2.4 7.2h6M2.4 10.8h3.6M11.6 8.6a2.2 2.2 0 1 0 0 4.4a2.2 2.2 0 1 0 0-4.4zM11.6 7.2v1.4M11.6 13v1.4M8.8 10.8h1.4M13 10.8h1.4';
// A post and two replies hanging from it: a forum's topics, the place of the rail they have.
const TOPIC = 'M2.4 2.6h11.2v4H2.4zM4.4 6.6v6.2M4.4 9.4h2M4.4 12.8h2M8.4 9.4h5.2M8.4 12.8h5.2';
// The product's mark: two strokes of a quill, filled, on a grid of its own.
// It stands for the program itself (the rail's way to the page about it), and is drawn in the colour
// of what is beside it as the rail's other places are.
const MARK_BOX = '0 0 519.43 508.9';
const MARK = [
  'M15.79,456.19c13.22-25.51,36.69-45.46,65.18-51.26C321.88,421.26,430.45,186.68,519.43,0c-31.2,196.95-134.33,454.47-371.23,445.37-37.94.38-95.93-2.94-132.41,10.82Z',
  'M0,507.17c85.85-122.45,338.72,112.94,491.12-321.1C439.45,551.34,309.59,507.31,0,507.17Z',
];
/** The small arrow beside what opens a menu. */
const CHEVRON = 'M4.4 6.4L8 10l3.6-3.6';

export const signs = {
  gear: () => <Sign d={GEAR} />,
  envelope: (size = 15) => <Sign d={ENVELOPE} size={size} />,
  plus: () => <Sign d={PLUS} />,
  pencil: () => <Sign d={PENCIL} />,
  bin: () => <Sign d={BIN} />,
  robot: () => <Sign d={ROBOT} />,
  door: () => <Sign d={DOOR} />,
  question: () => <Sign d={QUESTION} size={16} />,
  out: () => <Sign d={OUT} />,
  copy: () => <Sign d={COPY} />,
  sheets: () => <Sign d={SHEETS} />,
  chevron: () => <Sign d={CHEVRON} size={12} />,
  foldAll: () => <Sign d={FOLD_ALL} size={14} />,
  unfoldAll: () => <Sign d={UNFOLD_ALL} size={14} />,
  link: () => <Sign d={LINK} />,
  /** Filled once the thing is pinned. */
  pin: (on = false) => <Sign d={PIN} filled={on} />,
  share: () => <Sign d={SHARE} />,
  clock: () => <Sign d={CLOCK} />,
  bell: (on = false) => <Sign d={BELL} filled={on} />,
  down: () => <Sign d={DOWN} />,
  up: () => <Sign d={UP} />,
  path: () => <Sign d={PATH} />,
  globe: () => <Sign d={GLOBE} />,
  below: () => <Sign d={BELOW} size={14} />,
  above: () => <Sign d={ABOVE} size={14} />,
  cross: () => <Sign d={CROSS} size={14} />,
  guide: () => <Sign d={GUIDE} />,
  /** Before what a private channel is called; smaller in a line of the side panel. */
  lock: (size = 14) => <Sign d={LOCK} size={size} />,
  archive: (size = 15) => <Sign d={ARCHIVE} size={size} />,
  shield: () => <Sign d={SHIELD} />,
  stop: () => <Sign d={STOP} />,
  move: () => <Sign d={MOVE} />,
  chat: () => <Sign d={CHAT} />,
  person: () => <Sign d={PERSON} />,
  key: () => <Sign d={KEY} />,
  setUp: (size = 14) => <Sign d={SET_UP} size={size} />,
  topic: (size = 15) => <Sign d={TOPIC} size={size} />,
  /** The product's mark, filled in the colour of what is beside it. */
  mark: (size = 15) => (
    <svg class="sign mark" viewBox={MARK_BOX} width={size} height={size} fill="currentColor" aria-hidden="true">
      {MARK.map((d) => (
        <path key={d} d={d} />
      ))}
    </svg>
  ),
};

/**
 * What stands for a workspace where it has no picture: the first letter
 * of what it is called, on a quiet ground. `none` is what is in no
 * workspace, which is called nothing.
 */
export function Face({ name, none }: { name: string; none?: boolean }) {
  return (
    <span class={none ? 'space-face none' : 'space-face'} aria-hidden="true">
      {none ? '' : letter(name)}
    </span>
  );
}
