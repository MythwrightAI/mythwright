// A link to a page of the topics, drawn (`pagelinks.ts` reads it): the
// words written, as a link that opens the page, or the page at the reply
// it names, and that says the page's title as it is now where it is
// pointed at. The page is found by its id among the topics the client
// has read (`topicsread.ts`), so a new title or a move changes nothing of
// where it leads.
//
// A page the reader cannot open (it was removed, or they may not read
// topics) is never drawn with the words written, which are most often its
// title: it is said to be a page they cannot open, and nothing more.
// Until the topics are read, nothing of the words is shown either.
//
// Pointed at a moment, or reached by the keyboard, a link shows what the
// page is in a card over it (`pagepreview.tsx`); Escape puts it away. A
// finger does not point: a tap opens the page, and shows no card.

import type { ComponentChildren } from 'preact';
import { useEffect, useId, useLayoutEffect, useRef } from 'preact/hooks';
import { hrefOf } from '../address.ts';
import { covers } from '../holds.ts';
import { CANNOT_OPEN, CANNOT_OPEN_WHY } from '../pagelinks.ts';
import { me } from '../store.ts';
import { topics, topicsRead, wantTopics } from '../topicsread.ts';
import { showReply } from '../topicshown.ts';
import { useShownWhilePointed } from './cited.tsx';
import { PagePreview } from './pagepreview.tsx';

/** How long a link is pointed at before its card shows: long enough that a pointer passing over it shows none. */
const POINTED_MS = 350;

/** Whether whoever is signed in may read the topics at all. */
const mayReadTopics = () => covers(me.value?.principal.scopes ?? [], 'records:read');

export function PageLink({ topic, reply, children }: { topic: string; reply: number | null; children: ComponentChildren }) {
  const may = mayReadTopics();
  useEffect(() => {
    if (may) wantTopics();
  }, [may]);
  const link = useRef<HTMLAnchorElement>(null);
  const card = useShownWhilePointed();
  const waiting = useRef<number | null>(null);
  const described = useId();
  const wait = () => {
    if (waiting.current !== null) window.clearTimeout(waiting.current);
    waiting.current = null;
  };
  useEffect(() => wait, []);
  // Escape puts the card away however it was shown, and does nothing else while it is: heard from the moment the
  // card is drawn, before it is seen.
  useLayoutEffect(() => {
    if (!card.open) return;
    const escape = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.stopPropagation();
      card.hide();
    };
    window.addEventListener('keydown', escape, true);
    return () => window.removeEventListener('keydown', escape, true);
  }, [card.open]);
  const page = may && topicsRead.value ? topics.value.find((line) => line.id === topic) : undefined;
  if (!page) {
    const known = !may || topicsRead.value;
    return (
      <span class="page-link-none" title={known ? CANNOT_OPEN_WHY : undefined} data-topic-none="">
        {known ? CANNOT_OPEN : 'a page'}
      </span>
    );
  }
  const href = hrefOf({ kind: 'topic', id: page.id, title: page.title });
  const open = (event: MouseEvent) => {
    wait();
    card.hide();
    // A reply is shown where it is on its page; the page itself opens as any address of this factory does.
    if (reply === null || event.ctrlKey || event.metaKey || event.shiftKey) return;
    event.preventDefault();
    showReply(page.id, reply);
  };
  return (
    <>
      <a
        ref={link}
        class="page-link"
        href={href}
        data-topic={page.id}
        // Its card says the title and more; without one, the title is said where the link is pointed at.
        title={card.open ? undefined : reply === null ? page.title : `${page.title}, at a reply`}
        aria-describedby={card.open ? described : undefined}
        onPointerEnter={(event) => {
          if (event.pointerType === 'touch') return;
          wait();
          waiting.current = window.setTimeout(card.show, POINTED_MS);
        }}
        onPointerLeave={(event) => {
          if (event.pointerType === 'touch') return;
          wait();
          if (card.open) card.hideSoon();
        }}
        // Reached by the keyboard; a press of the pointer or a finger that gives it the keyboard is not.
        onFocus={(event) => {
          if (event.currentTarget.matches(':focus-visible')) card.show();
        }}
        onBlur={() => {
          wait();
          card.hide();
        }}
        onClick={open}
      >
        {children}
      </a>
      {card.open && <PagePreview anchor={link} line={page} reply={reply} id={described} onEnter={card.show} onLeave={card.hideSoon} onOpen={open} />}
    </>
  );
}
