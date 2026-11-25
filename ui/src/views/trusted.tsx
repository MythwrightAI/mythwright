// The right of a project's page's head (`pagehead.tsx`), with what its
// gear offers of trusting the project: trusting one that is not trusted,
// and taking that back from one that is, each for whoever manages the
// factory (`trusting.ts`), behind a question that says what it does.
//
// The daemon trusts it and says so (`project.trusted`,
// `project.untrusted`), and every page follows that by reading the
// projects again (`store.ts`): the mark that says a project is not
// trusted, here and in the side panel, goes or comes back with nothing
// read by hand. What the daemon refuses is said in its own sentence where
// it was asked, and the question stays.

import { useState } from 'preact/hooks';
import { call } from '../api.ts';
import { me, projects, toast } from '../store.ts';
import { trusting, type Trusting } from '../trusting.ts';
import { Modal } from '../ui.tsx';
import { PageActs } from './pagehead.tsx';
import type { Lines } from './pagehead.tsx';
import { signs } from './signs.tsx';

/** Trusts a project, or takes that back. */
const trust = (id: string, trusted: boolean) => call<unknown>('POST', `/projects/${encodeURIComponent(id)}/trust`, { trusted });

/** `more` is what else the page's gear has, before the line that trusts. */
export function ProjectActs({ id, more }: { id: string; more?: () => Lines }) {
  const [asking, setAsking] = useState<Trusting | null>(null);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const project = projects.value.find((one) => one.id === id);
  const offered = project ? trusting(project, me.value?.principal.scopes ?? []) : null;
  const close = () => {
    setAsking(null);
    setError('');
    setBusy(false);
  };
  const lines = (): Lines => [
    ...(more?.() ?? []),
    ...(offered ? [{ label: offered.line, mark: signs.shield(), act: () => setAsking(offered) }] : []),
  ];
  const yes = async () => {
    if (!asking || busy) return;
    setBusy(true);
    setError('');
    try {
      await trust(id, asking.to);
      toast(asking.done);
      close();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setBusy(false);
    }
  };
  return (
    <>
      <PageActs tab={{ kind: 'project', id }} menu={lines} last />
      {asking && (
        <Modal onClose={close}>
          <div class="stack trust-asked" data-to={asking.to ? 'trusted' : 'untrusted'}>
            <h2>{asking.asks}</h2>
            <p style="margin:0">{asking.says}</p>
            {error && <p class="error-text">{error}</p>}
            <div class="row modal-foot">
              <button type="button" onClick={close}>
                Cancel
              </button>
              <button type="button" class="primary" disabled={busy} onClick={() => void yes()}>
                {asking.button}
              </button>
            </div>
          </div>
        </Modal>
      )}
    </>
  );
}
