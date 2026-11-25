// The panel beside a document, opened from the speech bubble at the end
// of its head's row, just after the gear: what was said on it, what it is
// and what happened to it, a tab each (`doctools.ts`), as the Tools panel
// is beside a channel.
//
// It comes in and goes out as the Tools panel does (`toolsslide.ts`): the
// document is laid out once for an open and once for a close, and only the
// panel moves. It is as wide as it was left, dragged by its edge as that
// one is (`edge.tsx`), and kept in the browser apart from it. On a phone it
// has the screen, with the way back at its head. Whether it is open and
// which tab shows are the document's tab's (`useKept`, in `file.tsx`), so
// that it is as it was left when the tab is shown again.
//
// Comments are every comment on the document, on the whole of it and on a
// few of its words, oldest first, each on words saying what it is on and
// whether those words are still there, with the line one is added by; and,
// where a mark of the text is opened in a page with no room beside its
// column, that mark's comments at its head. Nothing here is read on a
// timer: what was said is the document's history, read once and again
// when a line is written to it.

import { signal } from '@preact/signals';
import { useEffect, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Collection, HistoryLine } from '../api.ts';
import type { Thread } from '../anchored.ts';
import type { Anchor } from '../anchors.ts';
import { tableCalled } from '../called.ts';
import { DEFAULT_WIDTH, LEAST_WIDTH, widthOf } from '../context.ts';
import { DOC_TABS, detailsOf } from '../doctools.ts';
import type { DocTab } from '../doctools.ts';
import { drawn } from '../drawer.ts';
import { saidOf } from '../edited.ts';
import { comments } from '../journey.ts';
import { phone } from '../phone.ts';
import { trees } from '../store.ts';
import { useEvents } from '../ui.tsx';
import { fileSize } from '../util.ts';
import { ThreadCard } from './anchored.tsx';
import { AddComment, CommentList } from './comments.tsx';
import { Edge, PANEL_WIDTH } from './edge.tsx';
import { useFileEdited } from './filehead.tsx';
import { Lines } from './journey.tsx';
import { OfRecord } from './pathlink.tsx';
import { marksPublic } from './shared.tsx';
import type { Slides } from './toolsslide.ts';

/** How the panel slides in and out beside the document's text (`toolsslide.ts`). */
export const DOC_SLIDES: Slides = { panel: 'doc-tools', coming: 'doc-tools-coming', leaving: 'doc-tools-leaving', of: 'Comments and details' };

/** Where how wide it was left is kept in the browser: the reader's, as the Tools panel's width is. */
const SAVED_WIDTH = 'client.doctools.width';

function keptWidth(): string | null {
  try {
    return localStorage.getItem(SAVED_WIDTH);
  } catch {
    return null;
  }
}

const docWidth = signal<number>(widthOf(keptWidth()));
/** Whether the reader has said how wide it is: until they do it is the usual width, which grows with the size of text. */
const docWidthSaid = signal<boolean>(keptWidth() !== null);

function setDocWidth(width: number, done: boolean): void {
  docWidth.value = width;
  docWidthSaid.value = true;
  if (!done) return;
  try {
    localStorage.setItem(SAVED_WIDTH, String(width));
  } catch {
    /* not remembered */
  }
}

/**
 * The style of what holds the document and the panel: the text, and the
 * panel beside it where it is open, as wide as it was left or as much of
 * that as leaves the text its share. The width is a property of the grid,
 * so that the panel's edge sets it straight onto the layout while it is
 * held (`edge.tsx`).
 */
export function besideStyle(beside: boolean): Record<string, string> {
  const width = docWidthSaid.value ? `${docWidth.value}px` : `calc(${docWidth.value}px * var(--size))`;
  return {
    [PANEL_WIDTH]: width,
    // The text keeps the greater share of a narrow tab: a heading's long word is not cut off for the panel.
    gridTemplateColumns: beside ? `minmax(min(360px, 60%), 1fr) minmax(min(${LEAST_WIDTH}px, 40%), var(${PANEL_WIDTH}))` : 'minmax(0, 1fr)',
  };
}

interface Props {
  project: string;
  path: string;
  /** The document it is, and the library it is of. */
  of: { library: Collection; record: string };
  /** Its history as the page read it, and how many lines its text has. */
  lines: HistoryLine[] | null;
  textLines: number;
  tab: DocTab;
  onTab: (tab: DocTab) => void;
  /** Puts the panel away. */
  onAway: () => void;
  /** Whether it comes in now, or is simply there (`toolsslide.ts`). */
  coming: boolean;
  /** How many times its comments were asked for by name: the line one is added by has the keyboard each time. */
  asked: number;
  /** A mark's comments opened here, with how many times one was asked for, and what puts them away. */
  thread: Thread | null;
  threadAsked: number;
  onCloseThread: () => void;
  /** Whether the words a comment is on are in the text as it is. */
  isFound: (anchor: Anchor) => boolean;
}

export function DocTools(props: Props) {
  const { tab, onTab } = props;
  const named = DOC_TABS.find((one) => one.id === tab)?.name ?? 'Comments';
  return (
    // It says when it is where it will be, as the Tools panel does: at once where nothing slides it in.
    <aside class={props.coming ? `doc-tools ${DOC_SLIDES.coming}` : 'doc-tools'} aria-label="Comments and details" ref={drawn}>
      <Edge what="panel of comments and details" usual={DEFAULT_WIDTH} onWidth={setDocWidth} />
      <div class="context-tabs tabs-ruled">
        {/* On a phone the panel has the screen, and the way back is at its own head. */}
        {phone.value && (
          <button class="ghost icon context-away" aria-label="Back to the document" onClick={props.onAway}>
            <svg width="20" height="20" viewBox="0 0 20 20" aria-hidden="true">
              <path d="M12.5 4.5 7 10l5.5 5.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          </button>
        )}
        <div role="tablist" aria-label="Comments and details" class="view-tabs-list">
          {DOC_TABS.map((one) => (
            <button key={one.id} role="tab" class="view-tab" aria-selected={one.id === tab} onClick={() => onTab(one.id)}>
              {one.name}
            </button>
          ))}
        </div>
      </div>
      <div class="context-body doc-tools-body" role="tabpanel" aria-label={named}>
        {tab === 'comments' && <Said {...props} />}
        {tab === 'details' && <Details {...props} />}
        {tab === 'history' && (
          <Lines
            lines={props.lines}
            empty="Nothing has been kept of this document yet. What is written, changed and said through here is; what another program does to the file is not, since nobody saw who did it."
          />
        )}
      </div>
    </aside>
  );
}

/** Every comment, oldest first, the line one is added by under them, and a mark's comments over them where one was opened here. */
function Said({ of, lines, asked, thread, threadAsked, onCloseThread, isFound }: Props) {
  const said = comments(lines ?? []);
  const box = useRef<HTMLElement>(null);
  // The newest is the last, by the line one is added by: in sight when it is added, and when the comments are asked for.
  const last = said.at(-1)?.line.seq;
  useEffect(() => {
    const body = box.current?.closest('.doc-tools-body');
    if (body && !thread) body.scrollTop = body.scrollHeight;
  }, [last, asked]);
  // A mark's comments are at the head, and in sight as they are opened.
  useEffect(() => {
    const body = box.current?.closest('.doc-tools-body');
    if (body && thread) body.scrollTop = 0;
  }, [thread, threadAsked]);
  return (
    <section class="doc-comments" aria-label="Comments" ref={box}>
      {thread && <ThreadCard key={JSON.stringify(thread.anchor)} record={of.record} thread={thread} asked={threadAsked} onClose={onCloseThread} aside />}
      {thread && <h3>Every comment</h3>}
      {lines === null && <p class="faint">Reading</p>}
      {lines !== null && said.length === 0 && <p class="faint">Nobody has commented on it yet. What is said here is kept with the document and seen by whoever reads it.</p>}
      {said.length > 0 && (
        <OfRecord record={of.record}>
          <CommentList list={said} onWords={isFound} />
        </OfRecord>
      )}
      <AddComment record={of.record} asked={asked} />
    </section>
  );
}

/** What the document is: where, of which library, how big, when it was changed and by whom, whether it is public, and the way to its history. */
function Details({ project, path, of, lines, textLines, onTab }: Props) {
  const edited = useFileEdited(project, path, of.record);
  const marks = marksPublic(of.library);
  const [shown, setShown] = useState<boolean | null>(null);
  // Whether it is marked public is the record's, read where its library has the mark, and again as it is changed.
  const read = () => {
    if (marks) void api.record(of.record).then((row) => setShown(row.data.public === true), () => setShown(null));
  };
  useEffect(read, [of.record, marks]);
  useEvents(['record.updated', 'project.file.changed'], (event) => event.data?.project === project && read(), [project, of.record, marks]);
  const entry = (trees.value[project] ?? []).find((one) => one.path === path);
  const rows = detailsOf({
    path,
    library: tableCalled(of.library),
    size: entry ? fileSize(entry.size) : null,
    lines: textLines,
    edited: edited ? saidOf(edited).whole : null,
    public: shown,
    marks,
    comments: lines === null ? null : comments(lines).length,
  });
  return (
    <section class="doc-details" aria-label="Details">
      <dl>
        {rows.map(([what, says]) => (
          <div key={what} class="doc-detail">
            <dt class="faint">{what}</dt>
            <dd>{what === 'Path' ? <code>{says}</code> : says}</dd>
          </div>
        ))}
      </dl>
      <button type="button" class="ghost link doc-history" onClick={() => onTab('history')}>
        Its history
      </button>
    </section>
  );
}
