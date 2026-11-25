// An environment loaded from a branch, in its drawer (`envdrawer.tsx`):
// the blueprint it says it is loaded by, a branch of the project's
// repository chosen (what it last ran chosen first), Fetch, which asks
// the remotes what they have now, Run, and the run Run started as it goes,
// drawn as a run's steps are drawn everywhere (`RunDetail`), each by what
// it says of itself where the run is of the text its instance has, with
// what it gave out once it is over. What is worked out is `envrun.ts`.
//
// Nothing here polls: the branches are read as the drawer opens and when
// somebody fetches, the environment as anything is said of it
// (`envsheld.ts`), and the run as it says things (`RunDetail`, which
// reads again where the stream says it fell behind).

import { useEffect, useState } from 'preact/hooks';
import { api, call } from '../api.ts';
import type { Run } from '../api.ts';
import { type Branches, branchSaid, chosen, gave, mayRun, runClosed, short } from '../envrun.ts';
import { readEnvironments } from '../envsheld.ts';
import { type Environment, mayOf } from '../projectenvs.ts';
import type { Drawn } from '../runsaid.ts';
import { me, toast } from '../store.ts';
import { ago } from '../util.ts';
import { at } from './envdrawer.tsx';
import { useKept } from './returned.tsx';
import { RunDetail } from './run.tsx';

/** Where a project's branches are asked for. */
const branchesOf = (project: string) => `/projects/${encodeURIComponent(project)}/branches`;

export function Loaded({ project, environment }: { project: string; environment: Environment }) {
  const scopes = me.value?.principal.scopes ?? [];
  const may = mayOf(scopes);
  const can = mayRun(scopes);
  const base = at(project, environment.name);
  const again = () => void readEnvironments(project);
  const [branches, setBranches] = useKept<Branches | null>(`env-branches:${project}`, null);
  const [picked, setPicked] = useKept(`env-branch:${project}/${environment.name}`, '');
  const [draft, setDraft] = useKept<string | null>(`env-deploy:${project}/${environment.name}`, null);
  const [busy, setBusy] = useState<'run' | 'fetch' | null>(null);
  const [step, setStep] = useKept<string | null>(`env-step:${project}/${environment.name}`, null);
  const [drawn, setDrawn] = useState<Drawn | null>(null);
  const ran = environment.ran ?? null;

  // The branches, read once as the drawer opens: they change by git, which says nothing, and are read again when somebody fetches.
  useEffect(() => {
    void call<Branches>('GET', branchesOf(project))
      .then(setBranches)
      .catch((err) => setBranches({ repository: false, branches: [], said: err instanceof Error ? err.message : String(err) }));
  }, [project]);

  // The text the run is of, so that its steps are said by what each says of itself.
  useEffect(() => {
    setDrawn(null);
    if (!ran?.instance) return;
    void api
      .instance(ran.instance)
      .then((detail) => setDrawn({ graph: detail.graph, revision: detail.instance.revision }))
      .catch(() => {});
  }, [ran?.instance]);

  const listed = branches?.branches ?? [];
  const branch = chosen(listed, picked, ran);
  const closed = runClosed({ deploy: environment.deploy, branches, branch, may: can.run });

  const sendDeploy = async (now: string) => {
    const said = now.trim();
    if (said === (environment.deploy ?? '')) return setDraft(null);
    try {
      await call('PUT', `${base}/deploy`, { blueprint: said === '' ? null : said });
      setDraft(null);
      again();
    } catch (err) {
      toast(err instanceof Error ? err.message : String(err), true);
    }
  };
  const fetch = async () => {
    setBusy('fetch');
    try {
      const fetched = await call<Branches & { said: string }>('POST', `${branchesOf(project)}/fetch`, {});
      setBranches({ repository: true, remotes: fetched.remotes, branches: fetched.branches });
      toast(fetched.said);
    } catch (err) {
      toast(err instanceof Error ? err.message : String(err), true);
    } finally {
      setBusy(null);
    }
  };
  const run = async () => {
    setBusy('run');
    try {
      await call('POST', `${base}/run`, { branch });
      setStep(null);
      toast(`${environment.name} is being loaded from ${branch}.`);
      again();
    } catch (err) {
      toast(err instanceof Error ? err.message : String(err), true);
    } finally {
      setBusy(null);
    }
  };

  return (
    <section class="envs-part envs-loaded">
      <h3>Loaded from a branch</h3>
      <label class="envs-field">
        <span>Blueprint it is loaded by</span>
        <input
          aria-label="The blueprint it is loaded by"
          placeholder="deploy"
          spellcheck={false}
          disabled={!may.change}
          value={draft ?? environment.deploy ?? ''}
          onInput={(event) => setDraft(event.currentTarget.value)}
          onKeyDown={(event) => event.key === 'Enter' && event.currentTarget.blur()}
          onChange={(event) => void sendDeploy(event.currentTarget.value)}
        />
        <span class="faint">Run applies it for the project under {environment.name}, handed the branch as its param branch.</span>
      </label>
      <div class="envs-branch">
        {listed.length > 0 && (
          <label class="envs-field">
            <span>Branch</span>
            <select aria-label="The branch to load it from" value={branch} onChange={(event) => setPicked(event.currentTarget.value)}>
              {listed.map((one) => (
                <option key={one.name} value={one.name}>
                  {branchSaid(one)}
                </option>
              ))}
            </select>
          </label>
        )}
        <div class="envs-branch-acts">
          {can.fetch && (branches?.remotes?.length ?? 0) > 0 && (
            <button type="button" disabled={busy !== null} onClick={() => void fetch()}>
              {busy === 'fetch' ? 'Fetching…' : 'Fetch'}
            </button>
          )}
          <button type="button" class="primary" disabled={closed !== null || busy !== null} aria-describedby="envs-run-closed" onClick={() => void run()}>
            {busy === 'run' ? 'Running…' : 'Run'}
          </button>
        </div>
      </div>
      {closed && (
        <p class="faint" id="envs-run-closed">
          {closed}
        </p>
      )}
      {ran ? (
        <div class="envs-ran" data-run={ran.run ?? ''}>
          <p class="muted">
            Last run of {ran.blueprint}: <strong>{ran.branch}</strong>
            {ran.commit && (
              <>
                {' '}
                at <code>{short(ran.commit)}</code>
              </>
            )}
            , {ago(ran.at)}, by {ran.by_name}
          </p>
          {ran.run ? <RunDetail id={ran.run} step={step} onStep={setStep} drawn={drawn} foot={(read) => <GaveOut run={read} />} /> : <p class="faint">It began no run.</p>}
        </div>
      ) : (
        <p class="faint">It has not been run yet.</p>
      )}
    </section>
  );
}

/** What a run gave out once it is over: each by name, an address of the web a link that opens in a tab of its own. */
function GaveOut({ run }: { run: Run }) {
  const things = run.status === 'running' ? [] : gave(run.outputs);
  if (things.length === 0) return null;
  return (
    <dl class="envs-gave">
      {things.map((thing) => (
        <div key={thing.name} class="envs-gave-one" data-gave={thing.name}>
          <dt>{thing.name}</dt>
          <dd>
            {thing.href ? (
              <a href={thing.href} target="_blank" rel="noopener noreferrer">
                {thing.text}
              </a>
            ) : (
              <code>{thing.text}</code>
            )}
          </dd>
        </div>
      ))}
    </dl>
  );
}
