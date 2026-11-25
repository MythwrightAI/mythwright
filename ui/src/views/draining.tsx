// Says on every page that the daemon is draining to stop, from the moment
// it begins, with what it waits for, and then that it stopped and how
// (`ui/src/drained.ts`): what it said of itself as the page was read, and
// then what `system.draining` and `system.stopping` say, with nothing read
// again but what it waits for, which is read when it begins and each time
// a turn or a run ends.

import { signal } from '@preact/signals';
import { useEffect } from 'preact/hooks';
import { call } from '../api.ts';
import { worn } from '../clock.ts';
import { drainSaid, drainingOf, stoppedSaid, waitsSaid } from '../drained.ts';
import { stream } from '../events.ts';
import { me, meta } from '../store.ts';

/** What it waits for, as last read; nothing until read, or where it may not be read. */
const waits = signal<string[] | null>(null);
/** What was said as a daemon that drained stopped. */
const stopped = signal<string[] | null>(null);

/** What a drain waits for, read again: one reading at a time, the last asked made. */
let reading = 0;
async function readWaits(): Promise<void> {
  if (!drainingOf(meta.value?.draining) || stopped.value) return;
  const asked = ++reading;
  const read = await call<unknown>('GET', '/system/drain').catch(() => null);
  if (asked === reading) waits.value = read === null ? null : waitsSaid(read);
}

stream.on(['system.draining'], (event) => {
  if (meta.value) meta.value = { ...meta.value, draining: drainingOf(event.data) };
  void readWaits();
});
// What it waits for changes as a turn or a run ends; a run's ending may
// be passed on and not kept, so a stream that fell behind reads again.
stream.on(['agent.turn.finished', 'run.finished'], () => void readWaits(), () => void readWaits());
stream.on(['system.stopping'], (event) => {
  const said = stoppedSaid(event.data);
  if (said) stopped.value = said;
});

/** The line that says so, where the daemon drains or drained; nothing where it does not. */
export function Draining() {
  const draining = drainingOf(meta.value?.draining);
  const signedIn = !!me.value;
  useEffect(() => {
    if (draining && signedIn) void readWaits();
  }, [draining?.since, signedIn]);
  const lines = stopped.value ?? (draining ? [drainSaid(draining, worn.value), ...(waits.value ?? [])] : null);
  if (!lines) return null;
  return (
    <div class="toast notice draining" role="status">
      {lines.map((line) => (
        <p key={line}>{line}</p>
      ))}
    </div>
  );
}
