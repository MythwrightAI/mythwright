// One run of any blueprint as a pipeline run, a page of its own with an
// address (`#/run/<id>`): what ran and how it stands, and its stages
// (`staged.ts`), each with its steps' status and time and the whole of what
// each step wrote, read a page at a time from its first line through the
// run's log route. A `call` stage unfolds into its child's run, in place.
//
// It reads once and follows what the daemon says of the run (`run.*` and
// `step.*`, read again where the stream says it fell behind: those are
// passed on and not kept) and the lines its steps write (`log.line`).
// Nothing is worked out here that `staged.ts` does not say. What it read
// is kept by its tab (`useKept`), so that a tab shown again is as it was.

import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'preact/hooks';
import { hrefOf, linkTo } from '../address.ts';
import { api, ApiError } from '../api.ts';
import type { Graph, Instance, LogLine, Run, Step } from '../api.ts';
import { coalesce } from '../events.ts';
import { keptFrom, letGo } from '../gave.ts';
import { namesOf } from '../envsheld.ts';
import { underSaid } from '../projectenvs.ts';
import { tell } from '../running.ts';
import { graphOf, logBetween, runIs, runsOf, tailLog } from '../runread.ts';
import { stepSays } from '../runsaid.ts';
import { childAt, childRun, drawnOf, earlierAsked, earlierSaid, earlierThan, failedAt, stageCalled, stagesOf, stageTook, WINDOW } from '../staged.ts';
import { scrollsItself } from '../returned.ts';
import { Badge, Empty, Pills, useEvents } from '../ui.tsx';
import { baseStep, classes, duration, when } from '../util.ts';
import { GraphView } from '../graph/view.tsx';
import { copy } from './pagehead.tsx';
import { useKept, useKeptRef } from './returned.tsx';
import { Running, WithRunning } from './running.tsx';
import { openTab } from '../store.ts';
import { signs } from './signs.tsx';
// What a run waits for a person to let it go on with, drawn under its head: Approve and Decline, by the run.
import { RunApproval as RunAsks } from './gopart.tsx';

/** How a run is looked at: its stages, and where the text it ran can be had, its graph lit by how each step went. */
const VIEWS: ['stages' | 'graph', string][] = [
  ['stages', 'Stages'],
  ['graph', 'Graph'],
];

/** What is said under the graph: it is looked at here, and a step is read in its stage. */
const GRAPH_SAYS = ['drag to move', 'scroll to zoom'];

/** How deep a call is unfolded into its child's run, and its child's, in place: past it the child is a link. */
const DEEPEST = 5;

/** What the page read of its run: the run, what it is a run of, and the graph of the very text it ran. */
interface Read {
  run: Run;
  instance: Instance;
  graph: Graph | null;
  /** Who applied what it is a run of, as its versions say. */
  by: string | null;
}

async function readRun(id: string): Promise<Read> {
  const run = await api.run(id);
  const detail = await api.instance(run.instance);
  tell(detail.instance.id, { blueprint: detail.instance.blueprint, key: detail.instance.key, project: detail.instance.project });
  runIs(id, detail.instance.blueprint);
  const graph = await graphOf(run, detail);
  // The version it was running as this run began: who applied that.
  const versions = await api.versions(run.instance).catch(() => []);
  const by = versions.find((one) => one.applied <= run.started)?.by_name ?? null;
  return { run, instance: detail.instance, graph, by };
}

export function PipelineRun({ id }: { id: string }) {
  const [kept, setRead] = useKept<Read | null>('pipelinerun.read', null);
  const read = kept?.run.id === id ? kept : null;
  const [missing, setMissing] = useState('');
  const [view, setView] = useKept<'stages' | 'graph'>('pipelinerun.view', 'stages');
  // What is folded is kept by the tab, by run and stage, for a child's run too: everything starts unfolded, so that a run is read whole.
  const [folded, setFolded] = useKept<string[]>('pipelinerun.folded', []);
  const folds: Folds = { folded, set: setFolded };
  const reload = useMemo(
    () =>
      coalesce(
        () =>
          void readRun(id)
            .then((one) => {
              setMissing('');
              setRead(one);
            })
            .catch((err) => setMissing(err instanceof ApiError && err.status === 404 ? `There is no run ${id}, or it is not yours to read.` : err instanceof Error ? err.message : String(err))),
        80,
      ),
    [id],
  );
  useEffect(reload, [reload]);
  useEvents(['run.*', 'step.*'], (event) => event.source?.run === id && reload(), [id], reload);

  if (missing && !read) return <Empty title="This run cannot be shown">{missing}</Empty>;
  if (!read) return <div class="empty">Reading the run</div>;
  const { run, instance, graph, by } = read;
  const now = Date.now();
  const under = underSaid(instance.environment, namesOf(instance.project).length > 0);
  const handed = (name: string) => (typeof instance.params?.[name] === 'string' ? (instance.params[name] as string) : null);
  const [branch, commit] = [handed('branch'), handed('commit')];
  const began = run.event && typeof run.event === 'object' && 'topic' in run.event ? String((run.event as { topic: unknown }).topic) : null;
  const statuses = Object.fromEntries((run.steps ?? []).map((one) => [`${run.flow}/${baseStep(one.step)}`, one.status]));
  const link = linkTo({ kind: 'run', id });
  return (
    <div class="pipeline-run">
      <div class="view-head">
        <h1 class="grow truncate">{instance.blueprint}</h1>
        <Pills value={view} options={graph ? VIEWS : VIEWS.slice(0, 1)} onChange={setView} />
        {link && (
          <button type="button" class="ghost icon" aria-label="Copy link" title="Copy an address that opens this run" onClick={() => void copy(link, 'Copied. It opens this run for whoever is signed in to this factory here.')}>
            {signs.link()}
          </button>
        )}
      </div>
      <div class="view-body stack pipeline-body">
        <div class="pipeline-facts" data-status={run.status}>
          <Badge status={run.status} />
          <span>
            {run.flow === 'main' ? 'its steps' : `on ${run.flow.replace('on.', '')}`}, began {when(run.started)}
            {run.finished ? `, took ${duration(run.finished - run.started)}` : ', under way'}
          </span>
          <span class="faint">
            {began ? `set off by ${began}` : instance.parent ? 'started by ' : by ? `applied by ${by}` : 'applied'}
            {!began && instance.parent && <Running id={instance.parent} />}
          </span>
          {under && <span class="faint">{under}</span>}
          {branch && (
            <span class="faint">
              branch <code>{branch}</code>
            </span>
          )}
          {commit && (
            <span class="faint">
              commit <code>{commit.slice(0, 12)}</code>
            </span>
          )}
          <span class="faint">
            <Running id={instance.id} />
          </span>
        </div>
        {run.error && (
          <pre class="code-box error-text pipeline-error">
            <WithRunning text={run.error} />
          </pre>
        )}
        <RunAsks run={run.id} />
        {keptFrom(run) && <p class="faint">{keptFrom(run)}</p>}
        {letGo(run) && <p class="faint">{letGo(run)}</p>}
        {view === 'graph' && graph ? (
          <div class="pipeline-graph">
            <GraphView graph={graph} statuses={statuses} selected={null} onSelect={() => {}} legend={GRAPH_SAYS} />
          </div>
        ) : (
          <Stages run={run} graph={graph} now={now} depth={0} folds={folds} />
        )}
      </div>
    </div>
  );
}

/** The longest the stage that failed is held in view, in frames, and the least, while what is above it is laid out. */
const HELD_FRAMES = [600, 30];

/**
 * Brings what `find` finds into view and holds it there a frame at a time
 * while what is above it is still being read (each step's output comes a
 * moment after the run, and pushes what is under it down), until all of
 * it has been read or the reader scrolls, presses or types. Says how to
 * end it.
 */
function holdInView(find: () => Element | null): () => void {
  const ends = ['wheel', 'pointerdown', 'keydown', 'touchstart'];
  let [frame, frames] = [0, 0];
  const done = () => {
    cancelAnimationFrame(frame);
    for (const what of ends) window.removeEventListener(what, done, true);
  };
  const look = () => {
    const there = find();
    there?.scrollIntoView({ block: 'start' });
    frames += 1;
    const reading = there?.closest('.pipeline-run')?.querySelector('.step-log.faint') ?? null;
    if (frames >= HELD_FRAMES[0]! || (frames >= HELD_FRAMES[1]! && there && !reading)) done();
    else frame = requestAnimationFrame(look);
  };
  for (const what of ends) window.addEventListener(what, done, true);
  look();
  return done;
}

/** Which stages are folded, by run and stage, and what folds one. */
interface Folds {
  folded: string[];
  set: (next: string[]) => void;
}

/** A run's stages, each a band that unfolds into its steps; a call's into its child's run. */
function Stages({ run, graph, now, depth, folds }: { run: Run; graph: Graph | null; now: number; depth: number; folds: Folds }) {
  const stages = stagesOf(run, graph, now);
  const { folded, set: setFolded } = folds;
  const kept = keptFrom(run) !== null || letGo(run) !== null;
  // The stage whose failure is the run's is marked, and brought into view once as the tab first reads the run:
  // a run that failed is opened to read why. Kept by the tab, so that the tab shown again is where it was left.
  const failed = depth === 0 ? failedAt(run, stages) : null;
  const brought = useKeptRef<string | null>('pipelinerun.brought', null);
  const list = useRef<HTMLOListElement>(null);
  useLayoutEffect(() => {
    if (!failed || brought.current === run.id) return;
    brought.current = run.id;
    scrollsItself();
    return holdInView(() => list.current?.querySelector(`[data-stage="${CSS.escape(failed)}"]`) ?? null);
  }, [failed, run.id]);
  if (stages.length === 0) return <p class="faint pipeline-none">No step has started yet.</p>;
  return (
    <ol class={classes('pipeline-stages', depth > 0 && 'inner')} data-run={run.id} ref={list}>
      {stages.map((stage) => {
        const fold = `${run.id}/${stage.key}`;
        const open = !folded.includes(fold);
        const begun = stage.started !== null;
        return (
          <li
            key={stage.key}
            class={classes('pipeline-stage', stage.beside && 'beside', stage.key === failed && 'failed-at')}
            data-stage={stage.key}
            data-status={stage.status}
            aria-current={stage.key === failed ? 'step' : undefined}
          >
            <button type="button" class="stage-band" aria-expanded={begun ? open : undefined} disabled={!begun} onClick={() => setFolded(open ? [...folded, fold] : folded.filter((one) => one !== fold))}>
              <span class="stage-mark">
                <Badge status={stage.status === 'not run' ? 'skipped' : stage.status}>{stage.tolerated ? 'failed, let fail' : stage.status}</Badge>
              </span>
              <span class="stage-name">
                <span class="stage-called">{stageCalled(stage)}</span>
                <span class="stage-key">
                  {stage.says ? <code>{stage.key}</code> : null}
                  {stage.kind !== 'step' && <span class="faint"> {stage.kind === 'task' ? `${stage.inside.length} steps` : stage.kind}</span>}
                  {stage.beside && <span class="faint"> · beside the one before</span>}
                </span>
              </span>
              <span class="stage-when faint">{stage.started !== null ? when(stage.started) : ''}</span>
              <span class="stage-took">{stageTook(stage, now, duration)}</span>
              <span class="stage-bar" aria-hidden="true">
                <span style={{ width: `${Math.round(stage.share * 100)}%` }} />
              </span>
            </button>
            {open && begun && (
              <div class="stage-inside">
                {stage.kind === 'task' || stage.inside.length > 0 ? (
                  stage.inside.map((one) => <StepOut key={one.step} run={run} step={one} graph={graph} kept={kept} />)
                ) : stage.head && stage.kind === 'step' ? (
                  <StepOut run={run} step={stage.head} graph={graph} kept={kept} alone />
                ) : null}
                {(stage.kind === 'call' || stage.kind === 'spawn') && stage.head && (
                  <>
                    {stage.head.error && <pre class="code-box error-text step-error">{stage.head.error}</pre>}
                    <ChildRun parent={run.instance} child={stage.child} since={stage.started} depth={depth + 1} now={now} folds={folds} />
                  </>
                )}
              </div>
            )}
          </li>
        );
      })}
    </ol>
  );
}

/** One step of a stage: how it stands, how long it took, why it failed, and the whole of what it wrote. */
function StepOut({ run, step, graph, kept, alone }: { run: Run; step: Step; graph: Graph | null; kept: boolean; alone?: boolean }) {
  const says = stepSays(graph ? { graph, revision: run.revision } : null, run, step.step);
  return (
    <div class="step-out" data-step={step.step} data-status={step.status}>
      {!alone && (
        <div class="step-out-line">
          <Badge status={step.status}>{step.tolerated && step.status === 'failed' ? 'failed, let fail' : step.status}</Badge>
          <span class="step-out-name">
            {says && <span class="step-says">{says}</span>}
            <code class={says ? 'step-key' : undefined}>{step.step}</code>
          </span>
          <span class="step-out-took">{step.finished ? duration(step.finished - step.started) : ''}</span>
        </div>
      )}
      {step.error && <pre class="code-box error-text step-error">{step.error}</pre>}
      {kept ? null : <StepLog run={run.id} step={step.step} going={step.status === 'running'} />}
    </div>
  );
}

/** What of a step's output the page has: its last lines read and written since, how many it draws at most, and where its beginning was let go. */
interface Tail {
  lines: LogLine[];
  most: number;
  let_go: string | null;
}

/**
 * What a step wrote, drawn from its end a window at a time (`WINDOW`):
 * the last lines kept are read from the end of its file, those written
 * as it goes are added and the oldest drawn let go past the window, and
 * Show earlier reads the window before the first that is drawn. However
 * long the output, so many lines are drawn and no more.
 */
function StepLog({ run, step, going }: { run: string; step: string; going: boolean }) {
  // Kept by the tab, by run and step: shown again at once when the tab is, and read again behind.
  const [tail, setTail] = useKept<Tail | null>(`pipelinerun.log:${run}/${step}`, null);
  const most = tail?.most ?? WINDOW;
  const box = useRef<HTMLDivElement>(null);
  const [earlier, setEarlier] = useState(false);
  const read = useMemo(
    () =>
      coalesce(
        () =>
          void tailLog(run, step, most)
            .then((got) => setTail((was) => ({ lines: got.lines, let_go: got.let_go, most: was?.most ?? WINDOW })))
            .catch(() => setTail((was) => was ?? { lines: [], let_go: null, most: WINDOW })),
        60,
      ),
    [run, step, most],
  );
  useEffect(read, [run, step]);
  // A step that ends after the page read it has what it wrote at the end read too.
  useEffect(() => {
    if (!going) read();
  }, [going]);
  useEvents(
    'log.line',
    (event) => {
      if (event.source?.run !== run || event.source?.step !== step) return;
      setTail((known) => {
        const lines: LogLine[] = known?.lines ?? [];
        const now = known?.most ?? WINDOW;
        const added = [...lines, { line: (lines.at(-1)?.line ?? 0) + 1, ts: event.ts, stream: event.data.stream, text: event.data.text }];
        return { let_go: known?.let_go ?? null, most: now, lines: drawnOf(added, now) };
      });
    },
    [run, step],
    read,
  );
  // Read the first time, it shows its end: what was written last is what is read first.
  const first = useRef(tail === null);
  useLayoutEffect(() => {
    if (!first.current || !tail || !box.current) return;
    first.current = false;
    box.current.scrollTop = box.current.scrollHeight;
  }, [tail]);
  if (!tail) return <div class="log step-log faint">Reading what it wrote</div>;
  if (tail.lines.length === 0) return <p class="faint step-log-none">{tail.let_go ?? (going ? 'Nothing written yet.' : 'It wrote nothing.')}</p>;
  const before = earlierThan(tail.lines);
  const showEarlier = async () => {
    const asked = earlierAsked(tail.lines[0]!.line, WINDOW);
    if (!asked) return;
    setEarlier(true);
    const got = await logBetween(run, step, asked.after, asked.limit).catch(() => []);
    setEarlier(false);
    setTail((was) => (was ? { ...was, lines: [...got, ...was.lines], most: was.most + got.length } : was));
  };
  return (
    <div class="log step-log" ref={box} data-lines={tail.lines.length} data-first={tail.lines[0]!.line}>
      {tail.let_go && before === 0 && <div class="s">{tail.let_go}</div>}
      {before > 0 && (
        <button type="button" class="ghost link step-log-earlier" disabled={earlier} onClick={() => void showEarlier()}>
          {earlier ? 'Reading what came before' : earlierSaid(before, WINDOW)}
        </button>
      )}
      {tail.lines.map((line) => (
        <div key={line.line} class={line.stream === 'e' ? 'e' : line.stream === 's' ? 's' : ''}>
          {line.text || ' '}
        </div>
      ))}
    </div>
  );
}

/** What a call stage started, unfolded in place: the child's own steps' run that began as the stage did, staged as any run is. */
function ChildRun({ parent, child, since, depth, now, folds }: { parent: string; child: string | null; since: number | null; depth: number; now: number; folds: Folds }) {
  // Kept by the tab, by the run it was started from and when: shown again at once when the tab is, and read again behind.
  const [found, setFound] = useKept<{ run: Run; graph: Graph | null; instance: Instance } | null>(`pipelinerun.child:${parent}/${since}`, null);
  const [which, setWhich] = useKept<string | null>(`pipelinerun.which:${parent}/${since}`, child);
  const read = useMemo(
    () =>
      coalesce(async () => {
        try {
          // Where the step has not said yet which child it started, the first made under its parent once it began.
          const id = child ?? childAt((await api.instance(parent)).children, since);
          setWhich(id);
          if (!id) return;
          const runs = await runsOf(id);
          const one = childRun(runs, since);
          if (!one) return;
          const [run, detail] = await Promise.all([api.run(one.id), api.instance(id)]);
          setFound({ run, graph: await graphOf(run, detail), instance: detail.instance });
        } catch {
          /* the child went away, or is not this reader's: the line under the stage says which it was */
        }
      }, 80),
    [parent, child, since],
  );
  useEffect(read, [read]);
  useEvents(['run.*', 'step.*'], (event) => (event.source?.instance === which || (found && event.source?.run === found.run.id)) && read(), [which, found?.run.id], read);
  if (!which) return <p class="faint child-none">It has not started anything yet.</p>;
  return (
    <div class="child-run" data-child={which}>
      <p class="child-head faint">
        {found ? (
          <a class="run-page-link" href={hrefOf({ kind: 'run', id: found.run.id })}>
            {found.instance.blueprint}
          </a>
        ) : null}{' '}
        <Running id={which} />
      </p>
      {found && depth <= DEEPEST ? <Stages run={found.run} graph={found.graph} now={now} depth={depth} folds={folds} /> : !found ? <p class="faint">Its steps have not begun.</p> : null}
    </div>
  );
}

/**
 * Opens the page of what something running last ran: its own steps' latest
 * run, else its latest of any; its own page where it never ran. This is
 * what opening a run means where only what runs is named (a told line's
 * "What it is about", the fleet's drawer).
 */
export async function showLatestRun(instance: string): Promise<void> {
  const runs = await runsOf(instance).catch(() => []);
  const latest = childRun(runs, null) ?? runs[0] ?? null;
  openTab(latest ? { kind: 'run', id: latest.id } : { kind: 'instance', id: instance });
}
