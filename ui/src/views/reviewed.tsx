// The parts of a line a blueprint told, on the line's own page: what the
// blueprint chose to show beside its words (`parts` of `inbox.tell`).
// Addresses are links the browser opens in a tab of its own, but one of
// this factory's page that names a thing (`#/run/<run>`), which is opened
// here as a tab beside what is open (`thingOfLink`, `opens`); the cards
// of a board in one column or with one label are a table, a row a card,
// each with its id and its title in columns of their own, whom it is
// assigned to, where it is and the last thing said on it, and Pass, Fail
// and its thread in its last cell for whoever may. Its title, whom it is
// assigned to and the last thing said are each one line, cut where they
// do not fit and said whole where they are pointed at; it is never the
// card's body. A part of a column lists the cards it was told of wherever
// each is now: one that moved on says where it is in that cell, with
// nothing to press, and one taken off the board says it is gone; where
// the part is narrow (a phone) a card's cells stand under its id and
// title, and the words wrap. A card's title opens it in
// the drawer beside the page, its own form and history (`toldcard.tsx`).
//
// What each part comes to is the daemon's, read as the reader may read
// it (`GET /api/inbox/<line>/parts`), and read again when a card of its
// project is moved or changed or a line is written of its history, so a
// page left open follows the board with nothing polled. Pass and Fail
// are the card's: what a press does is asked of the daemon, which moves
// the card or says on it that it passed, and what it refuses is said in
// its sentence. What is decided here is `reviewed.ts`, with its cases.
//
// A part may say which tab of the page it is in: the parts that say none
// are on a first tab, and where there is more than one tab a row of tabs
// is drawn over the parts, the first showing, which shows being kept by
// the line's tab (`useKept`) as anything a tab shows is. What the line
// is about stays under its words, over the tabs. Rows of a table, runs and
// measurements are drawn by `toldread.tsx`.

import { useEffect, useRef, useState } from 'preact/hooks';
import { call } from '../api.ts';
import { doneSaid, failClosed, headOf, lastSaid, noneListed, nothingSaid, offeredFor, partsOf, threadCalled, whereNow } from '../reviewed.ts';
import type { Card, CardsPart, MessagePart, Part } from '../reviewed.ts';
import { openTab } from '../store.ts';
import { thingOfLink } from '../tab.ts';
import { opens } from '../address.ts';
import { useEvents } from '../ui.tsx';
import { AskedPart } from './askedpart.tsx';
import { GoAskedPart } from './gopart.tsx';
import { Assignee } from './assignee.tsx';
import { showThread } from './channel.tsx';
import { useKept } from './returned.tsx';
import { openToldCard } from './toldcard.tsx';
import { MeasuresView, RowsView, RunsView } from './toldread.tsx';
import { showing, tabsOf } from '../toldread.ts';

const at = (id: string) => encodeURIComponent(id);

/** What the page has of a line's parts: as last read, or why they could not be. */
interface Read {
  parts: Part[];
  failed: string | null;
}

export function ToldParts({ line, kept }: { line: string; kept: boolean }) {
  const [read, setRead] = useKept<Read>('parts', { parts: [], failed: null });
  const asked = useRef(0);
  const readIt = async () => {
    const mine = ++asked.current;
    try {
      const found = await call<{ parts: unknown[] }>('GET', `/inbox/${at(line)}/parts`);
      if (mine === asked.current) setRead({ parts: partsOf(found), failed: null });
    } catch (error) {
      if (mine === asked.current) setRead((was) => ({ ...was, failed: error instanceof Error ? error.message : String(error) }));
    }
  };
  // A line told with none has nothing to read.
  useEffect(() => void (kept && readIt()), [line, kept]);
  const [chosen, choose] = useKept<string | null>('told-tab', null);
  const projects = read.parts.filter((part) => part.kind === 'cards' || part.kind === 'change' || part.kind === 'rows').map((part) => part.project);
  const goes = read.parts.flatMap((part) => (part.kind === 'go' && part.go ? [part.go.ask] : []));
  useEvents(
    // A change asked for is read again as it is decided or its record
    // changes, and a run that waits as it is decided.
    ['board.item.*', 'collection.history', 'record.ask.*', 'record.updated', 'record.created', 'record.deleted', 'project.approval.*'],
    (event) => {
      if (projects.includes(String(event.data?.project ?? '')) || goes.includes(String(event.data?.ask ?? ''))) void readIt();
    },
    [line, projects.join(' '), goes.join(' ')],
  );
  // A run listed that ends is read again with the rest, and so is everything where the stream fell behind.
  const listed = read.parts.flatMap((part) => (part.kind === 'runs' ? (part.runs ?? []).filter((run) => run.finished === null).map((run) => run.id) : []));
  useEvents('run.finished', (event) => listed.includes(String(event.source?.run ?? '')) && void readIt(), [line, listed.join(' ')], () => void readIt());
  if (!kept) return null;
  if (read.failed) return <p class="told-parts-failed" role="alert">{read.failed}</p>;
  const tabs = tabsOf(read.parts);
  const shown = showing(tabs, chosen);
  const drawn = (at: number) => {
    const part = read.parts[at];
    if (!part) return null;
    return part.kind === 'links' ? (
      <Links key={at} links={part.links} />
    ) : part.kind === 'message' ? (
      <Said key={at} part={part} />
    ) : part.kind === 'change' ? (
      <AskedPart key={at} part={part} again={readIt} />
    ) : part.kind === 'go' ? (
      <GoAskedPart key={at} part={part} again={readIt} />
    ) : part.kind === 'rows' ? (
      <RowsView key={at} part={part} />
    ) : part.kind === 'runs' ? (
      <RunsView key={at} part={part} />
    ) : part.kind === 'measures' ? (
      <MeasuresView key={at} part={part} />
    ) : (
      <Cards key={at} line={line} number={at + 1} part={part} again={readIt} />
    );
  };
  return (
    <div class="told-parts">
      {tabs.length > 1 && (
        <div class="view-tabs tabs-ruled told-tabs">
          <div class="view-tabs-list" role="tablist" aria-label="What this was told with">
            {tabs.map((tab) => (
              <button key={tab.name} type="button" role="tab" class="view-tab told-tab" data-tab-name={tab.name} aria-selected={tab === shown} onClick={() => choose(tab.name)}>
                {tab.called}
              </button>
            ))}
          </div>
        </div>
      )}
      {shown && (
        <div class="told-tab-panel" role={tabs.length > 1 ? 'tabpanel' : undefined} aria-label={tabs.length > 1 ? shown.called : undefined}>
          {shown.parts.map(drawn)}
        </div>
      )}
    </div>
  );
}

/** A message of a channel, opened where it was said: in its thread where it is in one. */
function Said({ part }: { part: MessagePart }) {
  if (part.refused) return <p class="told-part faint">{part.refused}</p>;
  const open = () => {
    if (part.thread) showThread(part.channel, part.thread);
    openTab({ kind: 'channel', id: part.channel }, { message: part.message });
  };
  return (
    <section class="told-part told-message">
      <button type="button" onClick={open}>
        {part.thread ? 'Open the thread' : 'Open the message'}
      </button>
    </section>
  );
}

/** One address of a part: opened in a tab of the browser's own, but one of this factory's page that names a thing, which is opened here as a tab (a run's link). */
function PartLink({ link }: { link: string }) {
  const here = typeof location === 'undefined' ? null : thingOfLink(link, location);
  return here ? (
    <a
      class="inbox-link"
      href={link}
      onClick={(event) => {
        if (opens(link)) event.preventDefault();
      }}
    >
      {link}
    </a>
  ) : (
    <a class="inbox-link" href={link} target="_blank" rel="noopener noreferrer">
      {link}
    </a>
  );
}

function Links({ links }: { links: string[] }) {
  return (
    <section class="told-part told-links">
      <h2 class="inbox-page-label">Links</h2>
      <ul>
        {links.map((link) => (
          <li key={link}>
            <PartLink link={link} />
          </li>
        ))}
      </ul>
    </section>
  );
}

function Cards({ line, number, part, again }: { line: string; number: number; part: CardsPart; again: () => Promise<void> }) {
  const head = headOf(part);
  const [done, setDone] = useState<string | null>(null);
  return (
    <section class="told-part told-cards" data-part={number}>
      <h2 class="inbox-page-label">{head.title}</h2>
      <p class="told-goes faint">{head.goes}</p>
      {part.refused !== undefined ? (
        <p class="told-refused faint">{part.refused}</p>
      ) : part.cards?.length ? (
        <table class="told-table">
          <thead>
            <tr>
              <th scope="col">ID</th>
              <th scope="col">Title</th>
              <th scope="col">Assigned to</th>
              <th scope="col">Column</th>
              <th scope="col">Last said</th>
              <th scope="col" aria-label="What can be done" />
            </tr>
          </thead>
          {part.cards.map((card) => (
            <CardRow key={card.id} line={line} number={number} part={part} card={card} again={again} done={setDone} />
          ))}
        </table>
      ) : (
        <p class="faint">{noneListed(part)}</p>
      )}
      {done && (
        <p class="told-done" role="status">
          {done}
        </p>
      )}
    </section>
  );
}

function CardRow({
  line,
  number,
  part,
  card,
  again,
  done,
}: {
  line: string;
  number: number;
  part: CardsPart;
  card: Card;
  again: () => Promise<void>;
  done: (said: string) => void;
}) {
  const [open, setOpen] = useKept<boolean>(`thread:${card.id}`, false);
  const [failing, setFailing] = useState(false);
  const [reason, setReason] = useKept<string>(`why:${card.id}`, '');
  const [said, setSaid] = useKept<string>(`say:${card.id}`, '');
  const [refused, setRefused] = useState<string | null>(null);
  const offered = offeredFor(card);
  const last = lastSaid(card);
  const now = whereNow(part, card);
  const press = async (verdict: 'pass' | 'fail') => {
    setRefused(null);
    try {
      const answer = await call<{ did: { moved?: string; pressed?: string; said?: string | null } }>('POST', `/inbox/${at(line)}/parts/${number}/cards/${at(card.id)}`, {
        verdict,
        reason: verdict === 'fail' ? reason : '',
      });
      setReason('');
      setFailing(false);
      done(doneSaid(card.id, answer.did, verdict));
      await again();
    } catch (error) {
      setRefused(error instanceof Error ? error.message : String(error));
    }
  };
  const comment = async () => {
    setRefused(null);
    try {
      await call('POST', `/records/${at(card.id)}/comments`, { body: said });
      setSaid('');
      await again();
    } catch (error) {
      setRefused(error instanceof Error ? error.message : String(error));
    }
  };
  const closed = failClosed(part, reason);
  const more = (failing && offered.press) || refused || open;
  return (
    // A card's body: its row, and under it what is unfolded of it.
    <tbody class="told-card" data-card={card.id}>
      <tr class="told-card-row">
        <td class="told-card-id faint">{card.id}</td>
        <td class="told-card-title">
          {card.gone ? (
            <span class="told-card-words faint">is gone</span>
          ) : (
            // Whole where it is pointed at: the client's tip says the title, cut or not.
            <button type="button" class="link told-card-open" title={card.title} onClick={() => openToldCard(part.project, card.id)}>
              <span class="told-card-words">{card.title}</span>
            </button>
          )}
        </td>
        <td class="told-card-who" data-label="Assigned to">
          {card.gone ? null : card.assignee ? (
            <span class="told-card-whom">
              <Assignee assignee={card.assignee} brief />
            </span>
          ) : (
            <span class="faint">nobody</span>
          )}
        </td>
        <td class="told-card-column faint" data-label="Column">
          {card.status}
        </td>
        <td class="told-card-last" data-label="Last said">
          {card.gone ? null : last ? (
            // One line in the table, cut where it does not fit and said whole by the tip.
            <span class="told-card-line" title={`${last.who}: ${last.words}`}>
              <span class="told-card-said">
                <strong>{last.who}:</strong> {last.words}
              </span>
            </span>
          ) : (
            <span class="faint">nothing said</span>
          )}
        </td>
        <td class="told-card-acts">
          <span class="told-card-buttons">
            {now && <span class="told-card-now faint">{now}</span>}
            {!card.gone && (
              <button type="button" class="ghost" aria-expanded={open} onClick={() => setOpen(!open)}>
                {threadCalled(card)}
              </button>
            )}
            {offered.press && (
              <>
                <button type="button" onClick={() => void press('pass')}>
                  Pass
                </button>
                <button type="button" aria-expanded={failing} onClick={() => setFailing(!failing)}>
                  Fail
                </button>
              </>
            )}
          </span>
        </td>
      </tr>
      {more && (
        <tr class="told-card-more">
          <td colSpan={6}>
            {failing && offered.press && (
              <div class="told-fail">
                <label class="told-fail-label">
                  Why it fails
                  <textarea rows={2} value={reason} onInput={(event) => setReason(event.currentTarget.value)} />
                </label>
                <div class="told-fail-acts">
                  <button type="button" class="danger" aria-disabled={closed !== null} title={closed ?? undefined} onClick={() => closed === null && void press('fail')}>
                    Fail it
                  </button>
                  <button type="button" class="ghost" onClick={() => setFailing(false)}>
                    Cancel
                  </button>
                  {closed && <span class="told-fail-why faint">{closed}</span>}
                </div>
              </div>
            )}
            {refused && (
              <p class="told-refused" role="alert">
                {refused}
              </p>
            )}
            {open && (
              <div class="told-thread">
                {card.thread.length === 0 ? (
                  <p class="faint">{nothingSaid(part)}</p>
                ) : (
                  <ol>
                    {card.thread.map((one, at) => (
                      <li key={at} class="told-said">
                        <strong>{one.who || one.actor}</strong>
                        {one.topic === 'card.asked' && <span class="faint"> asked</span>}
                        <p>{one.words}</p>
                      </li>
                    ))}
                  </ol>
                )}
                {offered.press && (
                  <div class="told-say">
                    <textarea aria-label={`Say something on ${card.id}`} rows={2} value={said} onInput={(event) => setSaid(event.currentTarget.value)} />
                    <button type="button" aria-disabled={said.trim() === ''} onClick={() => said.trim() !== '' && void comment()}>
                      Comment
                    </button>
                  </div>
                )}
              </div>
            )}
          </td>
        </tr>
      )}
    </tbody>
  );
}
