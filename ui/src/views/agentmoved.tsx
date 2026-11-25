// The question asked before an agent is moved to another project, or to
// none (`agentmoved.ts`): which, chosen among the projects there are, and
// what that changes and what it leaves as it was. What is sent is the
// project and nothing else (`PATCH /api/agents/<name>`, `project`), and
// what the daemon refuses (work in hand, a fleet that counts it, what
// whoever asks does not hold) is said in its sentence, with the question
// staying. Moved, its line follows from what the daemon says of it
// (`agent.slot.updated`), here as on any other page.

import { useState } from 'preact/hooks';
import { call } from '../api.ts';
import type { Slot } from '../api.ts';
import { movedSaid, movingSaid } from '../agentmoved.ts';
import { isExternal } from '../external.ts';
import { projects, refreshAgents, toast } from '../store.ts';
import { Field, Modal } from '../ui.tsx';

/** Moves an agent to `project`, or to none, and changes nothing else of it. */
const moved = (name: string, project: string | null) => call<Slot>('PATCH', `/agents/${encodeURIComponent(name)}`, { project });

export function MoveAgent({ slot, onClose }: { slot: Slot; onClose: () => void }) {
  const [to, setTo] = useState(slot.project ?? '');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const called = (id: string) => projects.value.find((one) => one.id === id)?.name ?? id;
  const toName = to ? called(to) : null;
  const move = async (event: Event) => {
    event.preventDefault();
    if (busy) return;
    setBusy(true);
    setError('');
    try {
      await moved(slot.name, to || null);
      await refreshAgents();
      toast(movedSaid(slot.name, toName));
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setBusy(false);
    }
  };
  return (
    <Modal onClose={onClose}>
      <form class="stack agent-move" data-agent={slot.name} onSubmit={(event) => void move(event)}>
        <h2>Move {slot.name} to another project</h2>
        <p class="faint">It works in {slot.project ? called(slot.project) : 'no project'} now.</p>
        <Field label="Project">
          <select value={to} onChange={(event) => setTo(event.currentTarget.value)} aria-label="The project it is to work in">
            <option value="">No project</option>
            {projects.value.map((one) => (
              <option key={one.id} value={one.id}>
                {one.name}
              </option>
            ))}
          </select>
        </Field>
        {movingSaid({ external: isExternal(slot), active: slot.active }, toName).map((line) => (
          <p key={line} class="agent-move-said">
            {line}
          </p>
        ))}
        {error && <p class="error-text">{error}</p>}
        <div class="row modal-foot">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" class="primary" disabled={busy || to === (slot.project ?? '')}>
            Move it
          </button>
        </div>
      </form>
    </Modal>
  );
}
