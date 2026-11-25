// An agent's whole setup, in the drawer, from Edit… first in its line's
// menu on the Agents tab. What is shown is where its settings are kept
// (`setupOf` in `agentedit.ts`):
//
// - added by hand, the form the dialog that adds one asks with
//   (`ProgramFields`, `WhereFields` of `addagent.tsx`), filled in from its
//   slot as it stands, with what its program may reach, what it signs in
//   with, the keys it is handed, what it may ask of the factory and what a
//   project's fork of `std/agent` takes beside; saved by applying its text
//   again with all of it, one request (`POST /api/agents` with `params`);
// - one of a fleet's, its row, in the row form the drawer draws for any
//   row (`One` of `records.tsx`), saved as the row, which the fleet follows;
// - one from outside, what it may ask and its project, one `PATCH`;
// - one something else brings up, said, with nothing to change here.
//
// What is closed to whoever reads it is what the daemon would refuse
// (`closedOf`), said where it is closed; the daemon refuses as before.
// What is typed is kept by the tab with the drawer (`useKept`), and what
// a save came to for the work in hand is the daemon's sentence (`said`).

import { useEffect, useState } from 'preact/hooks';
import { api, call } from '../api.ts';
import type { Fields, GraphParam, Slot } from '../api.ts';
import { answered, startOf, written as writtenAs } from '../applyasked.ts';
import { bodyOf, closedOf, outsideChange, OWN_PARAMS, sameWritten, setupOf, stoppedSaid, writtenOf } from '../agentedit.ts';
import type { Closed, Setup, SetupSlot, Written } from '../agentedit.ts';
import { held } from '../chosen.ts';
import { namesOf } from '../envsheld.ts';
import { isExternal } from '../external.ts';
import { covers, scopesAfter } from '../holds.ts';
import { agents, collections, drawer, me, meta, projects, refreshAgents } from '../store.ts';
import type { Field, Row } from '../table.ts';
import { workspaces } from '../spaces.ts';
import { listed } from '../workspaces.ts';
import { ProgramFields, WhereFields, modeOf } from './addagent.tsx';
import { ParamField } from './applying.tsx';
import { HoldsTable } from './holds.tsx';
import { One } from './records.tsx';
import { useKept } from './returned.tsx';

/** A slot as the list of agents hands it, with what this reads of it beyond what the list's own type says. */
type Listed = Slot & SetupSlot & { instance?: string };

/** Opens `name`'s setup in the drawer. */
export function showSetup(name: string): void {
  drawer.value = {
    title: `${name}: setup`,
    body: () => <AgentSetup key={name} name={name} />,
    gone: async () => !(await api.agents()).agents.some((slot) => slot.name === name),
  };
}

function AgentSetup({ name }: { name: string }) {
  const slot = agents.value.find((one) => one.name === name) as Listed | undefined;
  if (!slot) return <p class="faint">There is no agent called {name} any more.</p>;
  const tables = (collections.value[slot.project ?? ''] ?? []).map((one) => one.name);
  const setup = setupOf(slot, tables, isExternal(slot));
  const principal = me.value!.principal;
  const closed = closedOf(slot, setup, { kind: principal.kind, id: principal.id, scopes: principal.scopes });
  return (
    <div class="agent-setup" data-setup={setup.kind}>
      {setup.kind === 'hand' && <ByHand slot={slot} closed={closed} />}
      {setup.kind === 'row' && <InItsRow slot={slot} setup={setup} closed={closed} />}
      {setup.kind === 'outside' && <Outside slot={slot} closed={closed} />}
      {setup.kind === 'brought' && <Brought slot={slot} closed={closed} />}
    </div>
  );
}

/** The daemon's sentence for a save, and what it refused: kept with the drawer. */
function useSaid(name: string) {
  return useKept<{ said: string; bad: boolean } | null>(`setup.said:${name}`, null);
}

function SaidLine({ said }: { said: { said: string; bad: boolean } | null }) {
  if (!said) return null;
  return <p class={said.bad ? 'error-text setup-said' : 'setup-said'}>{said.said}</p>;
}

/** Why nothing of it is the reader's to change, over the form, once. */
function AllClosed({ why }: { why: string | null }) {
  return why ? <p class="faint setup-closed">{why}</p> : null;
}

/** The projects offered to work in: those of the workspace the side panel is of, and the one it works in now. */
function offeredProjects(now: string | null) {
  const offered = listed(projects.value, workspaces.value, held.value);
  const its = projects.value.find((one) => one.id === now);
  return its && !offered.some((one) => one.id === its.id) ? [its, ...offered] : offered;
}

/** What a fork of its text takes beside what the form says: read from the instance it is applied as. */
function useMore(slot: Listed): { params: GraphParam[]; known: Fields } {
  const [more, setMore] = useKept<{ params: GraphParam[]; known: Fields }>(`setup.more:${slot.name}`, { params: [], known: {} });
  useEffect(() => {
    if (!slot.instance) return;
    let left = false;
    api
      .instance(slot.instance)
      .then((detail) => {
        if (left) return;
        const params = (detail.graph?.params ?? []).filter((param) => !OWN_PARAMS.includes(param.name));
        setMore({ params, known: detail.instance.params });
      })
      .catch(() => undefined);
    return () => {
      left = true;
    };
  }, [slot.instance]);
  return more;
}

function ByHand({ slot, closed }: { slot: Listed; closed: Closed }) {
  const more = useMore(slot);
  const knownMore = Object.fromEntries(more.params.map((param) => [param.name, writtenAs(startOf(param, more.known))]));
  const start = writtenOf(slot, knownMore);
  const [typed, setTyped] = useKept<Written | null>(`setup.typed:${slot.name}`, null);
  const [said, setSaid] = useSaid(slot.name);
  const [busy, setBusy] = useState(false);
  const form = typed ?? start;
  const write = (next: Written) => {
    setTyped(next);
    if (said?.bad) setSaid(null);
  };
  const all = closed.all;
  const mine = me.value!.principal.scopes;
  const areas = meta.value?.scopes ?? [];
  const environments = namesOf(form.project || null);
  const changed = !sameWritten(form, start);
  const stopped = stoppedSaid(slot);
  const envMissing = environments.length > 0 && !environments.includes(form.environment);

  const save = async (event: Event) => {
    event.preventDefault();
    if (busy || all || !changed || envMissing) return;
    const told = answered(more.params, form.more);
    if ('wrong' in told) return setSaid({ said: told.wrong, bad: true });
    setBusy(true);
    try {
      const body = bodyOf(slot.name, { ...form, permission: modeOf(form.adapter, form.permission) }, told.values);
      const answer = await call<Slot & { said?: string }>('POST', '/agents', body);
      const words = answer.said ?? `${slot.name} is changed.`;
      setSaid({ said: words, bad: false });
      setTyped(null);
      void refreshAgents();
    } catch (err) {
      setSaid({ said: err instanceof Error ? err.message : String(err), bad: true });
    } finally {
      setBusy(false);
    }
  };

  // A part that would show its program more is closed only for turning it on: turning it off is anybody's who may change it.
  const shut = (why: string | null, on: boolean) => all ?? (on ? null : why);
  return (
    <form class="stack setup-form" onSubmit={(event) => void save(event)}>
      <p class="muted setup-about">Added by hand, it is set up by its own text, std/agent: saved, that is applied again with all of this, as you.</p>
      {stopped && <p class="muted setup-stopped">{stopped}</p>}
      <AllClosed why={all} />
      <h3>What runs it</h3>
      <ProgramFields
        value={{ adapter: form.adapter, model: form.model, mode: form.permission, instructions: form.instructions }}
        onChange={(next) => write({ ...form, adapter: next.adapter, model: next.model, permission: next.mode, instructions: next.instructions })}
        closed={all}
        fullClosed={closed.full}
      />
      <h3>Where it works</h3>
      <WhereFields
        project={form.project}
        under={form.environment}
        offered={offeredProjects(slot.project)}
        environments={environments}
        onProject={(project) => write({ ...form, project, environment: '' })}
        onUnder={(environment) => write({ ...form, environment })}
        closed={all}
      />
      <h3>What its program may reach</h3>
      <Check quiet={!!all} label="Behind a wall: shown its work and a home of its own, and nothing else of this device" on={form.wall} closed={shut(closed.device, !form.wall)} onChange={(wall) => write({ ...form, wall })} />
      <Lines quiet={!!all} label="What else it may read there, a folder or file a line" value={form.reads} closed={shut(closed.device, false)} onChange={(reads) => write({ ...form, reads })} />
      <Lines quiet={!!all} label="What else it may change there, a line each" value={form.changes} closed={shut(closed.device, false)} onChange={(changes) => write({ ...form, changes })} />
      <label>
        Its project's repository
        <select value={form.git} disabled={!!all} title={all ?? undefined} onChange={(event) => write({ ...form, git: event.currentTarget.value })}>
          <option value="off">off: it is not there</option>
          <option value="read">read: what it changes is committed for it</option>
          <option value="commit">commit: it commits its own work</option>
          <option value="write">write: all of it</option>
        </select>
      </label>
      <Check quiet={!!all} label="May use the web unasked" on={form.web} closed={shut(closed.device, form.web)} onChange={(web) => write({ ...form, web })} />
      <Check quiet={!!all} label="Goes on without the daemon, taken up where it is by the next one" on={form.outlives} closed={shut(closed.device, form.outlives)} onChange={(outlives) => write({ ...form, outlives })} />
      <h3>Sign-ins and keys</h3>
      <Lines quiet={!!all} label="What it signs in with, in order: subscription, key, or a sign-in's name, a line each" value={form.signins} closed={all ?? closed.secrets} onChange={(signins) => write({ ...form, signins })} />
      <Lines quiet={!!all} label="The keys it is handed, VARIABLE=secret a line each: names only, never a value" value={form.secrets} closed={all ?? closed.secrets} onChange={(secrets) => write({ ...form, secrets })} />
      <h3>What it may ask of the factory</h3>
      <HoldsTable scopes={form.holds} onChange={all ? undefined : (area, verbs) => write({ ...form, holds: scopesAfter(form.holds, areas, area, verbs) })} giver={mine} />
      {more.params.length > 0 && (
        <>
          <h3>What its project's fork of its text takes beside</h3>
          {more.params.map((param) => (
            <fieldset key={param.name} class="field-closed" disabled={!!all}>
              <ParamField param={param} now={form.more[param.name] ?? ''} known={more.known} onSet={(text) => write({ ...form, more: { ...form.more, [param.name]: text } })} />
            </fieldset>
          ))}
        </>
      )}
      <SaidLine said={said} />
      {envMissing && !all && <p class="faint setup-closed">Choose the environment it works under: its project has some.</p>}
      <div class="row setup-foot">
        <button type="button" disabled={!typed} onClick={() => setTyped(null)}>
          Put back
        </button>
        <span class="grow" />
        <button class="primary" type="submit" disabled={busy || !!all || !changed || envMissing} title={all ?? (changed ? undefined : 'Nothing is changed yet.')}>
          {stopped ? 'Save and bring it in' : 'Save'}
        </button>
      </div>
    </form>
  );
}

/** Something turned on and off, closed with why: said under it, but where all of the form is closed, which is said once over it (`quiet`). */
function Check({ label, on, closed, quiet, onChange }: { label: string; on: boolean; closed: string | null; quiet?: boolean; onChange: (on: boolean) => void }) {
  return (
    <label class="setup-check" title={closed ?? undefined}>
      <span>{label}</span>
      <input type="checkbox" checked={on} disabled={!!closed} onChange={(event) => onChange(event.currentTarget.checked)} />
      {closed && !quiet && <span class="faint setup-check-why">{closed}</span>}
    </label>
  );
}

/** A box that holds one thing a line, closed with why, said as a check's is. */
function Lines({ label, value, closed, quiet, onChange }: { label: string; value: string; closed: string | null; quiet?: boolean; onChange: (text: string) => void }) {
  return (
    <label>
      {label}
      <textarea rows={2} value={value} disabled={!!closed} title={closed ?? undefined} onInput={(event) => onChange(event.currentTarget.value)} />
      {closed && !quiet && <span class="faint">{closed}</span>}
    </label>
  );
}

function InItsRow({ slot, setup, closed }: { slot: Listed; setup: Extract<Setup, { kind: 'row' }>; closed: Closed }) {
  const [row, setRow] = useKept<Row | null | 'gone'>(`setup.row:${setup.record}`, null);
  useEffect(() => {
    api.record(setup.record).then(setRow, () => setRow('gone'));
  }, [setup.record]);
  const mine = me.value!.principal.scopes;
  const own = closed.all?.startsWith('An agent never') ? closed.all : null;
  const whole = own ?? (covers(mine, 'records:write') ? null : 'Changing a row of a table takes records:write, which you do not hold.');
  // A field that says what giving it a value takes is closed to whoever does not hold all of it.
  const closedFor = (field: Field) => {
    const takes = field.takes ? (Array.isArray(field.takes) ? field.takes : [field.takes]) : [];
    const lacking = takes.filter((scope) => !covers(mine, scope));
    return lacking.length > 0 ? `Giving it a value takes ${lacking.join(', ')}, which you do not hold.` : null;
  };
  return (
    <>
      <p class="muted setup-about setup-row-said">
        This agent's settings are its fleet row's: {setup.record} of {setup.table}. A change here is the row's, and the fleet sets up every agent of the row again as it then says, each taking it up from its next turn.
      </p>
      {row === null ? (
        <p class="faint">Reading its row</p>
      ) : row === 'gone' ? (
        <p class="faint">Its row cannot be read here: whoever may read the project's tables changes it there.</p>
      ) : (
        <One project={slot.project!} name={setup.table} row={row} closed={whole} closedFor={closedFor} />
      )}
    </>
  );
}

function Outside({ slot, closed }: { slot: Listed; closed: Closed }) {
  const [typed, setTyped] = useKept<{ holds: string[]; project: string } | null>(`setup.outside:${slot.name}`, null);
  const [said, setSaid] = useSaid(slot.name);
  const [busy, setBusy] = useState(false);
  const form = typed ?? { holds: slot.scopes, project: slot.project ?? '' };
  const change = outsideChange(slot, form.holds, form.project);
  const all = closed.all;
  const areas = meta.value?.scopes ?? [];
  const save = async (event: Event) => {
    event.preventDefault();
    if (busy || all || !change) return;
    setBusy(true);
    try {
      const answer = await call<Slot & { said?: string }>('PATCH', `/agents/${encodeURIComponent(slot.name)}`, change);
      const words = answer.said ?? `${slot.name} is changed.`;
      setSaid({ said: words, bad: false });
      setTyped(null);
      void refreshAgents();
    } catch (err) {
      setSaid({ said: err instanceof Error ? err.message : String(err), bad: true });
    } finally {
      setBusy(false);
    }
  };
  return (
    <form class="stack setup-form" onSubmit={(event) => void save(event)}>
      <p class="muted setup-about">An agent from outside is a program somebody runs for themselves: this factory starts nothing for it, so what is set up here is what it may ask of the factory and the project it works in.</p>
      <AllClosed why={all} />
      <h3>What it may ask of the factory</h3>
      <HoldsTable scopes={form.holds} onChange={all ? undefined : (area, verbs) => setTyped({ ...form, holds: scopesAfter(form.holds, areas, area, verbs) })} giver={me.value!.principal.scopes} />
      <h3>Where it works</h3>
      <WhereFields project={form.project} under="" offered={offeredProjects(slot.project)} environments={[]} onProject={(project) => setTyped({ ...form, project })} onUnder={() => undefined} closed={all} />
      <SaidLine said={said} />
      <div class="row setup-foot">
        <button type="button" disabled={!typed} onClick={() => setTyped(null)}>
          Put back
        </button>
        <span class="grow" />
        <button class="primary" type="submit" disabled={busy || !!all || !change} title={all ?? (change ? undefined : 'Nothing is changed yet.')}>
          Save
        </button>
      </div>
    </form>
  );
}

function Brought({ slot, closed }: { slot: Listed; closed: Closed }) {
  return (
    <div class="stack">
      <AllClosed why={closed.all} />
      <h3>What it may ask of the factory</h3>
      <HoldsTable scopes={slot.scopes} />
    </div>
  );
}
