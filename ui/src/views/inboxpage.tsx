// One line of the inbox as a page of its own: what a blueprint told
// whoever is signed in, opened by pressing its line in the box.
//
// It says who told it (the factory's bot, by its name) and when, on the
// clock and from now, and how often where it was told again; its words
// whole, each web address in them a link the browser opens in a tab of
// its own (`linked.ts`); what it is about, where it says, as something
// pressed that opens it as the line used to (a card on its board, a row
// over its table, a channel, something running); under that what it was
// told with, in tabs where its parts say any (`reviewed.tsx`); and what is
// done with the line itself: read, made to wait unread again, put away.
//
// It reads the line by its id, which only its owner may (`GET
// /api/inbox/<id>`), and reads it again whenever the daemon says
// something of that line (`inbox.told`, `.read`, `.unread`, `.away`),
// so a page left open follows a telling made again with nothing polled.
// A line that is gone (it went with what it opened) says so.

import { useEffect, useRef } from 'preact/hooks';
import { ApiError } from '../api.ts';
import { fromOf, isOf, offeredOf, recordSaid, sentenceOf, thingOf, toldWhen, wordsOf } from '../inbox.ts';
import { FollowedChanges } from './followedchanges.tsx';
import type { AboutRow, Changed, Line, Thing } from '../inbox.ts';
import { inbox, opened } from '../inboxed.ts';
import { hashed } from '../kept.ts';
import { piecesOf } from '../linked.ts';
import { attempt, channels, openTab, projects } from '../store.ts';
import { useEvents } from '../ui.tsx';
import { actorKind, classes, when } from '../util.ts';
import { WithCode } from './codeblock.tsx';
import { PathsIn, WordsOf } from './pathlink.tsx';
import { projectOfId } from '../pathlinked.ts';
import { openRow } from './records.tsx';
import { useKept } from './returned.tsx';
import { ToldParts } from './reviewed.tsx';
import { Running } from './running.tsx';
import { showLatestRun } from './pipelinerun.tsx';
import { runTitle } from '../runread.ts';
import { signs } from './signs.tsx';

/**
 * Words, with each web address in them a link the browser opens in a tab
 * of its own: the page fetches nothing from there, and the page opened
 * has no hold on this one.
 */
export function Linked({ words }: { words: string }) {
  return (
    <>
      {piecesOf(words).map((piece, at) =>
        piece.href ? (
          <a key={at} class="inbox-link" href={piece.href} target="_blank" rel="noopener noreferrer" title={piece.href}>
            {piece.text}
          </a>
        ) : (
          <PathsIn key={at} text={piece.text} />
        ),
      )}
    </>
  );
}

/** Shows what a blueprint said a line is about. A record is asked for first: one that is not the reader's to read, or is gone, shows nothing. */
export async function showThing(thing: Thing): Promise<void> {
  if (thing.kind === 'channel') return openTab({ kind: 'channel', id: thing.id });
  // What runs, and one run of it, are shown as a run: the run's page, of what runs its latest.
  if (thing.kind === 'running') return showLatestRun(thing.id);
  if (thing.kind === 'run') return openTab({ kind: 'run', id: thing.id });
  const row = await inbox.about(thing.id).catch(() => null);
  if (!row?.project || !row.collection) return;
  // A card is shown on its board, with its drawer; a row of any other table in the drawer, over its table.
  if (row.collection === 'board') return openTab({ kind: 'board', project: row.project }, { item: row.id });
  openTab({ kind: 'table', project: row.project, name: row.collection });
  openRow(row.project, row.collection, row);
}

/** What the page has of its line: the line as last read, or that it is gone, or why it could not be read. */
interface Read {
  line: Line | null;
  gone: boolean;
  failed: string | null;
}

/** What a told line is about, as something pressed that opens it; nothing where it is about nothing. */
function About({ line }: { line: Line }) {
  const thing = thingOf(
    line,
    channels.value.map((channel) => channel.id),
  );
  // The record it is about, read once for what it is called; read again with the line.
  const [row, setRow] = useKept<AboutRow | null | 'unread'>(`about:${line.place}`, 'unread');
  useEffect(() => {
    if (thing?.kind !== 'record') return;
    let live = true;
    void inbox
      .about(thing.id)
      .then((found) => live && setRow(found))
      .catch(() => live && setRow(null));
    return () => void (live = false);
  }, [thing?.kind, thing?.id, line.raised]);
  if (!thing) return null;
  const head = <h2 class="inbox-page-label">What it is about</h2>;
  if (thing.kind === 'running' || thing.kind === 'run') {
    // Said for what it is, and pressed for the page of the run: of what runs, its latest.
    return (
      <section class="inbox-page-about">
        {head}
        <button type="button" class="ghost inbox-page-opens" title={thing.kind === 'run' ? 'Open this run' : 'Open its latest run'} onClick={() => void showThing(thing)}>
          {thing.kind === 'run' ? <span class="inbox-page-opens-says">{runTitle(thing.id)}</span> : <Running id={thing.id} plain />}
          <span class="faint">{thing.kind === 'run' ? 'a run, as a page of its own' : 'its latest run'}</span>
        </button>
      </section>
    );
  }
  if (thing.kind === 'channel') {
    const channel = channels.value.find((one) => one.id === thing.id);
    return (
      <section class="inbox-page-about">
        {head}
        <button type="button" class="ghost inbox-page-opens" onClick={() => void showThing(thing)}>
          {channel ? hashed(channel) : thing.id}
        </button>
      </section>
    );
  }
  if (row === 'unread') {
    return (
      <section class="inbox-page-about">
        {head}
        <span class="faint">{thing.id}</span>
      </section>
    );
  }
  const name = projects.value.find((project) => project.id === row?.project)?.name ?? null;
  const said = recordSaid(thing.id, row, name);
  return (
    <section class="inbox-page-about">
      {head}
      {said.opens ? (
        <button type="button" class="ghost inbox-page-opens" title={`Open ${said.where}`} onClick={() => void showThing(thing)}>
          <span class="inbox-page-opens-says">{said.says}</span>
          <span class="faint">{said.where}</span>
        </button>
      ) : (
        <span class="inbox-page-gone">
          <code>{said.says}</code> <span class="faint">{said.where}</span>
        </span>
      )}
    </section>
  );
}

export function InboxPage({ id }: { id: string }) {
  const [read, setRead] = useKept<Read>('line', { line: null, gone: false, failed: null });
  // Of the readings on their way, only the one asked for last is taken.
  const asked = useRef(0);
  const readIt = async () => {
    const mine = ++asked.current;
    try {
      const line = await inbox.one(id);
      if (mine !== asked.current) return;
      opened.value = { ...opened.peek(), [id]: line };
      setRead({ line, gone: false, failed: null });
    } catch (error) {
      if (mine !== asked.current) return;
      if (error instanceof ApiError && error.status === 404) setRead({ line: null, gone: true, failed: null });
      else setRead((was) => ({ ...was, failed: error instanceof Error ? error.message : String(error) }));
    }
  };
  // Read as the page is made, and so again behind what it last had as it comes back.
  useEffect(() => void readIt(), [id]);
  useEvents('inbox.*', (event) => {
    if (isOf(id, event as unknown as Changed)) void readIt();
  }, [id]);

  const line = read.line;
  if (read.gone) {
    return (
      <div class="inbox-page">
        <div class="view-head">
          <h1>Inbox message</h1>
        </div>
        <p class="inbox-page-none" role="status">
          This is no longer in your inbox: a line goes with what it opened when that is taken away, and a line is only ever its owner's to read.
        </p>
      </div>
    );
  }
  if (!line) {
    return <div class="empty">{read.failed ? `It could not be read: ${read.failed}` : 'Reading it'}</div>;
  }
  const now = Date.now();
  const told = toldWhen(line, now);
  const offered = offeredOf(line);
  const bot = actorKind(line.from) === 'bot';
  const from = line.kind === 'following' ? fromOf(line) : line.from.slice(line.from.indexOf(':') + 1);
  const act = async (work: () => Promise<unknown>) => {
    await attempt(work);
    await readIt();
  };
  return (
    <div class="inbox-page">
      <div class="view-head">
        {/* Headed by its first sentence whole, on as many lines as it takes: its tab is called by its first words. */}
        <h1 class="inbox-page-title">{sentenceOf(line)}</h1>
      </div>
      <article class={classes('inbox-page-message', line.read === null && 'unread')}>
        <header class="inbox-page-head">
          <span class="inbox-page-from">
            <strong>{from}</strong>
            {bot && <span class="faint"> · the factory's bot</span>}
          </span>
          <time class="inbox-page-when" dateTime={new Date(line.raised).toISOString()} title={when(line.raised)}>
            {told.at} · {told.ago}
          </time>
        </header>
        {told.times && <p class="inbox-page-times faint">{told.times}</p>}
        <div class="inbox-page-words">
          {/* A file it names is of the project of what it is about, where that says one. */}
          <WordsOf.Provider value={channels.value.find((one) => one.id === line.place)?.project ?? projectOfId(line.place, projects.value.map((project) => project.id))}>
            <WithCode text={line.says ?? wordsOf(line)} words={(words) => <Linked words={words} />} />
          </WordsOf.Provider>
        </div>
        {line.kind === 'following' ? <FollowedChanges line={line} /> : <About line={line} />}
        <ToldParts line={line.id} kept={Array.isArray(line.parts) && line.parts.length > 0} />
        <footer class="inbox-page-acts">
          {line.away !== null && <span class="faint inbox-page-away">Put away. It is back in your inbox if it is told again.</span>}
          {offered.unread && (
            <button type="button" onClick={() => void act(() => inbox.unread(line.id))}>
              {signs.envelope(13)} Mark unread
            </button>
          )}
          {offered.read && (
            <button type="button" onClick={() => void act(() => inbox.read(line.id))}>
              Mark read
            </button>
          )}
          {offered.away && (
            <button type="button" onClick={() => void act(() => inbox.away(line.id))}>
              {signs.cross()} Put away
            </button>
          )}
        </footer>
      </article>
    </div>
  );
}
