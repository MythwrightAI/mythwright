// One run: its steps, and the log of whichever step is selected.
//
// A step's line says how it came out where there is something to say: why
// it failed, or in a few words what it gave out. The whole of either is
// opened under the line by pressing it. Where no step of the run has
// anything to say the list has no such column: it stood empty beside
// every step that went well.
//
// A run whose reader may not be told what set it off is handed without
// what its steps gave, and their logs are refused: it says so over its
// steps, and offers no log, which would read as a step that wrote nothing.
//
// A step is called by its key, and a key says little: where the step
// says what it is for, and the run is of the text the page has drawn,
// its line says that, with the key under it (`runsaid.ts`).

import { hrefOf } from '../address.ts';
import { type ComponentChildren, Fragment } from 'preact';
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { LogLine, Run } from '../api.ts';
import { coalesce } from '../events.ts';
import { anyResult, keptFrom, letGo, resultOf } from '../gave.ts';
import { stepSays } from '../runsaid.ts';
import type { Drawn } from '../runsaid.ts';
import { Badge, Empty, useEvents } from '../ui.tsx';
import { WithRunning } from './running.tsx';
import { classes, duration, show, when } from '../util.ts';
import { useKept } from './returned.tsx';

export interface RunDetailProps {
  id: string;
  step: string | null;
  onStep: (step: string | null) => void;
  /** The text the page has drawn, whose steps say what they are for: a run of that very text is said by it. */
  drawn?: Drawn | null;
  /** Drawn under its steps from the run as it was last read: what it gave out, for whoever shows a run beside something else. */
  foot?: (run: Run) => ComponentChildren;
}

export function RunDetail({ id, step, onStep, drawn, foot }: RunDetailProps) {
  // The run it last read is shown at once when its tab shows again, and read again behind; another run's is not.
  const [read, setRun] = useKept<Run | null>('run.detail', null);
  const run = read?.id === id ? read : null;
  const reload = useMemo(() => coalesce(() => void api.run(id).then(setRun).catch(() => {}), 80), [id]);
  useEffect(() => reload(), [id]);
  useEvents(['step.*', 'run.finished'], (event) => event.source?.run === id && reload(), [id], reload);

  // Which steps have the whole of how they came out opened under them.
  // Kept by its tab with the run they are of: another run has none opened.
  const [openedIn, setOpenedIn] = useKept<{ run: string; steps: Set<string> }>('run.opened', { run: id, steps: new Set() });
  const opened = openedIn.run === id ? openedIn.steps : new Set<string>();
  const setOpened = (steps: Set<string>) => setOpenedIn({ run: id, steps });

  if (!run) return <div class="empty">Reading the run</div>;
  const steps = run.steps ?? [];
  const results = anyResult(steps);
  // Where any step says what it is for, the steps' column is given more of the list's width.
  const told = steps.some((entry) => stepSays(drawn, run, entry.step) !== '');
  const kept = keptFrom(run);
  const gone = letGo(run);
  const open = (name: string) => {
    const next = new Set(opened);
    if (!next.delete(name)) next.add(name);
    setOpened(next);
  };
  return (
    <div class="stack" style="align-content:start">
      <div class="row" style="flex-wrap:wrap">
        <Badge status={run.status} />
        {/* Its address: the run's own page, as a pipeline run, with every step's whole output. */}
        <a class="run-page-link" href={hrefOf({ kind: 'run', id: run.id })} title="Open this run as a page of its own">
          <code>{run.id}</code>
        </a>
        <span class="muted">
          {run.flow === 'main' ? 'main flow' : run.flow.replace('on.', 'handler ')} · started {when(run.started)}
          {run.finished ? ` · took ${duration(run.finished - run.started)}` : ''}
        </span>
      </div>
      {run.error && (
        <pre class="code-box error-text">
          <WithRunning text={run.error} />
        </pre>
      )}
      {kept && <p class="faint">{kept}</p>}
      {gone && <p class="faint run-let-go">{gone}</p>}
      <div class="panel">
        <table class={told ? 'steps-say' : undefined}>
          <thead>
            <tr>
              <th>Step</th>
              <th>Status</th>
              <th class="num">Took</th>
              {results && <th>Result</th>}
            </tr>
          </thead>
          <tbody>
            {steps.map((entry) => {
              const result = resultOf(entry);
              const whole = result?.whole && opened.has(entry.step);
              const says = stepSays(drawn, run, entry.step);
              return (
                <Fragment key={entry.step}>
                  <tr
                    class={kept ? undefined : 'clickable'}
                    aria-selected={!kept && step === entry.step}
                    style={!kept && step === entry.step ? 'background:var(--accent-soft)' : ''}
                    onClick={() => !kept && onStep(step === entry.step ? null : entry.step)}
                  >
                    <td>
                      {says && <span class="step-says">{says}</span>}
                      <code class={says ? 'step-key' : undefined}>{entry.step}</code>
                    </td>
                    <td>
                      <Badge status={entry.status} />
                      {entry.attempt > 1 && <span class="faint"> ×{entry.attempt}</span>}
                    </td>
                    <td class="num">{entry.finished ? duration(entry.finished - entry.started) : ''}</td>
                    {results && (
                      <td class="run-result">
                        {result?.whole ? (
                          // Pressed for the whole of it, which is opened under the line; the line itself is pressed for the step's log.
                          <button
                            type="button"
                            class={classes('run-gave', result.failed && 'error-text')}
                            aria-expanded={!!whole}
                            title={whole ? 'Put the whole of it away' : result.failed ? 'Read the whole of why it failed' : 'Read the whole of what it gave'}
                            onClick={(event) => {
                              event.stopPropagation();
                              open(entry.step);
                            }}
                          >
                            <span class="truncate">
                              <WithRunning text={result.says} plain />
                            </span>
                          </button>
                        ) : (
                          (result?.says ?? '')
                        )}
                      </td>
                    )}
                  </tr>
                  {whole && (
                    <tr class="run-whole">
                      <td colSpan={4}>
                        <pre class={classes('code-box', result.failed && 'error-text')}>
                          <WithRunning text={result.whole === 'error' ? (entry.error ?? '') : show(entry.output)} />
                        </pre>
                      </td>
                    </tr>
                  )}
                </Fragment>
              );
            })}
            {steps.length === 0 && (
              <tr>
                <td colSpan={3} class="faint">
                  No step has started yet.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
      {foot?.(run)}
      {!kept && !gone && (step ? <StepLog run={id} step={step} /> : steps.length > 0 && <p class="faint">Select a step to read its log.</p>)}
    </div>
  );
}

/** A step's output: what is on file, then lines as they are written. */
export function StepLog({ run, step }: { run: string; step: string }) {
  const [lines, setLines] = useState<LogLine[] | null>(null);
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    setLines(null);
    void api.stepLog(run, step).then(setLines).catch(() => setLines([]));
  }, [run, step]);
  useEvents(
    'log.line',
    (event) => {
      if (event.source?.run !== run || event.source?.step !== step) return;
      setLines((known) => [...(known ?? []), { line: (known?.at(-1)?.line ?? 0) + 1, ts: event.ts, stream: event.data.stream, text: event.data.text }]);
    },
    [run, step],
  );
  useLayoutEffect(() => {
    if (box.current) box.current.scrollTop = box.current.scrollHeight;
  }, [lines?.length]);

  return (
    <div class="stack" style="gap:6px">
      <h3>
        Log of <code style="text-transform:none">{step}</code>
      </h3>
      <LogBox lines={lines} boxRef={box} />
    </div>
  );
}

export function LogBox({ lines, boxRef }: { lines: LogLine[] | null; boxRef?: { current: HTMLDivElement | null } }) {
  if (lines && lines.length === 0) return <Empty title="Nothing was written" />;
  return (
    <div class="log" ref={boxRef}>
      {lines?.map((line) => (
        <div key={line.line} class={line.stream === 'e' ? 'e' : line.stream === 's' ? 's' : ''}>
          {line.text || ' '}
        </div>
      ))}
    </div>
  );
}
