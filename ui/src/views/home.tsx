// The start page: where things stand, and the next thing worth doing.

import { useEffect, useState } from 'preact/hooks';
import { api } from '../api.ts';
import { agentsTable, examplePath, hasRealAgent, whatNext } from '../next.ts';
import type { Next } from '../next.ts';
import { agents, asking, channels, collections, learnUsed, meta, openTab, projects, providers, side, toast, used } from '../store.ts';
import { useEvents } from '../ui.tsx';

/** The blueprints that have been applied, by name; kept current by events, not by asking again on a timer. */
function useApplied(): Set<string> {
  const [applied, setApplied] = useState<Set<string>>(new Set());
  const read = () =>
    void api
      .instances({ limit: 200 })
      .then((instances) => setApplied(new Set(instances.map((instance) => instance.blueprint))))
      .catch(() => {});
  useEffect(read, []);
  useEvents('instance.*', read);
  return applied;
}

export function Home() {
  const applied = useApplied();
  const firstChannel = channels.value.find((one) => !one.conversation);
  const anyAgent = agents.value.some((slot) => slot.active);
  const firstProject = projects.value[0];
  // A blueprint of its own at the top of where they are kept, which the factory keeps for it and the repository has none of.
  const fleetFile = firstProject && (used.value[firstProject.id]?.own ?? []).find((one) => !one.name.includes('/'));
  const steps: { done: boolean; title: string; detail: string; go?: () => void; action?: string }[] = [
    {
      done: Boolean(firstChannel),
      title: 'Create a channel',
      detail: 'Channels are where people and agents talk.',
      go: () => (asking.value = 'channel'),
      action: 'Create one',
    },
    {
      done: anyAgent,
      title: 'Add an agent',
      detail: 'An agent joins a channel from a slot, with its own credential and permissions.',
      go: firstChannel && (() => openTab({ kind: 'channel', id: firstChannel.id })),
      action: 'Open the channel',
    },
    {
      done: Boolean(firstProject),
      title: 'Ask the agent to set up a project',
      detail:
        'For example: "set up a new project called Shop". It appears under Resources, with its files on its own page. One you add yourself, with the + beside Resources, starts with a board, a fleet and a library unless you turn one off, and the steps below are then done.',
      go: firstChannel && (() => openTab({ kind: 'channel', id: firstChannel.id })),
      action: 'Open the channel',
    },
    {
      done: applied.has('std/board'),
      title: 'Set up its board with a blueprint',
      detail:
        'A blueprint is a file that says how something is done. This small one sets up the board: read it, see it as a graph, choose the columns and press Apply.',
      go: firstProject && (() => openTab({ kind: 'blueprint', name: 'std/board', project: firstProject.id })),
      action: 'Open the blueprint',
    },
    {
      done: Boolean(fleetFile),
      title: 'Ask the agent to set up the fleet',
      detail: `It writes a blueprint, fleet.${meta.value?.blueprint_ext}, which the factory keeps for the project. It appears under the project's Blueprints in Resources.`,
    },
    {
      done: Boolean(fleetFile) && [...applied].some((name) => name !== 'std/board' && !name.startsWith('std/')),
      title: 'Open the fleet blueprint and apply it',
      detail: 'Read it as text, switch to Node to see it as a graph and click through its steps, then press Apply to start it.',
      go: firstProject && fleetFile ? () => openTab({ kind: 'file', project: firstProject.id, path: fleetFile.file }) : undefined,
      action: 'Open it',
    },
  ];
  const next = steps.findIndex((step) => !step.done);
  // Once every step is done, what is worth doing next with the project they were done for.
  const cli = meta.value?.cli ?? '';
  const ext = meta.value?.blueprint_ext ?? '';
  const own = firstProject ? used.value[firstProject.id] : undefined;
  const example = firstProject ? examplePath('research', 'ask', ext, own?.written ?? '@blueprints') : '';
  const table = firstProject ? agentsTable(collections.value[firstProject.id] ?? []) : null;
  const more: Next[] =
    next === -1 && firstProject
      ? whatNext({
          cli,
          project: { id: firstProject.id, name: firstProject.name },
          agents: table,
          real: hasRealAgent(agents.value, firstProject.id, providers.value.filter((provider) => provider.kind === 'builtin').map((provider) => provider.name)),
          example: (own?.own ?? []).some((one) => one.file === example),
        })
      : [];
  const go = (line: Next) => {
    if (!firstProject) return;
    const project = firstProject.id;
    if (line.key === 'card') openTab({ kind: 'board', project });
    else if (line.key === 'agent' && table) openTab({ kind: 'table', project, name: table });
    else if (line.key === 'agent') side.value = 'extensions';
    else if (line.key === 'device') openTab({ kind: 'settings' });
    else if (line.done) openTab({ kind: 'file', project, path: example });
    else
      void api.addExample(project, 'research').then(
        async () => {
          await learnUsed();
          toast(`research is among ${firstProject.name}'s blueprints.`);
          openTab({ kind: 'file', project, path: example });
        },
        (refused: Error) => toast(refused.message, true),
      );
  };

  return (
    <div class="view-body" style="max-width:720px;padding-top:28px">
      <div class="stack">
        <div class="panel">
          <div class="panel-head">
            <h2>Setup Guide</h2>
          </div>
          {steps.map((step, index) => (
            <div key={step.title} class="row" style="padding:10px 14px;border-bottom:1px solid var(--line);align-items:flex-start">
              <span class={`badge ${step.done ? 'ok' : index === next ? 'warning' : ''}`} style="margin-top:1px">
                {step.done ? 'done' : index === next ? 'next' : 'later'}
              </span>
              <div class="grow">
                <div style="font-weight:600">{step.title}</div>
                <div class="muted">{step.detail}</div>
              </div>
              {step.go && index === next && (
                <button class="primary" onClick={step.go}>
                  {step.action}
                </button>
              )}
            </div>
          ))}
        </div>
        {more.length > 0 && (
          <div class="panel" data-guide="next">
            <div class="panel-head">
              <h2>What next</h2>
            </div>
            {more.map((line) => (
              <div key={line.key} class="row" data-next={line.key} style="padding:10px 14px;border-bottom:1px solid var(--line);align-items:flex-start">
                {line.done && (
                  <span class="badge ok" style="margin-top:1px">
                    done
                  </span>
                )}
                <div class="grow">
                  <div style="font-weight:600">{line.title}</div>
                  <div class="muted">{line.detail}</div>
                </div>
                <button class={line.done ? '' : 'primary'} style="white-space:nowrap" onClick={() => go(line)}>
                  {line.action}
                </button>
              </div>
            ))}
          </div>
        )}
        <p class="faint">
          Everything here can also be done from a terminal with <code>{meta.value?.cli}</code>; agents use the same command.
          Try <code>{meta.value?.cli} help</code>.
        </p>
      </div>
    </div>
  );
}
