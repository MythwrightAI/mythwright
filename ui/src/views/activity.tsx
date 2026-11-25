// What is running and what has run: instances as a tree, then recent runs,
// and for the factory as a whole, what it has to say for itself.

import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { FleetEvent, Instance, Run } from '../api.ts';
import { namesOf } from '../envsheld.ts';
import { coalesce } from '../events.ts';
import { anyNoted, noteOf } from '../noted.ts';
import { line } from '../log.ts';
import { only } from '../narrowing.ts';
import { shown } from '../narrowed.ts';
import { underSaid } from '../projectenvs.ts';
import { isBlueprint, openTab } from '../store.ts';
import { Badge, Empty, Pills, Prose, useEvents } from '../ui.tsx';
import { ago, duration, when } from '../util.ts';
import { Narrow } from './narrow.tsx';
import { showDetails } from './instance.tsx';
import { Running } from './running.tsx';

export function Activity() {
  // Everything, or only what was applied for the projects what was chosen
  // comes to: those of the workspace the side panel is of, or one of them.
  // Until it is known which workspace that is, nothing is read.
  const of = shown.value;
  const projects = only(of);
  const one = projects?.length === 1;
  const [instances, setInstances] = useState<Instance[] | null>(null);
  const [runs, setRuns] = useState<Run[]>([]);
  const [scope, setScope] = useState<'live' | 'all'>('live');

  // What was asked for last is what is drawn: an answer for what was shown before, in after it, is not.
  const asked = useRef(0);
  const reload = useMemo(
    () =>
      coalesce(() => {
        if (of.kind === 'waiting') return;
        const mine = ++asked.current;
        const latest = <T,>(set: (read: T) => void) => (read: T) => void (asked.current === mine && set(read));
        // A workspace with no project has nothing applied for it, and is asked nothing: no project named is everything.
        if (projects?.length === 0) return void (setInstances([]), setRuns([]));
        const project = projects ?? undefined;
        void api.instances({ live: scope === 'live', project, limit: 300 }).then(latest(setInstances)).catch(() => {});
        void api.runs({ project, limit: 40 }).then(latest(setRuns)).catch(() => {});
      }, 150),
    [scope, of],
  );
  useEffect(reload, [reload]);
  useEvents(['instance.*', 'run.started', 'run.finished'], reload, [reload], reload);
  // Whether an instance is behind is worked out when it is read, so it is read again when a blueprint's file is said to have changed.
  useEvents(['project.file.changed', 'project.rescanned'], (event) => (!event.data?.path || isBlueprint(event.data.path)) && reload(), [reload]);

  // The factory's own log: read once, and again when it says something.
  const [log, setLog] = useState<FleetEvent[]>([]);
  const readLog = useMemo(() => coalesce(() => void api.recentEvents({ topics: 'system.*', limit: 12 }).then((answer) => setLog(answer.events)).catch(() => {}), 150), []);
  useEffect(readLog, [readLog]);
  useEvents('system.*', readLog, [readLog]);

  const rows = useMemo(() => tree(instances ?? []), [instances]);
  const noted = anyNoted(instances ?? []);
  const byId = useMemo(() => new Map((instances ?? []).map((one) => [one.id, one])), [instances]);

  return (
    <div>
      <div class="view-head">
        <h1>Activity</h1>
        <Narrow />
        <Pills
          value={scope}
          options={[
            ['live', 'Running'],
            ['all', 'All'],
          ]}
          onChange={setScope}
        />
      </div>
      <div class="view-body stack" style="gap:20px">
        <div class="panel">
          <div class="panel-head">
            <h2>Instances</h2>
            <span class="muted">
              {!projects
                ? 'blueprints that have been applied, and the children they started'
                : of.kind === 'projects' && of.project
                  ? 'blueprints that have been applied for this project, and the children they started'
                  : 'blueprints that have been applied for the projects of this workspace, and the children they started'}
            </span>
          </div>
          {instances && rows.length === 0 ? (
            <Empty title={scope === 'live' ? 'Nothing is running' : 'Nothing has been applied yet'}>
              <span>
                Open a blueprint and apply it, or ask an agent to set up a fleet.
                {/* Said, so that an empty list is not taken for an empty factory. */}
                {projects && ' What was applied for no project of this workspace, and the factory\'s own log, are under All workspaces.'}
              </span>
            </Empty>
          ) : (
            <table class="instances">
              <thead>
                <tr>
                  <th>Blueprint</th>
                  <th>Status</th>
                  {noted && <th class="activity-note">Note</th>}
                  <th>Key</th>
                  {!one && <th>Project</th>}
                  <th>Holds</th>
                  <th>Updated</th>
                </tr>
              </thead>
              <tbody>
                {rows.map(({ instance, depth }) => (
                  <tr key={instance.id} class="clickable" onClick={() => openTab({ kind: 'instance', id: instance.id })}>
                    <td style={`padding-left:${10 + depth * 18}px`}>
                      {depth > 0 && <span class="faint">└ </span>}
                      <strong>{instance.blueprint}</strong>
                      {/* On a phone, where there is no room for a column of it, the note is under the name. */}
                      <Noted instance={instance} under />
                    </td>
                    <td>
                      {/* Its status and whether it is behind, with room between them: side by side where the column has it, one under the other where it has not (`activity.css`). */}
                      <span class="activity-status">
                        <Badge status={instance.status} />
                        {instance.behind && (
                          <span class="lag-mark" title={instance.behind.why}>
                            <Badge status="warning">behind</Badge>
                          </span>
                        )}
                      </span>
                    </td>
                    {noted && (
                      <td class="activity-note">
                        <Noted instance={instance} />
                      </td>
                    )}
                    <td>
                      <code>{instance.key}</code>
                      {underSaid(instance.environment, namesOf(instance.project).length > 0) && (
                        <div class="faint">{underSaid(instance.environment, namesOf(instance.project).length > 0)}</div>
                      )}
                    </td>
                    {!one && <td>{instance.project}</td>}
                    <td class="muted">{instance.holds.join(', ')}</td>
                    <td class="muted">{ago(instance.updated)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        <div class="panel">
          <div class="panel-head">
            <h2>Recent runs</h2>
          </div>
          <table>
            <thead>
              <tr>
                <th>Started</th>
                <th>Blueprint</th>
                <th>Flow</th>
                <th>Status</th>
                <th class="num">Took</th>
                <th>Result</th>
              </tr>
            </thead>
            <tbody>
              {runs.map((run) => (
                <tr key={run.id} class="clickable" data-run={run.id} title="Open this run as a page of its own" onClick={() => openTab({ kind: 'run', id: run.id })}>
                  <td class="muted">{when(run.started)}</td>
                  <td>{byId.get(run.instance)?.blueprint ?? <Running id={run.instance} plain />}</td>
                  <td>{run.flow === 'main' ? 'main' : run.flow.replace('on.', 'on ')}</td>
                  <td>
                    <Badge status={run.status} />
                  </td>
                  <td class="num">{run.finished ? duration(run.finished - run.started) : ''}</td>
                  <td class="error-text truncate" style="max-width:340px">
                    {run.error}
                  </td>
                </tr>
              ))}
              {runs.length === 0 && (
                <tr>
                  <td colSpan={6} class="faint">
                    No runs yet.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        {/* Of the factory and of no project, so it is not shown for some. */}
        {of.kind === 'everything' && (
          <div class="panel factory-log">
            <div class="panel-head">
              <h2>Log</h2>
              <span class="muted">what the factory has to say for itself: when it started and stopped, and why</span>
            </div>
            <table>
              <tbody>
                {log.map((event) => {
                  const said = line(event);
                  return (
                    <tr key={event.seq}>
                      <td class="muted">{when(event.ts)}</td>
                      <td>
                        <strong>{said.what}</strong>
                      </td>
                      <td>
                        <Prose text={said.more} />
                      </td>
                    </tr>
                  );
                })}
                {log.length === 0 && (
                  <tr>
                    <td class="faint">Nothing has been said yet.</td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}

/**
 * What an instance's row says beside its status, in one line cut short:
 * red for a failure, and for what it went on past the colour a warning is
 * drawn in. Pressed, its details are in the drawer, with the whole of it.
 */
function Noted({ instance, under }: { instance: Instance; under?: boolean }) {
  const note = noteOf(instance);
  if (!note) return null;
  const plain = note.says.replace(/`/g, '');
  return (
    <button
      class={`activity-said ${note.is}${under ? ' under' : ''}`}
      title={plain}
      aria-label={`${note.is === 'failure' ? 'Why it failed' : 'A warning'}: ${plain}`}
      onClick={(event) => {
        event.stopPropagation();
        showDetails(instance);
      }}
    >
      <span class="truncate">
        <Prose text={note.says} />
      </span>
    </button>
  );
}

/** Instances in display order: each parent followed by its children. */
export function tree(instances: Instance[]): { instance: Instance; depth: number }[] {
  const known = new Set(instances.map((one) => one.id));
  const children = new Map<string, Instance[]>();
  const roots: Instance[] = [];
  for (const one of instances) {
    if (one.parent && known.has(one.parent)) {
      if (!children.has(one.parent)) children.set(one.parent, []);
      children.get(one.parent)!.push(one);
    } else {
      roots.push(one);
    }
  }
  const out: { instance: Instance; depth: number }[] = [];
  const visit = (one: Instance, depth: number) => {
    out.push({ instance: one, depth });
    for (const child of children.get(one.id) ?? []) visit(child, depth + 1);
  };
  for (const root of roots) visit(root, 0);
  return out;
}
