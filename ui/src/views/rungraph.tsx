// A run as a graph, read like a CI run: the graph of the text it ran, each
// tile with a mark of how its step went and how long it took, the long
// straight chains folded into a tile each (`graph/folds.ts`), and a step
// pressed opening what it wrote in the drawer beside the graph. It follows
// the steps under way as the run goes, else those that failed, at the size
// they are drawn; a run that went well is drawn whole, no smaller than its
// words are read at, from its start.
//
// Nothing here reads anything on a timer. How each step stands is the run
// as its view read it, read again by that view as the daemon says
// something of it (`run.*`, `step.*`); while a step is under way, how long
// it has gone on is the page's own clock, drawn again once a second and
// asking the daemon nothing (`useSeconds`). The drawer reads the run again
// itself, the same way, so it follows the step while it is open.
//
// `LatestRun` is the same graph for the newest of a list of runs, read and
// followed by itself: what a told line's runs show above their list.

import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Graph, GraphNode, Run } from '../api.ts';
import { coalesce } from '../events.ts';
import { keptFrom, letGo } from '../gave.ts';
import { chainsOf, ranOf } from '../graph/folds.ts';
import { says } from '../graph/tile.ts';
import { GraphView } from '../graph/view.tsx';
import { graphOf } from '../runread.ts';
import { drawer } from '../store.ts';
import { useEvents } from '../ui.tsx';
import { baseStep, classes } from '../util.ts';
import { useBesideDrawer } from './besidedrawer.ts';
import { useKept } from './returned.tsx';
import { StepOut } from './stepout.tsx';

/** What is said under a run's graph: how it is moved, and what a press on a step does; and on a fold, where it has one. */
const RUN_GRAPH_SAYS = ['drag to move', 'scroll to zoom', 'click a step for its output'];
const FOLD_SAYS = 'click a fold to open it';
/** The least a run's whole graph is drawn at, so that its tiles' words are read: past that, it is drawn from its start. */
const LEAST = 0.8;

/** Which view a run's page opens on, as this browser last looked at one: a run opens where the last was looked at. */
const VIEW_KEPT = 'client.run.view';

/** The view a run's page opens on: the one last chosen in this browser, else its stages. */
export function runViewKept(): 'stages' | 'graph' {
  try {
    return localStorage.getItem(VIEW_KEPT) === 'graph' ? 'graph' : 'stages';
  } catch {
    return 'stages';
  }
}

/** Keeps the view chosen of a run, for the next run opened. */
export function keepRunView(view: 'stages' | 'graph'): void {
  try {
    localStorage.setItem(VIEW_KEPT, view);
  } catch {
    /* not remembered */
  }
}

/**
 * The time now, drawn again each second while `going`, and not at all
 * otherwise: how long a step under way has gone on, counted by the page.
 */
function useSeconds(going: boolean): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!going) return;
    let timer: ReturnType<typeof setTimeout>;
    const tick = () => {
      setNow(Date.now());
      timer = setTimeout(tick, 1000 - (Date.now() % 1000));
    };
    tick();
    return () => clearTimeout(timer);
  }, [going]);
  return going ? now : Date.now();
}

/** A run drawn as the graph of the text it ran, lit by how each step went; a step pressed opens its output in the drawer. */
export function RunGraph({ run, graph, where }: { run: Run; graph: Graph; where?: string }) {
  // Laid out in the room left of the drawer, so that the graph is beside what a step wrote and not under it.
  const beside = useRef<HTMLDivElement>(null);
  useBesideDrawer(beside);
  const ran = useMemo(() => ranOf(run), [run]);
  const legend = useMemo(() => (chainsOf(graph).length > 0 ? [...RUN_GRAPH_SAYS, FOLD_SAYS] : RUN_GRAPH_SAYS), [graph]);
  const going = run.status === 'running';
  const now = useSeconds(going && Object.values(ran).some((one) => one.finished === null));
  const statuses = useMemo(() => Object.fromEntries(Object.entries(ran).map(([id, one]) => [id, one.status])), [ran]);
  // What is followed: the steps under way, else those that failed, at the size they are drawn; else the whole.
  const under = Object.keys(ran).filter((id) => ran[id]!.status === 'running');
  const followed = under.length > 0 ? under : Object.keys(ran).filter((id) => ran[id]!.status === 'failed');
  // Which folds the reader opened or shut, kept by the tab, by run.
  const [open, setOpen] = useKept<Record<string, boolean>>(`rungraph.open:${run.id}`, {});
  const mine = useRef<unknown>(null);
  const [chosen, setChosen] = useState<string | null>(null);
  const select = (node: GraphNode | null) => {
    if (!node) {
      if (drawer.value && drawer.value === mine.current) drawer.value = null;
      return;
    }
    setChosen(node.id);
    drawer.value = mine.current = {
      // Headed by what the step says of itself, as its tile is; its key is under that, in the drawer.
      title: says(node.description) || node.name,
      body: () => <StepDrawer key={`${run.id}/${node.id}`} run={run.id} node={node} graph={graph} />,
    };
  };
  // The tile whose output is in the drawer is marked while the drawer is this graph's.
  const selected = drawer.value && drawer.value === mine.current ? chosen : null;
  return (
    <div ref={beside} class={classes('run-graph', where)} data-run={run.id}>
      <div class="pipeline-graph">
        <GraphView
          graph={graph}
          statuses={statuses}
          ran={ran}
          going={going}
          now={now}
          folding={{ open, choose: (fold, shown) => setOpen({ ...open, [fold]: shown }) }}
          selected={selected}
          onSelect={select}
          legend={legend}
          focus={followed}
          least={LEAST}
        />
      </div>
    </div>
  );
}

/** What a step of a run wrote, and how it stands, in the drawer: read once, and again as the daemon says something of the run. */
function StepDrawer({ run, node, graph }: { run: string; node: GraphNode; graph: Graph }) {
  const [read, setRead] = useState<Run | null>(null);
  const [failed, setFailed] = useState('');
  const reload = useMemo(
    () =>
      coalesce(
        () =>
          void api
            .run(run)
            .then((one) => {
              setFailed('');
              setRead(one);
            })
            .catch((err) => setFailed(err instanceof Error ? err.message : String(err))),
        80,
      ),
    [run],
  );
  useEffect(reload, [reload]);
  useEvents(['run.*', 'step.*'], (event) => event.source?.run === run && reload(), [run], reload);
  if (!read) return <div class="empty">{failed || 'Reading the step'}</div>;
  const steps = (read.steps ?? []).filter((one) => baseStep(one.step) === node.name);
  const kept = keptFrom(read) !== null || letGo(read) !== null;
  return (
    <div class="step-drawer stack" data-step={node.name}>
      {says(node.description) && <code class="faint">{node.name}</code>}
      {kept && <p class="faint">{keptFrom(read) ?? letGo(read)}</p>}
      {steps.length === 0 ? (
        <p class="faint step-drawer-none">{read.status === 'running' ? 'It has not begun yet.' : 'It did not run.'}</p>
      ) : (
        steps.map((one) => <StepOut key={one.step} run={read} step={one} graph={graph} kept={kept} />)
      )}
    </div>
  );
}

/** The newest of some runs drawn as its graph, read and followed by itself; nothing where its text cannot be drawn. */
export function LatestRun({ runs }: { runs: readonly { id: string; started: number }[] }) {
  const latest = runs.reduce<{ id: string; started: number } | null>((newest, one) => (newest === null || one.started > newest.started ? one : newest), null)?.id ?? null;
  // Kept by the tab, by run: shown again at once when the tab is, and read again behind.
  const [found, setFound] = useKept<{ run: Run; graph: Graph | null } | null>(`latestrun.read:${latest}`, null);
  const read = useMemo(
    () =>
      coalesce(async () => {
        if (!latest) return;
        try {
          const run = await api.run(latest);
          const detail = await api.instance(run.instance);
          setFound({ run, graph: await graphOf(run, detail) });
        } catch {
          /* gone, or not this reader's: the list under it says what there is */
        }
      }, 80),
    [latest],
  );
  useEffect(read, [read]);
  useEvents(['run.*', 'step.*'], (event) => event.source?.run === latest && read(), [latest], read);
  const shown = found && found.run.id === latest ? found : null;
  if (!shown?.graph) return null;
  return <RunGraph run={shown.run} graph={shown.graph} where="told-run-graph" />;
}
