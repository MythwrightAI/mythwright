// A topic's article, the first of its page's tabs where it is one
// (`partsOf` in `topicpage.ts`): its summary where it has one, its post
// (the article itself, read, and written as blocks once Edit is pressed),
// its headings read each with what the page puts beside it (`headed`), its
// References and the pages that link to it (`articleend.tsx`), and the
// pages directly under it at its end. The article's column has room at its
// left for a column of its contents (`contents`) and at its right for boxes
// beside it (`boxes`):
// each drawn only where it is handed something, and the column keeps a
// reader's width whatever is beside it. A note its post has for its top is
// drawn over all of them, and a series its post places at its end
// after its text (`boxes.tsx`); the post then leaves those out.
//
// The pages under a page are listed here and, on a thread, under its
// summary (`PagesIn`). A tracked question is a thread whatever its post
// says: its post is drawn there over the pages and replies (`ThreadPost`).

import type { ComponentChildren } from 'preact';
import { hrefOf } from '../address.ts';
import { boxedOf } from '../boxes.ts';
import { read } from '../prose.ts';
import { statusSaid } from '../topicpage.ts';
import type { Status, TopicWhole } from '../topicpage.ts';
import type { Named } from '../wikipage.ts';
import { ArticleEnd } from './articleend.tsx';
import { BoxesApart, NoteDrawn, SeriesDrawn } from './boxes.tsx';
import { openOf } from '../topicediting.ts';
import type { Editing } from '../topicediting.ts';
import { DocField } from './topicdoc.tsx';
import { TOPIC_GLYPH } from './topics.tsx';
import type { Heading } from '../contents.ts';
import { Headed } from '../ui.tsx';

/** A page directly under the topic, as its reading hands it. */
export interface Under extends Named {
  status: Status | null;
}

/** A page's address, as a link on the page: opened as a tab beside what is open when it is pressed. */
export const pageHref = (page: Named): string => hrefOf({ kind: 'topic', id: page.id, title: page.title });

/** The pages directly under a page, in the order they stand, each opening its page; and, for whoever may write, a way to add one. */
export function PagesIn({ pages, mayAdd, onAdd }: { pages: Under[]; mayAdd: boolean; onAdd: () => void }) {
  if (pages.length === 0) return null;
  return (
    <section class="topic-pages" aria-label="Pages in it">
      <h2 class="topic-part-label">Pages in it</h2>
      <ul class="topic-pages-list">
        {pages.map((page) => {
          const how = statusSaid(page.status);
          return (
            <li key={page.id}>
              <a class="topic-pages-line" href={pageHref(page)} data-topic={page.id}>
                <span class="topic-pages-glyph" aria-hidden="true">
                  {TOPIC_GLYPH}
                </span>
                <span class="topic-pages-title">{page.title}</span>
                {how && <span class="faint topic-pages-status">{how}</span>}
              </a>
            </li>
          );
        })}
      </ul>
      {mayAdd && (
        <button type="button" class="ghost topic-new-page" onClick={onAdd}>
          + New page
        </button>
      )}
    </section>
  );
}

interface Props {
  topic: TopicWhole;
  pages: Under[];
  /** Whether whoever reads it may change it, and add a page under it. */
  mayChange: boolean;
  /** Its summary's band, where the page draws one: at the top of the article. */
  summary?: ComponentChildren;
  /** What of it is open to be written, and its post opened or left. */
  editing: Editing;
  onEditing: (open: boolean) => void;
  /** Saves its post. */
  onSave: (body: string) => Promise<boolean>;
  onAddPage: () => void;
  /** Saved empty, the post is no article: a topic that is no page of a wiki is a thread again. */
  onEmptied: () => void;
  /** Its post left before anything is saved in it, where Write an article began it: it is a thread again. */
  onLeave?: () => void;
  /** A column of what is in it, at its left; and boxes beside it, at its right. */
  contents?: ComponentChildren;
  boxes?: ComponentChildren;
  /** What stands beside each heading of its post, where it is read (`topiccontents.tsx`). */
  headed?: (heading: Heading) => ComponentChildren;
  /** Its post saved and closed, or left: the heading of the section the cursor was in, to be read at. */
  onBack?: (section: string | null) => void;
}

/** What says nothing is kept as nothing, and not as the empty line an emptied editor gives; saved so, the post is taken away. */
const postSaver = (onSave: (body: string) => Promise<boolean>, onEmptied: () => void) => async (body: string) => {
  const said = body.trim() ? body : '';
  const kept = await onSave(said);
  if (kept && !said) onEmptied();
  return kept;
};

/** A thread's post, where the thread is a question that says something or is being written: over the pages in it and its replies. Read until Edit, as an article's is. */
export function ThreadPost(props: {
  topic: TopicWhole;
  mayChange: boolean;
  editing: Editing;
  onEditing: (open: boolean) => void;
  onSave: (body: string) => Promise<boolean>;
  onEmptied: () => void;
  /** Left before anything is saved in it, where Add post began it: the question has no post again. */
  onLeave?: () => void;
}) {
  const { topic, mayChange } = props;
  return (
    <section class="topic-post" aria-label="Post">
      <h2 class="topic-part-label">Post</h2>
      <DocField
        name="body"
        record={topic.id}
        saved={topic.body}
        editable={mayChange}
        {...openOf(props.editing, 'body')}
        onOpen={props.onEditing}
        empty=""
        onSave={postSaver(props.onSave, props.onEmptied)}
        onLeave={props.onLeave}
      />
    </section>
  );
}

/** A topic's article: as a tab's panel, its note over all, its contents at its left, its boxes at its right, the article between. */
export function Article(props: Props) {
  const { topic, pages, mayChange } = props;
  const save = postSaver(props.onSave, props.onEmptied);
  // Whoever may not change it is shown no empty post on a page that holds pages: those are what it says.
  const post = mayChange || topic.body.trim() || pages.length === 0;
  // What its post draws apart from its text, as last saved: its note at the top, and a series at its end.
  const boxed = boxedOf(read(topic.body));
  return (
    <div class="topic-article" role="tabpanel" id={`topic-article-${topic.id}`} aria-labelledby={`topic-tab-article-${topic.id}`}>
      {boxed.head && (
        <section class="topic-head-note" aria-label="Note">
          <NoteDrawn box={boxed.head} record={topic.id} head />
        </section>
      )}
      {props.contents && <div class="topic-article-contents">{props.contents}</div>}
      <div class="topic-article-main topic-parts">
        {props.summary}
        {post && (
          <section class="topic-post" aria-label="Article">
            <Headed.Provider value={props.headed ?? null}>
              <BoxesApart.Provider value>
                <DocField
                  name="body"
                  record={topic.id}
                  saved={topic.body}
                  editable={mayChange}
                  {...openOf(props.editing, 'body')}
                  onOpen={props.onEditing}
                  empty={mayChange ? '' : 'Nothing is written here yet.'}
                  onSave={save}
                  onLeave={props.onLeave}
                  onBack={props.onBack}
                />
              </BoxesApart.Provider>
            </Headed.Provider>
          </section>
        )}
        {boxed.end.length > 0 && (
          <section class="topic-boxes-end" aria-label="Series">
            {boxed.end.map((box, at) => (
              <SeriesDrawn key={at} box={box} record={topic.id} />
            ))}
          </section>
        )}
        <ArticleEnd topic={topic.id} body={topic.body} />
        <PagesIn pages={pages} mayAdd={mayChange} onAdd={props.onAddPage} />
      </div>
      {props.boxes && <aside class="topic-article-boxes">{props.boxes}</aside>}
    </div>
  );
}
