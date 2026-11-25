// The part of a told line that names a change somebody asked for and may
// not make themselves (`kind: change`), on the line's own page: whose
// change of which record, why they asked, a row a cell with what it held,
// what it holds now where that is otherwise and what it is to hold, each
// scope it takes and whether the reader holds it; how it stands; and
// Approve and Decline where the daemon says the reader may. Decline asks
// why, which may be left unsaid. What a press does is the daemon's, and
// what it refuses is said in its sentence; what is worked out here is
// `asked.ts`, with its cases.

import { useState } from 'preact/hooks';
import { call } from '../api.ts';
import { decidedSaid, headOf, offered, rowsOf, standing, wholeSaid } from '../asked.ts';
import type { Change, ChangePart } from '../asked.ts';
import { useKept } from './returned.tsx';

const at = (id: string) => encodeURIComponent(id);

export function AskedPart({ part, again }: { part: ChangePart; again: () => Promise<void> }) {
  const change = part.change;
  if (!change) return <p class="told-part told-refused faint">{part.refused ?? 'This change is not there any more.'}</p>;
  return <Asked change={change} again={again} />;
}

function Asked({ change, again }: { change: Change; again: () => Promise<void> }) {
  const [declining, setDeclining] = useState(false);
  const [why, setWhy] = useKept<string>(`decline:${change.ask}`, '');
  const [refused, setRefused] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);
  const offer = offered(change);
  const decide = async (how: 'approve' | 'decline') => {
    setRefused(null);
    try {
      const answer = await call<Change>('POST', `/asked/${at(change.ask)}/${how}`, how === 'decline' ? { why } : {});
      setWhy('');
      setDeclining(false);
      setDone(decidedSaid(answer));
      await again();
    } catch (error) {
      setRefused(error instanceof Error ? error.message : String(error));
    }
  };
  return (
    <section class="told-part told-change" data-ask={change.ask}>
      <h2 class="inbox-page-label">{headOf(change)}</h2>
      {change.why.trim() !== '' && <p class="told-change-why">“{change.why}”</p>}
      <table class="told-change-cells">
        <thead>
          <tr>
            <th scope="col">Cell</th>
            <th scope="col">Was</th>
            <th scope="col">To be</th>
            <th scope="col">Takes</th>
          </tr>
        </thead>
        <tbody>
          {rowsOf(change).map((row) => (
            <tr key={row.name} class="told-change-cell" data-cell={row.name}>
              <th scope="row" class="told-change-name">
                {row.name}
              </th>
              <td class="told-change-was" data-label="Was">
                {row.was}
                {row.now !== null && <span class="told-change-now faint"> (now {row.now})</span>}
              </td>
              <td class="told-change-to" data-label="To be">
                {row.to}
              </td>
              <td class="told-change-takes" data-label="Takes">
                {row.takes.length === 0 ? (
                  <span class="faint">nothing more</span>
                ) : (
                  row.takes.map((one) => (
                    <span key={one.scope} class={one.held ? 'told-change-scope' : 'told-change-scope lacking'} title={one.held ? 'you hold it' : 'you do not hold it'}>
                      {one.scope}
                      {!one.held && <span class="faint"> (not yours)</span>}
                    </span>
                  ))
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {wholeSaid(change) && <p class="told-change-whole faint">{wholeSaid(change)}</p>}
      <p class="told-change-standing faint" role="status">
        {standing(change)}
      </p>
      {offer.decide ? (
        <div class="told-change-acts">
          <button type="button" onClick={() => void decide('approve')}>
            Approve
          </button>
          <button type="button" class="ghost" aria-expanded={declining} onClick={() => setDeclining(!declining)}>
            Decline
          </button>
        </div>
      ) : (
        offer.why && <p class="told-change-whynot faint">{offer.why}</p>
      )}
      {declining && offer.decide && (
        <div class="told-fail">
          <label class="told-fail-label">
            Why not (they are told)
            <textarea rows={2} value={why} onInput={(event) => setWhy(event.currentTarget.value)} />
          </label>
          <div class="told-fail-acts">
            <button type="button" class="danger" onClick={() => void decide('decline')}>
              Decline it
            </button>
            <button type="button" class="ghost" onClick={() => setDeclining(false)}>
              Cancel
            </button>
          </div>
        </div>
      )}
      {refused && (
        <p class="told-refused" role="alert">
          {refused}
        </p>
      )}
      {done && (
        <p class="told-done" role="status">
          {done}
        </p>
      )}
    </section>
  );
}
