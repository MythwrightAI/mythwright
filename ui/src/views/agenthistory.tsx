// An agent's history, in the drawer: the times it was asked.
//
// At its head what the list comes to, counted and never scored: how
// often it was asked and how many of those failed, the tokens sent to
// its program and written by it, and what that cost where anything did.
// Then a line for each time, by day: how it ended, where it was asked,
// when and how long it took, and at the right a bar as long as that
// turn's tokens against the largest turn listed, with the figure beside
// it. Pointed at, or reached by the keyboard, a line says its figures
// exactly; pressed, it opens under itself into what was asked and what
// was answered.
//
// The daemon hands an agent's latest turns and no more (`LATEST`), and a
// list that long says so. One asked in a channel kept to its members is
// handed to whoever does not read that channel with its figures and none
// of its words: it is a line like any other, and opens to nothing.
//
// What is worked out is in `agenthistory.ts`; this draws it. The list is read
// when the drawer opens and again when something is said of a turn of
// this agent's, the last reading asked for being the one that is kept.

import { useEffect, useId, useMemo, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import { coalesce } from '../events.ts';
import { KEPT, LATEST, byDay, endedAs, exactly, lengths, short, totals, whereAsked } from '../agenthistory.ts';
import type { Bar, Turn } from '../agenthistory.ts';
import { agents, channels, drawer } from '../store.ts';
import { isExternal } from '../external.ts';
import { ExternalHistory } from './external.tsx';
import { calledOf } from '../kept.ts';
import { Empty, Prose, useEvents } from '../ui.tsx';
import { amount, classes, clock, duration, when } from '../util.ts';

/** How long the track of a turn's bar is, in every line. */
const TRACK = 96;

export function showHistory(slot: string): void {
  // Each agent's is drawn afresh: a drawer that is open on one and is handed another must not say the second's name over the first's turns.
  // One the factory does not start has no turns: what it did is said from the journal.
  const outside = isExternal(agents.value.find((one) => one.name === slot));
  drawer.value = { title: `${slot}: history`, body: () => (outside ? <ExternalHistory key={slot} slot={slot} /> : <History key={slot} slot={slot} />) };
}

function History({ slot }: { slot: string }) {
  const [turns, setTurns] = useState<Turn[] | null>(null);
  // The one line that is open under itself, for as long as the drawer is.
  const [open, setOpen] = useState<string | null>(null);
  // Of two readings on their way, the one asked for last is kept.
  const asked = useRef(0);
  const read = useMemo(() => {
    const now = () => {
      const mine = (asked.current += 1);
      void api.turns(slot).then(
        (got) => mine === asked.current && setTurns(got),
        () => mine === asked.current && setTurns((before) => before ?? []),
      );
    };
    return { now, soon: coalesce(now, 200) };
  }, [slot]);
  useEffect(() => read.now(), [read]);
  useEvents(['agent.turn.*'], (event) => event.data?.slot === slot && read.soon(), [slot, read]);

  if (!turns) return <div class="empty">Reading</div>;
  if (turns.length === 0) return <Empty title="This agent has not been asked anything" />;
  const all = totals(turns);
  const bars = lengths(turns, TRACK);
  const bar = new Map(turns.map((turn, at) => [turn.id, bars[at] ?? null]));
  const most = Math.max(...turns.map((turn) => turn.input_tokens + turn.output_tokens));
  const channelName = (id: string) => {
    const found = channels.value.find((one) => one.id === id);
    return found ? calledOf(found) : null;
  };
  const figures: [string, string, string][] = [
    ['asked', 'Asked', String(all.asked)],
    ['failed', 'Failed', String(all.failed)],
    ...(all.calledOff > 0 ? [['called-off', 'Called off', String(all.calledOff)] as [string, string, string]] : []),
    ...(all.inHand > 0 ? [['in-hand', 'In hand', String(all.inHand)] as [string, string, string]] : []),
    ['sent', 'Sent to it', all.sent > 0 ? short(all.sent) : '–'],
    ['written', 'Written by it', all.written > 0 ? short(all.written) : '–'],
    // What those that said cost; where none said, that it was not reported, and never nought.
    ...(all.cost > 0 || all.costUnsaid > 0 ? [['cost', 'Cost', all.cost > 0 ? amount(all.cost, 'usd') : 'not reported'] as [string, string, string]] : []),
  ];
  return (
    <div class="turns">
      <div class="turns-head">
        <p class="faint turns-of">{turns.length >= LATEST ? `The latest ${LATEST} times it was asked. Earlier ones are not listed here.` : 'Every time it was asked.'}</p>
        <dl class="turns-figures">
          {figures.map(([what, called, figure]) => (
            <div key={what} data-figure={what} title={what === 'sent' || what === 'written' ? `${(what === 'sent' ? all.sent : all.written).toLocaleString('en-US')} tokens` : undefined}>
              <dd>{figure}</dd>
              <dt>{called}</dt>
            </div>
          ))}
        </dl>
        {most > 0 ? (
          <p class="faint turns-key">
            <span class="turn-part sent" aria-hidden="true" /> sent to it <span class="turn-part written" aria-hidden="true" /> written by it. A bar is a turn's tokens against the largest here, {short(most)}.
          </p>
        ) : (
          <p class="faint turns-key">No turn here says how many tokens it took.</p>
        )}
      </div>
      {byDay(turns).map((day) => (
        <section key={day.day} class="turns-day" aria-label={day.day}>
          <h3>{day.day}</h3>
          {day.turns.map((turn) => (
            <Line key={turn.id} turn={turn} bar={bar.get(turn.id) ?? null} where={whereAsked(turn, channelName)} open={open === turn.id} onToggle={() => setOpen(open === turn.id ? null : turn.id)} />
          ))}
        </section>
      ))}
    </div>
  );
}

const EXACT_WIDTH = 210;
/** Roughly how tall the card of a turn's figures is, to keep it in the window. */
const EXACT_HEIGHT = 86;
/** The room between the card and the drawer it is beside, and how long it stays once the pointer has left its line: long enough to cross to it. */
const GAP = 6;
const LINGER_MS = 180;

/** Takes away the card that is showing, if one is: there is only ever one. */
let putAway: (() => void) | null = null;

/** One time it was asked: a line, the card of its figures over the page while it is pointed at, and what was said under it while it is open. */
function Line({ turn, bar, where, open, onToggle }: { turn: Turn; bar: Bar | null; where: string; open: boolean; onToggle: () => void }) {
  const id = useId();
  const top = useRef<HTMLDivElement>(null);
  const [card, setCard] = useState<{ left: number; top?: number; bottom?: number } | null>(null);
  const going = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const how = endedAs(turn);
  const tokens = turn.input_tokens + turn.output_tokens;
  const took = turn.started && turn.finished ? duration(turn.finished - turn.started) : null;
  // One whose words are kept from whoever looks has nothing to open into.
  const kept = turn.kept_from === true;
  const hide = useRef(() => {
    clearTimeout(going.current);
    setCard(null);
  }).current;
  const show = () => {
    clearTimeout(going.current);
    const at = top.current?.getBoundingClientRect();
    if (!at) return;
    // One that is still on its way out goes at once when another is shown.
    if (putAway && putAway !== hide) putAway();
    putAway = hide;
    // Beside the drawer, level with its line, where the page has the room: the lines under this one stay clear for the pointer on its way down them.
    const frame = top.current?.closest('.drawer')?.getBoundingClientRect();
    if (frame && frame.left - GAP - EXACT_WIDTH >= 8) return setCard({ left: frame.left - GAP - EXACT_WIDTH, top: Math.max(8, Math.min(at.top, window.innerHeight - EXACT_HEIGHT - 8)) });
    // Else under the line and touching it, or over it where there is no room under.
    const left = Math.max(8, Math.min(at.right - EXACT_WIDTH, window.innerWidth - EXACT_WIDTH - 8));
    setCard(at.bottom + EXACT_HEIGHT > window.innerHeight ? { left, bottom: window.innerHeight - at.top - 1 } : { left, top: at.bottom - 1 });
  };
  // There is a gap between the line and a card beside the drawer: it goes a moment after the pointer leaves, and coming on to it within that moment keeps it.
  const leave = () => {
    clearTimeout(going.current);
    going.current = setTimeout(hide, LINGER_MS);
  };
  useEffect(
    () => () => {
      clearTimeout(going.current);
      if (putAway === hide) putAway = null;
    },
    [],
  );
  // What a line opens into says its figures itself.
  const shown = card !== null && !open;
  useEffect(() => {
    if (!shown) return;
    // What the line is in scrolled: the card is where the line was, and is put away.
    window.addEventListener('scroll', hide, true);
    return () => window.removeEventListener('scroll', hide, true);
  }, [shown]);
  const said = (
    <>
      <span class="turn-dot" data-ended={how} role="img" aria-label={how} title={how} />
      <span class="turn-said">
        <span class={classes('turn-where', where === KEPT && 'muted')}>{where}</span>
        <span class="turn-when" title={when(turn.created)}>
          {clock(turn.created)}
          {took ? ` · ${took}` : ''}
          {how !== 'done' ? ` · ${how}` : ''}
        </span>
      </span>
      <span class="turn-track" style={{ width: `${TRACK}px` }} aria-hidden="true">
        {bar && (
          <span class="turn-bar" style={{ width: `${bar.whole}px` }}>
            {bar.sent > 0 && <span class="turn-part sent" style={{ width: `${bar.sent}px` }} />}
            {bar.written > 0 && <span class="turn-part written" style={{ width: `${bar.written}px` }} />}
          </span>
        )}
      </span>
      <span class="turn-figure" aria-label={tokens > 0 ? `${tokens.toLocaleString('en-US')} tokens` : 'no tokens said'}>
        {tokens > 0 ? short(tokens) : '–'}
      </span>
    </>
  );
  // A mouse points before it presses, and the keyboard arrives; a finger does neither, and is shown the figures where the line is opened.
  const pointed = {
    onPointerEnter: (event: PointerEvent) => event.pointerType === 'mouse' && show(),
    onPointerLeave: (event: PointerEvent) => event.pointerType === 'mouse' && leave(),
  };
  return (
    <div class={classes('turn', open && 'open')} data-turn={turn.id} data-ended={how} data-tokens={tokens}>
      <div class="turn-top" ref={top} {...pointed}>
        {kept ? (
          <div class="turn-line" tabIndex={0} aria-describedby={shown ? id : undefined} onFocus={show} onBlur={hide}>
            {said}
          </div>
        ) : (
          <button
            type="button"
            class="turn-line"
            aria-expanded={open}
            aria-controls={open ? `${id}-whole` : undefined}
            aria-describedby={shown ? id : undefined}
            onClick={onToggle}
            onFocus={(event) => event.currentTarget.matches(':focus-visible') && show()}
            onBlur={hide}
          >
            {said}
          </button>
        )}
        {shown && card && (
          <div class="turn-exact" id={id} role="tooltip" style={{ width: `${EXACT_WIDTH}px`, left: `${card.left}px`, top: card.top === undefined ? undefined : `${card.top}px`, bottom: card.bottom === undefined ? undefined : `${card.bottom}px` }}>
            <dl>
              {exactly(turn).map(([what, figure]) => (
                <>
                  <dt>{what}</dt>
                  <dd>{figure}</dd>
                </>
              ))}
            </dl>
          </div>
        )}
      </div>
      {open && !kept && <Whole turn={turn} how={how} id={`${id}-whole`} />}
    </div>
  );
}

/**
 * What a line opens into. Where the daemon hands the words that asked
 * for the turn apart (`asked`: what the message it answers said), those
 * are first, as words; then what it answered or why it failed, and what
 * it chose; and the whole of what its agent was sent, which has the
 * words at its end after where it is and what was said before, is
 * folded away behind one line for whoever wants it. Where none are
 * handed (a piece of work a step asked for, whose words are the whole;
 * a daemon from before) what it was sent is shown as it always was.
 * Nothing here takes the words out of what was sent.
 */
function Whole({ turn, how, id }: { turn: Turn; how: string; id: string }) {
  // Whether the whole of what it was sent is unfolded, for as long as the line is open.
  const [sent, setSent] = useState(false);
  const apart = Boolean(turn.asked);
  return (
    <div class="turn-whole" id={id}>
      {apart ? (
        <div class="turn-words">
          <Prose text={turn.asked!} />
        </div>
      ) : (
        <div class="code-box turn-asked">{turn.prompt}</div>
      )}
      {turn.error ? (
        <p class={classes('turn-answer', how === 'called off' ? 'muted' : 'error-text')}>{turn.error}</p>
      ) : turn.reply.trim() ? (
        <div class="turn-answer">
          <Prose text={turn.reply} />
        </div>
      ) : (
        <p class="faint">No answer yet.</p>
      )}
      {turn.decision && <p class="muted">Chose: {turn.decision}</p>}
      {apart && (
        <>
          <button type="button" class="ghost turn-sent" aria-expanded={sent} aria-controls={sent ? `${id}-sent` : undefined} onClick={() => setSent(!sent)}>
            {/* The mark that says whether it is unfolded is drawn, and is no part of what the line is called. */}
            <span class="turn-sent-mark" aria-hidden="true" />
            The whole of what it was sent
          </button>
          {sent && (
            <div class="code-box turn-asked" id={`${id}-sent`}>
              {turn.prompt}
            </div>
          )}
        </>
      )}
      <p class="faint turn-figures">
        {exactly(turn)
          .map(([what, figure]) => `${what} ${figure}`)
          .join(' · ')}
      </p>
    </div>
  );
}
