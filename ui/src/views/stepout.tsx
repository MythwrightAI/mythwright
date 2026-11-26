// One step of a run as its page and its graph's drawer show it: how it
// stands, how long it took, why it failed, and the whole of what it
// wrote, drawn from its end a window at a time through the run's log
// route and followed as it writes (`log.line`). What it read is kept by
// its tab (`useKept`), so that a tab shown again is as it was.

import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'preact/hooks';
import type { Graph, LogLine, Run, Step } from '../api.ts';
import { coalesce } from '../events.ts';
import { logBetween, tailLog } from '../runread.ts';
import { stepSays } from '../runsaid.ts';
import { drawnOf, earlierAsked, earlierSaid, earlierThan, WINDOW } from '../staged.ts';
import { Badge, useEvents } from '../ui.tsx';
import { duration } from '../util.ts';
import { useKept } from './returned.tsx';

/** One step of a stage: how it stands, how long it took, why it failed, and the whole of what it wrote. */
export function StepOut({ run, step, graph, kept, alone }: { run: Run; step: Step; graph: Graph | null; kept: boolean; alone?: boolean }) {
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
