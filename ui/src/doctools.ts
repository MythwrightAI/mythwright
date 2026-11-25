// What the panel beside a document holds, as plain values (`views/doctools.tsx` draws it).
//
// A document of a library has what was said on it, what it is and what
// happened to it in a panel beside its text, opened from the speech bubble
// in its head beside the gear, as the Tools panel is opened beside a
// channel. It has a few tabs, in one order: Comments, every comment on the
// document with the line one is added by; Details, what the document is;
// and History, everything that happened to it. Which shows, and whether
// the panel is open, is kept by the document's tab, so that it is as it
// was left when the tab is shown again.

/** A tab of the panel, by what it is kept as. */
export type DocTab = 'comments' | 'details' | 'history';

/** The tabs, in the order they are drawn, each with what it is called. */
export const DOC_TABS: { id: DocTab; name: string }[] = [
  { id: 'comments', name: 'Comments' },
  { id: 'details', name: 'Details' },
  { id: 'history', name: 'History' },
];

/** The tab that shows for what was kept: one of the tabs, else Comments. */
export function tabOf(kept: unknown): DocTab {
  return DOC_TABS.some((one) => one.id === kept) ? (kept as DocTab) : 'comments';
}

/** What the button in the head is called for whoever hears it read out, with how many comments there are where any are. */
export function toolSaid(count: number): string {
  return count === 0 ? 'Comments and details' : `Comments and details, ${count === 1 ? '1 comment' : `${count} comments`}`;
}

/** What the panel says a document is, a line each: what is known of it and nothing made up for what is not. */
export interface Known {
  path: string;
  /** What its library is called for people. */
  library: string;
  /** How big its file is, as the project's list of files says, where it does. */
  size: string | null;
  lines: number;
  /** When it was last changed and by whom, in words, where that is known. */
  edited: string | null;
  /** Whether it is marked public, where its library has the mark: nothing where it has none, or it is not read yet. */
  public: boolean | null;
  /** Whether its library has the mark at all. */
  marks: boolean;
  /** How many comments there are on it, once its history is read. */
  comments: number | null;
}

/** The lines of Details: what each is, and what it says. */
export function detailsOf(known: Known): [string, string][] {
  const lines: [string, string][] = [
    ['Path', known.path],
    ['Library', known.library],
    ['Size', [known.size, known.lines === 1 ? '1 line' : `${known.lines} lines`].filter(Boolean).join(' · ')],
  ];
  if (known.edited) lines.push(['Changed', known.edited]);
  if (known.marks) lines.push(['Public', known.public === null ? 'Reading' : known.public ? 'Yes: written out when its library is' : 'No']);
  if (known.comments !== null) lines.push(['Comments', known.comments === 0 ? 'None yet' : String(known.comments)]);
  return lines;
}
