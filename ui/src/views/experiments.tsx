// Settings, Experiments: what is being built of this program
// itself and may be turned on for this factory before it is whole, each
// naming the card it is for, off unless turned on, by whoever manages the
// factory. The list is the program's (`kernel::experiments::LIST`); with
// none in it, the part says so.

import { useEffect, useState } from 'preact/hooks';
import { call } from '../api.ts';
import { toast } from '../store.ts';
import { useEvents } from '../ui.tsx';

interface Experiment {
  name: string;
  card: string;
  says: string;
  on: boolean;
}

export function ExperimentsPanel({ admin }: { admin: boolean }) {
  const [list, setList] = useState<Experiment[] | null>(null);
  const read = () => void call<{ experiments: Experiment[] }>('GET', '/experiments').then((answer) => setList(answer.experiments), () => setList([]));
  useEffect(read, []);
  useEvents('system.experiment', read, []);
  const turn = async (one: Experiment, on: boolean) => {
    try {
      const answer = await call<{ experiments: Experiment[] }>('PUT', `/experiments/${encodeURIComponent(one.name)}`, { on });
      setList(answer.experiments);
    } catch (err) {
      toast(err instanceof Error ? err.message : String(err), true);
    }
  };
  return (
    <div class="panel experiments-panel" aria-label="Experiments">
      <div class="panel-head">
        <h2>Experiments</h2>
        <span class="muted grow">what is not yet whole, for this factory to try</span>
      </div>
      <div class="stack" style="padding:14px">
        {list === null ? (
          <p class="faint">Reading them…</p>
        ) : list.length === 0 ? (
          <p class="faint experiments-none">
            None is listed in this version. A feature of several pieces may land piece by piece behind one, off until it is turned on here, and
            is taken out of the list when it is whole.
          </p>
        ) : (
          list.map((one) => (
            <label key={one.name} class="row experiments-line" data-experiment={one.name}>
              <input type="checkbox" checked={one.on} disabled={!admin} onChange={(event) => void turn(one, event.currentTarget.checked)} />
              <span class="grow">
                <strong>{one.name}</strong> <span class="muted">{one.says}</span>
              </span>
              <code class="faint">{one.card}</code>
            </label>
          ))
        )}
        {list !== null && list.length > 0 && !admin && <p class="faint">Turned on and off by whoever manages the factory.</p>}
      </div>
    </div>
  );
}
