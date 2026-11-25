// A project's Pipelines page (`pipelines.ts`): each of its own blueprints
// under `pipelines/` a row, with what it says it is for, its latest runs as
// a strip of marks (each opening that run's page) and Run; under them,
// folded, everything else that runs for the project with its latest runs,
// so that the fleet's rounds and a deploy are seen as pipeline runs too;
// and New pipeline…, which makes `@blueprints/pipelines/<name>.bp` through
// the route that makes a file and opens it.
//
// Run applies the pipeline as whoever presses it, through the apply route,
// with `force` so that its steps run again: asked first in the drawer where
// it is handed something or the project runs under an environment
// (`Applying`, as a blueprint's page asks), and its run's page opened once
// it began. Where the project's triggers are kept (`std/pipeline`), Run
// asks them instead (`pipeline.asked`, said as whoever presses): they run
// it as a row that runs it by hand says, under its key and environment,
// once they have asked what whoever pressed holds, and tell them in their
// inbox where they do not; the page says so too, and opens the run's page
// once one begins. A row has room for what sets it off, and none is drawn
// yet.
//
// It reads once and reads again when a run of the project ends or begins
// (`run.*`, read again too where the stream says it fell behind) and when
// what the project has changes.

import { hrefOf } from '../address.ts';
import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { api, call } from '../api.ts';
import type { Fields, GraphParam, Instance } from '../api.ts';
import { covers } from '../holds.ts';
import { namesOf } from '../envsheld.ts';
import { asked as environmentsAsked } from '../projectenvs.ts';
import { coalesce } from '../events.ts';
import { askedOf, asksFirst, asksTriggers, firstPipeline, latestOf, othersOf, pipelineNamed, pipelinesOf, refusedOf, shortName } from '../pipelines.ts';
import type { Mark } from '../pipelines.ts';
import { ranAs, runsOf } from '../runread.ts';
import { attempt, callable, drawer, learnBlueprints, learnUsed, me, openTab, projects, toast, used } from '../store.ts';
import type { Drawer } from '../store.ts';
import { Badge, Empty, Modal, useEvents, useFocus } from '../ui.tsx';
import { ago, classes } from '../util.ts';
import { Applying } from './applying.tsx';
import { PageActs } from './pagehead.tsx';
import { mayMake } from './repomade.tsx';
import { useKept } from './returned.tsx';
import { whatIs } from './running.tsx';
import { headWord, ThingWas } from './thingwas.tsx';

/** How many of what else runs for a project are read, when that is unfolded. */
const OTHERS = 40;

/** A pipeline's latest runs, by its name. */
type Marks = Record<string, Mark[]>;

async function marksOf(project: string, names: string[]): Promise<Marks> {
  const all = await Promise.all(
    names.map(async (name) => {
      const ran = await ranAs(project, name, 6).catch(() => [] as Instance[]);
      return [name, latestOf(await Promise.all(ran.map((one) => runsOf(one.id, 5).catch(() => []))))] as const;
    }),
  );
  return Object.fromEntries(all);
}

const holds = (scope: string) => covers(me.value?.principal.scopes ?? [], scope);

export function PipelinesView({ project }: { project: string }) {
  const name = projects.value.find((one) => one.id === project)?.name ?? project;
  const pipelines = pipelinesOf(used.value[project]);
  const others = othersOf(used.value[project]);
  const [marks, setMarks] = useKept<Marks>('pipelines.marks', {});
  const [elsewise, setElsewise] = useKept<Marks>('pipelines.others', {});
  const [othersOpen, setOthersOpen] = useKept('pipelines.othersOpen', false);
  const [naming, setNaming] = useState(false);
  const described = callable.value[project];
  const said = new Map((described ?? []).map((one) => [one.name, one]));
  const names = pipelines.map((one) => one.name).join('\n');
  const read = useMemo(
    () =>
      coalesce(() => {
        void marksOf(project, names ? names.split('\n') : []).then(setMarks);
      }, 120),
    [project, names],
  );
  const readOthers = useMemo(() => coalesce(() => void marksOf(project, others.slice(0, OTHERS)).then(setElsewise), 120), [project, others.join('\n')]);
  useEffect(read, [read]);
  useEffect(() => void learnBlueprints(project), [project]);
  useEffect(() => {
    if (othersOpen) readOthers();
  }, [othersOpen, readOthers]);
  const again = () => {
    read();
    if (othersOpen) readOthers();
  };
  // A run of something of another project's is nothing of this page's; one of something not read yet may be.
  const ours = (instance: string | undefined) => {
    const what = instance ? whatIs(instance) : 'gone';
    return what === undefined || (what !== 'gone' && what.project === project);
  };
  useEvents(['run.started', 'run.finished'], (event) => ours(event.source?.instance) && again(), [read, readOthers, othersOpen, project], again);
  const written = used.value[project]?.written ?? null;
  const makes = written !== null && mayMake().length > 0;
  const applies = holds('blueprints:run');

  return (
    <div class="pipelines-page">
      <div class="view-head">
        <h1 class="grow truncate">
          {name} {headWord(project, 'pipelines', 'pipelines')}
          <ThingWas project={project} thing="pipelines" />
        </h1>
        <PageActs tab={{ kind: 'pipelines', project }} last={!makes} />
        {makes && (
          <button class="primary" onClick={() => setNaming(true)}>
            New pipeline…
          </button>
        )}
      </div>
      <div class="view-body stack pipelines-body">
        {naming && <NewPipeline project={project} written={written ?? '@blueprints'} onClose={() => setNaming(false)} />}
        <div class="panel pipelines-panel">
          {pipelines.length === 0 ? (
            <Empty title="No pipelines yet">
              <p class="muted">
                A pipeline is a blueprint of the project's own kept under <code>pipelines/</code>. Each of its stages is a task with its steps, and every run of it has a page of its own.
              </p>
            </Empty>
          ) : (
            <ul class="pipelines-list">
              {pipelines.map((one) => (
                <li key={one.name} class="pipeline-row" data-pipeline={one.name}>
                  <div class="pipeline-what">
                    <button type="button" class="link pipeline-name" title={`Open ${one.file}`} onClick={() => openTab({ kind: 'file', project, path: one.file })}>
                      {shortName(one.name)}
                    </button>
                    <span class="faint pipeline-said">{said.get(one.name)?.description ?? ''}</span>
                  </div>
                  {/* What sets it off by itself is said here, once the triggers say any. */}
                  <Strip marks={marks[one.name]} />
                  {applies && <RunButton project={project} name={one.name} params={said.get(one.name)?.params ?? null} />}
                </li>
              ))}
            </ul>
          )}
        </div>
        {others.length > 0 && (
          <div class="panel pipelines-others">
            <button type="button" class="pipelines-fold" aria-expanded={othersOpen} onClick={() => setOthersOpen(!othersOpen)}>
              <span aria-hidden="true">{othersOpen ? '▾' : '▸'}</span> Everything else that runs here <span class="faint">({others.length})</span>
            </button>
            {othersOpen && (
              <ul class="pipelines-list">
                {others.slice(0, OTHERS).map((other) => (
                  <li key={other} class="pipeline-row" data-blueprint={other}>
                    <div class="pipeline-what">
                      <button type="button" class="link pipeline-name" onClick={() => openTab({ kind: 'blueprint', name: other, project })}>
                        {other}
                      </button>
                    </div>
                    <Strip marks={elsewise[other]} />
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

/** A pipeline's latest runs, the newest first, each a mark that opens its page. */
function Strip({ marks }: { marks: Mark[] | undefined }) {
  if (!marks) return <span class="pipeline-strip faint">reading its runs</span>;
  if (marks.length === 0) return <span class="pipeline-strip faint">never run</span>;
  return (
    <span class="pipeline-strip">
      {marks.map((mark) => (
        <a key={mark.run} class={classes('run-mark', mark.status)} href={hrefOf({ kind: 'run', id: mark.run })} title={`${mark.status}, ${ago(mark.started)}`} aria-label={`A run, ${mark.status}, ${ago(mark.started)}`}>
          <Badge status={mark.status}>{mark.status === 'ok' ? '✓' : mark.status === 'failed' ? '✕' : mark.status === 'running' ? '…' : '–'}</Badge>
        </a>
      ))}
      <span class="faint pipeline-last">{ago(marks[0]!.started)}</span>
    </span>
  );
}

/**
 * Applies a pipeline as whoever presses, asking first where it is handed
 * something or runs under an environment, and opens its run; or, where the
 * project's triggers are kept, asks them for it and opens its run once one
 * begins, saying why where they refuse.
 */
function RunButton({ project, name, params }: { project: string; name: string; params: GraphParam[] | null }) {
  const [busy, setBusy] = useState(false);
  // What this press named its asking of the triggers, while the run they
  // begin for it is waited for: named before it is asked, so that what the
  // triggers say of it at once is heard as its own.
  const waiting = useRef<string | null>(null);
  const [asking, setAsking] = useState<string | null>(null);
  const triggered = asksTriggers(used.value[project]);
  const done = () => ((waiting.current = null), setAsking(null));
  const look = async () => {
    const asked = waiting.current;
    if (asked === null) return;
    const begun = await call<{ run: string | null }>('GET', `/pipelines/begun?project=${encodeURIComponent(project)}&asked=${encodeURIComponent(asked)}`).catch(() => null);
    if (begun?.run && waiting.current === asked) {
      done();
      openTab({ kind: 'run', id: begun.run });
    }
  };
  useEvents(
    // A run begun by a conductor that started again is kept as asked for once the step that asked for it is taken up.
    ['run.started', 'step.started', 'pipeline.refused'],
    (event) => {
      if (waiting.current === null) return;
      if (event.topic !== 'pipeline.refused') return void look();
      if (refusedOf(event.data, waiting.current)) {
        done();
        toast(String(event.data?.says ?? `${name} was not run.`).replaceAll('`', ''), true);
      }
    },
    [name, project],
    () => void look(),
  );
  // Begun before the asking was answered, it is there already.
  useEffect(() => void look(), [asking]);
  const run = async (values: Fields = {}, under?: string): Promise<boolean> => {
    setBusy(true);
    if (triggered) {
      const named = crypto.randomUUID();
      waiting.current = named;
      const asked = await attempt(() => call('POST', '/emit/pipeline.asked', askedOf(project, name, values, named)));
      setBusy(false);
      if (asked === undefined) return (done(), false);
      setAsking(named);
      toast(`Asked for a run of ${shortName(name)}; its page opens once it begins.`);
      return true;
    }
    const applied = await attempt(() => api.apply({ project, blueprint: name, params: values, environment: under, force: true }));
    setBusy(false);
    if (!applied) return false;
    for (const line of applied.said ?? []) toast(line, true);
    if (applied.run) openTab({ kind: 'run', id: applied.run });
    else toast(`${name} began no run.`, true);
    void learnUsed();
    return true;
  };
  const press = () => {
    // What the triggers run a pipeline under is the row's to say: they are asked for none.
    const environments = triggered ? [] : namesOf(project);
    if (!asksFirst(params?.length ?? 0, environments)) return void run();
    const kept = `pipeline:${project}:${name}`;
    const mine: Drawer = {
      title: `Run ${name}`,
      body: () => (
        <Applying
          key={kept}
          kept={kept}
          params={params ?? []}
          known={{}}
          environments={() => environmentsAsked(environments, null)}
          onClose={() => drawer.value === mine && (drawer.value = null)}
          onApply={(values, _where, under) => run(values, under)}
        />
      ),
    };
    drawer.value = mine;
  };
  return (
    <button type="button" class="pipeline-run-button" disabled={busy || params === null} title={params === null ? 'Reading what it is handed' : `Run ${name} as you`} onClick={press}>
      {busy ? 'Starting' : 'Run'}
    </button>
  );
}

/** Names a new pipeline, and makes it as a file of the project's own under `pipelines/`, opened to be written in. */
function NewPipeline({ project, written, onClose }: { project: string; written: string; onClose: () => void }) {
  const [name, setName] = useState('');
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const input = useFocus<HTMLInputElement>();
  const has = (used.value[project]?.own ?? []).map((one) => one.name);
  const ships = (callable.value[''] ?? []).map((one) => one.name);
  const came = pipelineNamed(name, written, has, ships);
  const make = async (event: Event) => {
    event.preventDefault();
    if (busy || !('path' in came)) return;
    setBusy(true);
    setWrong(null);
    try {
      const entry = await api.newFile(project, came.path, firstPipeline(came.name));
      await learnUsed();
      onClose();
      openTab({ kind: 'file', project, path: entry.path });
    } catch (refused) {
      setBusy(false);
      setWrong((refused instanceof Error ? refused.message : String(refused)).replaceAll('`', ''));
    }
  };
  return (
    <Modal onClose={onClose}>
      <h2>A new pipeline</h2>
      <form class="stack repo-made" onSubmit={(event) => void make(event)}>
        <label>
          Name
          <input ref={input} value={name} placeholder="gates" aria-describedby="pipeline-made-says" onInput={(event) => (setName(event.currentTarget.value), setWrong(null))} />
        </label>
        <p id="pipeline-made-says" class="faint repo-made-says">
          {name.trim() !== '' && 'wrong' in came ? came.wrong : `It is kept by the factory under pipelines/, as ${'path' in came ? came.name : 'pipelines/<name>'}, with one stage to begin from, and opens to be written in.`}
        </p>
        {wrong && (
          <p role="alert" class="repo-made-wrong">
            {wrong}
          </p>
        )}
        <div class="row" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" type="submit" disabled={busy || !('path' in came)}>
            {busy ? 'Making it' : 'Make the pipeline'}
          </button>
        </div>
      </form>
    </Modal>
  );
}
