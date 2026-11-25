// Comments on a few words of a document, on its page: the marks over the
// words in Write and in Source, a comment begun on words chosen, and the
// comments of a mark opened beside the text where there is room for them
// and in the Comments tab of the panel beside the document where there is
// not (`anchored.ts` says which).
//
// What is opened is put away by its own button, by Escape, and by a press
// anywhere but on it or on another mark, as a menu is; what was being
// written in it is kept for those words while it is away.

import { useEffect, useMemo, useState } from 'preact/hooks';
import { onWords, threadOf } from '../anchored.ts';
import type { OnWords, Shown, Thread } from '../anchored.ts';
import { aboutOf, blocksOfMarkdown, found } from '../anchors.ts';
import type { Anchor } from '../anchors.ts';
import { comments } from '../journey.ts';
import { classes } from '../util.ts';
import type { Placed } from '../write/writer.tsx';
import { AddComment, CommentList } from './comments.tsx';
import { useJourney } from './journey.tsx';

/** A mark's comments, or a comment begun on words chosen: what they are about, each said, and the line one is added by. */
export function ThreadCard({ record, thread, asked, onClose, aside }: { record: string; thread: Thread; asked: number; onClose?: () => void; aside?: boolean }) {
  const lines = useJourney(record);
  const list = threadOf(comments(lines ?? []), thread);
  return (
    <section class={classes('words-thread', aside && 'aside')} aria-label={`Comments on “${aboutOf(thread.anchor)}”`}>
      <div class="words-thread-head">
        <q class="words-thread-on">{aboutOf(thread.anchor, 120)}</q>
        {onClose && (
          <button type="button" class="ghost words-thread-close" aria-label="Close the comments" title="Close (Escape)" onClick={onClose}>
            ×
          </button>
        )}
      </div>
      {list.length > 0 && <CommentList list={list} />}
      <AddComment record={record} on={thread.anchor} asked={asked} />
    </section>
  );
}

/** No comment's card is open. */
const NONE: number[] = [];

/** Opened in the margin: where, and on what. */
interface Opened {
  thread: Thread;
  top: number;
  margin: { left: number; width: number };
}

/**
 * Everything a document's page needs for comments on its words: which
 * are on words, whether those are found in the text as it is, and what a
 * mark pressed or a comment begun opens. `beside` is what lies in the
 * margin of the page that scrolls, for the block editor to hold. Where
 * there is no room in the margin they are opened in the panel beside the
 * document (`aside`: the thread showing there, if any, and what shows one).
 */
export function useThreads(record: string | null, text: string, shown: Shown, aside: { thread: Thread | null; show: (thread: Thread) => void }) {
  const lines = useJourney(record ?? '');
  const marked: OnWords[] = useMemo(() => (record ? onWords(comments(lines ?? [])) : []), [record, lines]);
  const blocks = useMemo(() => blocksOfMarkdown(text), [text]);
  const isFound = useMemo(() => {
    const seen = new Map<Anchor, boolean>();
    return (anchor: Anchor) => {
      if (!seen.has(anchor)) seen.set(anchor, found(anchor, blocks) !== null);
      return seen.get(anchor)!;
    };
  }, [blocks]);
  const [opened, setOpened] = useState<Opened | null>(null);
  const [asked, setAsked] = useState(0);
  // The comments whose card is open, wherever it is: their mark is drawn as the open one.
  const current = opened?.thread ?? aside.thread;
  const openSeqs = useMemo(
    () => (current ? [...new Set([...current.seqs, ...threadOf(comments(lines ?? []), current).map(({ line }) => line.seq)])] : NONE),
    [current, lines],
  );
  // Whether the page made room beside its column, which what stands over the text keeps to.
  const [margined, setMargined] = useState(false);

  // Away with it where the page is no longer one that marks, or the document is another.
  useEffect(() => setOpened(null), [record, shown]);
  useEffect(() => {
    if (!opened) return;
    const away = (event: Event) => {
      const on = event.target as Element | null;
      if (on?.closest?.('.writer-card, .commented, .cm-commented, .select-bar')) return;
      setOpened(null);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      // It is the card's alone: the frame would put a drawer away on the same key.
      event.stopPropagation();
      setOpened(null);
    };
    document.addEventListener('mousedown', away, true);
    window.addEventListener('keydown', escape, true);
    return () => {
      document.removeEventListener('mousedown', away, true);
      window.removeEventListener('keydown', escape, true);
    };
  }, [opened !== null]);

  const show = (thread: Thread, placed: Pick<Placed, 'top' | 'margin'>) => {
    if (!record) return;
    setAsked((times) => times + 1);
    if (placed.margin) {
      setOpened({ thread, top: placed.top, margin: placed.margin });
      return;
    }
    setOpened(null);
    aside.show(thread);
  };
  const begin = (anchor: Anchor, placed: Placed) => show({ anchor, seqs: [] }, placed);
  const open = (seqs: number[], anchor: Anchor, placed: Pick<Placed, 'top' | 'margin'>) => show({ anchor, seqs }, placed);
  const beside = opened && record && (
    <div class="writer-card" style={{ top: `${opened.top}px`, left: `${opened.margin.left}px`, width: `${opened.margin.width}px` }}>
      <ThreadCard record={record} thread={opened.thread} asked={asked} onClose={() => setOpened(null)} />
    </div>
  );
  return { lines, marked, isFound, begin, open, asked, beside, opened: openSeqs, margined, setMargined };
}
