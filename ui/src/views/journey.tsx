// A record's history: what happened to it, oldest first.
//
// The daemon keeps it, a line an event, as each happens, and never changes
// a line or takes one away. This reads it and shows it, and reads it again
// when it is told another line was written. It is shown wherever a card
// is: in its drawer on the board, in the tools panel, and on the fleet.
//
// What was said on a record (what anyone said, and on a card something an
// agent asked) is in the same history, and is drawn as what was said and
// by whom. A record's form shows it apart from what happened, in two of
// the one row of tabs at its top (`RecordTabs`, which a card's `CardTabs`
// is too); elsewhere the two are drawn together, in the order it all did.

import type { ComponentChildren } from 'preact';
import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import { aboutOf } from '../anchors.ts';
import type { HistoryLine } from '../api.ts';
import type { Speaker, Spoken } from '../journey.ts';
import { coalesce } from '../events.ts';
import { comments, drawn, happened, paragraphsOf, saidNow, speakerOf, viaOf } from '../journey.ts';
import { oneOf } from '../sits.ts';
import { aboutCards, tabOf } from '../cardtabs.ts';
import { showCardTab } from '../cardasked.ts';
import { agents, collections, speaker } from '../store.ts';
import { knownBy, said as cellSaid } from '../table.ts';
import type { Row } from '../table.ts';
import { useEvents } from '../ui.tsx';
import { actorKind, classes, initials, when } from '../util.ts';
import { WithCode } from './codeblock.tsx';
import { PathsIn } from './pathlink.tsx';
import { openRow } from './records.tsx';
import { readAbout } from './seen.tsx';

/** What a history says where nothing of it has been kept. */
const UNKEPT = "Nothing has been kept of this yet. A board keeps its cards' history once its blueprint says what that is; apply it again and what has already happened is filled in.";

/** A record's history as it is now: null while it is first being read. */
export function useJourney(record: string): HistoryLine[] | null {
  const [lines, setLines] = useState<HistoryLine[] | null>(null);
  // Of no record, nothing is read: a page that may be of one asks all the same.
  const reload = useMemo(() => coalesce(() => void (record ? api.history(record) : Promise.resolve([])).then(setLines, () => setLines([])), 100), [record]);
  useEffect(() => {
    setLines(null);
    reload();
  }, [reload]);
  useEvents(['collection.history'], (event) => event.data?.record === record && reload(), [record]);
  return lines;
}

export function Journey({ record }: { record: string }) {
  return <Lines lines={useJourney(record)} />;
}

/** What the page knows of who is who, for drawing whoever a line is of: the agents there are, and every table read. */
function useKnown(): { agents: Set<string>; tables: { project: string; name: string }[] } {
  const names = new Set(agents.value.map((one) => one.name));
  const tables = Object.entries(collections.value).flatMap(([project, all]) => all.map((one) => ({ project, name: one.name })));
  return { agents: names, tables };
}

/**
 * Whoever a line is of, with their picture beside their name,
 * the same face as beside what they say in a channel; the two pressed, or
 * reached by the keyboard, open where they are read about (`readAbout`).
 * A record that asked is drawn as the bot that speaks for it, the record
 * named after, pressed to open its row. With `alone`, the picture alone,
 * for a line whose sentence says the name.
 */
function Speaking({ who, alone }: { who: Speaker; alone?: boolean }) {
  const kind = who.actor ? actorKind(who.actor) : 'system';
  // Its letters are drawn by the style sheet (`data-letters`), so that what a line says, read or copied, is its words alone.
  const face = <span class={classes('avatar', 'face', kind)} aria-hidden="true" data-letters={who.actor ? initials(who.name) : '·'} />;
  const named = alone ? null : <strong class="journey-who">{who.name}</strong>;
  const by = who.actor ? (
    <button type="button" class="journey-by" data-actor={who.actor} aria-label={alone ? `Read about ${who.name}` : undefined} title={`Read about ${who.name}`} onClick={() => readAbout(who.actor!, who.name)}>
      {face}
      {named}
    </button>
  ) : (
    <span class="journey-by">
      {face}
      {named}
    </span>
  );
  return (
    <>
      {by}
      {who.record && <ForRecord record={who.record} />}
    </>
  );
}

/** The record a bot spoke for, after the bot: `, for the contest of shop-370`, pressed to open its row. */
function ForRecord({ record }: { record: NonNullable<Speaker['record']> }) {
  const [row, setRow] = useState<Row | null>(null);
  useEffect(() => void api.record(record.id).then(setRow, () => setRow(null)), [record.id]);
  const table = (collections.value[record.project] ?? []).find((one) => one.name === record.table);
  const about = table?.about && row ? cellSaid(row.data[table.about]).trim() : '';
  const words = row && table ? (about ? `the ${oneOf(table.name)} of ${about}` : knownBy(row, table.fields)) : record.id;
  return (
    <>
      {', for '}
      {row ? (
        <button type="button" class="ghost link journey-for" data-record={record.id} title={`Open ${record.id}`} onClick={() => openRow(record.project, record.table, row)}>
          {words}
        </button>
      ) : (
        <span class="journey-for">{words}</span>
      )}
    </>
  );
}

/**
 * Where what a question said on a card for a record that asks is marked: the card's tab of that record's table, where
 * its row and what the question names under it are (`cardtabs.tsx`). Nothing for anything else, or a table that is no
 * tab of a card.
 */
function TabOfRecord({ line, record }: { line: HistoryLine; record: NonNullable<Speaker['record']> }) {
  const card = cellSaid(line.data?.id);
  const tab = aboutCards('board', collections.value[record.project] ?? []).find((one) => one.name === record.table);
  if (!tab || line.data?.collection !== 'board' || !card) return null;
  return (
    <button type="button" class="ghost link journey-tab" data-tab={tab.name} onClick={() => showCardTab(card, tab.name)}>
      See it in {tabOf(tab)}
    </button>
  );
}

/** One line of a history as it is drawn: something said, by whom, or something that happened. `at` marks the one a line of somebody's inbox opened. */
function Line({ line, said, bot, at }: { line: HistoryLine; said: Spoken | null; bot: string | null; at?: boolean }) {
  const known = useKnown();
  const who = speakerOf(line, said, bot, known.agents, known.tables);
  return said ? (
    <li key={line.seq} data-seq={line.seq} class={classes('journey-spoken', said.asks && 'asks', at && 'journey-at')}>
      <span class="journey-when faint">
        <Speaking who={who} /> {said.asks ? 'asked' : said.answers ? 'answered' : 'said'} · {when(line.ts)}
        {said.public && (
          <span class="journey-public" title="Marked public by whoever said it: it may be told outside the factory. What is not marked stays here.">
            public
          </span>
        )}
      </span>
      {said.on && <span class="comment-on">on “{aboutOf(said.on)}”</span>}
      <div class="journey-words">
        <WithCode text={said.words} words={(words) => <PathsIn text={words} named />} />
      </div>
      {said.asks && who.record && <TabOfRecord line={line} record={who.record} />}
    </li>
  ) : (
    <li key={line.seq} data-seq={line.seq}>
      <span class="journey-when faint">{when(line.ts)}</span>
      <span class="journey-said">
        <Speaking who={who} alone />
        <WithCode text={saidNow(line, bot)} words={(words) => paragraphsOf(words).map((one, at) => <span class="journey-para" key={at}>{one}</span>)} />
        {/* The blueprint that did it is detail, never who did it: that is the factory's bot. */}
        {viaOf(line) && <span class="journey-via faint"> · via {viaOf(line)}</span>}
      </span>
    </li>
  );
}

/** The history itself, for whatever has already read it; `children` go between its heading and its lines. */
export function Lines({ lines, children, empty }: { lines: HistoryLine[] | null; children?: ComponentChildren; empty?: string }) {
  const bot = speaker.value?.name ?? null;
  return (
    <div class="journey" aria-label="History">
      <h3>History</h3>
      {children}
      {lines === null && <p class="faint">Reading</p>}
      {lines?.length === 0 && <p class="faint">{empty ?? UNKEPT}</p>}
      {lines && lines.length > 0 && (
        <ol>
          {drawn(lines).map(({ line, said }) => (
            <Line key={line.seq} line={line} said={said} bot={bot} />
          ))}
        </ol>
      )}
    </div>
  );
}

/**
 * What was said on a record, as a tab's panel: the box something is said
 * in (`box`) and every line of its history that is something said
 * (`comments` in `journey.ts`). `at` is the comment a line of somebody's
 * inbox opened, by its place in the history: marked, and brought into view
 * once it is drawn, once only for each asking (a comment asked for while the
 * history is still being read again, one said a moment ago, is brought into
 * view when it comes).
 */
export function SaidPanel({ lines, box, at }: { lines: HistoryLine[] | null; box?: ComponentChildren; at?: number | null }) {
  const bot = speaker.value?.name ?? null;
  const said = lines && comments(lines);
  const list = useRef<HTMLOListElement>(null);
  const there = at != null && (said ?? []).some(({ line }) => line.seq === at);
  useEffect(() => {
    if (there) list.current?.querySelector(`[data-seq="${at}"]`)?.scrollIntoView({ block: 'center' });
  }, [at, there]);
  return (
    <div class="talked-panel" role="tabpanel" aria-label="Comments">
      {box}
      {lines === null && <p class="faint">Reading</p>}
      {said?.length === 0 && <p class="faint">Nothing has been said on it yet.</p>}
      {said && said.length > 0 && (
        <ol ref={list}>
          {said.map(({ line, said: spoken }) => (
            <Line key={line.seq} line={line} said={spoken} bot={bot} at={line.seq === at} />
          ))}
        </ol>
      )}
    </div>
  );
}

/** What happened to a record, as a tab's panel (`happened` in `journey.ts`), with no box. */
export function HappenedPanel({ lines, empty }: { lines: HistoryLine[] | null; empty?: string }) {
  const bot = speaker.value?.name ?? null;
  const rest = lines && happened(lines);
  return (
    <div class="talked-panel" role="tabpanel" aria-label="History">
      {lines === null && <p class="faint">Reading</p>}
      {rest?.length === 0 && <p class="faint">{empty ?? UNKEPT}</p>}
      {rest && rest.length > 0 && (
        <ol>
          {rest.map((line) => (
            <Line key={line.seq} line={line} said={null} bot={bot} />
          ))}
        </ol>
      )}
    </div>
  );
}
