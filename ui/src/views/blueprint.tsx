// A blueprint as a document: its text, its graph, and its runs.
//
// The text is the source of truth. The graph is drawn from the same check
// the editor's squiggles come from, so the two never disagree; when the
// blueprint is running, each node shows how its step went.

import { namesOf } from '../envsheld.ts';
import { checkingSays, checkSays, instanceSays, instanceWord } from '../pills_said.ts';
import { asked as askedOf } from '../projectenvs.ts';
import type { ComponentChildren } from 'preact';
import { lazy, Suspense } from 'preact/compat';
import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { appliedSaid, shownOnceApplied } from '../applied.ts';
import type { Mode } from '../applied.ts';
import { api } from '../api.ts';
import type { Checked, Diagnostic, Fields, GraphNode, Instance, Moved, Run } from '../api.ts';
import { applyingKey } from '../applyasked.ts';
import { CodeEditor } from '../editor/editor.tsx';
import { coalesce } from '../events.ts';
import { NodeDetails } from '../graph/details.tsx';
import { covers } from '../holds.ts';
import { runOf } from '../runsaid.ts';
import { besides, keepBeside, keepSandbox, sandboxes } from '../sandboxkept.ts';
import { defaultNamed } from '../forked.ts';
import { firstView, startsAt } from '../summary.ts';
import { says } from '../graph/tile.ts';
import { GraphView } from '../graph/view.tsx';
import { agents, attempt, callable, deviceNames, drawer, learnBlueprints, learnDevices, me, partMissing, projectName, toast } from '../store.ts';
import type { Drawer } from '../store.ts';
import { Badge, Empty, Pills, useEvents } from '../ui.tsx';
import { baseStep, duration, when } from '../util.ts';
import { moved } from '../versions.ts';
import { withSaid } from '../warned.ts';
import { Applying } from './applying.tsx';
import { compareWays } from './compared.tsx';
import { useForking } from './forking.tsx';
import { History } from './history.tsx';
import Preview from './preview.tsx';
import { RunDetail } from './run.tsx';
import { SummaryView } from './summary.tsx';
import { useKept, useKeptRef } from './returned.tsx';
import { Testing } from './testing.tsx';
import { useBesideDrawer } from './besidedrawer.ts';


// A sandbox test is run by whoever asks for one, and by nobody else: what runs and draws it is fetched when first shown.
const SandboxView = lazy(() =>
  import('./sandboxed.tsx').catch((err) => {
    partMissing(err);
    return { default: () => <Empty title="The sandbox test could not be loaded">Reload the page to carry on.</Empty> };
  }),
);

// Sandbox tests side by side, the same way: fetched when first shown.
const SideBySideView = lazy(() =>
  import('./sidebyside.tsx').catch((err) => {
    partMissing(err);
    return { default: () => <Empty title="Side by side could not be loaded">Reload the page to carry on.</Empty> };
  }),
);

export interface BlueprintDocProps {
  title: string;
  source: string;
  project: string | null;
  /** The file this is, when it is one; applying then applies the file. */
  path?: string;
  /** It is a blueprint that ships, as it ships: its Summary says how what it does is changed, and it can be forked. */
  ships?: boolean;
  /** Show this instance rather than looking for one running this blueprint. */
  instanceId?: string;
  readOnly?: boolean;
  dirty?: boolean;
  line?: number;
  /** Which asking for `line` this is: asked again while the page is showing, its text is shown there again. */
  lineAsked?: number;
  onChange?: (text: string) => void;
  onSave?: () => Promise<boolean>;
  /** Lays the text out the one way, where it is a text that can be changed. */
  onFormat?: () => void;
  /**
   * For a blueprint that was opened for no project: applying it asks which
   * project it is for, or none. Called with the project it was applied to.
   */
  asksProject?: (project: string) => void;
  /** Extra controls for the header. */
  children?: ComponentChildren;
}

const STEP_STATUS: Record<string, string> = {
  'step.started': 'running',
  'step.retrying': 'running',
  'step.waiting': 'running',
  'step.failed': 'failed',
  'step.skipped': 'skipped',
  'step.cancelled': 'cancelled',
};

const MODES = 'client.blueprint.view';
const KNOWN: Mode[] = ['summary', 'text', 'node', 'runs', 'history'];

/** The view of a blueprint that was last looked at, if one was kept. */
function lastMode(key: string): Mode | null {
  try {
    const all = JSON.parse(localStorage.getItem(MODES) ?? '{}') as Record<string, unknown>;
    const mode = all[key];
    return KNOWN.includes(mode as Mode) ? (mode as Mode) : null;
  } catch {
    return null;
  }
}

function keepMode(key: string, mode: Mode, usual: Mode): void {
  try {
    const all = JSON.parse(localStorage.getItem(MODES) ?? '{}') as Record<string, Mode>;
    // Where this blueprint starts for whoever is looking need not be kept; that keeps the list short.
    if (mode === usual) delete all[key];
    else all[key] = mode;
    localStorage.setItem(MODES, JSON.stringify(all));
  } catch {
    /* not remembered */
  }
}

export function BlueprintDoc(props: BlueprintDocProps) {
  const { source, project } = props;
  // Which of its views was last looked at is kept, for each blueprint, so
  // that it is the one shown when its tab comes back or the page is read
  // again. The view is only on the page while its tab is showing.
  const kept = props.instanceId ? `instance:${props.instanceId}` : `${project ?? ''}:${props.path ?? props.title}`;
  // Where it starts is what it says in plain words, for a blueprint that
  // ships and for whoever may not change its file; the text for whoever
  // may, who came to write; and the graph of something running.
  const starts = { running: !!props.instanceId, changes: !props.readOnly && covers(me.value?.principal.scopes ?? [], 'projects:write') };
  const [mode, setShown] = useState<Mode>(() => firstView(lastMode(kept), { ...starts, at: !!props.line }));
  const show = (next: Mode) => {
    setShown(next);
    keepMode(kept, next, startsAt(starts));
  };
  // How many times somebody has chosen one of its views: an apply that
  // answers after they have leaves what they chose showing (`applied.ts`).
  const pressed = useRef(0);
  const setMode = (next: Mode) => {
    pressed.current += 1;
    show(next);
  };
  // What was read, and which run, step and tile were chosen, are as they were left when its tab shows again; what
  // was read is read again behind.
  const [checked, setChecked] = useKept<Checked | null>('blueprint.checked', null);
  const [instance, setInstance] = useKept<Instance | null>('blueprint.instance', null);
  const [runs, setRuns] = useKept<Run[]>('blueprint.runs', []);
  const [statuses, setStatuses] = useKept<Record<string, string>>('blueprint.statuses', {});
  const [selected, setSelected] = useKept<string | null>('blueprint.selected', null);
  const [shownRun, setShownRun] = useKept<string | null>('blueprint.run', null);
  const [shownStep, setShownStep] = useKept<string | null>('blueprint.step', null);
  const [goTo, setGoTo] = useState({ line: props.line ?? 0, tick: 0 });
  // What it is applied with is asked in the drawer, beside the text. The
  // drawer is the frame's, and is put away by a key, by its own button and
  // by another tab being shown: it is this one's for as long as what is in
  // it is what was put there from here.
  // Kept by its tab with the drawer, which is given back to it when it shows again.
  const asked = useKeptRef<Drawer | null>('blueprint.asked', null);
  // The drawer is read whatever was asked, so that this is drawn again when it changes: read only once something
  // was asked, the drawing that asked never heard of it, and pressing another view put what it asked away.
  const shown = drawer.value;
  const asking = asked.current !== null && shown === asked.current;
  // Its preview is in the drawer too, kept beside whichever of its views is read, and drawn again as the text is checked again.
  const previewed = useKeptRef<Drawer | null>('blueprint.previewed', null);
  const previewing = previewed.current !== null && shown === previewed.current;
  // What stays in the drawer as another of its views is chosen: what is asked of it, and its preview.
  const stays = asking || previewing;
  const [left, setLeft] = useState<{ texts: Moved[]; params?: Fields; where?: string | null } | null>(null);
  const [said, setSaid] = useState<Diagnostic[]>([]);
  // A sandbox test of the text is kept by the tab this is a view of, which the frame says on the view's box.
  // Its button is in the head from the first drawing, as the others are: one that came a moment later moved
  // them from under whoever was pressing one.
  const root = useRef<HTMLDivElement>(null);
  // Laid out within the room left of an open drawer, so that Testing and Apply are never under it.
  useBesideDrawer(root);
  const [tab, setTab] = useState('');
  const tabOf = () => root.current?.closest<HTMLElement>('[data-tab]')?.dataset.tab ?? kept;
  useEffect(() => setTab(tabOf()), []);
  const flows = useRef(new Map<string, string>());
  const details = useRef(new Map<string, Run>());

  // What the page has of forking (`forking.tsx`): its button, its dialogs, and where a line of the Summary leads.
  const applies = useRef<() => void>(() => {});
  const forking = useForking({ name: checked?.name ?? null, project, ships: !!props.ships, summary: checked?.summary, apply: () => applies.current() });
  // Check a moment after the text stops changing; draw from the answer.
  // The daemon is told what the text is (a file of the project, or what
  // ships), by which the Summary says where it stands; and is asked
  // again when a fork of it was made or taken away.
  useEffect(() => {
    let stale = false;
    const is = props.instanceId ? undefined : props.path ? { path: props.path } : props.ships ? { ships: true } : undefined;
    const timer = setTimeout(
      () => void api.check(source, project, is).then((answer) => !stale && setChecked(answer)).catch(() => {}),
      checked ? 250 : 0,
    );
    return () => {
      stale = true;
      clearTimeout(timer);
    };
  }, [source, project, forking.stood]);

  useEffect(() => {
    if (props.line) setGoTo((now) => ({ line: props.line!, tick: now.tick + 1 }));
  }, [props.line]);

  // What the editor offers that is in neither the language nor this text:
  // the blueprints that can be called from here, the agents and the devices.
  useEffect(() => {
    void learnBlueprints(project);
    void learnDevices();
  }, [project]);
  const world = useMemo(
    () => () => ({
      blueprints: callable.value[project ?? ''] ?? [],
      agents: agents.value.map((agent) => agent.name),
      devices: deviceNames.value,
    }),
    [project],
  );

  // Which instance is running this, and how its latest runs went.
  const name = checked?.name ?? null;
  const find = useMemo(
    () =>
      coalesce(async () => {
        try {
          let found: Instance | null = null;
          if (props.instanceId) found = (await api.instance(props.instanceId)).instance;
          else if (name) {
            const all = await api.instances({ project: project ?? undefined });
            found = all.find((one) => one.blueprint === name && !one.parent && one.project === project) ?? null;
          }
          setInstance(found);
          const list = found ? await api.runs({ instance: found.id, limit: 30 }) : [];
          for (const run of list) flows.current.set(run.id, run.flow);
          setRuns(list);
          // The newest run of each flow says how each of its steps went.
          const newest = new Map<string, Run>();
          for (const run of list) if (!newest.has(run.flow)) newest.set(run.flow, run);
          const next: Record<string, string> = {};
          for (const run of newest.values()) {
            const kept = run.status !== 'running' ? details.current.get(run.id) : undefined;
            const detail = kept ?? (await api.run(run.id));
            // Only what a run said once it had ended is kept. One read while
            // it was going says its last step is still running, and was
            // believed for good when the run was next listed as finished.
            if (detail.status !== 'running') details.current.set(run.id, detail);
            for (const step of detail.steps ?? []) next[`${run.flow}/${baseStep(step.step)}`] = step.status;
          }
          setStatuses(next);
        } catch {
          /* the daemon went away; the stream will say when it is back */
        }
      }, 80),
    [name, project, props.instanceId],
  );
  useEffect(find, [find]);

  useEvents(
    ['run.*', 'step.*', 'instance.*'],
    (event) => {
      const from = event.source?.instance;
      if (!instance || from !== instance.id) {
        // A new instance of this blueprint: start following it.
        if (event.topic === 'instance.started' && event.data?.blueprint === name) find();
        return;
      }
      const run = event.source?.run;
      if (event.topic === 'run.started' && run) {
        const flow: string = event.data.flow;
        flows.current.set(run, flow);
        setStatuses((now) => Object.fromEntries(Object.entries(now).filter(([id]) => !id.startsWith(`${flow}/`))));
        find();
      } else if (event.topic.startsWith('step.') && run && event.source?.step) {
        const flow = flows.current.get(run);
        // Finished is said after any ending, with how it went: after a
        // failure it must not paint the step as having gone well.
        const status = event.topic === 'step.finished' ? ((event.data?.status as string | undefined) ?? 'ok') : STEP_STATUS[event.topic];
        if (flow && status) setStatuses((now) => ({ ...now, [`${flow}/${baseStep(event.source!.step!)}`]: status }));
      } else {
        find();
      }
    },
    [instance?.id, name],
    find,
  );

  const graph = checked?.graph ?? null;
  // What the last apply of this text said that the check does not: the
  // check is not told where a text will be applied, and an apply for no
  // project says which handlers will then hear nothing. It is kept until
  // the text is written in again: what was applied is the text laid
  // out, so it is not gone by the text it was said of.
  const problems = withSaid(checked?.diagnostics ?? [], said);
  const errors = problems.filter((problem) => problem.severity === 'error').length;

  const showText = (line: number) => {
    setMode('text');
    setGoTo((now) => ({ line, tick: now.tick + 1 }));
    drawer.value = null;
  };
  // Asked from elsewhere for a line of it (where the Reference says a word is used).
  useEffect(() => {
    if (props.line && props.lineAsked) showText(props.line);
  }, [props.lineAsked]);

  const select = (node: GraphNode | null) => {
    setSelected(node?.id ?? null);
    if (!node) {
      drawer.value = null;
      return;
    }
    const flow = node.group;
    const run = runs.find((one) => one.flow === flow);
    const step = run && details.current.get(run.id)?.steps?.find((one) => baseStep(one.step) === node.name);
    drawer.value = {
      // Headed by what the step says of itself, as its tile is; its key is under that, in the drawer.
      title: says(node.description) || (node.kind === 'trigger' ? `when: ${node.target}` : node.name),
      body: () => (
        <NodeDetails
          node={node}
          status={statuses[node.id]}
          step={step || undefined}
          onShowText={() => showText(node.line)}
          onShowLog={
            run && step
              ? () => {
                  setShownRun(run.id);
                  setShownStep(step.step);
                  setMode('runs');
                  drawer.value = null;
                }
              : undefined
          }
        />
      ),
    };
  };

  // `where` is what was answered when applying asks which project: a
  // project, or `null` for none. Not asked, it is the project this is for.
  // What is asked of an environment for what it is applied for: nothing
  // where that has none, or what is running already runs under one.
  const environmentsFor = (to: string | null) =>
    askedOf(namesOf(to), instance && (instance.status === 'active' || instance.status === 'starting') ? (instance.environment ?? '') : null);
  // `under` is the environment chosen where the project has some and
  // nothing running already says which (`projectenvs::asked`).
  const apply = async (params?: Fields, where?: string | null, take?: boolean, under?: string): Promise<boolean> => {
    // What it takes is known once its text has been checked; nothing is applied blind before then.
    if (!checked) return false;
    if (errors > 0) {
      toast('This blueprint has problems; fix them before applying it.', true);
      setMode('text');
      return false;
    }
    const unanswered = !!props.asksProject && where === undefined;
    const choose = environmentsFor(props.asksProject ? (where ?? null) : project);
    if ((params === undefined && graph && graph.params.length > 0) || unanswered || (choose && under === undefined)) {
      ask();
      return false;
    }
    // Saved first, which lays it out: what is applied is the file, and the
    // file is laid out the one way. One with nothing to save is not written.
    if (props.onSave && !(await props.onSave())) return false;
    const to = props.asksProject ? (where ?? null) : project;
    const before = pressed.current;
    const applied = await attempt(() =>
      api.apply(props.path ? { project: to, environment: under, path: props.path, params, take } : { project: to, environment: under, source, params, take }),
    );
    if (!applied) return false;
    // What ships that is other than this was applied with: left, with how
    // to take it, or taken and said.
    const reached = applied.reached;
    setLeft(reached && reached.left.length > 0 ? { texts: reached.left, params, where } : null);
    const took = !!reached && reached.taken.length > 0;
    if (took) toast(`Moved to what ships now: ${reached.taken.map(moved).join('. ')}.`);
    const there = to && to !== project ? ` for ${projectName(to)}` : '';
    // Taking what ships is something done, though the instance's own text is as it was.
    if (applied.created || applied.changed || !took)
      toast(appliedSaid(applied, name, there));
    // What the daemon said of it beside going ahead (a fleet applied where there is no board to work) is said here too: it was said from a terminal alone.
    for (const line of applied.said ?? []) toast(line, true);
    // Applied to a project from the factory's list, it is that project's
    // from here: whoever opened this is shown it there.
    if (props.asksProject && to) {
      props.asksProject(to);
      return true;
    }
    // What it said beside going ahead is listed under the text, with the
    // problems, and the text is what is shown: nothing else says it.
    const more = withSaid(checked?.diagnostics ?? [], applied.warnings ?? []).length - (checked?.diagnostics ?? []).length;
    setSaid(applied.warnings ?? []);
    if (more > 0) toast(more === 1 ? 'Applied, with one thing said of it: it is under the text.' : `Applied, with ${more} things said of it: they are under the text.`, true);
    const next = shownOnceApplied(more, pressed.current !== before);
    if (next) show(next);
    find();
    return true;
  };
  // Another text of it, played side by side, applied as it is with what it was told there: for the project
  // this page is of, or, opened for none, for none.
  const applyText = async (text: string, params: Fields) => {
    const applied = await attempt(() => api.apply({ project, source: text, params }));
    if (!applied) return;
    toast(applied.created ? `Started ${name} as it was played side by side.` : applied.changed ? `Updated ${name} to the text played side by side.` : `${name} is already running that text; nothing to do.`);
    for (const line of applied.said ?? []) toast(line, true);
    find();
  };
  // The text can be changed while the drawer is open beside it, so what
  // the drawer applies is the text as it is when its button is pressed,
  // and what it asks is what the text takes now.
  const latest = useRef(apply);
  latest.current = apply;
  applies.current = () => void apply();
  const asks = graph?.params;
  const ask = () => {
    const kept = applyingKey(props.path ?? props.title, project);
    const mine: Drawer = {
      title: `Apply ${props.title}`,
      body: () => (
        <Applying
          key={kept}
          kept={kept}
          params={asks ?? []}
          known={instance?.params ?? {}}
          asksProject={!!props.asksProject}
          environments={(where) => environmentsFor(props.asksProject ? where : project)}
          onClose={() => drawer.value === asked.current && (drawer.value = null)}
          onApply={(values, where, under) => latest.current(values, where, undefined, under)}
        />
      ),
    };
    asked.current = mine;
    drawer.value = mine;
  };
  useEffect(() => {
    // Asked of the drawer as it is now, and not as it was when this was drawn: this is done a moment after the
    // drawing, and a drawer put away in that moment was opened again by it.
    if (asked.current !== null && drawer.value === asked.current) ask();
  }, [asks, instance?.params]);

  // Previewed from the check's answer the page has, which follows the text as it is written: drawn again with each answer.
  const preview = () => {
    const mine: Drawer = {
      title: `Preview of ${props.title}`,
      body: () => <Preview key="preview" checked={checked} problems={problems} />,
    };
    previewed.current = mine;
    drawer.value = mine;
  };
  useEffect(() => {
    if (previewed.current !== null && drawer.value === previewed.current) preview();
  }, [checked, said]);
  // Why it is not previewed, in the check's own words: a text that does not check has no Summary to play.
  const unpreviewed = checked && !checked.summary ? (problems.find((problem) => problem.severity === 'error')?.message ?? 'Its text does not check.') : null;

  const live = instance && (instance.status === 'active' || instance.status === 'starting');
  const current = shownRun ?? runs[0]?.id ?? null;

  return (
    <div class="split blueprint-page" ref={root}>
      <div class="headed">
        <div class="view-head">
          <h1 class="truncate">{props.title}</h1>
          <Pills
            value={mode}
            options={[
              ['summary', 'Summary'],
              ['text', 'Text'],
              ['node', 'Node'],
              ['runs', 'Runs'],
              // Once it has been applied, what it was applied with each time.
              ...(instance ? ([['history', 'History']] as [Mode, string][]) : []),
              // Once a sandbox test of it was run from this tab, what came of it.
              ...(mode === 'sandbox' || sandboxes.value[tab] ? ([['sandbox', 'Sandbox']] as [Mode, string][]) : []),
              // Once texts of it were played side by side from this tab, what came of them.
              ...(mode === 'beside' || besides.value[tab] ? ([['beside', 'Side by side']] as [Mode, string][]) : []),
            ]}
            onChange={(next) => {
              setMode(next);
              // What it is applied with, and its preview, stay beside whichever of its views is read.
              if (!stays) drawer.value = null;
            }}
          />
          <span class="check-slot">
          {errors > 0 ? (
            <Badge status="failed" says={checkSays(errors)}>
              {errors === 1 ? '1 problem' : `${errors} problems`}
            </Badge>
          ) : checked ? (
            <Badge status="ok" says={checkSays(0)}>
              checks
            </Badge>
          ) : (
            // Its place is held while the text is checked, so that the head is the shape it
            // will be: drawn only when the answer came, it wrapped the head's buttons onto
            // another line under a pointer that was about to press one.
            <Badge status="queued" says={checkingSays()}>
              checking
            </Badge>
          )}
          </span>
          {instance && (
            <Badge status={instance.status} says={instanceSays(instance.status)}>
              {instanceWord(instance.status)}
            </Badge>
          )}
          {props.dirty && <span class="faint">unsaved</span>}
          <span class="grow" />
          {props.children}
          {live && (
            <button
              class="danger"
              onClick={() => void attempt(() => api.stopInstance(instance.id)).then(() => toast(`Stopped ${instance.blueprint}.`))}
            >
              Stop
            </button>
          )}
          {forking.button}
          {/* What tries the text without changing what runs, under one button: there from the first drawing, its lines
              closed until the text has been checked, so that nothing comes into the head under a pointer. */}
          <Testing
            standing={{
              reads: covers(me.value?.principal.scopes ?? [], 'blueprints:read'),
              instance: !!props.instanceId,
              checked: !!checked,
              errors,
              named: !!checked?.name,
              unpreviewed,
              previewing,
            }}
            answered={sandboxes.value[tab]?.answered ?? 0}
            besideAnswered={besides.value[tab]?.answered ?? 0}
            does={{
              preview: () => {
                if (!checked || unpreviewed) return;
                if (previewing) drawer.value = null;
                else preview();
              },
              sandbox: () => {
                setMode('sandbox');
                // Its view runs it when it is shown: once for this press.
                keepSandbox(tabOf(), { pressed: true });
                if (!stays) drawer.value = null;
              },
              beside: () => {
                setMode('beside');
                keepBeside(tabOf(), { pressed: true });
                if (!stays) drawer.value = null;
              },
              compare: () => checked?.name && compareWays(checked.name, project),
            }}
          />
          {!props.instanceId && (
            // Closed until its text has been checked: before then what it takes is not known,
            // and pressed in that moment it was applied with nothing asked of it.
            <button
              class="primary"
              disabled={!checked}
              onClick={() => void apply()}
              title={checked ? 'Start this blueprint, or update what is running to match it' : 'Once its text has been checked: start this blueprint, or update what is running to match it'}
            >
              {live ? 'Apply changes' : 'Apply'}
            </button>
          )}
        </div>

        {left && (
          <div class="reached-left row">
            <div class="stack grow" style="gap:4px">
              <span>What ships with this program is other than what this was applied with. What is running goes on with it as it was.</span>
              {left.texts.map((one) => (
                <span key={one.name} class="lag-moved muted">
                  {moved(one)}
                </span>
              ))}
            </div>
            <button onClick={() => void apply(left.params, left.where, true)} title="Move what is running to the blueprints that ship now. It is refused, saying why, where this text does not fit them.">
              Take what ships now
            </button>
            <button class="ghost" onClick={() => setLeft(null)} title="It goes on with the blueprints it has until somebody takes them.">
              Leave it
            </button>
          </div>
        )}
      </div>

      {mode === 'summary' && (
        <SummaryView
          checked={checked}
          problems={problems}
          project={project}
          ran={`${runs.length}:${runs[0]?.id ?? ''}:${runs[0]?.status ?? ''}`}
          onShow={showText}
          onProblems={() => setMode('text')}
          follows={forking.follows}
        />
      )}
      {forking.dialogs}

      {mode === 'text' && (
        <div class="split" style="grid-template-rows:1fr auto">
          <CodeEditor
            value={source}
            language="blueprint"
            readOnly={props.readOnly}
            problems={problems}
            world={world}
            line={goTo.line}
            lineTick={goTo.tick}
            onChange={(text) => {
              if (said.length > 0) setSaid([]);
              props.onChange?.(text);
            }}
            onSave={() => void props.onSave?.()}
            onFormat={props.onFormat}
          />
          {problems.length > 0 && (
            <div class="problems">
              {problems.map((problem, index) => (
                <button key={index} class="problem" onClick={() => showText(problem.span.start.line)}>
                  <span class="at">
                    {problem.span.start.line}:{problem.span.start.col}
                  </span>
                  <span class={problem.severity === 'error' ? 'error-text' : 'muted'}>{problem.severity}</span>
                  <span>{problem.message}</span>
                </button>
              ))}
            </div>
          )}
        </div>
      )}

      {mode === 'node' &&
        (graph ? (
          <GraphView graph={graph} statuses={statuses} selected={drawer.value && !asking ? selected : null} onSelect={select} />
        ) : (
          <Empty title={checked ? 'This cannot be drawn until it reads as a blueprint' : 'Reading the blueprint'}>
            {checked && <button onClick={() => setMode('text')}>See the problems</button>}
          </Empty>
        ))}

      {mode === 'runs' &&
        (runs.length === 0 ? (
          <Empty title="This blueprint has not run">
            {!props.instanceId && <span>Apply it to start it. Each run is listed here with its steps and their logs.</span>}
          </Empty>
        ) : (
          <div class="runs-beside" style="display:grid;grid-template-columns:270px 1fr;min-height:0;height:100%">
            <div style="overflow:auto;border-right:1px solid var(--line);padding:4px 8px">
              {runs.map((run) => {
                // A text's own steps are said in words, with what the text calls them under that, smaller.
                const of = runOf(run.flow);
                return (
                  <button
                    key={run.id}
                    class="nav-item run-line"
                    style="height:auto;padding:6px 8px;display:grid;gap:2px"
                    aria-current={current === run.id}
                    onClick={() => {
                      setShownRun(run.id);
                      setShownStep(null);
                    }}
                  >
                    <span class="row">
                      <Badge status={run.status} />
                      <span class="truncate run-of">{of.words}</span>
                    </span>
                    <span class="faint" style="font-size:calc(11px * var(--size))">
                      {of.key && <code class="run-called">{of.key}</code>}
                      {of.key ? ' · ' : ''}
                      {when(run.started)}
                      {run.finished ? ` · ${duration(run.finished - run.started)}` : ''}
                    </span>
                  </button>
                );
              })}
            </div>
            <div style="overflow:auto;padding:12px 20px 24px">
              {current && <RunDetail id={current} step={shownStep} onStep={setShownStep} drawn={checked} />}
            </div>
          </div>
        ))}

      {mode === 'history' && (instance ? <History instance={instance} tab={tab} /> : <Empty title="This blueprint has not been applied" />)}

      {mode === 'beside' && tab && (
        <Suspense fallback={<div class="empty">Opening side by side</div>}>
          <SideBySideView
            tab={tab}
            source={source}
            project={project}
            checked={checked}
            errors={errors}
            known={instance?.params ?? {}}
            path={props.path}
            dirty={props.dirty}
            isDefault={!!props.ships}
            ships={!!props.ships || (!!name && defaultNamed(checked?.summary) === name)}
            onApply={(text, params) => void (text === source ? apply(params) : applyText(text, params))}
            onFork={forking.fork}
            onText={props.path && starts.changes ? () => setMode('text') : null}
          />
        </Suspense>
      )}

      {mode === 'sandbox' && tab && (
        <Suspense fallback={<div class="empty">Opening the sandbox test</div>}>
          <SandboxView tab={tab} source={source} project={project} checked={checked} errors={errors} known={instance?.params ?? {}} onProblems={() => setMode('text')} />
        </Suspense>
      )}

    </div>
  );
}
