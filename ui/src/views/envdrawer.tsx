// One of a project's environments, opened in the frame's one drawer beside
// its page, as a card's or a row's form is opened: its variables, its
// secrets, and its taking away. The page is `projectenvs.tsx`; what is
// worked out is `projectenvs.ts`, and what is read and followed is
// `envsheld.ts`. The daemon answers with no secret's value, so nothing
// here could show one.
//
// It is the tab's it was opened beside (`drawerkept.ts`): what is being
// typed (a variable's new name and value, a value being written, what a
// secret is to be handed to agents as) is kept by the tab's drawer scope,
// and given back it asks whether the environment is still there (`gone`).
// One taken away while the drawer is open, from here or anywhere, puts the
// drawer away. Escape is first for a question it asked.

import { useEffect, useRef, useState } from 'preact/hooks';
import { call } from '../api.ts';
import { goneBy } from '../drawerkept.ts';
import { environmentsOf, readEnvironments } from '../envsheld.ts';
import { type May, type Secret, type Variable, handedAs, mayOf, secretSaid, valueOf, written } from '../projectenvs.ts';
import { type Drawer, drawer, me, toast } from '../store.ts';
import { Modal } from '../ui.tsx';
import { ago } from '../util.ts';
import { Loaded } from './envrun.tsx';
import { RulesPart, RunsPart } from './envrules.tsx';
import { FlagsIn } from './projectflags.tsx';
import { useKept } from './returned.tsx';

/** Where an environment is asked for of the daemon. */
export const at = (project: string, environment: string) => `/projects/${encodeURIComponent(project)}/environments/${encodeURIComponent(environment)}`;

/** Asks the daemon, saying what it refuses in its sentence; whether it was done. */
export async function asking(work: () => Promise<unknown>, done?: string): Promise<boolean> {
  try {
    await work();
    if (done) toast(done);
    return true;
  } catch (err) {
    toast(err instanceof Error ? err.message : String(err), true);
    return false;
  }
}

/** Which environment each drawer opened here is of: the page marks its row by the drawer that is open. */
const opened = new WeakMap<Drawer, { project: string; name: string }>();

/** The environment the open drawer is of, where it is one. */
export const openedEnvironment = (): { project: string; name: string } | null => (drawer.value && opened.get(drawer.value)) ?? null;

/** Opens `name` of `project` in the drawer. */
export function openEnvironment(project: string, name: string): void {
  const made: Drawer = {
    title: name,
    body: () => <Opened key={`${project}/${name}`} project={project} name={name} />,
    gone: goneBy(() => call('GET', at(project, name))),
  };
  opened.set(made, { project, name });
  drawer.value = made;
}

/** Puts the drawer away where it is of this environment, and only then. */
function putAway(project: string, name: string): void {
  const open = drawer.peek();
  const of = open && opened.get(open);
  if (of && of.project === project && of.name === name) drawer.value = null;
}

/** One environment, in the drawer: its variables, its secrets, and its taking away. */
function Opened({ project, name }: { project: string; name: string }) {
  const may = mayOf(me.value?.principal.scopes ?? []);
  const listed = environmentsOf.value[project];
  const environment = listed?.find((one) => one.name === name) ?? null;
  const base = at(project, name);
  const again = () => void readEnvironments(project);
  const [setting, setSetting] = useState<string | null>(null);
  const [leaving, setLeaving] = useState<string | null>(null);
  const [removing, setRemoving] = useState(false);
  const [adding, setAdding] = useKept(`env-adding:${project}/${name}`, { name: '', value: '' });
  // A value being written, kept until it is sent (as the box is left) and the daemon has answered.
  const [drafts, setDrafts] = useKept<Record<string, string>>(`env-drafts:${project}/${name}`, {});

  // Not among what was last read: one just made is read a moment later, and one taken away is gone. The daemon is asked which.
  const missing = listed !== undefined && environment === null;
  useEffect(() => {
    if (!missing) return;
    void goneBy(() => call('GET', base))().then((gone) => gone && putAway(project, name));
  }, [missing, base]);

  // Escape is first for a question asked here: the frame would put the drawer away with it.
  const asked = useRef<(() => void) | undefined>(undefined);
  asked.current = setting !== null ? () => setSetting(null) : leaving !== null ? () => setLeaving(null) : removing ? () => setRemoving(false) : undefined;
  useEffect(() => {
    const first = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || !asked.current) return;
      event.stopPropagation();
      asked.current();
    };
    window.addEventListener('keydown', first, true);
    return () => window.removeEventListener('keydown', first, true);
  }, []);

  if (!environment) return <p class="muted envs-pad">Reading {name}…</p>;
  // A feature flag's variable is said with the flags, and changed as the flag and never here.
  const vars = Object.entries(environment.vars).filter(([variable]) => !/^FLAG_/i.test(variable));
  const secrets = environment.secrets ? Object.entries(environment.secrets) : null;

  const setVar = (variable: string, body: Record<string, unknown>) => asking(() => call('PUT', `${base}/vars/${encodeURIComponent(variable)}`, body)).then(again);
  const sendDraft = (variable: string, was: Variable['value'], now: string) => {
    if (written(was) === now) return setDrafts(({ [variable]: _, ...rest }) => rest);
    void setVar(variable, { value: valueOf(now) }).then(() => setDrafts(({ [variable]: _, ...rest }) => rest));
  };
  const addVar = async (event: Event) => {
    event.preventDefault();
    const variable = adding.name.trim();
    if (!variable) return;
    if (await asking(() => call('PUT', `${base}/vars/${encodeURIComponent(variable)}`, { value: valueOf(adding.value) }))) {
      setAdding({ name: '', value: '' });
      again();
    }
  };

  return (
    <div class="envs-opened" data-opened={environment.name}>
      <Loaded project={project} environment={environment} />
      <RulesPart base={base} rules={environment.rules} onChanged={again} asking={asking} />
      <RunsPart runs={environment.runs} />
      <section class="envs-part">
        <h3>Variables</h3>
        {vars.length === 0 && <p class="faint">None yet.</p>}
        {vars.length > 0 && (
          <div class="envs-vars">
            {vars.map(([variable, kept]) => (
              <div key={variable} class="envs-var" data-var={variable}>
                <code class="envs-var-name">{variable}</code>
                <input
                  aria-label={`The value of ${variable}`}
                  disabled={!may.change}
                  value={drafts[variable] ?? written(kept.value)}
                  onInput={(event) => {
                    const now = event.currentTarget.value;
                    setDrafts((was) => ({ ...was, [variable]: now }));
                  }}
                  onKeyDown={(event) => event.key === 'Enter' && event.currentTarget.blur()}
                  onChange={(event) => sendDraft(variable, kept.value, event.currentTarget.value)}
                />
                <label class="envs-switch">
                  <input type="checkbox" checked={kept.agents} disabled={!may.change} onChange={(event) => void setVar(variable, { agents: event.currentTarget.checked })} />
                  <span>For agents</span>
                </label>
                {may.change && (
                  <button class="ghost icon" aria-label={`Remove ${variable}`} title={`Remove ${variable}`} onClick={() => void asking(() => call('DELETE', `${base}/vars/${encodeURIComponent(variable)}`)).then(again)}>
                    ×
                  </button>
                )}
              </div>
            ))}
          </div>
        )}
        {may.change && (
          <form class="envs-var envs-new" onSubmit={(event) => void addVar(event)}>
            <input aria-label="A new variable's name" placeholder="name" spellcheck={false} value={adding.name} onInput={(event) => setAdding({ ...adding, name: event.currentTarget.value })} />
            <input aria-label="Its value" placeholder="value" value={adding.value} onInput={(event) => setAdding({ ...adding, value: event.currentTarget.value })} />
            <button type="submit" disabled={adding.name.trim() === ''}>
              Add a variable
            </button>
          </form>
        )}
        <p class="faint">Nothing is handed to an agent unless it is marked for agents. A variable is not for secrets: it is shown to whoever may read the project.</p>
      </section>

      <FlagsIn project={project} environment={environment.name} />

      <section class="envs-part">
        <div class="row">
          <h3 class="grow">Secrets</h3>
          {may.secrets && <button onClick={() => setSetting('')}>Set a secret…</button>}
        </div>
        {secrets === null && <p class="faint">Their names are shown to whoever may use what the factory keeps secret.</p>}
        {secrets && secrets.length === 0 && <p class="faint">None yet.</p>}
        {secrets?.map(([secret, said]) => (
          <SecretLine key={secret} base={base} name={secret} secret={said} may={may} onSet={() => setSetting(secret)} onLeave={() => setLeaving(secret)} onChanged={again} />
        ))}
      </section>

      {may.change && (
        <section class="envs-part envs-away">
          <button class="danger" onClick={() => setRemoving(true)}>
            Remove this environment…
          </button>
        </section>
      )}

      {setting !== null && <SetSecret base={base} name={setting} replaces={!!secrets?.some(([secret, said]) => secret === setting && said.kept !== false)} onClose={() => setSetting(null)} onSet={again} />}
      {leaving !== null && <TakeSecret base={base} name={leaving} onClose={() => setLeaving(null)} onGone={again} />}
      {removing && <RemoveEnvironment base={base} name={environment.name} onClose={() => setRemoving(false)} onGone={() => (again(), putAway(project, name))} />}
    </div>
  );
}

/** A secret's line: whether a value is kept and who set it, what an agent is handed it as, and what may be done with it. */
function SecretLine(props: { base: string; name: string; secret: Secret; may: May; onSet: () => void; onLeave: () => void; onChanged: () => void }) {
  const { name, secret, may } = props;
  const [variable, setVariable] = useKept(`env-hand:${props.base}/${name}`, secret.agents ?? '');
  // What is written follows what the daemon says only when that changes: set on the line's first drawing too, an
  // effect that runs a frame after it was drawn took away what had been typed in that frame.
  const said = useRef(secret.agents);
  useEffect(() => {
    if (said.current === secret.agents) return;
    said.current = secret.agents;
    setVariable(secret.agents ?? '');
  }, [secret.agents]);
  const mark = (agents: string | null) => void asking(() => call('PUT', `${props.base}/secrets/${encodeURIComponent(name)}/agents`, { agents })).then(props.onChanged);
  return (
    <div class="envs-secret" data-secret={name}>
      <div class="envs-secret-head">
        <code>{name}</code>
        <span class={secret.kept === false ? 'badge warning plain' : 'muted'}>{secretSaid(secret, ago)}</span>
        <span class="grow" />
        {may.secrets && (
          <span class="envs-secret-acts">
            <button onClick={props.onSet}>{secret.kept === false ? 'Set…' : 'Replace…'}</button>
            <button class="danger" onClick={props.onLeave}>
              Remove…
            </button>
          </span>
        )}
      </div>
      <div class="envs-secret-agents">
        <span class="muted">{handedAs(secret)}</span>
        {may.secrets && (
          <form class="row envs-hand" onSubmit={(event) => (event.preventDefault(), variable.trim() && mark(variable.trim()))}>
            <input aria-label={`What an agent is handed ${name} as`} placeholder="STRIPE_KEY" spellcheck={false} value={variable} onInput={(event) => setVariable(event.currentTarget.value)} />
            <button type="submit" disabled={variable.trim() === '' || variable.trim() === secret.agents}>
              Hand to agents
            </button>
            {secret.agents && (
              <button type="button" onClick={() => mark(null)}>
                Not handed
              </button>
            )}
          </form>
        )}
      </div>
    </div>
  );
}

/** Sets an environment's secret, typed once in a field that hides it. */
function SetSecret(props: { base: string; name: string; replaces: boolean; onClose: () => void; onSet: () => void }) {
  const [name, setName] = useState(props.name);
  const [value, setValue] = useState('');
  const [error, setError] = useState('');
  const submit = async (event: Event) => {
    event.preventDefault();
    try {
      await call('PUT', `${props.base}/secrets/${encodeURIComponent(name.trim())}`, { value });
      toast(`${name.trim()} is set. It is not shown again.`);
      props.onSet();
      props.onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };
  return (
    <Modal onClose={props.onClose}>
      <form class="stack secret-form" onSubmit={(event) => void submit(event)}>
        <h2>{props.name ? (props.replaces ? `Replace ${props.name}` : `Set ${props.name}`) : 'Set a secret'}</h2>
        <label>
          Name
          <input required disabled={props.name !== ''} value={name} placeholder="stripe" autocomplete="off" spellcheck={false} onInput={(event) => setName(event.currentTarget.value)} />
        </label>
        <label>
          Value
          <input required type="password" autocomplete="new-password" value={value} onInput={(event) => setValue(event.currentTarget.value)} />
          <span class="faint">{props.replaces ? 'What is there now is not shown, and is gone once this is set.' : 'It is hidden as it is typed, and not shown again once it is set.'}</span>
        </label>
        {error && <p class="error-text">{error}</p>}
        <div class="row modal-foot">
          <button type="button" onClick={props.onClose}>
            Cancel
          </button>
          <button class="primary" type="submit" disabled={value === '' || name.trim() === ''}>
            {props.replaces ? 'Replace it' : 'Set it'}
          </button>
        </div>
      </form>
    </Modal>
  );
}

/** Where a secret is named, as phrases: what would fail for want of it. */
interface Named {
  instances: { key: string; blueprint: string }[];
  services: { name: string; variable: string }[];
  agents: { name: string; variable: string }[];
}

/** Asks before an environment's secret is taken away, saying what names it, where whoever asks may be told. */
function TakeSecret(props: { base: string; name: string; onClose: () => void; onGone: () => void }) {
  const [named, setNamed] = useState<Named | null>(null);
  const [error, setError] = useState('');
  useEffect(() => {
    void call<{ secrets: { name: string; named: Named }[] }>('GET', `${props.base}/secrets`)
      .then((read) => setNamed(read.secrets.find((one) => one.name === props.name)?.named ?? null))
      .catch(() => {});
  }, [props.name]);
  const phrases = named
    ? [
        ...named.agents.map((agent) => `the agent ${agent.name}, as ${agent.variable}`),
        ...named.services.map((service) => `the service ${service.name}, as ${service.variable}`),
        ...named.instances.map((instance) => `${instance.blueprint} (${instance.key}), which is running`),
      ]
    : [];
  const take = async () => {
    try {
      await call('DELETE', `${props.base}/secrets/${encodeURIComponent(props.name)}`);
      toast(`${props.name} is gone.`);
      props.onGone();
      props.onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };
  return (
    <Modal onClose={props.onClose}>
      <div class="stack secret-form">
        <h2>Remove {props.name}?</h2>
        {phrases.length === 0 ? (
          <p class="muted">Nothing that is running, no service and no agent names it.</p>
        ) : (
          <>
            <p>It is named by {phrases.length === 1 ? 'one thing, which' : `${phrases.length} things, each of which`} goes without it from then on:</p>
            <ul class="secret-named asked">
              {phrases.map((phrase) => (
                <li key={phrase}>{phrase}</li>
              ))}
            </ul>
          </>
        )}
        <p class="faint">Its value cannot be had back: to have it again it is set again.</p>
        {error && <p class="error-text">{error}</p>}
        <div class="row modal-foot">
          <button type="button" onClick={props.onClose}>
            Cancel
          </button>
          <button class="danger" onClick={() => void take()}>
            Remove it
          </button>
        </div>
      </div>
    </Modal>
  );
}

/** Asks before an environment is taken away, in the daemon's own words of what goes with it. */
function RemoveEnvironment(props: { base: string; name: string; onClose: () => void; onGone: () => void }) {
  const [said, setSaid] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  useEffect(() => {
    void call<{ said: string; running: string[] }>('GET', `${props.base}/remove`)
      .then((read) => (setSaid(read.said), setBusy(read.running.length > 0)))
      .catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }, [props.base]);
  const remove = async () => {
    try {
      await call('DELETE', props.base);
      toast(`${props.name} is taken away.`);
      props.onGone();
      props.onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };
  return (
    <Modal onClose={props.onClose}>
      <div class="stack secret-form">
        <h2>Remove {props.name}?</h2>
        <p class="muted">{said || 'Reading what goes with it…'}</p>
        {error && <p class="error-text">{error}</p>}
        <div class="row modal-foot">
          <button type="button" onClick={props.onClose}>
            Cancel
          </button>
          <button class="danger" disabled={!said || busy} onClick={() => void remove()}>
            Remove it
          </button>
        </div>
      </div>
    </Modal>
  );
}
