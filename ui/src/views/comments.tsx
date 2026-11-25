// Comments on a record: what was said on it, apart from what happened to it.
//
// A record's history keeps both, in the order it all happened. A comment
// is a line of it that is something said; this draws those by themselves,
// with a line to add one, wherever the thing commented on is looked at:
// in the panel beside a document's text on its own page (`doctools.tsx`),
// and in the drawer for a mockup.
// A list of records shows which of them have any, with a mark that says
// how many and opens the record with its comments showing.
//
// Nothing here is asked for on a timer. A record's comments are read with
// its history and again when a line is written to it; how many each
// record of a table has is read once for the table and again when a line
// of something said is written for one of them.

import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { api, call } from '../api.ts';
import type { HistoryLine } from '../api.ts';
import { keyOf, saidOn } from '../anchored.ts';
import type { Anchor } from '../anchors.ts';
import { commentedOn, Latest, markCalled, WRITTEN } from '../commented.ts';
import { coalesce } from '../events.ts';
import { comments, happened } from '../journey.ts';
import type { Comment } from '../journey.ts';
import { attempt, draftOf, keepDraft } from '../store.ts';
import { OfRecord, Written } from './pathlink.tsx';
import { useEvents } from '../ui.tsx';
import { classes, when } from '../util.ts';
import { Lines, useJourney } from './journey.tsx';
import { useKept } from './returned.tsx';

/** A speech bubble: what stands for comments wherever they are counted or opened. */
export function CommentIcon() {
  return (
    <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round" aria-hidden="true">
      <path d="M2.5 3.5h11v7.5h-6l-3 2.5v-2.5h-2z" />
    </svg>
  );
}

/**
 * How many things were said on each record of a table, by the record's
 * id; nothing for one nothing was said on, and nothing at all until it is
 * read, or where it may not be. One request for the table, and another
 * when a line of something said is written for one of its records.
 */
export function useCommented(project: string, name: string): Record<string, number> {
  const [counts, setCounts] = useKept<Record<string, number>>(`commented:${project}/${name}`, {});
  // Another table's counts are not this one's; the counts its tab last had are, until they are read again.
  const was = useRef(`${project}/${name}`);
  const reload = useMemo(() => {
    const latest = new Latest();
    const read = () => call<{ commented: Record<string, number> }>('GET', `/collections/${encodeURIComponent(project)}/${encodeURIComponent(name)}/commented`);
    // Where it cannot be read the marks say nothing of how many, and are still there to open.
    return coalesce(() => void latest.read(read).then((got) => got && setCounts(got.read.commented), () => {}), 100);
  }, [project, name]);
  useEffect(() => {
    if (was.current !== `${project}/${name}`) setCounts({});
    was.current = `${project}/${name}`;
    reload();
  }, [reload]);
  useEvents([WRITTEN], (event) => commentedOn(event, project, name) && reload(), [project, name]);
  return counts;
}

/**
 * The mark at the end of a record's line in a list. Where something was
 * said it shows how many; where nothing was it is drawn only while its
 * line is pointed at or it is reached by keyboard, in room that is kept
 * for it, so that nothing in the line moves.
 */
export function CommentMark({ count, title, onOpen }: { count: number; title: string; onOpen: () => void }) {
  return (
    <button class={classes('ghost comment-mark', count > 0 && 'has')} aria-label={markCalled(count, title)} title={count > 0 ? 'Open it with what was said on it' : 'Open it to comment on it'} onClick={onOpen}>
      <CommentIcon />
      {count > 0 && <span class="comment-count">{count}</span>}
    </button>
  );
}

/**
 * What is being written on a record and is not sent yet. It is kept in the store: the page it is written on is put
 * away when another tab is shown. On a few words of a document (`on`), it is kept for those words and sent with them.
 */
function useSaying(record: string, asked = 0, on?: Anchor): { saying: string; write: (text: string) => void; say: () => Promise<void> } {
  const unsaid = on ? `say:${record}@${keyOf(on)}` : `say:${record}`;
  const [saying, setSaying] = useState(draftOf(unsaid)?.text ?? '');
  // Another record shown where this one was has its own; and asked for, what was being written may have been begun
  // elsewhere (a block's Comment), and is read again.
  useEffect(() => setSaying(draftOf(unsaid)?.text ?? ''), [unsaid, asked]);
  const write = (text: string) => {
    setSaying(text);
    keepDraft(unsaid, text.trim() ? { text, base: '' } : null);
  };
  const say = async () => {
    const words = saying.trim();
    if (words && (await attempt(() => api.say(record, words, false, on)))) write('');
  };
  return { saying, write, say };
}

/**
 * A comment begun from elsewhere on the page (a block's Comment, which names
 * the block): what was being written is kept with what it begins, and the
 * comments are then asked for, which puts the cursor at its end.
 */
export function commentBegun(record: string, begun: (draft: string) => string, ask: () => void): void {
  const unsaid = `say:${record}`;
  const text = begun(draftOf(unsaid)?.text ?? '');
  keepDraft(unsaid, text.trim() ? { text, base: '' } : null);
  ask();
}

/**
 * The line a comment is added by. Enter sends it and Shift with Enter
 * begins another line; it is as tall as what is written in it. `asked`
 * counts the times the comments were asked for by name, and puts the
 * cursor here each time.
 */
export function AddComment({ record, asked, on }: { record: string; asked?: number; on?: Anchor }) {
  const { saying, write, say } = useSaying(record, asked, on);
  const box = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    const area = box.current;
    if (!area) return;
    area.style.height = 'auto';
    area.style.height = `${area.scrollHeight + 2}px`;
  }, [saying]);
  useEffect(() => {
    if (!asked) return;
    box.current?.focus();
    // At the end of what is there, once it is drawn: after what a block's Comment began.
    requestAnimationFrame(() => {
      const area = box.current;
      if (area) area.setSelectionRange(area.value.length, area.value.length);
    });
  }, [asked]);
  return (
    <div class="comment-add">
      <textarea
        ref={box}
        rows={1}
        value={saying}
        aria-label="Add a comment"
        placeholder="Add a comment…"
        onInput={(event) => write(event.currentTarget.value)}
        onKeyDown={(event) => {
          if (event.key !== 'Enter' || event.shiftKey || event.isComposing) return;
          event.preventDefault();
          void say();
        }}
      />
      <button type="button" disabled={!saying.trim()} onClick={() => void say()}>
        Comment
      </button>
    </div>
  );
}

/**
 * Comments as they are read: who, when, whether it was marked public, and the words. One on a few words of a
 * document says what they were where it is shown away from them (`onWords`: whether they are still in the text).
 */
export function CommentList({ list, onWords }: { list: Comment[]; onWords?: (anchor: Anchor) => boolean }) {
  return (
    <ol class="comment-list">
      {list.map(({ line, said }) => (
        <li key={line.seq} class="comment">
          {said.on && onWords && <span class={classes('comment-on', !onWords(said.on) && 'gone')}>{saidOn(said.on, onWords(said.on))}</span>}
          <span class="comment-by faint">
            <strong class="comment-who">{said.who}</strong>
            {said.asks ? ' asked' : ''} · <time dateTime={new Date(line.ts).toISOString()}>{when(line.ts)}</time>
            {said.public && (
              <span class="journey-public" title="Marked public by whoever said it: it may be told outside the factory. What is not marked stays here.">
                public
              </span>
            )}
          </span>
          <span class="comment-words">
            <Written text={said.words} />
          </span>
        </li>
      ))}
    </ol>
  );
}

/**
 * The comments on something that has no page of its own to show them on,
 * in the drawer: all of them, the line to add one, and after them what
 * happened to it, which is the rest of its history.
 */
export function Commented({ record, path, empty }: { record: string; path: string; empty: string }) {
  const lines = useJourney(record);
  const said = comments(lines ?? []);
  const rest: HistoryLine[] | null = lines && happened(lines);
  return (
    <div class="stack">
      <code class="faint truncate">{path}</code>
      <section class="comments in-drawer" aria-label="Comments">
        <h3>Comments</h3>
        {lines === null && <p class="faint">Reading</p>}
        {lines !== null && said.length === 0 && <p class="faint">Nobody has commented on it yet.</p>}
        {said.length > 0 && (
          <OfRecord record={record}>
            <CommentList list={said} />
          </OfRecord>
        )}
        <AddComment record={record} />
      </section>
      <Lines lines={rest} empty={empty} />
    </div>
  );
}
