// The run a message is about (`about_run`, a run that failed, which the
// fleet keeps with what it says of a failure), opened from the message's
// details at the step that failed (`failedAt` in `staged.ts`). It is
// asked for once as the details are opened, so that one that is gone is
// said to be gone where it would be pressed, and never pressed to find so.

import { useEffect, useState } from 'preact/hooks';
import { api, ApiError } from '../api.ts';
import { openTab } from '../store.ts';

export function FailedRun({ run }: { run: string }) {
  const [there, setThere] = useState<boolean | null>(null);
  useEffect(() => {
    let current = true;
    api
      .run(run)
      .then(() => current && setThere(true))
      // A run that is not there any more, or not this reader's: said so. Anything else is tried by pressing.
      .catch((err) => current && setThere(!(err instanceof ApiError && err.status === 404)));
    return () => {
      current = false;
    };
  }, [run]);
  if (there === false) return <span class="faint failed-run-gone">The run that failed is no longer kept.</span>;
  return (
    <button class="ghost link failed-run" disabled={there === null} onClick={() => openTab({ kind: 'run', id: run })}>
      Open the run that failed
    </button>
  );
}
