// A forum's topic as a page of its own, and a page of a
// wiki: where it is in its wiki over its title, each page above it pressed
// to open it, the last being the way up; what it is called and how it
// stands at its head, its title written in place by whoever may change it.
// Under that it is one of two things (`partsOf` in `topicpage.ts`), with
// nothing stored to say which:
//
// - An **article**, where it is no tracked question and its post says
//   something or it is a page under another: three tabs under its head,
//   Article (`topicarticle.tsx`: its summary, its post, its References
//   and the pages that link to it (`articleend.tsx`), the pages under it),
//   Talk with how many replies it has (`topictalk.tsx`: what answered it,
//   its replies as a tree) and History (`topichistory.tsx`: each revision,
//   two compared, one restored).
// - A **thread**, where it is neither, and a tracked question (open,
//   answered or closed) whatever its post says: its summary, what answered
//   it, a question's post and the end of its article (its References and
//   the pages that link to it), the pages under it and its replies, with
//   no tabs; History at its head shows its history in their place.
//
// Each part is drawn only where it says something, and offered at the head
// to whoever may change it where it does not: Add summary, Add post on a
// question, and on any other thread Write an article, which brings the
// tabs. Which tab shows is kept by the page's tab, said in the address bar
// (`#talk`, `#history/3`), and asked for by an address and by an inbox
// line of a reply, which opens Talk at that reply (`topicshown.ts`); an
// address of the topic alone opens its article.
//
// What it says is read, by whoever may change it as by anybody else, until
// Edit at its head opens its post (the article) and summary to be written
// in place (`Editing`, kept by the page's tab; `edit`, which also opens the
// article at one of its headings, as Edit beside each heading does); one
// that says nothing is open to be written. A long article has Contents at
// its left, and each heading a link to it (`#s/<slug>`:
// `topiccontents.tsx`); a question's post has neither.
//
// It reads the topic whole by its id or its title (`GET /api/topics/<it>`),
// which hands where it is (`path`) and the pages under it (`pages`) beside
// it, and reads it again whenever the daemon says something of it, of a
// page above it or under it, or of a page moved to it or away (`record.*`,
// `topic.*`), and whenever the topics as the page reads them say another
// page is under it than it has, so a page left open follows what is said
// there and what it is changed to, with nothing polled. An address that
// names it by its title, and its wiki's, is found among the topics read
// (`wikipage.ts`); where several pages are called so, the page says which
// they are. The body and the summary are documents kept in its row
// (`topicdoc.tsx`), saved through the door any record is changed by (`PATCH
// /api/records/<id>`), which goes with its revision whatever door changed
// it; its title the same, said on Enter and as its field is left, once for
// both. What is being written, there, in its title and in a reply's box,
// is kept by the tab while another is shown.
//
// A topic that is a question has a status, shown at its head and chosen
// there by whoever may change it, among the four or none (Not tracked); one
// with none (a wiki's page, a thread that gathers what is known) shows none
// and offers no reply to be marked as what answered it.
//
// Whoever may change records may change it and mark what answered it, and
// whoever may write records may reply: who may view a topic, and at which
// level, is still to be built.

import { useEffect, useRef, useState } from 'preact/hooks';
import { hrefOf } from '../address.ts';
import { api, ApiError, call } from '../api.ts';
import { covers } from '../holds.ts';
import { replyOf } from '../replies.ts';
import type { Reply } from '../replies.ts';
import { attempt, me, openTab, projects } from '../store.ts';
import { isTopicId, partsOf, statusChoices, statusOf, titled, topicOfEvent, wholeOf } from '../topicpage.ts';
import type { Handed, Status, TopicLine, TopicPart, TopicWhole } from '../topicpage.ts';
import { readTopics, topics, topicsApi, topicsRead, wantTopics } from '../topicsread.ts';
import { freshPages, sayTopicShowing, topicAsked } from '../topicshown.ts';
import { useEvents } from '../ui.tsx';
import { classes } from '../util.ts';
import { workspaces } from '../spaces.ts';
import { crumbsOf, movedHereOrAway, namedIn, pathSaid, treeOfLines, underOf } from '../wikipage.ts';
import type { Named } from '../wikipage.ts';
import { pathOf } from '../wikis.ts';
import { useKept } from './returned.tsx';
import { offer } from './rowmenu.tsx';
import { ArticleEnd } from './articleend.tsx';
import { Article, PagesIn, ThreadPost, pageHref } from './topicarticle.tsx';
import { sideBoxes } from './boxes.tsx';
import type { Under } from './topicarticle.tsx';
import { editAll, opened } from '../topicediting.ts';
import type { DocName, Editing } from '../topicediting.ts';
import { Summary } from './topicdoc.tsx';
import { History } from './topichistory.tsx';
import { Talk } from './topictalk.tsx';
import { TOPIC_GLYPH } from './topics.tsx';
import { Appearance } from './appearance.tsx';
import { CONTENTS_FROM } from '../contents.ts';
import { Contents, headingMarks, useSections } from './topiccontents.tsx';
import { FollowButton } from './followbutton.tsx';

/** What the page has of its topic: as last read, or that it is not there, or why it could not be read. */
interface Read {
  topic: TopicWhole | null;
  replies: Reply[];
  /** The pages above it, its wiki first; and those directly under it, in the order they stand. */
  path: Named[];
  pages: Under[];
  gone: string | null;
  failed: string | null;
}

const NOTHING: Read = { topic: null, replies: [], path: [], pages: [], gone: null, failed: null };

const namedOf = (value: unknown): Named => {
  const one = (value ?? {}) as Record<string, unknown>;
  const id = typeof one.id === 'string' ? one.id : '';
  return { id, title: typeof one.title === 'string' && one.title ? one.title : id };
};

/**
 * How a topic stands, at its head: to whoever may change it, pressed to
 * choose another among the four or none (Not tracked), which a topic with
 * none is offered as Add status; to anybody else, said where it has one.
 */
function StatusPick({ status, mayChange, onChoose }: { status: Status | null; mayChange: boolean; onChoose: (status: Status | null) => void }) {
  if (!mayChange) return status ? <span class={classes('topic-status', `is-${status}`)}>{status}</span> : null;
  const choose = (event: MouseEvent) => {
    const from = event.currentTarget as HTMLElement;
    const box = from.getBoundingClientRect();
    const items = statusChoices(status).map((one) => ({
      label: one.label,
      chosen: one.chosen,
      apart: one.value === null,
      ...(one.closed ? { closed: one.closed } : {}),
      act: () => void (one.chosen || onChoose(one.value)),
    }));
    offer({ x: box.left, y: box.bottom + 2 }, items, { from, label: 'Status' });
  };
  return status ? (
    <button type="button" class={classes('topic-status', 'topic-status-pick', `is-${status}`)} aria-haspopup="menu" title="Change its status" onClick={choose}>
      {status}
    </button>
  ) : (
    <button type="button" class="ghost topic-status-add" aria-haspopup="menu" title="Track it as a question" onClick={choose}>
      Add status
    </button>
  );
}

/** Where a topic is, said under its title: the project it is of, or the workspace it is the own of. */
function placeSaid(topic: TopicWhole): string {
  if (topic.project) return `in ${projects.value.find((one) => one.id === topic.project)?.name ?? topic.project}`;
  const space = workspaces.value.find((one) => one.id === topic.workspace);
  return space ? `in ${space.name}` : '';
}

/**
 * Where a page is, over its title: its wiki and each page above it, each
 * pressed to open it, the last being the way up. A long path keeps the
 * wiki and the last two, and the rest are a menu under `…`.
 */
function Path({ path }: { path: Named[] }) {
  if (path.length === 0) return null;
  const { shown, folded } = crumbsOf(path);
  const fold = (event: MouseEvent) => {
    const from = event.currentTarget as HTMLElement;
    const box = from.getBoundingClientRect();
    const items = folded.map((page) => ({ label: page.title, act: () => openTab({ kind: 'topic', id: page.id, title: page.title }) }));
    offer({ x: box.left, y: box.bottom + 2 }, items, { from, label: 'The pages between' });
  };
  return (
    <nav class="topic-path" aria-label="Where it is">
      {shown.map((page, at) => (
        <span class="topic-path-part" key={page === 'fold' ? 'fold' : page.id}>
          {at > 0 && (
            <span class="topic-path-sep" aria-hidden="true">
              ›
            </span>
          )}
          {page === 'fold' ? (
            <button type="button" class="ghost topic-path-fold" aria-haspopup="menu" aria-label={`${folded.length} pages between`} onClick={fold}>
              …
            </button>
          ) : (
            <a class="topic-path-page" href={pageHref(page)} title={at === shown.length - 1 ? `Up to ${page.title}` : page.title}>
              {page.title}
            </a>
          )}
        </span>
      ))}
    </nav>
  );
}

/**
 * What a page is called, written in place by whoever may change it: said on
 * Enter and as the field is left, once for both, through the door any
 * record is changed by. A page just made from a page's foot opens with it
 * chosen, to be written.
 */
function TitleField({ topic, onSaved }: { topic: TopicWhole; onSaved: () => void }) {
  const [typed, setTyped] = useKept<string | null>('title:typed', null);
  const [wrong, setWrong] = useState<string | null>(null);
  const asked = useRef<string | null>(null);
  const field = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    if (!freshPages.delete(topic.id)) return;
    field.current?.focus();
    field.current?.select();
  }, [topic.id]);
  const save = async () => {
    if (typed === null) return;
    const { title, refused } = titled(typed);
    if (refused) return setWrong(refused);
    if (title === topic.title) return setTyped(null);
    if (title === asked.current) return;
    asked.current = title;
    setWrong(null);
    try {
      await api.setRecord(topic.id, { title });
      setTyped(null);
      onSaved();
    } catch (error) {
      setWrong(error instanceof Error ? error.message : String(error));
    } finally {
      asked.current = null;
    }
  };
  return (
    <form
      class="topic-title-form"
      onSubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      {/* A box that wraps, as a long title does: as wide as what is written in it, and as tall, which what stands
          behind it with the same words says (`.topic-title-sized`). */}
      <span class="topic-title-sized" data-said={`${typed ?? topic.title} `}>
        <textarea
          ref={field}
          class={classes('topic-page-title', 'topic-title-field', typed !== null && 'unsaved')}
          aria-label="Title"
          rows={1}
          cols={1}
          value={typed ?? topic.title}
          maxLength={200}
          aria-invalid={wrong ? 'true' : undefined}
          aria-describedby={wrong ? 'topic-title-wrong' : undefined}
          onInput={(event) => {
            setTyped(event.currentTarget.value);
            setWrong(null);
          }}
          onKeyDown={(event) => {
            // Enter says it: a title is one line.
            if (event.key === 'Enter') {
              event.preventDefault();
              return void save();
            }
            if (event.key !== 'Escape' || typed === null) return;
            event.stopPropagation();
            setTyped(null);
            setWrong(null);
          }}
          onBlur={() => void save()}
        />
      </span>
      {wrong && (
        <p id="topic-title-wrong" role="alert" class="error-text topic-title-wrong">
          {wrong}
        </p>
      )}
    </form>
  );
}

/** Where several pages are called what an address said: which they are, each by where it is, each a link to its page. */
function Several({ name, lines }: { name: string; lines: TopicLine[] }) {
  const tree = treeOfLines(topics.value);
  const titleOf = (id: string) => topics.value.find((one) => one.id === id)?.title ?? id;
  return (
    <div class="topic-page">
      <div class="view-head">
        <h1>{name}</h1>
      </div>
      <p class="topic-page-none" role="status">
        {lines.length} pages are called “{name}”. Which one?
      </p>
      <section class="topic-pages" aria-label="Pages called so">
        <ul class="topic-pages-list">
          {lines.map((line) => (
            <li key={line.id}>
              <a class="topic-pages-line" href={hrefOf({ kind: 'topic', id: line.id })} data-topic={line.id}>
                <span class="topic-pages-glyph" aria-hidden="true">
                  {TOPIC_GLYPH}
                </span>
                <span class="topic-pages-title">{pathSaid(pathOf(tree, line.id).slice(0, -1).map(titleOf), line.title)}</span>
              </a>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}

export function TopicPage({ id, wiki }: { id: string; wiki?: string }) {
  const [read, setRead] = useKept<Read>('topic', NOTHING);
  // The topics as the page reads them: what finds a page by its title, and says which pages are under one.
  useEffect(() => wantTopics(), []);
  const byName = !isTopicId(id);
  const found = byName && topicsRead.value ? namedIn(topics.value, wiki ?? null, id) : null;
  // By its id; by the one page its title, and its wiki's, says; by its title where none is called so, which the daemon then says is not there.
  const reading = !byName ? id : found && 'found' in found ? found.found.id : topicsRead.value && !found ? id : null;
  const named = useRef(reading);
  named.current = reading;
  // Of the readings on their way, only the one asked for last is taken.
  const asked = useRef(0);
  const readIt = async () => {
    const it = named.current;
    if (it === null) return;
    const mine = ++asked.current;
    try {
      const whole = await topicsApi.read(it);
      if (mine !== asked.current) return;
      const pages = (whole.pages ?? []).map((page) => ({ ...namedOf(page), status: statusOf((page as Record<string, unknown>).status) }));
      setRead({ topic: wholeOf(whole.topic), replies: whole.replies.map(replyOf), path: (whole.path ?? []).map(namedOf), pages, gone: null, failed: null });
    } catch (error) {
      if (mine !== asked.current) return;
      if (error instanceof ApiError && error.status === 404) setRead({ ...NOTHING, gone: error.message });
      else setRead((was) => ({ ...was, failed: error instanceof Error ? error.message : String(error) }));
    }
  };
  // Read as the page is made, and so again behind what it last had as it comes back.
  useEffect(() => void readIt(), [reading]);
  const ours = read.topic?.id ?? reading ?? id;
  // What is said of it, of a page above it or under it, and of a page moved to it or away from it.
  const near = [ours, ...read.path.map((page) => page.id), ...read.pages.map((page) => page.id)];
  useEvents(
    ['record.*', 'topic.*'],
    (event) => {
      const of = topicOfEvent(event)?.topic;
      if ((of && near.includes(of)) || movedHereOrAway(event, ours)) void readIt();
    },
    [near.join(' ')],
  );
  // A page made under it, or taken away from under it, is said of that page: the topics as they are read say so.
  const under = topicsRead.value && read.topic ? underOf(topics.value, read.topic.id).join(' ') : null;
  useEffect(() => {
    if (under !== null && under !== read.pages.map((page) => page.id).join(' ')) void readIt();
  }, [under]);
  // The tab this reader chose, and the reply an address or an inbox line asked it to be opened at; kept by the tab.
  const [chosen, setChosen] = useKept<TopicPart | null>('part', null);
  const [askedAt, setAskedAt] = useKept<number | null>('asked', null);
  // Write an article and Add summary, pressed: what draws the part before anything is written in it.
  const [writing, setWriting] = useKept('writing', false);
  const [summing, setSumming] = useKept('summing', false);
  // What of it is open to be written; and the revision its history shows, where it shows one.
  const [editing, setEditing] = useKept<Editing>('editing', {});
  const [revision, setRevision] = useKept<number | null>('revision', null);
  // Where its article is taken, and its headings (`topiccontents.tsx`).
  const onArticle = chosen === null || chosen === 'article';
  const sections = useSections(read.topic?.body ?? '', onArticle, [read.topic?.body, chosen, editing.body]);
  const wanted = topicAsked.value;
  useEffect(() => {
    if (!wanted || wanted.topic !== ours) return;
    setChosen(wanted.part);
    if (wanted.part === 'history') setRevision(wanted.seq);
    else if (wanted.seq !== null) setAskedAt(wanted.seq);
    if (wanted.section) sections.jumpTo(wanted.section);
    topicAsked.value = null;
  }, [wanted, ours]);
  const topic = read.topic;
  const scopes = me.value?.principal.scopes ?? [];
  const mayChange = covers(scopes, 'records:write');
  const parts = topic ? partsOf(topic, { mayChange, chosen, writing, summing }) : null;
  // What the address bar says of it after the `#`: its talk or its history (at the revision shown) where that shows, its
  // article's heading it was taken to, and nothing for its article otherwise or a thread.
  const showingAt = parts?.showing === 'talk' || parts?.showing === 'history' ? parts.showing : '';
  const showingRevision = showingAt === 'history' && revision !== null ? revision : undefined;
  const showingSection = parts?.showing === 'article' ? sections.jump?.section : undefined;
  useEffect(() => {
    if (topic) sayTopicShowing(topic.id, showingAt, showingRevision, showingSection);
  }, [topic?.id, showingAt, showingRevision, showingSection]);

  if (found && 'several' in found) return <Several name={id} lines={found.several} />;
  if (read.gone) {
    return (
      <div class="topic-page">
        <div class="view-head">
          <h1>Topic</h1>
        </div>
        <p class="topic-page-none" role="status">
          {read.gone}
        </p>
      </div>
    );
  }
  if (!topic || !parts) return <div class="empty">{read.failed ? `It could not be read: ${read.failed}` : 'Reading the topic'}</div>;
  const changed = () => {
    void readIt();
    void readTopics();
  };
  // Read again before it is said to be kept, so that what was written is what is read as the editor closes.
  const set = async (fields: Record<string, unknown>) => {
    const done = await attempt(() => api.setRecord(topic.id, fields));
    if (done === undefined) return false;
    void readTopics();
    await readIt();
    return true;
  };
  /** Opens its article to be written, with its summary, or brought to one of its headings (`at`), the summary as it was. */
  const edit = (at?: string) => {
    // A thread's post is written where it is drawn; an article's on its Article tab.
    if (parts.article) setChosen('article');
    setEditing((was) => editAll(was, at ?? null, topic.summaryWhole.trim() !== ''));
  };
  const editingOf = (name: DocName) => (open: boolean) => setEditing((was) => opened(was, name, open));
  // A page made under it at once, called so until its title is written: opened with its title chosen in its field.
  const newPage = async () => {
    const made = await attempt(() => call<Handed>('POST', '/topics', { title: 'Untitled', under: topic.id }));
    if (made === undefined) return;
    freshPages.add(made.id);
    changed();
    openTab({ kind: 'topic', id: made.id, title: 'Untitled' });
  };
  const addSummary = () => {
    setSumming(true);
    setChosen('article');
  };
  const writeArticle = () => {
    setWriting(true);
    setChosen('article');
  };
  const saveBody = (body: string) => set({ body });
  const summary = parts.summary && (
    <Summary
      record={topic.id}
      saved={topic.summaryWhole}
      editable={mayChange}
      editing={editing}
      onEditing={editingOf('summary')}
      onSave={(summary) => set({ summary })}
      onGone={() => setSumming(false)}
    />
  );
  // What a thread draws between what answered it and its replies: its post, where it is a question that has one, its sources and the pages that link to it, then the pages in it.
  const between = (
    <>
      {parts.post && (
        <ThreadPost
          topic={topic}
          mayChange={mayChange}
          editing={editing}
          onEditing={editingOf('body')}
          onSave={saveBody}
          onEmptied={() => setWriting(false)}
          onLeave={writing ? () => setWriting(false) : undefined}
        />
      )}
      <ArticleEnd topic={topic.id} body={topic.body} />
      <PagesIn pages={read.pages} mayAdd={mayChange} onAdd={() => void newPage()} />
    </>
  );
  const talk = (tabbed: boolean) => (
    <Talk topic={topic} replies={read.replies} mayChange={mayChange} asked={askedAt} onChanged={changed} tabbed={tabbed} between={tabbed ? undefined : between} />
  );
  return (
    <div ref={sections.page} class="topic-page reading-column" data-topic={topic.id} data-shows={parts.showing ?? 'thread'}>
      <Path path={read.path} />
      <div class="view-head topic-page-head">
        {mayChange ? <TitleField topic={topic} onSaved={changed} /> : <h1 class="topic-page-title">{topic.title}</h1>}
        <StatusPick status={topic.status} mayChange={mayChange} onChoose={(status) => void set({ status })} />
        <span class="faint topic-page-where">
          {placeSaid(topic)}
          {topic.replies || read.replies.length ? ` · ${read.replies.length === 1 ? '1 reply' : `${read.replies.length} replies`}` : ''}
        </span>
        <span class="grow" />
        <FollowButton thing={{ kind: 'page', thing: topic.id }} />
        <Appearance />
        {/* What the head offers: a control of how the page is shown mounts first here, then Edit (an article's, or a
            question's post), what adds a part, then Close or Open again; a thread's History last. */}
        {mayChange && (parts.showing === 'article' || (parts.post && parts.showing === null)) && topic.body.trim() !== '' && !editing.body && (
          <button type="button" class="topic-edit" onClick={() => edit()}>
            Edit
          </button>
        )}
        {parts.addSummary && (
          <button type="button" class="ghost topic-add-summary" onClick={addSummary}>
            Add summary
          </button>
        )}
        {parts.writeArticle && (
          <button type="button" class="ghost topic-write-article" onClick={writeArticle}>
            Write an article
          </button>
        )}
        {parts.writePost && (
          <button type="button" class="ghost topic-write-post" onClick={() => setWriting(true)}>
            Add post
          </button>
        )}
        {mayChange &&
          topic.status !== null &&
          (topic.status === 'closed' ? (
            <button type="button" onClick={() => void set({ status: 'open' })}>
              Open again
            </button>
          ) : (
            <button type="button" onClick={() => void set({ status: 'closed' })}>
              Close
            </button>
          ))}
        {!parts.article && (
          <button type="button" class="ghost topic-history-toggle" aria-pressed={parts.showing === 'history'} onClick={() => setChosen(parts.showing === 'history' ? null : 'history')}>
            History
          </button>
        )}
        {parts.article && parts.showing && <Tabs topic={topic} replies={read.replies} showing={parts.showing} choose={setChosen} />}
      </div>
      {parts.showing === 'article' ? (
        <Article
          topic={topic}
          pages={read.pages}
          mayChange={mayChange}
          summary={summary}
          editing={editing}
          onEditing={editingOf('body')}
          onSave={saveBody}
          onAddPage={() => void newPage()}
          onEmptied={() => setWriting(false)}
          onLeave={writing ? () => setWriting(false) : undefined}
          contents={sections.headings.length >= CONTENTS_FROM ? <Contents topic={topic.id} headings={sections.headings} onJump={sections.jumpTo} /> : undefined}
          headed={headingMarks(topic.id, mayChange, sections.jumpTo, edit)}
          onBack={(section) => section && sections.jumpTo(section)}
          boxes={sideBoxes(topic.body, topic.id)}
        />
      ) : parts.showing === 'talk' ? (
        talk(true)
      ) : parts.showing === 'history' ? (
        <History topic={topic} mayChange={mayChange} tabbed={parts.article} shown={revision} onShow={setRevision} onRestore={set} />
      ) : (
        <>
          {summary}
          {talk(false)}
        </>
      )}
    </div>
  );
}

/** Article, Talk and History, under the head of a topic that is an article: Talk says how many replies it has. */
function Tabs({ topic, replies, showing, choose }: { topic: TopicWhole; replies: Reply[]; showing: TopicPart; choose: (part: TopicPart) => void }) {
  const tab = (part: TopicPart, label: string, count?: number) => (
    <button
      type="button"
      role="tab"
      class="view-tab topic-tab"
      id={`topic-tab-${part}-${topic.id}`}
      data-part={part}
      aria-selected={showing === part}
      aria-controls={`topic-${part}-${topic.id}`}
      onClick={() => choose(part)}
    >
      {label}
      {count ? <span class="faint topic-tab-count">{count}</span> : null}
    </button>
  );
  return (
    <div class="view-tabs tabs-ruled topic-tabs">
      <div class="view-tabs-list" role="tablist" aria-label="Article, talk and history">
        {tab('article', 'Article')}
        {tab('talk', 'Talk', replies.length)}
        {tab('history', 'History')}
      </div>
    </div>
  );
}
