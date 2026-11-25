// A key revoked, and a key rolled, from its line in Settings.
//
// Revoking asks first (`revoking`, which changes nothing): the keys made
// with it, which go with it, and what they applied that is running, which
// is stopped with it unless it is left; then says it in a question, as
// other removals are, with a switch to leave what it applied running.
// Rolling gives the key another secret, shown once, and the one it had is
// refused from then: the key is the same, so what it applied goes on.

import { useEffect, useState } from 'preact/hooks';
import type { Credential } from '../api.ts';
import { type Revoking, goesWith, revoke, revokedSaid, revoking, roll } from '../keyrevoked.ts';
import { toast } from '../store.ts';
import { Field, Modal } from '../ui.tsx';

/** The two buttons on a key's line, with the question each asks. */
export function KeyActs({ credential, onChanged }: { credential: Credential; onChanged: () => void }) {
  const [asking, setAsking] = useState<'revoke' | 'roll' | null>(null);
  return (
    <span class="key-acts">
      <button onClick={() => setAsking('roll')}>Roll…</button>
      <button class="danger" onClick={() => setAsking('revoke')}>
        Revoke…
      </button>
      {asking === 'revoke' && <Revoke credential={credential} onClose={() => setAsking(null)} onGone={onChanged} />}
      {asking === 'roll' && <Roll credential={credential} onClose={() => setAsking(null)} />}
    </span>
  );
}

function Revoke(props: { credential: Credential; onClose: () => void; onGone: () => void }) {
  const { credential } = props;
  const [found, setFound] = useState<Revoking | null>(null);
  const [leave, setLeave] = useState(false);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let left = false;
    revoking(credential.id).then(
      (answer) => !left && setFound(answer),
      (err) => !left && setError(err instanceof Error ? err.message : String(err)),
    );
    return () => {
      left = true;
    };
  }, [credential.id]);

  const take = async () => {
    if (busy) return;
    setBusy(true);
    try {
      toast(revokedSaid(await revoke(credential.id, leave)));
      props.onGone();
      props.onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setBusy(false);
    }
  };

  const goes = found ? goesWith(found, leave) : [];
  return (
    <Modal onClose={props.onClose}>
      <div class="stack key-revoke" data-key={credential.id}>
        <h2>Revoke {credential.name}?</h2>
        <p>From now it signs nobody in. This cannot be put back: a key for a script that is to go on is rolled instead.</p>
        {!found && !error && <p class="faint">Reading what goes with it</p>}
        {found && goes.length === 0 && <p class="muted">Nothing goes with it: no key was made with it, and nothing it applied is running.</p>}
        {found && goes.length > 0 && (
          <>
            <p>With it:</p>
            <ul class="key-revoke-goes">
              {goes.map((line) => (
                <li key={line}>{line}</li>
              ))}
            </ul>
          </>
        )}
        {found && found.running.length > 0 && (
          <label class="row key-revoke-leave">
            <input type="checkbox" checked={leave} onChange={(event) => setLeave(event.currentTarget.checked)} />
            <span>Leave what it applied running, with what the key held</span>
          </label>
        )}
        {found && <p class="faint">What it applied stays done where it is stopped: a card where it is, what was merged and what was said.</p>}
        {error && <p class="error-text">{error}</p>}
        <div class="row modal-foot">
          <button type="button" onClick={props.onClose}>
            Cancel
          </button>
          <button class="danger" disabled={busy || !found} onClick={() => void take()}>
            Revoke it
          </button>
        </div>
      </div>
    </Modal>
  );
}

function Roll(props: { credential: Credential; onClose: () => void }) {
  const { credential } = props;
  const [token, setToken] = useState('');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const rolled = async () => {
    if (busy) return;
    setBusy(true);
    try {
      setToken((await roll(credential.id)).token);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
    setBusy(false);
  };
  return (
    <Modal onClose={props.onClose}>
      <div class="stack key-roll" data-key={credential.id}>
        <h2>{token ? `${credential.name} is rolled` : `Roll ${credential.name}?`}</h2>
        {!token && (
          <p>
            It is given another secret, and the one it has is refused from then: whatever uses it is to be given the
            new one. It is the same key, so what it applied goes on.
          </p>
        )}
        {token && (
          <Field label="Copy this now: it is shown once and only its hash is kept">
            <code class="code-box" onClick={() => void navigator.clipboard?.writeText(token).then(() => toast('Copied.'))}>
              {token}
            </code>
          </Field>
        )}
        {error && <p class="error-text">{error}</p>}
        <div class="row modal-foot">
          <button type="button" onClick={props.onClose}>
            {token ? 'Done' : 'Cancel'}
          </button>
          {!token && (
            <button class="primary" disabled={busy} onClick={() => void rolled()}>
              Roll it
            </button>
          )}
        </div>
      </div>
    </Modal>
  );
}
