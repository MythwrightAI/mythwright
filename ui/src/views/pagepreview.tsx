// The card over a link to a page (`pagelink.tsx`): what the page is, where
// it is and the first lines of what it says, or of the reply the link is
// to, so that a reader can see where a link leads without leaving the
// page they are on. What it says is worked out in `pagepreview.ts`.
//
// It is drawn from the line of the page the client holds already. Where
// that does not say enough (a page with no summary, or a link to a reply),
// the page whole is read once, as the reader, and kept while the client
// runs; what the daemon says of a page lets go of what was kept of it, so
// that the next card reads it again as it is. Nothing is read for being
// pointed at again, and nothing is read on a timer.

import { effect, signal } from '@preact/signals';
import type { RefObject } from 'preact';
import { useEffect } from 'preact/hooks';
import { stream } from '../events.ts';
import { previewOf, wantsWhole } from '../pagepreview.ts';
import { replyOf } from '../replies.ts';
import type { Reply } from '../replies.ts';
import { topicOfEvent, wholeOf } from '../topicpage.ts';
import type { TopicLine, TopicWhole } from '../topicpage.ts';
import { topics, topicsApi, topicsRead } from '../topicsread.ts';
import { Floating } from './offered.tsx';

/** How wide the card is, where the window has the room. */
const CARD_WIDTH = 340;

type Whole = { topic: TopicWhole; replies: Reply[] };

/** Each page read whole for a card, by its id; `failed` where the reader could not read it. */
const wholes = signal<ReadonlyMap<string, Whole | 'failed'>>(new Map());
/** Each page being read, and whether something was said of it since it was asked for: an answer that may be from before is not kept. */
const asking = new Map<string, boolean>();

function keep(id: string, whole: Whole | 'failed' | null): void {
  const next = new Map(wholes.peek());
  if (whole === null) next.delete(id);
  else next.set(id, whole);
  wholes.value = next;
}

/** Reads a page whole, where it is neither kept nor being read. */
function wantWhole(id: string): void {
  if (wholes.peek().has(id) || asking.has(id)) return;
  asking.set(id, false);
  const answered = (whole: Whole | 'failed') => {
    const since = asking.get(id) ?? true;
    asking.delete(id);
    // Something was said of it while it was read: what was read may be from before, and a card shown reads it again.
    keep(id, since ? null : whole);
  };
  topicsApi.read(id).then(
    (read) => answered({ topic: wholeOf(read.topic), replies: read.replies.map(replyOf) }),
    () => answered('failed'),
  );
}

/** Lets go of what was kept of a page, or of every page. */
function letGo(id: string | null): void {
  for (const one of asking.keys()) if (id === null || one === id) asking.set(one, true);
  if (id === null) wholes.value = new Map();
  else if (wholes.peek().has(id)) keep(id, null);
}

stream.on(['record.*', 'topic.*'], (event) => {
  const of = topicOfEvent(event);
  if (of) letGo(of.topic);
});
// What was said while the page could not hear was not heard; and nothing is kept for whoever signs in next.
stream.onStatus((up) => {
  if (up) letGo(null);
});
effect(() => {
  if (!topicsRead.value) letGo(null);
});

/** The card over a link to the page `line`, or to its reply said at `reply`. `id` is what the link is described by. */
export function PagePreview({ anchor, line, reply, id, onEnter, onLeave, onOpen }: { anchor: RefObject<HTMLElement | null>; line: TopicLine; reply: number | null; id: string; onEnter: () => void; onLeave: () => void; onOpen: (event: MouseEvent) => void }) {
  const wanted = wantsWhole(line, reply);
  const all = wholes.value;
  const kept = all.get(line.id);
  // Asked again whenever what is kept changes: what was let go of while the card is shown is read again.
  useEffect(() => {
    if (wanted) wantWhole(line.id);
  }, [wanted, line.id, all]);
  const failed = kept === 'failed';
  const shown = previewOf(topics.value, line, reply, kept && kept !== 'failed' ? kept : null);
  const reading = shown.reading && !failed;
  const href = anchor.current?.getAttribute('href') ?? undefined;
  return (
    <Floating
      beside={() => {
        const box = anchor.current?.getBoundingClientRect();
        return box ? { left: box.left - 12, above: box.top - 4, below: box.bottom + 4, first: 'under', across: CARD_WIDTH } : null;
      }}
      class="page-card"
      role="tooltip"
      label={`About ${line.title}`}
    >
      <div class="page-card-in" id={id} onMouseEnter={onEnter} onMouseLeave={onLeave} data-topic={line.id}>
        <div class="page-card-head">
          <strong class="page-card-title">{shown.title}</strong>
          {(shown.wiki || shown.under) && (
            <span class="page-card-where">
              {shown.wiki && <>in {shown.wiki}</>}
              {shown.wiki && shown.under && ' · '}
              {shown.under && <>under {shown.under}</>}
            </span>
          )}
        </div>
        {reply !== null ? (
          <div class="page-card-reply">
            {shown.reply === null ? (
              <span class="faint">{failed ? 'The reply could not be read.' : 'Reading the reply…'}</span>
            ) : shown.reply === 'gone' ? (
              <span class="faint">This reply is no longer there.</span>
            ) : (
              <>
                <span class="page-card-who">{shown.reply.who} said</span>
                <span class="page-card-says">{shown.reply.says ?? <span class="faint">Nothing written.</span>}</span>
              </>
            )}
          </div>
        ) : (
          <p class="page-card-says">{shown.says ?? <span class="faint">{reading ? 'Reading…' : 'Nothing written yet.'}</span>}</p>
        )}
        <div class="page-card-foot">
          <span class="faint">{shown.replies}</span>
          <a class="page-card-open" href={href} tabIndex={-1} onClick={onOpen}>
            Open
          </a>
        </div>
      </div>
    </Floating>
  );
}
