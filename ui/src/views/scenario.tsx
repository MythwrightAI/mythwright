// A word shown working, in the drawer: the example of an entry of the
// Reference run in a sandbox, with what the scenario says is done to it,
// and what came of it drawn as the Sandbox view of a blueprint's page
// draws it (`sandboxed.tsx`), with no second drawing of anything.
//
// The sandbox is the daemon's, made for this opening of the drawer and
// asked one thing at a time, as a sandbox test is (its events are on no
// stream). It is kept beside the sandbox tests of the tabs
// (`sandboxkept.ts`), by a key of its own each time it is opened
// (`scenarioKey`), and ended when the drawer is put away or handed
// something else, as closing a blueprint's tab ends its sandbox: the
// view is on the page for as long as the drawer is, so its leaving the
// page is the drawer's going. What the scenario says to notice is its
// own sentence; that it was a sandbox test, and what was stood in, are
// the daemon's, at the foot.

import { useEffect, useState } from 'preact/hooks';
import { ApiError, api, call } from '../api.ts';
import type { Diagnostic, Graph } from '../api.ts';
import type { Scenario } from '../blueprint/told.ts';
import type { Shown, Standing } from '../sandbox.ts';
import { doneSays, scenarioKey } from '../scenario.ts';
import { NOTHING, endSandbox, forgetSandbox, keepSandbox, sandboxOf, sandboxes } from '../sandboxkept.ts';
import { drawer } from '../store.ts';
import { Empty } from '../ui.tsx';
import { Foot, Happened } from './sandboxed.tsx';

const why = (err: unknown): string => (err instanceof Error ? err.message : String(err));

/** Opens the drawer on `name`'s scenario, run afresh. */
export function openScenario(name: string, scenario: Scenario): void {
  const key = scenarioKey();
  drawer.value = { title: `${name}, run in a sandbox`, body: () => <Running key={key} keep={key} scenario={scenario} /> };
}

/**
 * Makes the sandbox, does each thing the scenario does in order, and reads
 * what came of it. Once the drawer has gone nothing more is asked, and a
 * sandbox the daemon made after it went is ended at once.
 */
async function play(keep: string, scenario: Scenario): Promise<void> {
  const still = () => sandboxOf(keep) !== null;
  const kept = (change: Parameters<typeof keepSandbox>[1]) => void (still() && keepSandbox(keep, change));
  keepSandbox(keep, { busy: 'Running it in a sandbox' });
  let made: Standing & { ok: boolean; diagnostics?: Diagnostic[] };
  try {
    made = await call('POST', '/sandboxes', { source: scenario.example, apply: false, answers: scenario.answers });
  } catch (err) {
    kept({ busy: null, refused: why(err) });
    return;
  }
  if (!made.ok) {
    kept({ busy: null, problems: made.diagnostics ?? [] });
    return;
  }
  if (!still()) return void endSandbox(made.sandbox);
  keepSandbox(keep, { id: made.sandbox });
  const at = `/sandboxes/${encodeURIComponent(made.sandbox)}`;
  for (const act of scenario.acts) {
    if (!still()) return;
    kept({ busy: doneSays(act) });
    try {
      await call<Standing>('POST', `${at}/acts`, act);
    } catch (err) {
      kept({ refused: `${doneSays(act)} was refused, and what was to be done after it was not done: ${why(err)}` });
      break;
    }
  }
  if (!still()) return;
  try {
    const shown = await call<Shown>('GET', at);
    kept({ shown, busy: null, answered: (sandboxOf(keep)?.answered ?? 0) + 1 });
  } catch (err) {
    kept(err instanceof ApiError && err.status === 404 ? { id: null, busy: null, gone: why(err) } : { busy: null, refused: why(err) });
  }
}

/** A sentence with `code` in backticks, as it should look. */
function Words({ text }: { text: string }) {
  return (
    <>
      {text.split('`').map((part, index) => (index % 2 === 1 ? <code key={index}>{part}</code> : part))}
    </>
  );
}

function Running({ keep, scenario }: { keep: string; scenario: Scenario }) {
  const [graph, setGraph] = useState<Graph | null>(null);
  useEffect(() => {
    let here = true;
    void play(keep, scenario);
    // The example's graph, as the editor reads it, to be lit by what it did.
    api.check(scenario.example).then(
      (checked) => here && setGraph(checked.graph ?? null),
      () => {},
    );
    return () => {
      here = false;
      forgetSandbox(keep);
    };
  }, [keep]);
  const kept = sandboxes.value[keep] ?? NOTHING;
  const shown = kept.shown;
  return (
    <div class="sandbox scenario" data-scenario={scenario.name} data-sandbox={kept.id ?? ''} data-busy={kept.busy !== null} data-answered={kept.answered}>
      <div class="scenario-body">
        <p class="scenario-notice">
          <Words text={scenario.notice} />
        </p>
        <section class="sandbox-part" aria-label="What was done to it">
          <h3>What was done to it</h3>
          <ol class="scenario-done">
            {scenario.acts.map((act, index) => (
              <li key={index}>
                <Words text={doneSays(act)} />
              </li>
            ))}
          </ol>
        </section>
        {kept.refused && (
          <p class="sandbox-note bad" role="alert">
            {kept.refused}
          </p>
        )}
        {kept.gone && (
          <p class="sandbox-note bad" role="alert">
            The daemon has this sandbox no longer. {kept.gone}
          </p>
        )}
        {kept.problems && (
          <p class="sandbox-note bad" role="alert">
            This text does not check, so it was not run in a sandbox: {kept.problems.map((problem) => problem.message).join(' ')}
          </p>
        )}
        <section class="sandbox-part" aria-label="What happened">
          <h3>What happened</h3>
          {shown ? <Happened kept={kept} shown={shown} graph={graph} /> : !kept.refused && !kept.problems && <Empty title={kept.busy ?? 'Running it in a sandbox'} />}
        </section>
      </div>
      {shown && <Foot shown={shown} />}
    </div>
  );
}
