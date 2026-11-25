// Extensions: what agents can run on, beyond what is built in, and the
// actions and blueprints that set up somebody else's product.
//
// The application knows no agent by name, and no other product either. Each
// comes as a package, and this is where one is found, installed and looked
// at: a list in the side panel (reached from the rail), and a page for each
// in a tab.

import { signal } from '@preact/signals';
import { useEffect, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { ActionSpec, Extension, Model } from '../api.ts';
import { DIFFERS, isBehind, mayUpdate, said } from '../came.ts';
import { stream } from '../events.ts';
import { attempt, me, meta, openTab, toast } from '../store.ts';
import { grab } from '../titlebar.tsx';
import { Badge, Empty, Prose, SideFold } from '../ui.tsx';
import { askUpdate } from './came.tsx';

const installed = signal<Extension[]>([]);
const offered = signal<Extension[]>([]);
let asked = false;

// A daemon that was started before packages could bring actions says nothing
// of them, and this page may be newer than the daemon it is talking to.
const whole = (extension: Extension): Extension => ({ ...extension, actions: extension.actions ?? [], install: extension.install ?? '' });

function reload(): void {
  void api
    .extensions()
    .then((answer) => {
      installed.value = answer.extensions.map(whole);
      offered.value = answer.offered.map(whole);
    })
    .catch(() => {});
}
stream.on(['extension.added', 'extension.removed'], reload);

/** Reads the lists the first time anything shows them; after that the event stream keeps them current. */
function useExtensions(): void {
  useEffect(() => {
    if (!asked) {
      asked = true;
      reload();
    }
  }, []);
}

const titleOf = (extension: Extension) => extension.title || extension.name;

async function install(what: { name: string } | { source: string }): Promise<void> {
  const answer = await attempt(() => api.installExtension(what));
  if (!answer) return;
  const added = whole(answer);
  toast(
    added.available
      ? `${titleOf(added)} is installed. ${added.agent ? 'Agents can run on it now.' : added.blueprints.length > 0 ? 'Its blueprints can be applied now.' : 'Blueprints can do its actions now.'}`
      : `${titleOf(added)} is installed, but its program, ${added.program}, is not on the daemon's device yet. ${added.install}`.trim(),
    !added.available,
  );
  reload();
}

async function remove(extension: Extension): Promise<void> {
  if (await attempt(() => api.removeExtension(extension.name))) reload();
}

function Row({ extension, isInstalled, admin }: { extension: Extension; isInstalled: boolean; admin: boolean }) {
  // Installed as a copy, it may not be the one that came with this version: said, and taken only by being asked for.
  const behind = isInstalled && isBehind(extension);
  return (
    <div class="ext-row" data-extension={extension.name}>
      <button class="ext-open" onClick={() => openTab({ kind: 'extension', name: extension.name })}>
        <strong class="truncate">{titleOf(extension)}</strong>
        <span class="faint truncate">{extension.description}</span>
        {behind && (
          <span class="ext-behind truncate" title={`${DIFFERS} with this version`}>
            {DIFFERS}
          </span>
        )}
      </button>
      {behind && admin && (
        <button class="small ext-update" onClick={() => askUpdate(extension)}>
          Update
        </button>
      )}
      {isInstalled ? (
        <span
          class={`ext-dot ${extension.available ? 'ok' : 'off'}`}
          title={extension.available ? 'Ready' : `${extension.program} is not installed on the daemon's device`}
        />
      ) : (
        admin && (
          <button class="primary small" onClick={() => void install({ name: extension.name })}>
            Install
          </button>
        )
      )}
    </div>
  );
}

/** The side panel: what is installed, and what came with the product and is not. */
export function ExtensionsSide() {
  useExtensions();
  const admin = mayUpdate(me.value?.principal.scopes);
  const file = useRef<HTMLInputElement>(null);
  const fromFile = async (event: Event) => {
    const input = event.currentTarget as HTMLInputElement;
    const chosen = input.files?.[0];
    input.value = '';
    if (chosen) await install({ source: await chosen.text() });
  };
  return (
    <nav class="sidebar" aria-label="Extensions">
      <div class="brand" onMouseDown={grab}>
        <span>Extensions</span>
        <span class="brand-acts">
          {admin && (
            <>
              <input ref={file} type="file" accept=".yaml,.yml" hidden onChange={(event) => void fromFile(event)} />
              <button
                class="ghost icon"
                aria-label="Install from a file"
                title="Install a package from its extension.yaml"
                onClick={() => file.current?.click()}
              >
                +
              </button>
            </>
          )}
          <SideFold class="ghost icon side-fold" />
        </span>
      </div>
      <div class="nav">
        <h3>Installed</h3>
        {installed.value.map((extension) => (
          <Row key={extension.name} extension={extension} isInstalled admin={admin} />
        ))}
        {installed.value.length === 0 && (
          <p class="faint" style="padding:2px 8px;font-size:calc(12px * var(--size))">
            None yet. Out of the box agents run on a built-in assistant, which needs no model and understands only a few requests.
          </p>
        )}
        {offered.value.length > 0 && <h3>Came with this installation</h3>}
        {offered.value.map((extension) => (
          <Row key={extension.name} extension={extension} isInstalled={false} admin={admin} />
        ))}
      </div>
    </nav>
  );
}

/** One extension, in a tab: what it is, whether it can run here, and the models it offers. */
export function ExtensionView({ name }: { name: string }) {
  useExtensions();
  const admin = mayUpdate(me.value?.principal.scopes);
  const here = installed.value.find((extension) => extension.name === name);
  const extension = here ?? offered.value.find((entry) => entry.name === name);
  const [models, setModels] = useState<Model[] | null>(null);
  const [trouble, setTrouble] = useState('');
  const ask = (fresh: boolean) => {
    setModels(null);
    setTrouble('');
    api
      .models(name, fresh)
      .then(setModels)
      .catch((err) => setTrouble(err instanceof Error ? err.message : String(err)));
  };
  const usable = Boolean(here?.available && here.agent);
  useEffect(() => {
    if (usable) ask(false);
  }, [name, usable]);

  if (!extension) return <Empty title="This extension is not here">It may have been removed.</Empty>;
  return (
    <div class="page">
      <div class="view-head">
        <h1 class="truncate">{titleOf(extension)}</h1>
        <span class="faint">{extension.name}</span>
        <span class="grow" />
        {here ? (
          <Badge status={extension.available ? 'active' : 'stopped'}>{extension.available ? 'ready' : 'program not installed'}</Badge>
        ) : (
          <span class="faint">not installed</span>
        )}
        {admin &&
          (here ? (
            <button class="danger" onClick={() => void remove(extension)}>
              Remove
            </button>
          ) : (
            <button class="primary" onClick={() => void install({ name: extension.name })}>
              Install
            </button>
          ))}
      </div>
      <div class="stack ext-body" style="padding:16px 20px">
        {isBehind(here) && here && (
          <div class="panel ext-came" role="note">
            <div class="ext-came-says">
              {said(here, meta.value?.name ?? 'the program').map((line) => (
                <p key={line}>
                  <Prose text={line} />
                </p>
              ))}
            </div>
            {admin && (
              <button class="primary" onClick={() => askUpdate(here)}>
                Update
              </button>
            )}
          </div>
        )}
        <p>{extension.description}</p>
        {extension.program && (
          <p class="muted">
            It runs the program <code>{extension.program}</code>, which has to be installed and signed in on the daemon's device as
            whoever runs the daemon.{' '}
            {extension.available ? 'It is there.' : `It is not there yet, so nothing can run on this until it is. ${extension.install}`.trim()}{' '}
            {extension.agent
              ? "What an agent is asked and what it reads go to that program's service, under that account; the application itself sends nothing anywhere."
              : 'What it then does is that program\'s doing, as whoever runs the daemon; the application itself sends nothing anywhere.'}
          </p>
        )}
        {extension.actions.length > 0 && (
          <div class="panel">
            <div class="panel-head">
              <h2>Actions</h2>
              <span class="muted grow">
                {here
                  ? `for a blueprint to do; using them takes extensions:${extension.name}`
                  : 'there for blueprints to do once it is installed'}
              </span>
            </div>
            <table>
              <tbody>
                {extension.actions.map((action) => (
                  <tr key={action.name}>
                    <td>
                      <code>{action.name}</code>
                    </td>
                    <td class="muted">{shapeOf(action)}</td>
                    <td>{action.summary}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        {extension.blueprints.length > 0 && (
          <div class="panel">
            <div class="panel-head">
              <h2>Blueprints</h2>
              <span class="muted grow">
                {here ? "among the factory's Blueprints, applied like any other" : 'there to be applied once it is installed'}
              </span>
            </div>
            <table>
              <tbody>
                {extension.blueprints.map((blueprint) => (
                  <tr key={blueprint}>
                    <td>
                      <code>{blueprint}</code>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        {extension.agent && <Models extension={extension} here={Boolean(here)} usable={usable} models={models} trouble={trouble} ask={ask} />}
      </div>
    </div>
  );
}

/** What an action takes, with what may be left out in brackets, and what it gives. */
function shapeOf(action: ActionSpec): string {
  const takes = action.args.map((arg) => (arg.required ? arg.name : `[${arg.name}]`)).join(', ');
  const gives = action.outputs.map(([name]) => name).join(', ');
  return gives ? `${takes} \u2192 ${gives}`.trim() : takes;
}

/** The models an extension's program offers, as the program itself lists them. */
function Models({
  extension,
  here,
  usable,
  models,
  trouble,
  ask,
}: {
  extension: Extension;
  here: boolean;
  usable: boolean;
  models: Model[] | null;
  trouble: string;
  ask: (fresh: boolean) => void;
}) {
  return (
    <div class="panel">
      <div class="panel-head">
        <h2>Models</h2>
        <span class="muted grow">as the program itself lists them</span>
        {usable && <button onClick={() => ask(true)}>Ask again</button>}
      </div>
      {!here ? (
        <p class="muted" style="padding:12px 14px">
          Install it to see which models it offers.
        </p>
      ) : !usable ? (
        <p class="muted" style="padding:12px 14px">
          Its program is not installed, so it cannot be asked.
        </p>
      ) : trouble ? (
        <p class="muted" style="padding:12px 14px">
          It could not be asked: {trouble}
        </p>
      ) : models === null ? (
        <p class="muted" style="padding:12px 14px">
          Asking {titleOf(extension)} which models it offers
        </p>
      ) : models.length === 0 ? (
        <p class="muted" style="padding:12px 14px">
          It does not say; a model is named by hand when an agent is brought in.
        </p>
      ) : (
        <table>
          <tbody>
            {models.map((model) => (
              <tr key={model.id}>
                <td>
                  <strong>{model.label}</strong>
                  {model.default && <span class="faint"> its default</span>}
                </td>
                <td>
                  <code>{model.id}</code>
                </td>
                <td class="muted">{model.note}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
