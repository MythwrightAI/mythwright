// A sandbox test of a blueprint, as a view of its page.
//
// The text as it stands on the page is run in a sandbox: a factory of
// its own, which the daemon makes for the test and which nothing of the
// factory in use is in. The view is three parts that keep to their own
// room at any width: what is done to it (`sandboxacts.tsx`), what
// happened, and, at its foot and never scrolled away, what this was, in
// the daemon's own sentences: that it was a sandbox test, what was
// stood in and so not exercised, and that passing here is not passing
// for real.
//
// What happened is what the daemon hands, drawn: the timeline under the
// run each line is of, each step by what it says of itself with its key
// under it and how it came out in a word, as a run's steps are drawn;
// the text's own graph lit by it; the tables, what was said in channels
// and what somebody was told. Nothing of a sandbox is the factory's:
// what is named here (an instance, a run, a record) is words and opens
// nothing, since it is nowhere but in the sandbox.
//
// The sandbox's events are on no stream. The page asks one thing at a
// time and reads after each (`sandboxed.ts`), and what it has is kept
// by the tab (`sandboxkept.ts`), so it is here when the view comes back.

import { useEffect, useRef, useState } from 'preact/hooks';
import type { Checked, Fields, Graph } from '../api.ts';
import { brief, resultOf } from '../gave.ts';
import { GraphView } from '../graph/view.tsx';
import { LIT, called, differs, grouped, itself, lit, needsFirst, outcome, paramsOf } from '../sandbox.ts';
import type { Asking, Group, Line, Lit, Shown } from '../sandbox.ts';
import { runSandbox } from '../sandboxed.ts';
import { NOTHING, keepSandbox, sandboxOf, sandboxes } from '../sandboxkept.ts';
import type { Kept } from '../sandboxkept.ts';
import { meta } from '../store.ts';
import { Badge, Empty } from '../ui.tsx';
import { baseStep, classes, show } from '../util.ts';
import { Acts, toldOf } from './sandboxacts.tsx';

export interface SandboxProps {
  /** The tab this is a view of: what the sandbox test is kept by. */
  tab: string;
  source: string;
  project: string | null;
  /** The text as the page has it checked: its name, its graph and what it is told. */
  checked: Checked | null;
  /** How many things are wrong with the text: one with any is not run. */
  errors: number;
  /** What the blueprint was last applied with in the factory, where it runs there. */
  known: Fields;
  /** Shows where the text's problems are said. */
  onProblems: () => void;
}

/** The mark on a tile's corner for each way it is lit: said in the list under the graph. */
const MARK: Record<Lit, string> = { ok: '✓', stood: 'S', skipped: '–', let: '!', failed: '✕', running: '…' };

export default function SandboxView(props: SandboxProps) {
  const { tab, checked } = props;
  const graph = checked?.graph ?? null;
  // What goes first starts as what the daemon says the text needs, once, when the tab first has a sandbox test.
  const name = checked?.name ?? null;
  const kept: Kept = sandboxes.value[tab] ?? NOTHING;
  const [wrong, setWrong] = useState('');
  const cannot = props.errors > 0 ? 'This text has problems, so it is not run: what is wrong with it is said under the text.' : !graph ? 'The text is being read.' : wrong || null;

  const run = (kept: Kept) => {
    if (!graph || props.errors > 0) return;
    const told = paramsOf(graph.params, toldOf(kept, graph, props.known));
    if ('wrong' in told) return setWrong(told.wrong);
    const params = told.params;
    setWrong('');
    void runSandbox(tab, { source: props.source, project: props.project, first: kept.plan.first, params, told: kept.plan.told, answers: kept.plan.answers });
  };
  // Pressed in the page's head, it is run, once for each press; shown again by its pill, it is as it was.
  useEffect(() => {
    // A text that does not check is not run, now or when it is put right: the press is spent.
    if (sandboxOf(tab)?.pressed && checked && (props.errors > 0 || !graph)) keepSandbox(tab, { pressed: false });
    if (!name || !graph || props.errors > 0) return;
    let now = sandboxOf(tab) ?? keepSandbox(tab, {});
    if (!now.begun) now = keepSandbox(tab, { begun: true, plan: { ...now.plan, first: needsFirst(name, meta.value?.project_starts ?? []) } });
    if (!now.pressed) return;
    run(keepSandbox(tab, { pressed: false }));
  }, [tab, name, graph !== null, kept.pressed, checked !== null, props.errors]);

  const shown = kept.shown;
  return (
    <div class="sandbox" data-sandbox={kept.id ?? ''} data-busy={kept.busy !== null} data-answered={kept.answered}>
      <div class="sandbox-body">
        <section class="sandbox-acts" aria-label="What is done to it">
          <h2>What is done to it</h2>
          <Acts tab={tab} kept={kept} project={props.project} graph={graph} known={props.known} cannot={cannot} onRun={() => run(kept)} />
        </section>
        <section class="sandbox-happened" aria-label="What happened">
          <h2>What happened</h2>
          {kept.problems && (
            <div class="sandbox-note bad" role="alert">
              <p>This text does not check, so it was not run in a sandbox. What is wrong with it:</p>
              <ul>
                {kept.problems.map((problem, index) => (
                  <li key={index}>
                    <span class="faint">
                      {problem.span.start.line}:{problem.span.start.col}
                    </span>{' '}
                    {problem.message}
                  </li>
                ))}
              </ul>
              <button onClick={props.onProblems}>See them in the text</button>
            </div>
          )}
          {kept.gone && (
            <div class="sandbox-note bad" role="alert">
              <p>
                The daemon has this sandbox no longer, so nothing more can be done to it. {kept.gone.charAt(0).toUpperCase()}
                {kept.gone.slice(1)}.
              </p>
              <button class="primary" disabled={kept.busy !== null || !!cannot} onClick={() => run(kept)}>
                Run it again
              </button>
              {shown && <p class="faint">What it had come to is still shown below.</p>}
            </div>
          )}
          {kept.refused && (
            <p class="sandbox-note bad" role="alert">
              {kept.refused}
            </p>
          )}
          {kept.skipped.map((why, index) => (
            <p key={index} class="sandbox-note">
              {why}
            </p>
          ))}
          {shown ? (
            <Happened kept={kept} shown={shown} graph={graph} />
          ) : (
            !kept.problems && (
              <Empty title={kept.busy ? 'Running it in a sandbox' : 'Nothing has been run in a sandbox here yet'}>
                {!kept.busy && <span>Run runs this text, as it stands on the page, in a factory of its own. Nothing in it touches the factory in use.</span>}
              </Empty>
            )
          )}
        </section>
      </div>
      {shown && <Foot shown={shown} />}
    </div>
  );
}

/** What this was, in the daemon's own sentences: that it was a sandbox test, and what was stood in, each with how often. */
export function Foot({ shown }: { shown: Shown }) {
  return (
    <footer class="sandbox-foot" aria-label="What this was">
      <p class="sandbox-said">{shown.said}</p>
      <p class="sandbox-stood-said">
        {shown.stood_said}
        {shown.stood.map((one) => (
          <span key={one.action} class="sandbox-stood">
            <code>{one.action}</code> × {one.times}
          </span>
        ))}
      </p>
    </footer>
  );
}

/** Everything that came of the sandbox: how it stands, what differs from the run before, the graph lit, the timeline, and what it ended with. */
export function Happened({ kept, shown, graph }: { kept: Pick<Kept, 'before'>; shown: Shown; graph: Graph | null }) {
  const stands = outcome(shown);
  const own = itself(shown);
  const tiles = lit(shown.timeline, own?.id ?? null);
  const [picked, setPicked] = useState<string | null>(null);
  const box = useRef<HTMLDivElement>(null);
  // A tile that is pressed brings its step's line into view.
  useEffect(() => {
    if (picked) box.current?.querySelector(`[data-tile="${CSS.escape(picked)}"]`)?.scrollIntoView({ block: 'nearest' });
  }, [picked]);
  const drawn = graph && own && Object.keys(tiles).length > 0;
  return (
    <div class="sandbox-came" ref={box}>
      <p class={classes('sandbox-outcome', stands.how)} role="status">
        {stands.says}
      </p>
      {kept.before && (
        <section class="sandbox-part" aria-label="What differs from the run before">
          <h3>What differs from the run before</h3>
          <ul class="sandbox-differs">
            {differs(kept.before, shown).map((line, index) => (
              <li key={index}>{line}</li>
            ))}
          </ul>
        </section>
      )}
      {drawn && (
        <section class="sandbox-part" aria-label="The text, lit by what it did">
          <h3>The text, lit by what it did</h3>
          <div class="sandbox-graph">
            <GraphView
              graph={graph}
              statuses={tiles}
              badges={Object.fromEntries(Object.entries(tiles).map(([id, how]) => [id, MARK[how]]))}
              selected={picked}
              onSelect={(node) => setPicked(node?.id ?? null)}
            />
          </div>
          <ul class="sandbox-legend" aria-label="What each mark means">
            {LIT.map(([how, words]) => (
              <li key={how}>
                <span class={`sandbox-mark lit-${how}`}>{MARK[how]}</span> {words}
              </li>
            ))}
            <li class="faint">a tile with no mark did not run</li>
          </ul>
        </section>
      )}
      <section class="sandbox-part" aria-label="What happened, in order">
        <h3>In the order it happened</h3>
        {grouped(shown.timeline).map((group, index) => (
          <Ran key={index} group={group} shown={shown} own={own?.id ?? null} picked={picked} />
        ))}
      </section>
      <Ended shown={shown} />
    </div>
  );
}

/** How a step came out, in a word, and how its badge is drawn. */
function came(line: Line): [string, string] {
  const stood = (line.stood ?? []).length > 0;
  switch (line.status) {
    case 'ok':
      return stood ? ['stood', 'stood in'] : ['ok', 'ran'];
    case 'failed':
      return line.tolerated ? ['let', 'failed, went on'] : ['failed', 'failed'];
    case 'skipped':
      return ['skipped', 'passed over'];
    case 'cancelled':
      return ['cancelled', 'cut short'];
    default:
      return ['running', 'under way'];
  }
}

/** What an instance of the sandbox did, in a line. Its name is words: it is nowhere but in the sandbox. */
function ofInstance(line: Line, name: string): string | null {
  switch (line.did) {
    case 'started':
      return `${name} was started`;
    case 'active':
      return `${name} is up: its steps went well and it stays, listening`;
    case 'failed':
      return `${name} failed${line.error ? `: ${line.error}` : ''}`;
    case 'stopped':
      return `${name} was stopped`;
    case 'finished':
      return line.status === 'done' ? `${name} is over` : null;
    default:
      return null;
  }
}

/** One run's lines, under what it is a run of; or what was said for no run. */
function Ran({ group, shown, own, picked }: { group: Group; shown: Shown; own: string | null; picked: string | null }) {
  const [opened, setOpened] = useState<Set<string>>(new Set());
  const open = (key: string) => {
    const next = new Set(opened);
    if (!next.delete(key)) next.add(key);
    setOpened(next);
  };
  const name = called(shown.instances, group.instance);
  const heading = group.run ? `${name}: ${group.flow === 'main' ? 'its steps' : `when ${(group.flow ?? '').replace('on.', '')}`}` : null;
  return (
    <div class="sandbox-run" data-run={group.run ?? ''}>
      {heading && <h4>{heading}</h4>}
      {group.lines.map((line, index) => {
        if (line.what === 'instance') {
          const says = ofInstance(line, called(shown.instances, line.instance) || 'An instance');
          return says ? (
            <p key={index} class={classes('sandbox-line', 'of-instance', line.did === 'failed' && 'error-text')}>
              {says}
            </p>
          ) : null;
        }
        if (line.what === 'run') {
          if (line.did === 'started' && line.heard)
            return (
              <p key={index} class="sandbox-line">
                heard <code>{line.heard}</code>
              </p>
            );
          if (line.did === 'finished' && (line.status === 'failed' || line.status === 'cancelled'))
            return (
              <p key={index} class={classes('sandbox-line', line.status === 'failed' && 'error-text')}>
                {line.status === 'failed' ? `It ended: failed. ${line.error ?? ''}` : 'It was cut short.'}
              </p>
            );
          return null;
        }
        if (line.what === 'waits')
          return (
            <p key={index} class="sandbox-line">
              <code>{line.step}</code> waits {line.event ? <>for <code>{line.event}</code> to be said</> : 'for the clock to be moved on'}
            </p>
          );
        if (line.what === 'said')
          return (
            <p key={index} class="sandbox-line of-said">
              said <code>{line.topic}</code> <span class="faint">{brief(line.data)}</span>
            </p>
          );
        const [badge, word] = came(line);
        const result = resultOf({ error: line.error ?? null, tolerated: !!line.tolerated, output: line.gave ?? null });
        const key = `${line.run}/${line.step}`;
        const whole = result?.whole && opened.has(key);
        const askings = shown.asked.filter((asking) => (line.stood ?? []).includes(asking.id));
        const tile = line.instance === own && line.flow ? `${line.flow}/${baseStep(line.step ?? '')}` : undefined;
        return (
          <div key={index} class={classes('sandbox-step', tile !== undefined && tile === picked && 'picked')} data-step={line.step} data-came={badge} data-tile={tile}>
            <span class="sandbox-how">
              <Badge status={badge}>{word}</Badge>
            </span>
            <span class="sandbox-what">
              {line.says && <span class="step-says">{line.says}</span>}
              <code class={line.says ? 'step-key' : undefined}>{line.step}</code>
            </span>
            <span class="sandbox-gave">
              {askings.map((asking) => (
                <Stood key={asking.id} asking={asking} />
              ))}
              {askings.length === 0 &&
                (result?.whole ? (
                  <button type="button" class={classes('run-gave', result.failed && 'error-text')} aria-expanded={!!whole} title={whole ? 'Put the whole of it away' : result.failed ? 'Read the whole of why it failed' : 'Read the whole of what it gave'} onClick={() => open(key)}>
                    <span class="truncate">{result.says}</span>
                  </button>
                ) : (
                  (result?.says ?? '')
                ))}
            </span>
            {whole && <pre class={classes('code-box', 'sandbox-whole', result.failed && 'error-text')}>{result.whole === 'error' ? (line.error ?? '') : show(line.gave)}</pre>}
          </div>
        );
      })}
    </div>
  );
}

/** What a stand-in was asked for a step, and what it answered. */
function Stood({ asking }: { asking: Asking }) {
  const by = asking.by === 'told' ? 'what it was told to' : asking.by === 'answered' ? 'what was answered here' : asking.by === 'called off' ? 'nothing: its step was stopped' : 'what a stand-in for it usually answers';
  return (
    <span class="sandbox-stood-in">
      <code>{asking.action}</code> was stood in. Asked <span class="faint">{brief(asking.asked)}</span>,{' '}
      {asking.waiting ? (
        <strong>it waits to be answered.</strong>
      ) : asking.failed ? (
        <span class="error-text">it failed as it was told to: {asking.failed}</span>
      ) : (
        <>
          it answered <span class="faint">{brief(asking.answered ?? {})}</span> ({by}).
        </>
      )}
    </span>
  );
}

/** What the sandbox ended with: its tables with their rows, what was said in its channels, and what somebody was told. */
function Ended({ shown }: { shown: Shown }) {
  return (
    <>
      <section class="sandbox-part" aria-label="Its tables">
        <h3>Its tables</h3>
        {shown.tables.length === 0 && <p class="faint">It made no table.</p>}
        {shown.tables.map((table) => (
          <details key={table.name} class="sandbox-table" open={table.records.length > 0 && table.records.length <= 6}>
            <summary>
              <code>{table.name}</code> <span class="faint">{table.records.length === 1 ? '1 row' : `${table.records.length} rows`}</span>
            </summary>
            {table.records.length === 0 ? (
              <p class="faint">It has no rows.</p>
            ) : (
              <ul>
                {table.records.map((record) => (
                  <li key={record.id}>
                    <code>{record.id}</code> <span class="faint">{brief(record.data)}</span>
                  </li>
                ))}
              </ul>
            )}
          </details>
        ))}
      </section>
      {shown.channels.some((channel) => channel.messages.length > 0) && (
        <section class="sandbox-part" aria-label="What was said in its channels">
          <h3>What was said in its channels</h3>
          <ul class="sandbox-talk">
            {shown.channels.flatMap((channel) =>
              channel.messages.map((message) => (
                <li key={message.id}>
                  <span class="faint">
                    #{channel.name} · {message.author_name}
                  </span>{' '}
                  {message.body}
                </li>
              )),
            )}
          </ul>
        </section>
      )}
      {shown.inbox.length > 0 && (
        <section class="sandbox-part" aria-label="What somebody was told">
          <h3>What somebody was told</h3>
          <ul class="sandbox-talk">
            {[...shown.inbox].reverse().map((line) => (
              <li key={line.id}>
                <span class="faint">
                  {line.owner === shown.somebody ? 'whoever applied it' : line.owner} · {line.why}
                  {line.count > 1 ? ` · ${line.count} times` : ''}
                </span>{' '}
                {line.says ?? ''}
              </li>
            ))}
          </ul>
        </section>
      )}
    </>
  );
}
