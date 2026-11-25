// The question asked before an agent is removed for good: who it is,
// what goes with it and what stays, and why not where it cannot be.
//
// It is asked of the daemon first, which changes nothing by being asked
// (`asked`), so what is said is what is so: the channels it is in,
// the work it was asked, the cards given to it. Where the daemon says it
// cannot be taken away as things stand (it has work in hand; something
// that is running counts it) that is said in its place, with nothing to
// press but the way out. Taken away, its line goes from every list when
// the daemon says so (`agent.slot.removed`), here as on any other page.

import { useEffect, useState } from 'preact/hooks';
import { call } from '../api.ts';
import { toast } from '../store.ts';
import { type Removal, goes, stays } from '../takenaway.ts';
import { Modal, Prose } from '../ui.tsx';

/** What taking an agent away would come to, and why it cannot be as things stand: nothing is changed by asking. */
const asked = (name: string) => call<Removal>('GET', `/agents/${encodeURIComponent(name)}/remove`);
/** Takes an agent away for good. Stopping one keeps it; this does not. */
const taken = (name: string) => call<Removal>('POST', `/agents/${encodeURIComponent(name)}/remove`);

export function TakeAway({ name, onClose }: { name: string; onClose: () => void }) {
  const [found, setFound] = useState<Removal | null>(null);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let left = false;
    asked(name).then(
      (answer) => !left && setFound(answer),
      (err) => !left && setError(err instanceof Error ? err.message : String(err)),
    );
    return () => {
      left = true;
    };
  }, [name]);

  const take = async () => {
    if (busy) return;
    setBusy(true);
    try {
      const gone = await taken(name);
      toast(gone.folder_left ? `${name} is removed. ${gone.folder_left}` : `${name} is removed.`);
      onClose();
    } catch (err) {
      // What was in the way may have come about since it was asked: said where it was asked.
      setError(err instanceof Error ? err.message : String(err));
      setBusy(false);
    }
  };

  const refused = found?.refused ?? null;
  return (
    <Modal onClose={onClose}>
      <div class="stack take-away" data-agent={name}>
        <h2>Remove {name}?</h2>
        {!found && !error && <p class="faint">Reading what goes with it</p>}
        {refused && (
          <>
            <p>It cannot be removed as things stand.</p>
            {/* The daemon's own sentence, which says what to do, with what it names drawn as what is typed. */}
            <p class="take-away-why">
              <Prose text={refused} />
            </p>
          </>
        )}
        {found && !refused && (
          <>
            <p>This cannot be put back. Stopping it would keep it; this does not. With it go:</p>
            <ul class="take-away-goes">
              {goes(found).map((line) => (
                <li key={line}>{line}</li>
              ))}
            </ul>
            <p>What stays, under its name:</p>
            <ul class="take-away-stays">
              {stays(found).map((line) => (
                <li key={line}>{line}</li>
              ))}
            </ul>
            <p class="faint">An agent that is later called {name} is another agent, and has none of this for its own.</p>
          </>
        )}
        {error && <p class="error-text">{error}</p>}
        <div class="row modal-foot">
          <button type="button" onClick={onClose}>
            {found && !refused ? 'Cancel' : 'Close'}
          </button>
          {found && !refused && (
            <button class="danger" disabled={busy} onClick={() => void take()}>
              Remove it
            </button>
          )}
        </div>
      </div>
    </Modal>
  );
}
