// What the drawer shows when a node of the graph is clicked.

import type { ComponentChildren } from 'preact';
import type { GraphNode, Step } from '../api.ts';
import { Badge, Field } from '../ui.tsx';
import { duration, hasContent, show } from '../util.ts';
import { WithRunning } from '../views/running.tsx';
import { keyOf, runs, says } from './tile.ts';

const KINDS: Record<GraphNode['kind'], string> = {
  action: 'Runs an action',
  call: 'Runs another blueprint and waits for it',
  spawn: 'Starts another blueprint as a child that lives on',
  task: 'Runs the steps inside it as one',
  trigger: 'Starts this handler',
  attachment: 'A blueprint attached to this one',
};

const HOOKS: Record<string, string> = {
  on_start: 'before it starts',
  on_success: 'when it has gone well',
  on_fail: 'if it fails',
  on_timeout: 'if it times out',
  on_finish: 'when it has ended, however it went',
};

export interface NodeDetailsProps {
  node: GraphNode;
  status?: string;
  /** The step as it went in the run being shown, when there is one. */
  step?: Step;
  onShowText: () => void;
  onShowLog?: () => void;
}

export function NodeDetails({ node, status, step, onShowText, onShowLog }: NodeDetailsProps) {
  const rows: [string, ComponentChildren][] = [];
  if (node.when) rows.push(['Only if', <code>{node.when}</code>]);
  if (node.for_each) rows.push(['Once for each of', <code>{node.for_each}</code>]);
  if (node.device) rows.push(['On device', <code>{node.device}</code>]);
  if (node.timeout) rows.push(['Gives up after', node.timeout]);
  if (node.retry) rows.push(['Retries', node.retry]);
  if (node.allow_failure) rows.push(['If it fails', 'the run carries on']);
  if (node.measure) rows.push(['Measured as', <code>{node.measure}</code>]);

  return (
    <>
      <div class="stack" style="gap:6px">
        <div class="row">
          <span class="muted grow">{KINDS[node.kind]}</span>
          {status && <Badge status={status} />}
        </div>
        {/* What it says of itself is the drawer's own heading where it says anything; here is the whole of it, and then what the text calls it. */}
        {node.description && <p class="node-says">{node.description}</p>}
        {says(node.description) ? (
          <span class="node-key">
            <code>{keyOf(node)}</code> <span class="faint">·</span> <code>{runs(node)}</code>
          </span>
        ) : (
          <code style="font-size:calc(13px * var(--size))">{node.target}</code>
        )}
      </div>

      {step && (
        <Field label="In this run">
          <div class="stack" style="gap:6px">
            <span>
              {step.status}
              {step.attempt > 1 ? `, attempt ${step.attempt}` : ''}
              {step.finished ? ` in ${duration(step.finished - step.started)}` : ''}
              {step.tolerated ? ' (failure tolerated)' : ''}
            </span>
            {step.error && <pre class="code-box error-text">{step.error}</pre>}
            {hasContent(step.output) && (
              // What it gave, with anything it started said for what it is and opened from here.
              <pre class="code-box step-gave">
                <WithRunning text={show(step.output)} />
              </pre>
            )}
            {onShowLog && (
              <div>
                <button onClick={onShowLog}>Show its log</button>
              </div>
            )}
          </div>
        </Field>
      )}

      {hasContent(node.with) && (
        <Field label={node.kind === 'action' ? 'With' : 'Parameters'}>
          <pre class="code-box">
            <WithRunning text={show(node.with)} />
          </pre>
        </Field>
      )}

      {rows.length > 0 && (
        <div class="stack" style="gap:8px">
          {rows.map(([label, value]) => (
            <Field key={label} label={label}>
              <span>{value}</span>
            </Field>
          ))}
        </div>
      )}

      {node.hooks.length > 0 && (
        <Field label="Hooks">
          <div class="stack" style="gap:6px">
            {node.hooks.map((hook, index) => (
              <div key={index}>
                <span class="muted">{HOOKS[hook.point] ?? hook.point}: </span>
                <code>{hook.action}</code>
                {hook.when && <span class="muted"> if {hook.when}</span>}
                {hasContent(hook.with) && <pre class="code-box" style="margin-top:4px">{show(hook.with)}</pre>}
              </div>
            ))}
          </div>
        </Field>
      )}

      {node.watch.length > 0 && (
        <Field label="Watchers">
          <div class="stack" style="gap:6px">
            {node.watch.map((watch, index) => (
              <div key={index}>
                <span class="muted">{watch.rule}: </span>
                <code>{watch.action}</code>
                {watch.fail && <span class="error-text"> and fail the step</span>}
                {hasContent(watch.with) && <pre class="code-box" style="margin-top:4px">{show(watch.with)}</pre>}
              </div>
            ))}
          </div>
        </Field>
      )}

      <div class="row">
        <button onClick={onShowText}>
          Show in text{node.line ? ` (line ${node.line})` : ''}
        </button>
      </div>
    </>
  );
}
