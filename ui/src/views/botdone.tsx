// What the factory's bot did lately, where it is read about (its page
// under Channel members): the latest things the daemon hands, newest
// first, under a heading for each day as an agent's history has, each in
// the words the records' histories use with the blueprint after it, and
// pressed to open what it was done to or said in.
//
// Read once as the page is drawn, and again (once for many) when a line
// of a history is written or something is said in a channel: the line
// carries no words of what the reader may not read, so it is read again
// from the daemon and never added to.

import { useEffect, useMemo, useState } from 'preact/hooks';
import { byDay } from '../agenthistory.ts';
import { type Done, doneLately, doneWords, LATEST, type Lately } from '../botdone.ts';
import { coalesce } from '../events.ts';
import { hashed } from '../kept.ts';
import { channels, openTab } from '../store.ts';
import { useEvents } from '../ui.tsx';
import { clock } from '../util.ts';
import { showThing } from './inboxpage.tsx';

/** Opens what a line is about: a record, by its id as a told line's is; or the channel, at what was said. */
function open(line: Done): void {
  if (line.kind === 'record' && line.record) void showThing({ kind: 'record', id: line.record });
  else if (line.channel) openTab({ kind: 'channel', id: line.channel }, line.message ? { message: line.message } : undefined);
}

export function BotDone() {
  const [read, setRead] = useState<Lately | null | 'failed'>(null);
  const reload = useMemo(() => coalesce(() => void doneLately().then(setRead, () => setRead((was) => (was && was !== 'failed' ? was : 'failed'))), 300), []);
  useEffect(() => reload(), [reload]);
  useEvents(['collection.history', 'channel.message'], (event) => {
    if (event.topic === 'collection.history' || event.data?.author_kind === 'bot') reload();
  });
  if (read === null) return <p class="faint">Reading</p>;
  if (read === 'failed') return <p class="faint">What it did could not be read.</p>;
  if (read.done.length === 0) return <p class="faint">Nothing that you may read yet.</p>;
  const called = (id: string) => {
    const channel = channels.value.find((one) => one.id === id);
    return channel ? hashed(channel) : 'a channel';
  };
  const lines = read.done.map((line) => ({ ...line, created: line.ts }));
  return (
    <div class="bot-done">
      {byDay(lines).map((day) => (
        <section key={day.day} class="turns-day" aria-label={day.day}>
          <h3>{day.day}</h3>
          {day.turns.map((line, at) => (
            <button key={`${line.kind}:${line.record ?? line.message}:${at}`} class="context-row bot-done-line" title={line.record ?? line.message} onClick={() => open(line)}>
              <span class="faint">{clock(line.ts)}</span>
              <span class="bot-done-words">
                {doneWords(line, read.name, called)}
                {line.via && <span class="faint"> · via {line.via}</span>}
              </span>
            </button>
          ))}
        </section>
      ))}
      {read.done.length >= LATEST && <p class="faint">The latest {LATEST}.</p>}
    </div>
  );
}
