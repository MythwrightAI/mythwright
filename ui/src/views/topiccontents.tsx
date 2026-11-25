// A topic's article as a long page is read: its Contents, drawn from its
// headings once it has at least `CONTENTS_FROM` of them (`contents.ts`),
// each entry taking the reader to its heading; and beside each heading, a
// link mark that copies the address of the article there (`#s/<slug>`)
// and, to whoever may change the topic, Edit, which opens the article at
// that section (`TopicPage`'s `edit`).
//
// On a wide page the Contents are a column at the article's left, staying
// in view as the article scrolls, the section being read marked; on a
// narrow one, a box at its top, folded until it is pressed. Hide puts them
// away for the reader in every article (`contents` of their appearance,
// kept with their account: `appearance.ts`), leaving a small Contents that
// shows them again, as the Appearance menu does.
//
// Where the article is taken (an entry pressed, an address that names a
// heading, the editor closed with the cursor in a section) is the page's,
// kept by its tab (`useSections`), and said in the address bar while the
// article shows; it is taken there once the heading is drawn, in the
// article read or in the editor, whatever is read or drawn meanwhile.

import type { ComponentChildren } from 'preact';
import { useEffect, useRef, useState } from 'preact/hooks';
import { sectionLink } from '../address.ts';
import { changed } from '../appearance.ts';
import { appearance, keepAppearance } from '../appearancekept.ts';
import { contentsOf, headingsOf, sectionAt, slugsOf } from '../contents.ts';
import type { Entry, Heading } from '../contents.ts';
import { read } from '../prose.ts';
import { Twist } from '../ui.tsx';
import { classes } from '../util.ts';
import { copy } from './pagehead.tsx';
import { useKept, useKeptRef } from './returned.tsx';
import { signs } from './signs.tsx';

/** Where the article is asked to be taken: a heading, by its slug, and how many times it was asked, so that the same one asked again is taken again. */
export interface Jump {
  section: string;
  nth: number;
}

/** The article's headings as they are drawn, read or in the editor, each with its slug. */
export function sectionsIn(page: HTMLElement): { slug: string; at: HTMLElement }[] {
  const read = [...page.querySelectorAll<HTMLElement>('.topic-post [data-section]')];
  if (read.length > 0) return read.map((at) => ({ slug: at.dataset.section ?? '', at }));
  const written = [...page.querySelectorAll<HTMLElement>('.topic-post .ProseMirror :is(h1, h2, h3)')];
  const slugs = slugsOf(written.map((at) => at.textContent?.trim() ?? ''));
  return written.map((at, nth) => ({ slug: slugs[nth] ?? '', at }));
}

/** What scrolls a page: the nearest element round it that scrolls, and the document where none does. */
function scrollerOf(page: HTMLElement): HTMLElement {
  for (let up: HTMLElement | null = page; up; up = up.parentElement) {
    const { overflowY } = getComputedStyle(up);
    if ((overflowY === 'auto' || overflowY === 'scroll') && up.scrollHeight > up.clientHeight) return up;
  }
  return (document.scrollingElement as HTMLElement | null) ?? document.documentElement;
}

/**
 * What a topic's page keeps of where its article is taken, and the
 * article's headings: the page's element is `page`; `drawn` is whatever,
 * changed, may have drawn the heading asked for (what was read, which tab
 * shows, whether the editor is open). Leaving the article forgets it, so
 * that the address bar says no heading while another tab shows.
 */
export function useSections(body: string, onArticle: boolean, drawn: unknown[]) {
  const page = useRef<HTMLDivElement>(null);
  const [jump, setJump] = useKept<Jump | null>('jump', null);
  const done = useKeptRef('jumped', 0);
  const jumpTo = (section: string) => setJump((was) => ({ section, nth: (was?.nth ?? done.current) + 1 }));
  useEffect(() => {
    if (!jump || jump.nth === done.current || !page.current) return;
    const found = sectionsIn(page.current).find((one) => one.slug === jump.section);
    if (!found) return;
    done.current = jump.nth;
    found.at.scrollIntoView({ block: 'start' });
  }, [jump?.nth, ...drawn]);
  useEffect(() => {
    if (!onArticle && jump) setJump(null);
  }, [onArticle]);
  const headings = headingsOf(read(body));
  return { page, jump, jumpTo, headings };
}

/** What stands beside each heading of the article read: a link mark that copies its address, and to whoever may change it, Edit. */
export function headingMarks(topic: string, mayChange: boolean, onJump: (section: string) => void, onEdit: (section: string) => void) {
  return (heading: Heading): ComponentChildren => {
    const link = sectionLink(topic, heading.slug);
    return (
      <span class="prose-heading-acts">
        <a
          class="prose-heading-mark"
          href={link}
          aria-label={`Copy link to ${heading.title}`}
          title="Copy link"
          onClick={(event) => {
            if (event.button !== 0 || event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) return;
            event.preventDefault();
            onJump(heading.slug);
            void copy(link, 'Copied. It opens this section for whoever is signed in to this factory here.');
          }}
        >
          {signs.link()}
        </a>
        {mayChange && (
          <button type="button" class="ghost prose-heading-edit" aria-label={`Edit section ${heading.title}`} onClick={() => onEdit(heading.slug)}>
            Edit
          </button>
        )}
      </span>
    );
  };
}

/** The entries of the contents, nested as the headings are, each a link to its heading. */
function Entries({ entries, topic, current, onJump }: { entries: Entry[]; topic: string; current: string | null; onJump: (section: string) => void }) {
  return (
    <ol class="contents-list">
      {entries.map((entry) => (
        <li key={entry.heading.slug}>
          <a
            class="contents-entry"
            href={sectionLink(topic, entry.heading.slug)}
            aria-current={current === entry.heading.slug ? 'location' : undefined}
            onClick={(event) => {
              if (event.button !== 0 || event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) return;
              event.preventDefault();
              onJump(entry.heading.slug);
            }}
          >
            <span class="contents-number">{entry.number}</span>
            <span class="contents-words">{entry.heading.title}</span>
          </a>
          {entry.under.length > 0 && <Entries entries={entry.under} topic={topic} current={current} onJump={onJump} />}
        </li>
      ))}
    </ol>
  );
}

/**
 * The Contents of an article: beside it on a wide page, a folded box at its top on a narrow one, or hidden. Which of
 * the first two is the stylesheet's, by how wide the article is (`contents.css`), so that the article is laid out
 * once, where it is to be read, whatever is measured after.
 */
export function Contents({ topic, headings, onJump }: { topic: string; headings: Heading[]; onJump: (section: string) => void }) {
  const own = useRef<HTMLElement & HTMLDivElement>(null);
  const hidden = appearance.value.contents === 'hidden';
  const [open, setOpen] = useKept('contents:open', false);
  const [current, setCurrent] = useState<string | null>(null);
  const said = headings.map((heading) => heading.slug).join(' ');
  // The section being read, as the page scrolls: whatever scrolls it, which is the frame the page is drawn in.
  useEffect(() => {
    const mark = () => {
      const page = own.current?.closest<HTMLElement>('.topic-page');
      if (!page) return;
      const scroller = scrollerOf(page);
      const top = Math.max(scroller.getBoundingClientRect().top, 0);
      const sections = sectionsIn(page);
      const tops = sections.map((one) => one.at.getBoundingClientRect().top - top);
      const end = scroller.scrollTop + scroller.clientHeight >= scroller.scrollHeight - 2 ? scroller.clientHeight : undefined;
      setCurrent(sections[sectionAt(tops, Math.min(scroller.clientHeight / 3, 120), end)]?.slug ?? null);
    };
    document.addEventListener('scroll', mark, { capture: true, passive: true });
    window.addEventListener('resize', mark);
    mark();
    return () => {
      document.removeEventListener('scroll', mark, { capture: true });
      window.removeEventListener('resize', mark);
    };
  }, [said, hidden]);
  const setting = (word: string) => void keepAppearance(changed(appearance.peek(), 'contents', word));
  const jump = (section: string) => {
    // As a box, it folds again, so that the section taken to shows.
    setOpen(false);
    onJump(section);
  };
  if (hidden) {
    return (
      <div ref={own} class="contents is-hidden">
        <button
          type="button"
          class="ghost contents-show"
          aria-expanded="false"
          title="Show contents"
          onClick={() => {
            setOpen(true);
            setting('shown');
          }}
        >
          {signs.sheets()}
          Contents
        </button>
      </div>
    );
  }
  return (
    <nav ref={own} class={classes('contents', open && 'is-open')} aria-label="Contents">
      <div class="contents-head">
        <h2 class="contents-title">Contents</h2>
        <button type="button" class="ghost contents-fold" aria-expanded={open} onClick={() => setOpen(!open)}>
          <Twist open={open} />
          Contents
        </button>
        <button type="button" class="ghost contents-hide" aria-label="Hide contents" onClick={() => setting('hidden')}>
          Hide
        </button>
      </div>
      <Entries entries={contentsOf(headings)} topic={topic} current={current} onJump={jump} />
    </nav>
  );
}
