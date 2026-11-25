// The Topics panel as one tree of wikis: each
// wiki a line at the top in the order it stands, the pages under it to any
// depth, every page that holds pages a fold of the panel's (`TOPICS.page`,
// folded as it starts), a closed page faint where it is and a closed wiki
// under Closed at the panel's end. What is listed, and what the panel's
// box finds, is `topics.tsx`; the plain values are `wikis.ts`.
//
// Each line has, for whoever may write records, a `+` that makes a page
// under it at once, called Untitled, unfolds the line and opens the page,
// with its title in a field in the line, selected, to be written (saved on
// Enter and as it is left, Escape keeping Untitled; on a phone, where the
// panel is put away as a page is opened, the page is opened once its title
// is written); and a menu: New page,
// Rename…, Move to… (`wikimoved.tsx`), Open in a new tab in a browser, and
// Move to Trash. New topic at the panel's head makes a wiki the same way, the
// own of the workspace the panel is of: it asks no place.
//
// A line is carried to another place in the tree by the general carry
// (`treedrag.tsx`): every page says what it is (`data-place`) and, since
// any page can hold pages, that it can hold lines (`data-outline`), on its
// own element with what is under it drawn inside it. Where it lands is one
// move (`topicsApi.move`), and only where the daemon would take it
// (`landing` in `wikis.ts`): never under itself, and never into a wiki of
// another workspace. A page carried to the top is a wiki of its own.
//
// Nothing is drawn ahead of what the daemon answers: a move may be refused,
// and what is drawn is the reading after it (`topicsread.ts`).

import { effect, signal } from '@preact/signals';
import { useEffect, useRef, useState } from 'preact/hooks';
import { held } from '../chosen.ts';
import { TOPICS, topicFoldsGone } from '../folds.ts';
import { names, inPlace } from '../inplace.ts';
import { phone } from '../phone.ts';
import { holds, ofTopics, shows } from '../panelsought.ts';
import type { Found } from '../panelsought.ts';
import { api } from '../api.ts';
import { active, attempt, openTab, projects, tabKey, toast, turn, turned, turnedTo } from '../store.ts';
import type { Tab } from '../store.ts';
import { statusSaid, titled, whereSaid } from '../topicpage.ts';
import type { TopicLine } from '../topicpage.ts';
import { readTopics, topics, topicsApi, topicsRead, wantTopics } from '../topicsread.ts';
import { Twist } from '../ui.tsx';
import { classes } from '../util.ts';
import { landing, UNTITLED } from '../wikis.ts';
import type { Drawn } from '../wikis.ts';
import { More } from './grouped.tsx';
import { Fold } from './navtree.tsx';
import type { Offered } from './grouped.tsx';
import { newTabLines } from './newtab.tsx';
import { Named, unfolded } from './outline.tsx';
import { NothingFound } from './panelseek.tsx';
import { signs } from './signs.tsx';
import { mayMakeTopic, TOPIC_GLYPH, topicsListedNow } from './topics.tsx';
import { saidDeleted } from './trash.tsx';
import { useTreeCarry } from './treedrag.tsx';
import { WikiMoved } from './wikimoved.tsx';

/** What the daemon said, as a page says it: its own sentence, without the marks a terminal reads commands by. */
const said = (refused: unknown) => (refused instanceof Error ? refused.message : String(refused)).replaceAll('`', '');

/** The page just made whose title is being written in its line, until it is saved or let be. */
const writing = signal<string | null>(null);

/** What is being asked of a page, until it is answered or put away. */
const asking = signal<{ kind: 'rename' | 'move'; id: string } | null>(null);

// The fold of a page that is not there is dropped once the topics are read: nothing else would take it away.
effect(() => {
  if (!topicsRead.value) return;
  const gone = topicFoldsGone(turned.value, topics.value.map((one) => one.id));
  if (gone.length > 0) turnedTo([...turned.value].filter((one) => !gone.includes(one)));
});

/**
 * Makes a page under `under`, or a wiki of the workspace the panel is of
 * where nothing, called Untitled: read, unfolded where it is, opened, and
 * its title in its line to be written. What the daemon refuses is said.
 */
async function makePage(under: TopicLine | null): Promise<void> {
  try {
    const now = held.value;
    const made = under ? await topicsApi.makeUnder(UNTITLED, under.id) : await topicsApi.make(UNTITLED, now.kind === 'space' ? { workspace: now.id } : {});
    await readTopics();
    if (under) unfolded(TOPICS.page(under.id));
    // On a phone the panel is put away as a page is opened, and the title is written in it first: the page is
    // opened once it is.
    if (!phone.value) openTab({ kind: 'topic', id: made.id, title: UNTITLED });
    writing.value = made.id;
  } catch (refused) {
    toast(said(refused), true);
  }
}

/** Moves `page` under `under` (the top where nothing), before `before` (the end where nothing): whether it came there; a refusal is said. */
async function movePage(page: string, under: string | null, before: string | null): Promise<boolean> {
  try {
    await topicsApi.move(page, under, before);
    await readTopics();
    return true;
  } catch (refused) {
    toast(said(refused), true);
    return false;
  }
}

/** What a page's menu offers: for whoever may write records, a page under it, it called something else, moved and moved to the Trash; and in a browser, it opened in a tab of its own. */
function pageLines(line: TopicLine): Offered[] {
  const tab: Tab = { kind: 'topic', id: line.id, title: line.title };
  if (!mayMakeTopic()) return newTabLines(tab);
  const trash = async () => {
    const gone = await attempt(() => api.removeRecord(line.id));
    if (gone === undefined) return;
    // Into the Trash, with Undo, which gives it back where it was.
    saidDeleted(gone as { trash?: string; kept?: number | null }, line.title);
    await readTopics();
  };
  return [
    { label: 'New page', mark: signs.plus(), act: () => void makePage(line) },
    { label: 'Rename…', mark: signs.pencil(), act: () => (asking.value = { kind: 'rename', id: line.id }) },
    { label: 'Move to…', mark: signs.move(), act: () => (asking.value = { kind: 'move', id: line.id }) },
    ...newTabLines(tab),
    { label: 'Move to Trash', mark: signs.bin(), look: 'grave', apart: true, act: () => void trash() },
  ];
}

/** New topic, at the panel's head, for whoever may make one: a wiki of the workspace the panel is of, made at once. */
export function NewWikiButton() {
  if (!mayMakeTopic()) return null;
  return (
    <button class="ghost icon brand-act" aria-label="New topic" title="New topic" onClick={() => void makePage(null)}>
      +
    </button>
  );
}

/**
 * A page's title written in its line as it is made: selected, saved on
 * Enter and as it is left, and let be (Untitled) by Escape.
 */
function TitleField({ line }: { line: TopicLine }) {
  const [typed, setTyped] = useState(line.title);
  const input = useRef<HTMLInputElement>(null);
  const done = useRef(false);
  useEffect(() => {
    const at = requestAnimationFrame(() => {
      input.current?.focus();
      input.current?.select();
      input.current?.scrollIntoView({ block: 'nearest' });
    });
    return () => cancelAnimationFrame(at);
  }, []);
  const finish = async (save: boolean) => {
    if (done.current) return;
    done.current = true;
    writing.value = null;
    const { title, refused } = titled(typed);
    const named = save && refused === null && title !== line.title;
    if (phone.value) openTab({ kind: 'topic', id: line.id, title: named ? title : line.title });
    if (named) await attempt(() => api.setRecord(line.id, { title }));
  };
  return (
    <span class="nav-item wiki-writing">
      <span class="glyph">{TOPIC_GLYPH}</span>
      <input
        ref={input}
        class="wiki-title-field"
        aria-label="Page title"
        value={typed}
        onInput={(event) => setTyped(event.currentTarget.value)}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            event.preventDefault();
            void finish(true);
          } else if (event.key === 'Escape') {
            // Escape is the field's: it lets the page be, called what it was made, and puts nothing else away.
            event.preventDefault();
            event.stopPropagation();
            void finish(false);
          }
        }}
        onBlur={() => void finish(true)}
      />
    </span>
  );
}

/**
 * A page's line, and the pages under it where it holds any: its title,
 * which opens it, faintly how it stands where it is not open and, on a
 * wiki, the project it is of; the menu and the `+` at its end. A page with
 * pages folds them by its arrow, and is held open while something under it
 * is found by the panel's box.
 */
function PageLine({ page, found, may, apart }: { page: Drawn<TopicLine>; found: Found | null; may: boolean; apart?: boolean }) {
  const { line } = page;
  if (!shows(found, ofTopics.topic(line.id))) return null;
  const fold = TOPICS.page(line.id);
  const hold = holds(found, ofTopics.topic(line.id));
  const any = page.pages.length > 0;
  const open = any && (hold || turned.value.has(fold));
  const tab: Tab = { kind: 'topic', id: line.id, title: line.title };
  const current = active.value === tabKey(tab);
  // A wiki under Closed is not said to be closed again: the branch it is under says it.
  const hint = [apart ? null : statusSaid(line.status), line.parent ? null : whereSaid(line, projects.value)].filter(Boolean).join(' · ') || undefined;
  const title = line.summary ? `${line.title}: ${line.summary}` : undefined;
  const acts = (
    <>
      {(may || inPlace) && <More label={`${line.title}: more`} items={() => pageLines(line)} />}
      {may && (
        <button class="ghost icon row-more wiki-add" aria-label={`New page in ${line.title}`} title={`New page in ${line.title}`} data-untipped onClick={() => void makePage(line)}>
          +
        </button>
      )}
    </>
  );
  const named =
    writing.value === line.id ? (
      <TitleField line={line} />
    ) : any ? (
      <button class="nav-item" aria-current={current} data-names={names(tab)} title={title} onClick={() => openTab(tab)}>
        <span class="truncate">{line.title}</span>
        {hint && <span class="nav-hint">{hint}</span>}
      </button>
    ) : (
      <button class="nav-item" aria-current={current} data-names={names(tab)} title={title} onClick={() => openTab(tab)}>
        <span class="glyph">{TOPIC_GLYPH}</span>
        <span class="truncate">{line.title}</span>
        {hint && <span class="nav-hint">{hint}</span>}
      </button>
    );
  return (
    <div class={classes('topic-line', 'wiki-page', line.status === 'closed' && 'closed')} data-topic={line.id} data-place={line.id} data-outline={line.id}>
      {any ? (
        <div class={classes('nav-row', 'inner', current && 'current')}>
          <button class="nav-twist" aria-label={open ? `Fold ${line.title}` : `Unfold ${line.title}`} aria-expanded={open} aria-disabled={hold || undefined} onClick={() => !hold && turn(fold)}>
            <Twist open={open} />
          </button>
          {named}
          {acts}
        </div>
      ) : (
        <div class="nav-line">
          {named}
          {acts}
        </div>
      )}
      {open && (
        <div class="tree">
          {page.pages.map((one) => (
            <PageLine key={one.line.id} page={one} found={found} may={may} />
          ))}
        </div>
      )}
    </div>
  );
}

/**
 * The wikis of the workspace the panel is of, directly in the panel's list
 * as the channels are in theirs, and the closed ones under Closed. What is
 * typed in the panel's box (`found`) leaves out what does not say it, and
 * holds open what something found is under; nothing more is read for it.
 */
export function WikiTree({ found, text }: { found: Found | null; text: string }) {
  // Read once the panel is first shown, and followed (`topicsread.ts`).
  useEffect(wantTopics, []);
  const { tree, listed, closed } = topicsListedNow();
  const may = mayMakeTopic();
  const carried = useTreeCarry(
    may
      ? {
          land: async (node, to, by) => {
            const came = await movePage(node, to.into ?? null, to.before ?? null);
            if (came && by === 'keys' && to.into) unfolded(TOPICS.page(to.into));
            return came;
          },
          allows: (node, to) => !('refused' in landing(tree, node, to.into ?? null, to.before ?? null)),
        }
      : null,
  );
  return (
    <div class="wikis" {...carried}>
      {listed.map((one) => (
        <PageLine key={one.line.id} page={one} found={found} may={may} />
      ))}
      {closed.length > 0 && shows(found, ofTopics.closed) && (
        <Fold at={TOPICS.closed} label="Closed" hint={String(closed.length)} held={holds(found, ofTopics.closed)} title={`The wikis of this workspace that are closed: ${closed.length}`}>
          {closed.map((one) => (
            <PageLine key={one.line.id} page={one} found={found} may={may} apart />
          ))}
        </Fold>
      )}
      {!found && !topicsRead.value && <span class="nav-who faint">Reading the topics</span>}
      {!found &&
        topicsRead.value &&
        listed.length === 0 &&
        closed.length === 0 &&
        (may ? (
          <button class="nav-item topics-first" onClick={() => void makePage(null)}>
            <span class="glyph">+</span>
            <span>Start the first topic</span>
          </button>
        ) : (
          <span class="nav-who faint">No wikis here yet</span>
        ))}
      {found && found.shown.size === 0 && <NothingFound text={text} />}
    </div>
  );
}

/** What is asked of a page, while something is: drawn once, by the Topics panel. */
export function WikiAsked() {
  const now = asking.value;
  if (!now) return null;
  const close = () => (asking.value = null);
  const line = topics.value.find((one) => one.id === now.id);
  if (!line) return null;
  if (now.kind === 'move') return <WikiMoved key={`move-${line.id}`} page={line} onClose={close} />;
  return (
    <Named
      key={`rename-${line.id}`}
      title={`Rename ${line.title}`}
      label="Title"
      typed={line.title}
      says="What everybody reads it as. Its replies and the pages under it stay as they are."
      check={(typed) => titled(typed).refused}
      same={(typed) => titled(typed).title === line.title}
      save={async (typed) => {
        await api.setRecord(line.id, { title: titled(typed).title });
        await readTopics();
      }}
      onClose={close}
    />
  );
}
