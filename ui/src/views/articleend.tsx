// What a topic's article ends with: its References, the sources its post
// gives (`cited.ts`), numbered as its markers are and each with a way back
// to where it is used; and **Linked from**, the pages that link to it
// (`GET /api/topics/<topic>/linked`), each with the wiki it is in and
// opening it. Each says nothing where it has nothing. Every page of a
// series carries the same series box, so the pages whose only link is
// such a box are gathered at the end of Linked from into one folded line
// for each series, saying how many and which (`gatheredLinks`), unfolding
// to them.
//
// The topic's page mounts it once, at the end of its article:
//
//   <ArticleEnd topic={topic.id} body={topic.body} />
//
// and draws its post with its sources apart (`<Prose notes="apart">`), so
// they are listed here and not twice. The sources listed are those of the
// post as it was saved. Which pages link to it is read as it is shown and
// again whenever the daemon says something of a topic (a page's text
// changed, a page renamed or taken away), with nothing polled.
//
// Optional sections of a page (Notes, See also, External links) are
// headings the writer adds to the post, as an encyclopedia's are: nothing
// here draws them.

import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { hrefOf } from '../address.ts';
import { call } from '../api.ts';
import { notesOf } from '../cited.ts';
import { gatheredLinks } from '../linkedfrom.ts';
import type { Linking } from '../linkedfrom.ts';
import { coalesce } from '../events.ts';
import { read } from '../prose.ts';
import { topicOfEvent } from '../topicpage.ts';
import { Prose, Twist, useEvents } from '../ui.tsx';
import { NotesList } from './cited.tsx';
import { TOPIC_GLYPH } from './topics.tsx';

/** The markers of a label in the post of the page `from` is on. */
const usesInPost = (from: Element, label: string): Element[] => [...(from.closest('.topic-page')?.querySelectorAll(`[data-field="body"] [data-cite=${JSON.stringify(label)}]`) ?? [])];

/** The sources the post gives, as its article's References. */
function References({ topic, body }: { topic: string; body: string }) {
  const notes = notesOf(read(body));
  if (notes.list.length === 0) return null;
  return (
    <section class="topic-references" aria-label="References">
      <h2 class="topic-part-label">References</h2>
      <NotesList notes={notes} uses={usesInPost} label="References" drawn={(note) => <Prose text={note.text ?? ''} attached={topic} notes="apart" />} />
    </section>
  );
}

/** The pages that link to the topic, read as it is shown and again as the daemon says something of a topic. */
function LinkedFrom({ topic }: { topic: string }) {
  const [linked, setLinked] = useState<Linking[]>([]);
  const asked = useRef(0);
  const readIt = async () => {
    const mine = ++asked.current;
    try {
      const said = await call<{ linked: Linking[] }>('GET', `/topics/${encodeURIComponent(topic)}/linked`);
      if (mine === asked.current) setLinked(said.linked);
    } catch {
      // Whoever may not read it is shown nothing of what links to it.
      if (mine === asked.current) setLinked([]);
    }
  };
  const latest = useRef(readIt);
  latest.current = readIt;
  const again = useMemo(() => coalesce(() => void latest.current()), []);
  useEffect(() => void readIt(), [topic]);
  useEvents(['record.*', 'topic.*'], (event) => topicOfEvent(event) && again(), [], again);
  if (linked.length === 0) return null;
  const { listed, series } = gatheredLinks(linked);
  return (
    <section class="topic-pages topic-linked" aria-label="Linked from">
      <h2 class="topic-part-label">Linked from</h2>
      {listed.length > 0 && <LinkedList pages={listed} />}
      {series.map((one) => (
        <SeriesLinked key={one.title} title={one.title} pages={one.pages} />
      ))}
    </section>
  );
}

/** Pages that link, each opening its page. */
function LinkedList({ pages }: { pages: Linking[] }) {
  return (
    <ul class="topic-pages-list">
      {pages.map((page) => (
        <li key={page.id}>
          <a class="topic-pages-line" href={hrefOf({ kind: 'topic', id: page.id, title: page.title })} data-topic={page.id}>
            <span class="topic-pages-glyph" aria-hidden="true">
              {TOPIC_GLYPH}
            </span>
            <span class="topic-pages-title">{page.title}</span>
            {page.wiki && <span class="faint topic-pages-status">in {page.wiki.title}</span>}
          </a>
        </li>
      ))}
    </ul>
  );
}

/** The pages of a series that link only through its box: one folded line saying how many and which series, unfolding to them. */
function SeriesLinked({ title, pages }: { title: string; pages: Linking[] }) {
  const [open, setOpen] = useState(false);
  const said = `${pages.length} ${pages.length === 1 ? 'page' : 'pages'} of ${title ? `the series ${title}` : 'a series'}`;
  return (
    <div class="topic-linked-series">
      <button type="button" class="ghost topic-linked-fold" aria-expanded={open} onClick={() => setOpen(!open)}>
        <Twist open={open} />
        <span>{said}</span>
        <span class="faint">through its series box</span>
      </button>
      {open && <LinkedList pages={pages} />}
    </div>
  );
}

/** The end of a topic's article: its References and the pages linked from. */
export function ArticleEnd({ topic, body }: { topic: string; body: string }) {
  return (
    <>
      <References topic={topic} body={body} />
      <LinkedFrom topic={topic} />
    </>
  );
}
