// Search across everything its reader may read: what is written in files,
// documents, blueprints, messages, cards, rows and topics (with what was
// said on them), and projects, channels, people, agents and runs by what
// they are called. Opened by Ctrl and K, and by Ctrl, Shift and F.
//
// The index is kept by the daemon in the background. While it is catching
// up, searching still works on what has been indexed so far, and the bar at
// the bottom says how far along it is.

import type { ComponentChildren } from 'preact';
import { useEffect, useRef, useState } from 'preact/hooks';
import { api, SEARCH_LIMIT } from '../api.ts';
import type { Hit } from '../api.ts';
import { attempt, indexing, openTab, paletteOpen, paletteWith, projectName } from '../store.ts';
import { showReply } from '../topicshown.ts';
import { Modal, useFocus } from '../ui.tsx';
import { count, found } from '../util.ts';
import { showRow } from './records.tsx';
import { readAbout } from './seen.tsx';

const MARK_START = '';
const MARK_END = '';
const KINDS: Record<Hit['kind'], string> = {
  file: 'file',
  blueprint: 'blueprint',
  message: 'message',
  card: 'card',
  document: 'document',
  record: 'row',
  topic: 'topic',
  project: 'project',
  channel: 'channel',
  person: 'person',
  agent: 'agent',
  run: 'run',
};
/** What a search can be narrowed to, as it is offered: everything, or one kind of thing. */
const NARROWED: [Hit['kind'] | null, string][] = [
  [null, 'All'],
  ['topic', 'Topics'],
  ['document', 'Documents'],
  ['record', 'Rows'],
  ['file', 'Files'],
  ['message', 'Messages'],
  ['card', 'Cards'],
  ['blueprint', 'Blueprints'],
  ['project', 'Projects'],
  ['channel', 'Channels'],
  ['person', 'People'],
  ['agent', 'Agents'],
  ['run', 'Runs'],
];

/** What a hit is shown as: its kind, and of something said on a topic, a reply. */
function kindOf(hit: Hit): string {
  return hit.kind === 'topic' && hit.link.reply !== undefined ? 'reply' : KINDS[hit.kind];
}

/** Where a hit is, in words: its project, and where in it where that says more. */
function whereOf(hit: Hit): string {
  return [hit.project ? projectName(hit.project) : null, hit.place].filter(Boolean).join(' · ');
}

export function Palette() {
  // What a side panel's box handed on, once: opened again by its keys it begins empty.
  const [text, setText] = useState(() => paletteWith.peek());
  const [hits, setHits] = useState<Hit[] | null>(null);
  const [chosen, setChosen] = useState(0);
  // The one kind of thing being looked for, or everything.
  const [kind, setKind] = useState<Hit['kind'] | null>(null);
  const input = useFocus<HTMLInputElement>();
  useEffect(() => {
    paletteWith.value = '';
    // Typed on from where it was left: the caret after what was handed on.
    input.current?.setSelectionRange(text.length, text.length);
  }, []);
  const list = useRef<HTMLDivElement>(null);
  const close = () => (paletteOpen.value = false);

  // Ask a moment after typing stops; drop answers to questions since replaced.
  useEffect(() => {
    const query = text.trim();
    if (!query) {
      setHits(null);
      return;
    }
    let stale = false;
    const timer = setTimeout(() => {
      void api
        .search(query, undefined, kind ?? undefined)
        .then((answer) => {
          if (stale) return;
          setHits(answer.hits);
          setChosen(0);
          indexing.value = answer.index;
        })
        .catch(() => !stale && setHits([]));
    }, 120);
    return () => {
      stale = true;
      clearTimeout(timer);
    };
  }, [text, kind]);

  useEffect(() => list.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' }), [chosen]);

  const open = (hit: Hit) => {
    close();
    openHit(hit);
  };

  const progress = indexing.value;
  const busy = progress && progress.phase !== 'idle';
  return (
    <Modal wide onClose={close}>
      <input
        ref={input}
        class="palette-input"
        value={text}
        placeholder="Search everything"
        aria-label="Search"
        onInput={(event) => setText(event.currentTarget.value)}
        onKeyDown={(event) => {
          if (!hits || hits.length === 0) return;
          if (event.key === 'ArrowDown') {
            event.preventDefault();
            setChosen((chosen + 1) % hits.length);
          } else if (event.key === 'ArrowUp') {
            event.preventDefault();
            setChosen((chosen - 1 + hits.length) % hits.length);
          } else if (event.key === 'Enter') {
            event.preventDefault();
            open(hits[chosen]!);
          }
        }}
      />
      {/* Under what is looked for: how much of it was found, and what the search can be made to do. */}
      <div class="palette-tools">
        <div class="pills" role="group" aria-label="Look among">
          {NARROWED.map(([one, name]) => (
            // The caret stays in the box: narrowing is part of asking.
            <button key={name} type="button" aria-pressed={kind === one} onMouseDown={(event) => event.preventDefault()} onClick={() => setKind(one)}>
              {name}
            </button>
          ))}
        </div>
        <span class="palette-found" aria-live="polite">
          {hits ? found(hits.length, SEARCH_LIMIT) : ''}
        </span>
        <span class="grow" />
        <button class="ghost" title="Read everything again from the start. Searching goes on working while it does." onClick={() => void attempt(() => api.reindex())}>
          Rebuild the index
        </button>
      </div>
      <div class="palette-results" ref={list} role="listbox">
        {hits === null && <div class="empty" style="padding:28px">Type to search everything you may read. Results open in a tab.</div>}
        {hits?.length === 0 && (
          <div class="empty" style="padding:28px">
            <strong>Nothing matches</strong>
            {busy && <span>The index is still being built; try again in a moment.</span>}
          </div>
        )}
        {hits?.map((hit, index) => (
          <button key={hit.reference} class="hit" role="option" aria-selected={index === chosen} onMouseMove={() => setChosen(index)} onClick={() => open(hit)}>
            <span class="row">
              <span class="badge plain">{kindOf(hit)}</span>
              <strong class="truncate">{hit.title}</strong>
              <span class="grow" />
              {whereOf(hit) && <span class="faint truncate hit-where">{whereOf(hit)}</span>}
            </span>
            <span class="snippet">{marked(hit.snippet)}</span>
          </button>
        ))}
      </div>
      <div class="palette-foot">
        {busy ? (
          <>
            <span>{progress.phase === 'scanning' ? 'Looking for what changed' : `Indexing ${progress.done} of ${progress.total}`}</span>
            <div class="bar grow" role="progressbar" aria-valuemin={0} aria-valuemax={progress.total} aria-valuenow={progress.done}>
              <i style={`width:${progress.total ? Math.round((progress.done / progress.total) * 100) : 5}%`} />
            </div>
          </>
        ) : (
          <span class="grow">{progress ? `${count(progress.docs)} documents indexed` : ''}</span>
        )}
        <span class="kbd">Esc</span>
      </div>
    </Modal>
  );
}

/** Opens what was found, at the place in it that matched. */
export function openHit(hit: Hit): void {
  const { link } = hit;
  if (hit.kind === 'message' && link.channel) openTab({ kind: 'channel', id: link.channel }, { message: link.id });
  else if (hit.kind === 'card' && hit.project) openTab({ kind: 'board', project: hit.project }, { item: link.id });
  // A document at the heading nearest what matched: its line, as text, and the heading itself, as blocks or a page.
  else if (hit.kind === 'document' && hit.project && link.path) {
    openTab({ kind: 'file', project: hit.project, path: link.path }, link.line ? { line: link.line } : undefined);
    if (link.heading) toHeading(link.heading);
  }
  else if (hit.kind === 'record' && link.at && link.table && link.id) showRow(link.at, link.table, link.id);
  // Something said on a topic opens its talk there; the topic itself, its page.
  else if (hit.kind === 'topic' && link.id) {
    if (link.reply !== undefined) showReply(link.id, link.reply);
    else openTab({ kind: 'topic', id: link.id });
  } else if (hit.kind === 'project' && link.id) openTab({ kind: 'project', id: link.id });
  else if (hit.kind === 'channel' && link.channel) openTab({ kind: 'channel', id: link.channel });
  else if ((hit.kind === 'person' || hit.kind === 'agent') && link.actor) readAbout(link.actor, link.name ?? hit.title);
  else if (hit.kind === 'run' && link.run) openTab({ kind: 'run', id: link.run });
  else if (hit.kind === 'run' && link.instance) openTab({ kind: 'instance', id: link.instance });
  else if (hit.project && link.path) openTab({ kind: 'file', project: hit.project, path: link.path });
}

/**
 * Brings the heading that says `words` into view in the page that has the
 * keyboard, as the document opened is drawn: watched for until it is
 * there, for as long as a document takes to be read, and not after.
 */
function toHeading(words: string): void {
  const find = () =>
    [...document.querySelectorAll<HTMLElement>('.view[data-focused="true"] :is(h1, h2, h3, h4, h5, h6)')].find((one) => one.textContent?.trim() === words);
  const bring = () => {
    const heading = find();
    if (!heading) return false;
    heading.scrollIntoView({ block: 'start' });
    return true;
  };
  if (bring()) return;
  const watching = new MutationObserver(() => bring() && watching.disconnect());
  watching.observe(document.body, { childList: true, subtree: true });
  setTimeout(() => watching.disconnect(), 4000);
}

/** A snippet with the matched words emphasised. */
export function marked(snippet: string): ComponentChildren[] {
  const out: ComponentChildren[] = [];
  let rest = snippet;
  while (rest.length > 0) {
    const start = rest.indexOf(MARK_START);
    if (start < 0) {
      out.push(rest);
      break;
    }
    const end = rest.indexOf(MARK_END, start);
    if (start > 0) out.push(rest.slice(0, start));
    out.push(<mark>{rest.slice(start + 1, end < 0 ? undefined : end)}</mark>);
    rest = end < 0 ? '' : rest.slice(end + 1);
  }
  return out;
}
