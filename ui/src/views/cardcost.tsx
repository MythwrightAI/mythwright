// What a card cost, in its drawer: a part of its own under its form and
// over its history. At its head what all of it came to, counted and
// never scored; then a line for each round (each time the card was
// worked and the work looked over) with what was decided at its end and
// a bar as long as the round's tokens against the largest round, drawn
// as an agent's history draws a turn's; and under each line its turns,
// one a line, and the words the decision was said with.
//
// It is the daemon's laying out (`GET /api/board/items/{id}/cost`) and
// nothing is worked out here but how it is drawn (`cardcost.ts`). It is
// read when the drawer opens and again when a line of the card's history
// is written or something is said of a turn of its, the last reading
// asked for being the one that is kept. Nothing is drawn where no agent
// has taken a turn on the card: a person's card costs nothing to say so.

import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { call } from '../api.ts';
import { short } from '../agenthistory.ts';
import { cameSaid, orderSaid, roundBars, spentSaid, touches, turnSaid } from '../cardcost.ts';
import type { CardCost as Laid } from '../cardcost.ts';
import { coalesce } from '../events.ts';
import { useEvents } from '../ui.tsx';
import { amount, duration } from '../util.ts';

/** How long the track of a round's bar is, in every line. */
const TRACK = 96;

export function CardCost({ id }: { id: string }) {
  const [laid, setLaid] = useState<Laid | null>(null);
  // Of two readings on their way, the one asked for last is kept.
  const asked = useRef(0);
  const read = useMemo(() => {
    const now = () => {
      const mine = (asked.current += 1);
      void call<Laid>('GET', `/board/items/${encodeURIComponent(id)}/cost`).then(
        (got) => mine === asked.current && setLaid(got),
        () => mine === asked.current && setLaid((before) => before),
      );
    };
    return { now, soon: coalesce(now, 200) };
  }, [id]);
  useEffect(() => read.now(), [read]);
  const known = useMemo(() => new Set((laid?.rounds ?? []).flatMap((round) => round.turns.map((turn) => turn.id))), [laid]);
  useEvents(['collection.history', 'agent.turn.*'], (event) => touches(id, event.topic, event.data, known) && read.soon(), [id, read, known]);

  if (!laid || laid.rounds.length === 0) return null;
  const all = laid.spent;
  const bars = roundBars(laid.rounds, TRACK);
  const figures: [string, string, string][] = [
    ['rounds', laid.rounds.length === 1 ? 'Round' : 'Rounds', String(laid.rounds.length)],
    ['turns', all.turns === 1 ? 'Turn' : 'Turns', String(all.turns)],
    ['sent', 'Sent', all.sent > 0 ? short(all.sent) : '–'],
    ['written', 'Written', all.wrote > 0 ? short(all.wrote) : '–'],
    // What those that said came to; where none said, that it was not reported, and never nought.
    ...(all.cost_usd > 0 || (all.cost_unsaid ?? 0) > 0 ? [['cost', 'Cost', all.cost_usd > 0 ? amount(all.cost_usd, 'usd') : 'not reported'] as [string, string, string]] : []),
    ['took', 'At work', all.took_ms > 0 ? duration(all.took_ms) : '–'],
  ];
  return (
    <section class="card-cost" aria-label="What it cost">
      <h3>What it cost</h3>
      <dl class="turns-figures">
        {figures.map(([what, called, figure]) => (
          <div key={what} data-figure={what}>
            <dd>{figure}</dd>
            <dt>{called}</dt>
          </div>
        ))}
      </dl>
      <p class="faint card-cost-key">
        {all.sent + all.wrote > 0 ? (
          <>
            <span class="turn-part sent" aria-hidden="true" /> sent <span class="turn-part written" aria-hidden="true" /> written. A bar is a round's tokens against the largest round.
          </>
        ) : (
          'No turn of this card says how many tokens it took.'
        )}
      </p>
      {orderSaid(laid.by_order) && <p class="faint card-cost-key">{orderSaid(laid.by_order)}</p>}
      {laid.rounds.map((round, at) => {
        const bar = bars[at];
        const tokens = round.spent.sent + round.spent.wrote;
        const words = round.decided.came === 'open' ? '' : (round.decided.said ?? '');
        return (
          <div key={round.n} class="cost-round" data-round={round.n} data-came={round.decided.came} data-tokens={tokens}>
            <div class="cost-round-line">
              <span class="cost-round-n">Round {round.n}</span>
              <span class="cost-round-came">{cameSaid(round.decided.came)}</span>
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
            </div>
            {words && <p class="cost-round-said">{words}</p>}
            <ul class="cost-turns">
              {round.turns.map((turn) => (
                <li key={turn.id} data-does={turn.does}>
                  {turnSaid(turn)}
                </li>
              ))}
            </ul>
            <p class="faint cost-round-spent">{spentSaid(round.spent)}</p>
          </div>
        );
      })}
    </section>
  );
}
