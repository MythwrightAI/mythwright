// A project's feature flags beside its environments: on the
// project's Environments page a table with a row for each flag and a cell
// for each environment, saying what the flag says there, pressed to change
// it; and in an environment's drawer what each flag says there. What each
// says is that environment's variable `FLAG_<NAME>`, read from what the
// daemon answers (`/api/projects/<p>/flags`) and nothing else; what is
// worked out is `flags.ts`.
//
// A flag's row (its name, what it is for, whose it is, its day) is a row of
// the project's `flags` table, added and changed there as any row is: Open
// the table opens it. Changing what a flag says takes writing the project,
// and a change the daemon says waits for a go-ahead is said so, in its own
// sentence, until one of those the environment's rule names lets it.

import { useEffect, useRef, useState } from 'preact/hooks';
import { call } from '../api.ts';
import { type Came, type Flag, type Listed, cameSaid, mayChangeFlags, pastItsDay, pressed, saidIn, typedValue } from '../flags.ts';
import { me, openTab, toast } from '../store.ts';
import { useEvents } from '../ui.tsx';
import { classes } from '../util.ts';

/** A project's flags as the daemon answers them: nothing yet while they are read, and `none` where it keeps none. */
function useFlags(project: string): [Listed | 'none' | undefined, () => void] {
  const [listed, setListed] = useState<Listed | 'none' | undefined>(undefined);
  const asked = useRef(0);
  const read = () => {
    const mine = ++asked.current;
    call<Listed>('GET', `/projects/${encodeURIComponent(project)}/flags`).then(
      (answered) => mine === asked.current && setListed(answered),
      () => mine === asked.current && setListed('none'),
    );
  };
  useEffect(read, [project]);
  const ours = (event: { data?: Record<string, unknown> }) => event.data?.project === project;
  useEvents(['project.flag.*', 'project.environment.*', 'collection.ensured'], (event) => ours(event) && read(), [project]);
  useEvents('record.*', (event) => ours(event) && event.data?.collection === 'flags' && read(), [project]);
  return [listed, read];
}

const today = (): string => new Date().toISOString().slice(0, 10);

/** Asks the daemon for `flag` to say `value` in `environment`, and says what came of it. */
async function setFlag(project: string, flag: Flag, environment: string, value: string | number): Promise<Came | null> {
  try {
    const path = `/projects/${encodeURIComponent(project)}/flags/${encodeURIComponent(flag.flag)}/${encodeURIComponent(environment)}`;
    const came = await call<Came>('PUT', path, { value });
    toast(cameSaid(came, flag.name));
    return came;
  } catch (err) {
    toast(err instanceof Error ? err.message : String(err), true);
    return null;
  }
}

/** The project's flags as a table beside its environments, on its Environments page. */
export function FlagsPart({ project }: { project: string }) {
  const [listed, read] = useFlags(project);
  const may = mayChangeFlags(me.value?.principal.scopes ?? []);
  // A change the daemon said waits, by flag and environment, until something is said of that flag there.
  const [waiting, setWaiting] = useState<Record<string, string>>({});
  // Written, or let go on after it was overtaken and so written not at all.
  useEvents(['project.flag.changed', 'project.flag.unchanged'], (event) => {
    const at = `${event.data?.flag}/${event.data?.environment}`;
    setWaiting(({ [at]: _, ...rest }) => rest);
  }, [project]);
  if (listed === undefined || listed === 'none') return null;
  const asked = async (flag: Flag, environment: string, value: string | number) => {
    const came = await setFlag(project, flag, environment, value);
    if (came?.came === 'asks') setWaiting((was) => ({ ...was, [`${flag.flag}/${environment}`]: came.shown ?? String(value) }));
    read();
  };
  return (
    <section class="panel flags-panel" aria-label="Flags">
      <div class="row flags-head">
        <h2 class="grow">Flags</h2>
        <button onClick={() => openTab({ kind: 'table', project, name: 'flags' })}>Open the table</button>
      </div>
      <p class="faint flags-said">
        What each says in an environment is its variable there, <code>FLAG_&lt;NAME&gt;</code>, which an app reads as any setting. Turning one on
        where the environment's rule says who approves waits for one of them; turning one off never waits.
      </p>
      {listed.flags.length === 0 ? (
        <p class="muted flags-none">No flags yet: a row of the table is one.</p>
      ) : (
        <div class="flags-scroll">
          <table class="flags-table">
            <thead>
              <tr>
                <th>Flag</th>
                {listed.environments.map((environment) => (
                  <th key={environment}>
                    <code>{environment}</code>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {listed.flags.map((flag) => (
                <tr key={flag.id} data-flag={flag.flag}>
                  <th scope="row">
                    <span class="flags-name">{flag.name}</span>
                    <span class="faint flags-kind">{flag.kind}</span>
                    {pastItsDay(flag, today()) && <span class="flags-due" title={`It was to be taken out by ${flag.until}`}>past its day</span>}
                  </th>
                  {listed.environments.map((environment) => (
                    <td key={environment} data-environment={environment}>
                      <Cell flag={flag} environment={environment} may={may} waits={waiting[`${flag.flag}/${environment}`]} onAsk={(value) => void asked(flag, environment, value)} />
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}

/** What a flag says in one environment: pressed, a switch turned the other way, and anything else asked what to say. */
function Cell(props: { flag: Flag; environment: string; may: boolean; waits?: string; onAsk: (value: string | number) => void }) {
  const { flag, environment, may } = props;
  const says = flag.says[environment];
  const [typing, setTyping] = useState<string | null>(null);
  const shown = says?.shown ?? 'off';
  const lit = says ? says.value !== null && says.value !== 'off' && says.value !== 0 && says.value !== '' : false;
  if (typing !== null) {
    const read = typedValue(flag.kind, typing);
    const send = (event: Event) => {
      event.preventDefault();
      if ('value' in read) {
        props.onAsk(read.value);
        setTyping(null);
      }
    };
    return (
      <form class="flags-typing" onSubmit={send}>
        <input
          aria-label={`What ${flag.name} says in ${environment}`}
          value={typing}
          autoFocus
          spellcheck={false}
          onInput={(event) => setTyping(event.currentTarget.value)}
          onKeyDown={(event) => event.key === 'Escape' && (event.stopPropagation(), setTyping(null))}
        />
        <button type="submit" disabled={'wrong' in read}>
          Set
        </button>
        {'wrong' in read && typing !== '' && <span class="error-text flags-wrong">{read.wrong}</span>}
      </form>
    );
  }
  const press = () => {
    const next = pressed(flag, environment);
    if (next !== null) props.onAsk(next);
    else setTyping(says?.value === null || says?.value === undefined ? '' : String(says.value));
  };
  return (
    <span class="flags-cell">
      <button
        class={classes('flags-value', lit && 'on')}
        disabled={!may}
        aria-pressed={flag.kind === 'switch' ? lit : undefined}
        aria-label={`${flag.name} in ${environment}: ${shown}`}
        title={may ? `Change what ${flag.name} says in ${environment}` : 'Changing a flag takes writing the project'}
        onClick={press}
      >
        {shown}
      </button>
      {props.waits && <span class="faint flags-waits">{props.waits} waits for a go-ahead</span>}
    </span>
  );
}

/** What each of the project's flags says in one environment, for its drawer. */
export function FlagsIn({ project, environment }: { project: string; environment: string }) {
  const [listed] = useFlags(project);
  if (listed === undefined || listed === 'none' || listed.flags.length === 0) return null;
  return (
    <section class="envs-part flags-in" aria-label="Flags">
      <h3>Flags</h3>
      <div class="envs-vars">
        {saidIn(listed, environment).map((one) => (
          <div key={one.variable} class="envs-var" data-flag-var={one.variable}>
            <code class="envs-var-name">{one.variable}</code>
            <span class={classes('flags-value', 'still', one.on && 'on')}>{one.shown}</span>
            <span class="faint">{one.name}</span>
          </div>
        ))}
      </div>
      <p class="faint">Changed as a flag, beside the environments on the project's Environments page.</p>
    </section>
  );
}
