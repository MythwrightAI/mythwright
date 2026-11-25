// A run that waits for a person to let it go on (`approval.ask`,
// `told::goes`): drawn as the part of a told line that names it (`kind:
// go`), on the line's own page, and on the run's page, where the waiting
// stage has Approve and Decline for whoever may (`RunApproval`, which a
// run's page places, handed the run). Both are the same route: what a
// press does is the daemon's, and what it refuses is said in its sentence.
// Decline asks why, which may be left unsaid and is what the step fails
// with. What is worked out here is `goasked.ts`, with its cases.

import { useEffect, useRef, useState } from 'preact/hooks';
import { call } from '../api.ts';
import { goDecidedSaid, goHead, goOffered, goStanding, runAsks } from '../goasked.ts';
import type { Go, GoPart } from '../goasked.ts';
import { useEvents } from '../ui.tsx';
import { useKept } from './returned.tsx';

const at = (id: string) => encodeURIComponent(id);

/** The part of a told line that names a run that waits. */
export function GoAskedPart({ part, again }: { part: GoPart; again: () => Promise<void> }) {
  const go = part.go;
  if (!go) return <p class="told-part told-refused faint">{part.refused ?? 'This run waits for nothing any more.'}</p>;
  return <GoDecide go={go} again={again} />;
}

/**
 * What a run's page draws of its asks to be let go on, read as whoever
 * reads may read them (`GET /api/asked?run=`) and again as one is asked
 * or decided: nothing where it never waited.
 */
export function RunApproval({ run }: { run: string }) {
  const [asks, setAsks] = useKept<Go[]>(`go:${run}`, []);
  const asked = useRef(0);
  const readIt = async () => {
    const mine = ++asked.current;
    try {
      const read = await call<{ asked: unknown }>('GET', `/asked?run=${at(run)}`);
      if (mine === asked.current) setAsks(runAsks(read));
    } catch {
      // Somebody who may not read blueprints is shown nothing of it.
      if (mine === asked.current) setAsks([]);
    }
  };
  useEffect(() => void readIt(), [run]);
  useEvents(['project.approval.*'], (event) => {
    if (event.data?.run === run) void readIt();
  }, [run]);
  if (asks.length === 0) return null;
  return (
    <div class="run-approval">
      {asks.map((go) => (
        <GoDecide key={go.ask} go={go} again={readIt} />
      ))}
    </div>
  );
}

function GoDecide({ go, again }: { go: Go; again: () => Promise<void> }) {
  const [declining, setDeclining] = useState(false);
  const [why, setWhy] = useKept<string>(`decline:${go.ask}`, '');
  const [refused, setRefused] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);
  const offer = goOffered(go);
  const decide = async (how: 'approve' | 'decline') => {
    setRefused(null);
    try {
      const answer = await call<Go>('POST', `/asked/${at(go.ask)}/${how}`, how === 'decline' ? { why } : {});
      setWhy('');
      setDeclining(false);
      setDone(goDecidedSaid(answer));
      await again();
    } catch (error) {
      setRefused(error instanceof Error ? error.message : String(error));
    }
  };
  return (
    <section class="told-part told-go" data-ask={go.ask} data-status={go.status}>
      <h2 class="inbox-page-label">{goHead(go)}</h2>
      {go.says.trim() !== '' && <p class="told-go-says">{go.says}</p>}
      {go.links.length > 0 && (
        <ul class="told-go-links">
          {go.links.map((link) => (
            <li key={link}>
              <a class="inbox-link" href={link} target="_blank" rel="noopener noreferrer">
                {link}
              </a>
            </li>
          ))}
        </ul>
      )}
      <p class="told-go-standing faint" role="status">
        {goStanding(go, Date.now())}
      </p>
      {offer.decide ? (
        <div class="told-go-acts">
          <button type="button" onClick={() => void decide('approve')}>
            Approve
          </button>
          <button type="button" class="ghost" aria-expanded={declining} onClick={() => setDeclining(!declining)}>
            Decline
          </button>
        </div>
      ) : (
        offer.why && <p class="told-go-whynot faint">{offer.why}</p>
      )}
      {declining && offer.decide && (
        <div class="told-fail">
          <label class="told-fail-label">
            Why not (the run fails saying it)
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
