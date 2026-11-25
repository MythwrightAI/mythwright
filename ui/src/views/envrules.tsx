// What an environment holds to, and the runs that acted under it, as two
// parts of its drawer (`envdrawer.tsx`; what is worked out is
// `envrules.ts`). Who approves what is started under it and which branches
// run under it are changed here by whoever may change its secrets, each
// change sent whole as it is made (`PUT .../rules`, what the daemon holds
// a change to), and said in words to anybody else. Each run opens its own
// page (`#/run/<id>`), the first of them being its last run.

import { hrefOf } from '../address.ts';
import { call } from '../api.ts';
import { type Acted, type Rules, actedSaid, addable, approveSaid, branchesOf, branchesSaid, branchesWritten, mayRule, nameOf } from '../envrules.ts';
import { usePeople } from '../peopleknown.ts';
import { me } from '../store.ts';
import { Badge } from '../ui.tsx';
import { ago } from '../util.ts';

/** Who approves what goes under it and which branches run under it. */
export function RulesPart({ base, rules, onChanged, asking }: { base: string; rules: Rules | undefined; onChanged: () => void; asking: (work: () => Promise<unknown>) => Promise<boolean> }) {
  const people = usePeople();
  const may = mayRule(me.value?.principal.scopes ?? []);
  const said = branchesWritten(rules);
  const send = (body: Rules) => void asking(() => call('PUT', `${base}/rules`, body)).then(onChanged);
  const approve = rules?.approve ?? [];
  const sendBranches = (text: string) => {
    const now = branchesOf(text);
    if (now.join(' ') !== said) send({ branches: now });
  };
  return (
    <section class="envs-part envs-rules" data-rules>
      <h3>Rules</h3>
      <div class="envs-rule" data-rule="approve">
        <span class="envs-rule-name">Who approves</span>
        {may && (
          <div class="envs-approvers">
            {approve.map((actor) => (
              <span key={actor} class="envs-approver" data-approver={nameOf(actor, people)}>
                {nameOf(actor, people)}
                <button type="button" class="ghost icon" aria-label={`${nameOf(actor, people)} no longer approves`} title="No longer approves" onClick={() => send({ approve: approve.filter((one) => one !== actor) })}>
                  ×
                </button>
              </span>
            ))}
            <select
              aria-label="Somebody else who approves"
              value=""
              onChange={(event) => {
                const actor = event.currentTarget.value;
                if (actor) send({ approve: [...approve, actor] });
              }}
            >
              <option value="">{approve.length === 0 ? 'Name somebody…' : 'And somebody else…'}</option>
              {addable(people, rules).map((one) => (
                <option key={one.actor} value={one.actor}>
                  {one.name}
                </option>
              ))}
            </select>
          </div>
        )}
        <p class="faint" data-said>
          {approveSaid(rules, people)}
        </p>
      </div>
      <div class="envs-rule" data-rule="branches">
        <span class="envs-rule-name">Branches</span>
        {may && (
          <input
            // Made anew as what it says changes, so that what was written is then what the daemon kept.
            key={said}
            aria-label="The branches it runs, by spaces"
            placeholder="any branch: main release/*"
            spellcheck={false}
            defaultValue={said}
            onKeyDown={(event) => event.key === 'Enter' && event.currentTarget.blur()}
            onChange={(event) => sendBranches(event.currentTarget.value)}
          />
        )}
        <p class="faint" data-said>
          {branchesSaid(rules)}
        </p>
      </div>
      <p class="faint">{may ? 'Held wherever something comes to run under it: by Run, an apply, or a step that names it.' : 'Said by whoever may change its secrets.'}</p>
    </section>
  );
}

/** The runs that acted under it, the newest first, each opening its page. */
export function RunsPart({ runs }: { runs: Acted[] | undefined }) {
  if (runs === undefined) return null;
  return (
    <section class="envs-part envs-runs" data-runs>
      <h3>Runs under it</h3>
      {runs.length === 0 && <p class="faint">None among the project's latest runs.</p>}
      {runs.length > 0 && (
        <ul class="envs-acted">
          {runs.map((acted, at) => (
            <li key={acted.run} data-acted={acted.run}>
              <Badge status={acted.status}>{acted.status === 'ok' ? '✓' : acted.status === 'failed' ? '✕' : acted.status === 'running' ? '…' : '–'}</Badge>
              <a href={hrefOf({ kind: 'run', id: acted.run })} title="Open this run">
                {actedSaid(acted, ago)}
              </a>
              {at === 0 && <span class="faint">its last run</span>}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
