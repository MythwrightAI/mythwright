// Measurements: how often things happen, how often they fail, how long
// they take. Every number here is something the daemon measured as it
// worked; nothing is sampled by the client.

import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Series, Stats } from '../api.ts';
import { amount, asks, changed, compare, failedChange, narrowedTo, narrowings, split, standing } from '../compare.ts';
import type { Choice, Labels } from '../compare.ts';
import { coalesce } from '../events.ts';
import { only as projectsOf } from '../narrowing.ts';
import { shown } from '../narrowed.ts';
import { comparing } from '../store.ts';
import { Pills, useEvents } from '../ui.tsx';
import { count, duration, megabytes, percent } from '../util.ts';
import { Trend } from './chart.tsx';
import type { Point } from './chart.tsx';
import { Comparisons } from './compared.tsx';
import { Narrow } from './narrow.tsx';
import { useKept } from './returned.tsx';

const WINDOWS = {
  '1h': { label: 'Last hour', ms: 3_600_000, step: '1m' },
  '24h': { label: 'Last 24 hours', ms: 86_400_000, step: '15m' },
  '7d': { label: 'Last 7 days', ms: 7 * 86_400_000, step: '2h' },
} as const;
type Window = keyof typeof WINDOWS;

interface Loaded {
  /** Whose it is: the projects it was read for, or `*` for everything. What is drawn says so (`data-of`), since it is drawn a moment after it is chosen. */
  of: string;
  at: number;
  runs: Series[];
  runTrend: Series[];
  steps: Series[];
  http: Series[];
  httpTrend: Series[];
  turns: Series[];
  cost: Series[];
  health: Series[];
  router: Series[];
  polls: Series[];
  stepTotal: Series[];
  turnTotal: Series[];
  load: Series[];
  memory: Series[];
  custom: Series[];
  /** The labels blueprints gave to what was measured in this window. */
  labels: Labels;
  /** What was being compared when this was read, and everything split that way. */
  asked: Choice;
  compared: Series[];
}

async function load(window: Window, projects: string[] | null, choice: Choice): Promise<Loaded> {
  const since = window;
  const step = WINDOWS[window].step;
  // For some projects, only what was measured for them is asked for: the
  // daemon is told each (`project:a,project:b`: a label said several times
  // over is any of them), since it is the daemon that can add up what is
  // typical of them together. Requests to the daemon and the state of the
  // device are nobody's in particular, and are left to the view of
  // everything. And of no project at all (a workspace that has none)
  // nothing was measured, and nothing is asked: no label is everything.
  const none = projects?.length === 0;
  const metrics = (params: Parameters<typeof api.metrics>[0]) => {
    if (!projects) return api.metrics(params);
    if (none || /^(http|device\.)/.test(params.name)) return Promise.resolve([] as Series[]);
    return api.metrics({ ...params, labels: narrowedTo(projects, '') });
  };
  // What there is to compare by is read with the rest, and when something is
  // compared by, what is under each of its values. A label that is gone by
  // the time this is drawn is one the choice no longer stands on.
  // A daemon too old to say what labels there are has none to compare by;
  // the rest of what it measured is shown all the same.
  const labelled = none ? Promise.resolve({} as Labels) : api.metricLabels({ since, labels: narrowedTo(projects, '') }).catch(() => ({}) as Labels);
  const split = choice.by && !none ? Promise.all(asks(choice.by).map((ask) => api.metrics({ ...ask, since, labels: narrowedTo(projects, choice.only) }))) : Promise.resolve([]);
  const [runs, runTrend, steps, http, httpTrend, turns, cost, health, router, polls, stepTotal, turnTotal, load, memory, all] = await Promise.all([
    metrics({ name: 'run', since, by: 'blueprint,flow' }),
    metrics({ name: 'run', since, step }),
    metrics({ name: 'step', since, by: 'blueprint,step,action' }),
    metrics({ name: 'http', since, by: 'method,route' }),
    metrics({ name: 'http', since, step }),
    metrics({ name: 'agent.turn', since, by: 'slot,kind' }),
    metrics({ name: 'agent.cost_usd', since }),
    metrics({ name: 'service.health', since, by: 'service' }),
    metrics({ name: 'router.connection', since, by: 'router,backend' }),
    metrics({ name: 'poll', since, by: 'blueprint,handler' }),
    metrics({ name: 'step', since }),
    metrics({ name: 'agent.turn', since }),
    metrics({ name: 'device.load_per_cpu', since, step }),
    metrics({ name: 'device.free_mem_mb', since, step }),
    metrics({ name: '*', since, by: 'blueprint,step' }),
  ]);
  // Whatever blueprints measure under names of their own (`measure: build`).
  const builtin = /^(run|step|http|poll)$|^(agent|service|router|device)\./;
  const custom = all.filter((series) => !builtin.test(series.name));
  const [labels, answers] = await Promise.all([labelled, split]);
  return { of: projects ? projects.join(',') : '*', at: Date.now(), runs, runTrend, steps, http, httpTrend, turns, cost, health, router, polls, stepTotal, turnTotal, load, memory, custom, labels, asked: choice, compared: answers.flat() };
}

export function Metrics() {
  // Everything, or only what was measured for the projects what was chosen
  // comes to: those of the workspace the side panel is of, or one of them.
  // Until it is known which workspace that is, nothing is read.
  const of = shown.value;
  const projects = projectsOf(of);
  // Requests to the daemon and the device's own figures are shown with everything, and not before it is known that everything is shown.
  const project = of.kind !== 'everything';
  const asked = useRef(0);
  const [window, setWindow] = useKept<Window>('metrics.window', '1h');
  // What was read is shown at once when its tab shows again, and read again behind: it says whose it is (`data-of`).
  // Only for the projects it is of now: kept for others, it would be shown as this workspace's until read again.
  const [data, setData] = useKept<Loaded | null>('metrics.data', null);
  const ours = projects ? projects.join(',') : '*';
  if (data && data.of !== ours && asked.current === 0) setData(null);
  const [stale, setStale] = useState(false);
  // What is asked for is what was chosen; what is drawn is as much of the
  // choice as the labels there are now still allow.
  const { by, only } = comparing.value;
  const choice = data ? standing(data.labels, { by, only }) : { by, only };

  const reload = useMemo(
    () => () => {
      if (of.kind === 'waiting') return;
      // What was asked for last is what is drawn: an answer for what was shown before, in after it, is not.
      const mine = ++asked.current;
      setStale(true);
      void load(window, projects, { by, only })
        .then((read) => asked.current === mine && setData(read))
        .catch(() => {})
        .finally(() => asked.current === mine && setStale(false));
    },
    [window, of, by, only],
  );
  useEffect(reload, [reload]);
  // Read again when work finishes, at most every fifteen seconds.
  const later = useMemo(() => coalesce(reload, 15_000), [reload]);
  useEvents(['run.finished', 'agent.turn.finished'], later, [later], later);

  // Totals come from the daemon, which can combine percentiles properly.
  const runs = data?.runTrend[0]?.total;
  const steps = data?.stepTotal[0]?.total;
  const http = data?.httpTrend[0]?.total;
  const turns = data?.turnTotal[0]?.total;
  const since = (data?.at ?? Date.now()) - WINDOWS[window].ms;

  return (
    <div>
      <div class="view-head">
        <h1>Measurements</h1>
        <Narrow />
        <Pills value={window} options={Object.entries(WINDOWS).map(([key, value]) => [key as Window, value.label] as [Window, string])} onChange={setWindow} />
        {data && <Comparing labels={data.labels} choice={choice} />}
        <span class="grow" />
        <button onClick={reload}>Refresh</button>
      </div>
      <div class="view-body stack measured" data-of={data?.of} style={`gap:20px;transition:opacity .15s;opacity:${stale && data ? 0.6 : 1}`}>
        {/* What is being compared by the daemon itself: each way with its counts, side by side. */}
        {of.kind !== 'waiting' && <Comparisons projects={projects} />}
        {/* Drawn from what was read, which says what it is of: a choice just made is not yet what is shown. */}
        {data && choice.by && data.asked.by && <SideBySide choice={data.asked} series={data.compared} />}
        <div class="tiles">
          <Tile label="Runs" value={runs ? count(runs.count) : '0'} note={runs ? `${percent(1 - runs.error_rate)} succeeded` : 'none yet'} />
          <Tile label="Run time, typical" value={runs ? duration(runs.p50) : '-'} note={runs ? `95% within ${duration(runs.p95)}` : ''} />
          <Tile label="Steps" value={steps ? count(steps.count) : '0'} note={steps ? `${count(steps.errors)} failed` : 'none yet'} />
          {!project && <Tile label="Requests" value={http ? count(http.count) : '0'} note={http ? `95% within ${duration(http.p95)}` : ''} />}
          <Tile
            label="Agent turns"
            value={turns ? count(turns.count) : '0'}
            note={data?.cost[0] ? `$${(data.cost[0].total.avg * data.cost[0].total.count).toFixed(2)} spent` : turns ? `typically ${duration(turns.p50)}` : 'none yet'}
          />
        </div>

        <div class="charts">
          <Chart title="Run time" note="average per run" series={data?.runTrend} pick={(stats) => stats.avg} format={duration} since={since} until={data?.at} />
          {!project && (
            <>
              <Chart title="Request time" note="average per request to the daemon" series={data?.httpTrend} pick={(stats) => stats.avg} format={duration} since={since} until={data?.at} />
              <Chart title="Load" note="one-minute load per CPU on this device; above 1 it is saturated" series={data?.load} pick={(stats) => stats.avg} format={(value) => value.toFixed(2)} since={since} until={data?.at} />
              <Chart title="Free memory" note="on this device" series={data?.memory} pick={(stats) => stats.avg} format={megabytes} since={since} until={data?.at} />
            </>
          )}
        </div>

        <Table title="Steps" note="what blueprints do, by step" series={data?.steps} keys={['blueprint', 'step', 'action']} />
        {data && data.custom.length > 0 && <Table title="Named measurements" note="steps that carry `measure:`" series={data.custom} keys={['blueprint', 'step']} named />}
        <Table title="Runs" note="by blueprint and flow" series={data?.runs} keys={['blueprint', 'flow']} />
        {!project && <Table title="Requests" note="to the daemon, by route" series={data?.http} keys={['method', 'route']} />}
        {data && data.turns.length > 0 && <Table title="Agent turns" series={data.turns} keys={['slot', 'kind']} />}
        {data && data.health.length > 0 && <Table title="Health checks" series={data.health} keys={['service']} />}
        {data && data.router.length > 0 && <Table title="Routed connections" note="how long each was held open" series={data.router} keys={['router', 'backend']} />}
        {data && data.polls.length > 0 && <Table title="Polls" note="each one is a blueprint asking the outside world a question" series={data.polls} keys={['blueprint', 'handler']} />}
      </div>
    </div>
  );
}

/** Choosing what to set side by side. Nothing, where no blueprint has labelled anything. */
function Comparing({ labels, choice }: { labels: Labels; choice: Choice }) {
  const names = Object.keys(labels).sort();
  if (names.length === 0) return null;
  const others = choice.by ? narrowings(labels, choice.by) : [];
  return (
    <>
      <span class="muted">Compare by</span>
      <select
        class="narrow"
        aria-label="Compare by"
        title="Set what was measured side by side, by a label a blueprint gave it"
        value={choice.by}
        onChange={(event) => (comparing.value = { by: event.currentTarget.value, only: '' })}
      >
        <option value="">nothing</option>
        {names.map((name) => (
          <option key={name} value={name}>
            {name}
          </option>
        ))}
      </select>
      {others.length > 0 && (
        <>
          <span class="muted">where</span>
          <select
            class="narrow"
            aria-label="Only where"
            title="Compare only what carries one value of another label"
            value={choice.only}
            onChange={(event) => (comparing.value = { by: choice.by, only: event.currentTarget.value })}
          >
            <option value="">anything</option>
            {others.map(({ label, values }) => (
              <optgroup key={label} label={label}>
                {values.map((value) => (
                  <option key={value} value={`${label}:${value}`}>
                    {label} is {value}
                  </option>
                ))}
              </optgroup>
            ))}
          </select>
        </>
      )}
    </>
  );
}

const FIGURES = ['Count', 'Failed', 'Average', 'Typical', '95% within'];

/** The same measurements under each value of a label, a group of columns a value. */
function SideBySide({ choice, series }: { choice: Choice; series: Series[] }) {
  const { values, rows } = compare(series, choice.by);
  const [onlyLabel, onlyValue] = split(choice.only);
  const [first, second] = values;
  const two = values.length === 2;
  return (
    <div class="panel compare">
      <div class="panel-head">
        <h2>By {choice.by}</h2>
        <span class="muted truncate">
          what was measured under each {choice.by}
          {onlyLabel && `, where ${onlyLabel} is ${onlyValue}`}
        </span>
      </div>
      <div class="compare-scroll">
        <table>
          <thead>
            <tr>
              <th rowSpan={2}>Measured</th>
              {values.map((value) => (
                <th key={value} colSpan={FIGURES.length} class="compare-value">
                  {value}
                </th>
              ))}
              {two && (
                <th rowSpan={2} class="num compare-edge">
                  Difference
                  <div class="compare-which">
                    {second} against {first}
                  </div>
                </th>
              )}
            </tr>
            <tr>
              {values.map((value) =>
                FIGURES.map((figure, index) => (
                  <th key={`${value} ${figure}`} class={index === 0 ? 'num compare-edge' : 'num'}>
                    {figure}
                  </th>
                )),
              )}
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={`${row.name} ${row.of}`}>
                {/* A measure as a whole, and under it the steps or flows it is made of. */}
                <td class={row.of ? 'compare-part' : undefined}>{row.of || <strong>{row.title}</strong>}</td>
                {row.cells.flatMap((cell, index) =>
                  cell
                    ? [
                        <td key={`${index} count`} class="num compare-edge">
                          {count(cell.count)}
                        </td>,
                        <td key={`${index} failed`} class="num">
                          <Failed stats={cell} />
                        </td>,
                        <td key={`${index} average`} class="num">
                          {amount(cell.avg, row.unit)}
                        </td>,
                        <td key={`${index} typical`} class="num">
                          {amount(cell.p50, row.unit)}
                        </td>,
                        <td key={`${index} most`} class="num">
                          {amount(cell.p95, row.unit)}
                        </td>,
                      ]
                    : [
                        <td key={index} colSpan={FIGURES.length} class="num faint compare-edge">
                          not measured
                        </td>,
                      ],
                )}
                {two && (
                  <td class="num compare-edge">
                    {row.difference ? (
                      <>
                        {changed(row.difference, row.unit)}
                        {failedChange(row.difference) && <div class="faint">{failedChange(row.difference)}</div>}
                      </>
                    ) : (
                      <span class="faint">-</span>
                    )}
                  </td>
                )}
              </tr>
            ))}
            {rows.length === 0 && (
              <tr>
                <td class="faint">Nothing in this window was measured under a {choice.by}.</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function Failed({ stats }: { stats: Stats }) {
  return stats.errors > 0 ? (
    <span class="badge failed">
      {count(stats.errors)} ({percent(stats.error_rate)})
    </span>
  ) : (
    <span class="faint">none</span>
  );
}

function Tile({ label, value, note }: { label: string; value: string; note: string }) {
  return (
    <div class="tile">
      <span class="muted">{label}</span>
      <span class="value">{value}</span>
      <span class="faint">{note || ' '}</span>
    </div>
  );
}

function Chart(props: {
  title: string;
  note: string;
  series: Series[] | undefined;
  pick: (stats: Stats) => number;
  format: (value: number) => string;
  since: number;
  until: number | undefined;
}) {
  const points: Point[] = (props.series?.[0]?.points ?? []).map(([at, stats]) => ({ at, value: props.pick(stats) }));
  return (
    <div class="panel">
      <div class="panel-head">
        <h2>{props.title}</h2>
        <span class="muted truncate">{props.note}</span>
      </div>
      <div style="padding:8px 6px 4px">
        <Trend points={points} since={props.since} until={props.until ?? Date.now()} format={props.format} label={props.title} />
      </div>
    </div>
  );
}

function Table(props: { title: string; note?: string; series: Series[] | undefined; keys: string[]; named?: boolean }) {
  const rows = [...(props.series ?? [])].sort((a, b) => b.total.count * b.total.avg - a.total.count * a.total.avg).slice(0, 40);
  const slowest = Math.max(1, ...rows.map((row) => row.total.p95));
  return (
    <div class="panel">
      <div class="panel-head">
        <h2>{props.title}</h2>
        {props.note && <span class="muted">{props.note}</span>}
      </div>
      {/* Scrolled across in itself where it is wider than the page, as a comparison's columns are. */}
      <div class="measured-scroll">
        <table>
          <thead>
            <tr>
              {props.named && <th>Name</th>}
              {props.keys.map((key) => (
                <th key={key}>{key}</th>
              ))}
              <th class="num">Count</th>
              <th class="num">Failed</th>
              <th class="num">Average</th>
              <th class="num">Typical</th>
              <th class="num" style="width:170px">
                95% within
              </th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row, index) => (
              <tr key={index}>
                {props.named && (
                  <td>
                    <strong>{row.name}</strong>
                  </td>
                )}
                {props.keys.map((key) => (
                  <td key={key} class="truncate" style="max-width:240px">
                    {row.labels[key]}
                  </td>
                ))}
                <td class="num">{count(row.total.count)}</td>
                <td class="num">
                  <Failed stats={row.total} />
                </td>
                <td class="num">{duration(row.total.avg)}</td>
                <td class="num">{duration(row.total.p50)}</td>
                <td class="num">
                  <div class="row" style="justify-content:flex-end">
                    <div class="bar" style="width:70px" aria-hidden="true">
                      <i class="measure" style={`width:${Math.max(2, (row.total.p95 / slowest) * 100)}%`} />
                    </div>
                    <span style="width:62px">{duration(row.total.p95)}</span>
                  </div>
                </td>
              </tr>
            ))}
            {rows.length === 0 && (
              <tr>
                <td colSpan={props.keys.length + 5 + (props.named ? 1 : 0)} class="faint">
                  {props.series ? 'Nothing was measured in this window.' : 'Reading'}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
