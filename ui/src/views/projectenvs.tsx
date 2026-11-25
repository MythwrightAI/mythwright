// A project's environments (`dev`, `staging`, `prod`): a table of them,
// and one opened in the drawer beside it with its variables and secrets
// (`envdrawer.tsx`), as a row's form is opened. What is worked out is
// `projectenvs.ts`; what is read and followed is `envsheld.ts`. The daemon answers with no secret's value,
// so nothing here could show one: a value is typed once, in a field that
// hides it, and is never read back.
//
// Nothing is offered that the daemon would refuse whoever looks: changing
// an environment takes writing the project, `shell` and `secrets:use`;
// a secret takes `environments:secrets`; seeing secrets' names takes
// `secrets:use`. What it refuses all the same is said in its sentence.

import { useEffect, useRef, useState } from 'preact/hooks';
import { api, call } from '../api.ts';
import { environmentsOf, readEnvironments } from '../envsheld.ts';
import { lastRunSaid } from '../envrun.ts';
import type { Last } from '../envrun.ts';
import { hrefOf, opens } from '../address.ts';
import { type Running, counted, mayOf, nameRefused, rowsOf } from '../projectenvs.ts';
import { drawer, me, openTab, projects } from '../store.ts';
import { Empty, useEvents } from '../ui.tsx';
import { ago, classes } from '../util.ts';
import { asking, openEnvironment, openedEnvironment } from './envdrawer.tsx';
import { PageActs } from './pagehead.tsx';
import { FlagsPart } from './projectflags.tsx';
import { headWord, ThingWas } from './thingwas.tsx';

export function ProjectEnvironments({ project }: { project: string }) {
  const may = mayOf(me.value?.principal.scopes ?? []);
  const name = projects.value.find((one) => one.id === project)?.name ?? project;
  const environments = environmentsOf.value[project];
  const [running, setRunning] = useState<Running[]>([]);
  const [adding, setAdding] = useState(false);

  // What runs under each, read once and again as anything of the project starts or ends.
  const readRunning = () => void api.instances({ live: true, project, limit: 1000 }).then((found) => setRunning(found as Running[])).catch(() => {});
  useEffect(() => {
    void readEnvironments(project);
    readRunning();
  }, [project]);
  // What each last ran is read again as anything of the project starts or ends, since what was applied under one is.
  const readBoth = () => (readRunning(), void readEnvironments(project));
  useEvents(['instance.active', 'instance.finished', 'instance.stopped', 'instance.failed'], readBoth, [project]);
  // How what each last ran went is read with it: a run it says ending, or one of what runs under it, has it read
  // again, and so does the stream saying it fell behind, since a run's ending is passed on and not kept.
  const runs = (environments ?? []).flatMap((one) => [one.ran?.run, one.last?.run]).filter(Boolean);
  const under = running.filter((one) => one.environment).map((one) => one.id);
  const ours = (event: { source?: { run?: string; instance?: string } }) => runs.includes(event.source?.run) || under.includes(event.source?.instance);
  useEvents('run.finished', (event) => ours(event) && void readEnvironments(project), [project, runs.join(), under.join()], () => void readEnvironments(project));

  const rows = rowsOf(environments ?? [], running);
  // The one open in the drawer, marked in its row; pressed again, the drawer is put away.
  const current = openedEnvironment();
  const open = current?.project === project ? current.name : null;
  const press = (name: string) => (open === name ? (drawer.value = null) : openEnvironment(project, name));

  return (
    <div class="envs-page">
      <div class="view-head">
        <h1 class="grow truncate">
          {name} {headWord(project, 'environments', 'environments')}
          <ThingWas project={project} thing="environments" />
        </h1>
        <PageActs tab={{ kind: 'environments', project }} last={!may.change} />
        {may.change && (
          <button class="primary" onClick={() => setAdding(true)}>
            Add an environment
          </button>
        )}
      </div>
      <div class="view-body stack envs-body">
        {adding && <AddEnvironment project={project} onClose={() => setAdding(false)} onMade={(made) => openEnvironment(project, made)} />}
        <div class="panel envs-panel">
          {environments === undefined ? (
            <p class="muted envs-pad">Reading its environments…</p>
          ) : rows.length === 0 ? (
            <Empty title="No environments yet">
              <p class="muted">A project with none runs everything as it always did. Once it has one, everything applied for it says which it runs under.</p>
            </Empty>
          ) : (
            <table class="envs-table">
              <thead>
                <tr>
                  <th>Name</th>
                  <th>Variables</th>
                  <th>Secrets</th>
                  <th>For agents</th>
                  <th>In use</th>
                  <th>Last run</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => (
                  <tr
                    key={row.name}
                    class={classes('clickable', open === row.name && 'current')}
                    data-environment={row.name}
                    aria-selected={open === row.name}
                    tabIndex={0}
                    onClick={() => press(row.name)}
                    onKeyDown={(event) => (event.key === 'Enter' || event.key === ' ') && (event.preventDefault(), press(row.name))}
                  >
                    <td>
                      <code>{row.name}</code>
                    </td>
                    <td class="muted">{counted(row.vars, 'variable')}</td>
                    <td class="muted">{row.secrets === null ? 'not shown' : counted(row.secrets, 'secret')}</td>
                    <td class="muted">{counted(row.agents, 'entry', 'entries')}</td>
                    <td class="muted">{row.running === 0 ? 'nothing runs under it' : `${row.running} running`}</td>
                    <td class="muted" data-last-run>
                      <LastRun last={environments?.find((one) => one.name === row.name)?.last} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
        {(environments?.length ?? 0) > 0 && <FlagsPart project={project} />}
      </div>
    </div>
  );
}

/** Makes an environment, by a name that reads as one. */
function AddEnvironment(props: { project: string; onClose: () => void; onMade: (name: string) => void }) {
  const [name, setName] = useState('');
  const box = useRef<HTMLInputElement>(null);
  useEffect(() => box.current?.focus(), []);
  const wrong = name === '' ? null : nameRefused(name);
  const make = async (event: Event) => {
    event.preventDefault();
    if (nameRefused(name)) return;
    const made = await asking(() => call('POST', `/projects/${encodeURIComponent(props.project)}/environments`, { name }), `${name} is made.`);
    if (!made) return;
    void readEnvironments(props.project);
    props.onMade(name);
    props.onClose();
  };
  return (
    <form class="panel envs-add" onSubmit={(event) => void make(event)}>
      <label class="envs-field">
        <span>Name</span>
        <input ref={box} value={name} placeholder="staging" spellcheck={false} autocomplete="off" onInput={(event) => setName(event.currentTarget.value)} onKeyDown={(event) => event.key === 'Escape' && (event.stopPropagation(), props.onClose())} />
        <span class={wrong ? 'error-text' : 'faint'}>{wrong ?? 'Lowercase letters, digits and -, such as staging.'}</span>
      </label>
      <div class="row envs-foot">
        <button type="button" onClick={props.onClose}>
          Cancel
        </button>
        <button class="primary" type="submit" disabled={name === '' || !!wrong}>
          Make it
        </button>
      </div>
    </form>
  );
}

/** Opens a project's environments: from its own page, and where something says it has some. */
export const openEnvironments = (project: string) => openTab({ kind: 'environments', project });

/** What an environment last ran, in the daemon's words and when; where that was a run, the words lead to its page. */
function LastRun({ last }: { last: Last | undefined }) {
  const said = lastRunSaid(last, ago);
  const to = last?.run ? hrefOf({ kind: 'run', id: last.run }) : null;
  return to ? (
    <a class="run-page-link env-last-run" href={to} title="Open that run as a page of its own" onClick={(event) => {
        // Kept from the row it is in, which opens the environment: so it is opened here, beside what is open.
        event.stopPropagation();
        if (opens(to)) event.preventDefault();
      }}>
      {said}
    </a>
  ) : (
    <>{said}</>
  );
}
