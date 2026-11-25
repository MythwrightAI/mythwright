// A topic's replies, drawn as a tree: each under the one it
// answers (`replies.ts`), with Reply beside it, which opens a box under it,
// and, for whoever may change the topic, what marks it as what answered the
// topic. A long chain starts folded past its third reply, saying how many
// are under the fold; the chain that holds the answer never does. What is
// typed in a box is kept by the topic's tab while another is shown, and
// who may be named there is offered as an `@` is typed (`TagBox`). A
// picture pasted, dropped or chosen there is attached to the topic and
// written in (`picturein.tsx`), and drawn in the reply (`pictured.tsx`).

import type { Ref } from 'preact';
import { useEffect, useRef, useState } from 'preact/hooks';
import { counted, startsFolded, tree } from '../replies.ts';
import { scrollsItself } from '../returned.ts';
import type { Node, Reply } from '../replies.ts';
import { attempt } from '../store.ts';
import { topicsApi } from '../topicsread.ts';
import { Prose } from '../ui.tsx';
import { classes, when } from '../util.ts';
import { usePicturesIn } from './picturein.tsx';
import { useKept, useKeptRef } from './returned.tsx';
import { TagBox } from './tagbox.tsx';

/** A box a reply is written in, under what it answers (`parent`), or for a new chain of the topic's own. */
export function ReplyBox(props: { topic: string; parent: number | null; onDone: () => void; onCancel?: () => void; label: string }) {
  const [draft, setDraft] = useKept<string>(`reply:${props.parent ?? 'topic'}`, '');
  const [busy, setBusy] = useState(false);
  // A picture pasted, dropped or chosen is attached to the topic and written in where the caret is.
  const pictures = usePicturesIn(props.topic, draft, setDraft);
  const send = async (event: Event) => {
    event.preventDefault();
    if (draft.trim() === '' || busy || pictures.arriving) return;
    setBusy(true);
    const sent = await attempt(() => topicsApi.reply(props.topic, draft, props.parent));
    setBusy(false);
    if (sent === undefined) return;
    setDraft('');
    props.onDone();
  };
  return (
    <form class="topic-reply-box" ref={pictures.held as Ref<HTMLFormElement>} onSubmit={send} onPaste={pictures.onPaste} onDragOver={pictures.onDragOver} onDrop={pictures.onDrop}>
      {/* Who may be named is offered as an `@` is typed, as on a card's comments; an agent named there answers under it. */}
      <TagBox
        among="topic"
        page={props.topic}
        label={props.label}
        placeholder={props.parent === null ? 'Say something in this topic' : 'Write a reply'}
        value={draft}
        rows={3}
        onInput={setDraft}
        onKeyDown={(event) => {
          if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) void send(event);
          if (event.key === 'Escape' && props.onCancel) {
            event.stopPropagation();
            props.onCancel();
          }
        }}
      />
      <div class="topic-reply-box-acts">
        {pictures.chooseButton}
        {pictures.arriving && <span class="faint topic-reply-arriving">Attaching…</span>}
        <span class="grow" />
        {props.onCancel && (
          <button type="button" class="ghost" onClick={props.onCancel}>
            Cancel
          </button>
        )}
        <button class="primary" type="submit" disabled={draft.trim() === '' || busy || pictures.arriving}>
          {busy ? 'Sending…' : props.parent === null ? 'Post' : 'Reply'}
        </button>
      </div>
    </form>
  );
}

interface Props {
  topic: string;
  replies: Reply[];
  /** The reply that answered the topic, by the number it was said at. */
  answer: number | null;
  /** Whether whoever reads it may reply, and may mark what answered it. */
  mayReply: boolean;
  mayMark: boolean;
  /** Read again, once something was said or marked. */
  onChanged: () => void;
  /** The reply the page was opened at (an inbox line of somebody named in it): marked, its chain unfolded, brought into view once. */
  asked?: number | null;
}

/** One reply, what it answers being drawn around it, and what answers it under it. */
function One(props: Props & { node: Node; folded: Set<number>; turn: (seq: number) => void; open: number | null; setOpen: (seq: number | null) => void }) {
  const { node } = props;
  const reply = node.reply;
  const isAnswer = props.answer === reply.seq;
  const folded = props.folded.has(reply.seq);
  const asked = props.asked === reply.seq;
  const drawn = useRef<HTMLLIElement>(null);
  // Brought into view once, as it is first drawn: a tab shown again is scrolled where it was left.
  const broughtTo = useKeptRef<number | null>('replies:brought', null);
  useEffect(() => {
    if (!asked || broughtTo.current === reply.seq) return;
    broughtTo.current = reply.seq;
    scrollsItself();
    const there = () => drawn.current?.scrollIntoView({ block: 'center' });
    there();
    // Kept in view as what is above it comes to its height (a post's or a summary's editor is loaded after the
    // replies are drawn, and pushed it out of view on a thread), until whoever reads does something on the page.
    const page = drawn.current?.closest('.topic-page');
    if (!page) return;
    const growing = new ResizeObserver(there);
    growing.observe(page);
    const acts = ['wheel', 'touchstart', 'pointerdown', 'keydown'];
    const stop = () => {
      growing.disconnect();
      for (const act of acts) window.removeEventListener(act, stop, true);
    };
    for (const act of acts) window.addEventListener(act, stop, true);
    return stop;
  }, [asked]);
  const mark = async () => {
    await attempt(() => topicsApi.answer(props.topic, isAnswer ? null : reply.seq));
    props.onChanged();
  };
  return (
    <li
      ref={drawn}
      class={classes('topic-reply', isAnswer && 'answer', asked && 'asked')}
      data-seq={reply.seq}
      data-depth={node.depth}
      aria-current={asked ? 'true' : undefined}
    >
      <article class="topic-reply-body">
        <header class="topic-reply-head">
          <strong class="topic-reply-who">{reply.who || reply.by || 'somebody'}</strong>
          <time class="faint" dateTime={new Date(reply.at).toISOString()} title={when(reply.at)}>
            {when(reply.at)}
          </time>
          {isAnswer && <span class="topic-answered-mark">answered it</span>}
        </header>
        <div class="topic-reply-words">
          <Prose text={reply.body} attached={props.topic} />
        </div>
        <footer class="topic-reply-acts">
          {props.mayReply && (
            <button type="button" class="ghost" aria-expanded={props.open === reply.seq} onClick={() => props.setOpen(props.open === reply.seq ? null : reply.seq)}>
              Reply
            </button>
          )}
          {props.mayMark && (
            <button type="button" class="ghost" onClick={() => void mark()}>
              {isAnswer ? 'Not the answer' : 'Mark as the answer'}
            </button>
          )}
        </footer>
        {props.open === reply.seq && (
          <ReplyBox
            topic={props.topic}
            parent={reply.seq}
            label={`Reply to ${reply.who || 'this'}`}
            onCancel={() => props.setOpen(null)}
            onDone={() => {
              props.setOpen(null);
              props.onChanged();
            }}
          />
        )}
      </article>
      {node.under.length > 0 &&
        (folded ? (
          <button type="button" class="ghost topic-reply-more" onClick={() => props.turn(reply.seq)}>
            {counted(node) === 1 ? '1 more reply' : `${counted(node)} more replies`}
          </button>
        ) : (
          <ul class="topic-replies-under">
            {node.under.map((under) => (
              <One key={under.reply.seq} {...props} node={under} />
            ))}
          </ul>
        ))}
    </li>
  );
}

/** The replies of a topic, as a tree. */
export function Replies(props: Props) {
  const top = tree(props.replies);
  // Which chains were turned from how they start, kept by the tab: a long one starts folded past its third reply.
  const [turned, setTurned] = useKept<number[]>('replies:turned', []);
  const [open, setOpen] = useKept<number | null>('replies:open', null);
  // The chain that holds the answer, and the one the page was opened at, never start folded.
  const first = startsFolded(
    top,
    [props.answer, props.asked ?? null].filter((seq): seq is number => seq !== null),
  );
  const folded = new Set([...first].filter((seq) => !turned.includes(seq)));
  const turn = (seq: number) => setTurned((was) => (was.includes(seq) ? was.filter((one) => one !== seq) : [...was, seq]));
  if (top.length === 0) return <p class="faint topic-replies-none">Nobody has replied yet.</p>;
  return (
    <ul class="topic-replies">
      {top.map((node) => (
        <One key={node.reply.seq} {...props} node={node} folded={folded} turn={turn} open={open} setOpen={setOpen} />
      ))}
    </ul>
  );
}
