// The factory's secrets: each by name, who set it and when, and what names
// it. A value goes in once, through a field that hides it, and is never
// read back: no route gives one out, so this page could not show one if it
// wanted to. It is the owner's page, and is offered to nobody else.

import { useEffect, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Secret } from '../api.ts';
import { me, meta, openTab, toast } from '../store.ts';
import { Empty, Modal, useEvents } from '../ui.tsx';
import { ago } from '../util.ts';

/** Whether whoever is signed in may set and take away secrets. */
export function managesSecrets(): boolean {
  const scopes = me.value?.principal.scopes ?? [];
  return scopes.includes('*') || scopes.includes('admin');
}

/** What a program that printed its secret is written down as having printed. */
const mark = (name: string) => `[the secret \`${name}\`]`;

/** What names a secret, as phrases: what would fail for want of it. */
function namedBy(secret: Secret): string[] {
  const { agents, services, instances } = secret.named;
  return [
    ...agents.map((agent) => `the agent ${agent.name}, as ${agent.variable}`),
    ...services.map((service) => `the service ${service.name}, as ${service.variable}`),
    ...instances.map((instance) => `${instance.blueprint}${instance.project ? ` in ${instance.project}` : ''}, which is running`),
  ];
}

export function Secrets() {
  const may = managesSecrets();
  const [secrets, setSecrets] = useState<Secret[] | null>(null);
  // Which is being set (`''` for a new one), and which is being asked about before it is taken away.
  const [setting, setSetting] = useState<string | null>(null);
  const [leaving, setLeaving] = useState<Secret | null>(null);

  // One reading at a time: what names a secret changes as instances come
  // and go, which in a busy factory is often, and each is told of here.
  const reading = useRef({ now: false, again: false });
  const read = () => {
    if (!may) return;
    if (reading.current.now) {
      reading.current.again = true;
      return;
    }
    reading.current = { now: true, again: false };
    void api
      .secrets()
      .then(setSecrets)
      .catch(() => {})
      .finally(() => {
        const again = reading.current.again;
        reading.current = { now: false, again: false };
        if (again) read();
      });
  };
  useEffect(read, [may]);
  useEvents(['secret.*', 'agent.slot.*', 'service.started', 'service.stopped', 'instance.active', 'instance.finished'], read, [may]);

  if (!may) {
    return (
      <div>
        <div class="view-head">
          <h1>Secrets</h1>
        </div>
        <div class="view-body">
          <Empty title="Secrets are the owner's to manage">
            <p class="muted">
              Setting one and taking one away take <code>admin</code>, which you do not hold.
            </p>
          </Empty>
        </div>
      </div>
    );
  }

  const cli = meta.value?.cli;
  return (
    <div>
      <div class="view-head">
        <h1 class="grow">Secrets</h1>
        <button class="primary" onClick={() => setSetting('')}>
          Add a secret
        </button>
      </div>
      <div class="view-body stack" style="gap:20px">
        <p class="muted secrets-lede">
          A value a program needs and nothing else should see: a payment service's key, a token for a repository. It is kept under a name, sealed, on
          this device, and goes into the environment of a program that names it. <strong>A value is typed once and never shown again</strong>: not
          here, and not by anything else. One that is lost is set again.
        </p>

        <div class="panel">
          <div class="panel-head">
            <h2 class="grow">Kept here</h2>
            <span class="muted">{secrets ? `${secrets.filter((secret) => secret.kept).length} set` : ''}</span>
          </div>
          {secrets && secrets.length === 0 && (
            <Empty title="No secrets yet">
              <p class="muted">
                Add one, then name it where a program reads it: <code>{'env: { STRIPE_KEY: { secret: stripe } }'}</code> in a blueprint, or the{' '}
                <code>secrets</code> cell of an agent's row.
              </p>
            </Empty>
          )}
          {(secrets ?? []).map((secret) => {
            const by = namedBy(secret);
            return (
              <div key={secret.name} class="secret-kept" data-secret={secret.name}>
                <div class="secret-head">
                  <code class="secret-name">{secret.name}</code>
                  {secret.kept ? (
                    <span class="muted grow">
                      set {secret.set ? ago(secret.set) : ''}
                      {secret.set_by ? ` by ${secret.set_by}` : ''}
                    </span>
                  ) : (
                    <span class="grow">
                      <span class="badge warning plain">not set</span> <span class="muted">what names it will not start</span>
                    </span>
                  )}
                  <button onClick={() => setSetting(secret.name)}>{secret.kept ? 'Replace' : 'Set'}</button>
                  {secret.kept && (
                    <button class="danger" onClick={() => setLeaving(secret)}>
                      Remove
                    </button>
                  )}
                </div>
                {by.length === 0 ? (
                  <p class="faint secret-named">Named nowhere: no agent is handed it, and nothing that is running names it.</p>
                ) : (
                  <ul class="secret-named">
                    {secret.named.agents.map((agent) => (
                      <li key={`agent:${agent.name}:${agent.variable}`}>
                        <span class="faint">agent</span> <strong style="color:var(--agent)">{agent.name}</strong>{' '}
                        <span class="muted">
                          is handed it as <code>{agent.variable}</code>
                          {agent.active ? '' : ' (stopped)'}
                        </span>
                      </li>
                    ))}
                    {secret.named.services.map((service) => (
                      <li key={`service:${service.name}:${service.variable}`}>
                        <span class="faint">service</span> <strong>{service.name}</strong>{' '}
                        <span class="muted">
                          was started with it as <code>{service.variable}</code>
                        </span>
                      </li>
                    ))}
                    {secret.named.instances.map((instance) => (
                      <li key={`instance:${instance.id}`}>
                        <span class="faint">running</span>{' '}
                        <button class="link" onClick={() => openTab({ kind: 'instance', id: instance.id })}>
                          {instance.blueprint}
                        </button>{' '}
                        <span class="muted">
                          names it in an <code>env</code>
                          {instance.project ? `, in ${instance.project}` : ''}
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            );
          })}
        </div>

        <p class="faint secrets-lede">
          A program that prints what it was handed has it written down as <code>{mark('name')}</code> and never as the value, in its log and in
          everything made of what it printed. What it sends elsewhere is the program's own doing. A key of several lines can also be set from a
          file: <code>{cli} secret set name --file key.pem</code>
        </p>
      </div>

      {setting !== null && (
        <SetSecret
          name={setting}
          replaces={(secrets ?? []).some((secret) => secret.name === setting && secret.kept)}
          onClose={() => setSetting(null)}
          onSet={read}
        />
      )}
      {leaving && <TakeAway secret={leaving} onClose={() => setLeaving(null)} onGone={read} />}
    </div>
  );
}

/** Sets a secret: its name, unless it is one there already, and its value, typed once. */
function SetSecret(props: { name: string; replaces: boolean; onClose: () => void; onSet: () => void }) {
  const [name, setName] = useState(props.name);
  // What was typed is kept here and nowhere else, and goes when the dialog does.
  const [value, setValue] = useState('');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const lines = value.split('\n').length;

  const submit = async (event: Event) => {
    event.preventDefault();
    if (busy) return;
    setBusy(true);
    try {
      await api.setSecret(name.trim(), value);
      toast(`${name.trim()} is set. It is not shown again.`);
      props.onSet();
      props.onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setBusy(false);
    }
  };

  return (
    <Modal onClose={props.onClose}>
      <form class="stack secret-form" onSubmit={(event) => void submit(event)}>
        <h2>{props.name ? (props.replaces ? `Replace ${props.name}` : `Set ${props.name}`) : 'Add a secret'}</h2>
        <label>
          Name
          <input
            required
            disabled={props.name !== ''}
            value={name}
            placeholder="stripe"
            autocomplete="off"
            spellcheck={false}
            onInput={(event) => setName(event.currentTarget.value)}
          />
          {props.name === '' && (
            <span class="faint">
              Lowercase letters, digits, <code>-</code>, <code>_</code> and <code>.</code>: what a blueprint or an agent's row names it by.
            </span>
          )}
        </label>
        <label>
          Value
          {lines > 1 ? (
            // A field of one line would join a key of several into one, and it would be kept wrong with nothing said.
            <span class="row secret-pasted">
              <span class="grow">{lines} lines were pasted, and are kept as they were pasted.</span>
              <button type="button" onClick={() => setValue('')}>
                Clear
              </button>
            </span>
          ) : (
            <input
              required
              type="password"
              autocomplete="new-password"
              value={value}
              onInput={(event) => setValue(event.currentTarget.value)}
              onPaste={(event) => {
                const pasted = event.clipboardData?.getData('text') ?? '';
                if (!pasted.trim().includes('\n')) return;
                event.preventDefault();
                setValue(pasted);
              }}
            />
          )}
          <span class="faint">
            {props.replaces
              ? 'What is there now is not shown, and is gone once this is set. A service already started has the old one until it is next started.'
              : 'It is hidden as it is typed, and not shown again once it is set.'}
          </span>
        </label>
        {error && <p class="error-text">{error}</p>}
        <div class="row modal-foot">
          <button type="button" onClick={props.onClose}>
            Cancel
          </button>
          <button class="primary" type="submit" disabled={busy || value === ''}>
            {props.replaces ? 'Replace it' : 'Set it'}
          </button>
        </div>
      </form>
    </Modal>
  );
}

/** Asks before a secret is taken away, saying what names it: that is what will fail. */
function TakeAway(props: { secret: Secret; onClose: () => void; onGone: () => void }) {
  const { secret } = props;
  const by = namedBy(secret);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);

  const take = async () => {
    if (busy) return;
    setBusy(true);
    try {
      await api.removeSecret(secret.name);
      toast(`${secret.name} is gone.`);
      props.onGone();
      props.onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setBusy(false);
    }
  };

  return (
    <Modal onClose={props.onClose}>
      <div class="stack secret-form">
        <h2>Remove {secret.name}?</h2>
        {by.length === 0 ? (
          <p class="muted">Nothing names it: no agent is handed it, and nothing that is running names it.</p>
        ) : (
          <>
            <p>It is named by {by.length === 1 ? 'one thing, which' : `${by.length} things, each of which`} fails for want of it when it next starts:</p>
            <ul class="secret-named asked">
              {by.map((phrase) => (
                <li key={phrase}>{phrase}</li>
              ))}
            </ul>
          </>
        )}
        <p class="faint">Its value cannot be had back: to have it again it is set again. A service already started has it until it is next started.</p>
        {error && <p class="error-text">{error}</p>}
        <div class="row modal-foot">
          <button type="button" onClick={props.onClose}>
            Cancel
          </button>
          <button class="danger" disabled={busy} onClick={() => void take()}>
            Remove it
          </button>
        </div>
      </div>
    </Modal>
  );
}
