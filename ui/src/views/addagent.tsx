// The dialog that adds an agent, the one dialog for it wherever it is
// opened: beside a channel (`participants.tsx`), where the agent is
// brought into that channel and may be one there is already; and on the
// Agents tab (`agentlist.tsx`), where it is a new one and is put in no
// channel. Both ask first, by the same cards, whether it is one this
// factory runs or one from outside (`WayChoice`), each offered on the
// page only to whoever the daemon would let (`waysOffered`).
//
// Both ask the same: its name, what it runs on and its model, what it
// may do unasked and what it is told. Beside a channel it is asked first,
// where whoever adds may take both ways (`waysOffered`, as on the page),
// whether the agent is one this factory runs or one from outside, a
// program somebody runs for themselves: that one is invited with what the
// Agents tab's dialog asks (`external.tsx`, its parts and not a copy),
// brought into the channel at once, where it is listed as invited until
// it joins, and its invitation is shown once. A name an agent has is
// that agent, of either kind, brought in with nothing else asked: one
// this factory runs as a member is, by somebody who may not apply its
// text (`broughtBy`). From the page it is also asked
// which project it works in, of the workspace the side panel is of, and
// where that project has environments which one it works under, since
// the daemon refuses one there that says none (`Agents::none_said`).
// What the page has decided is `agentadded.ts`. The new agent is listed
// by what the daemon says of it (`agent.slot.created`), which the store
// follows, so the page reads nothing again for it; what the daemon
// refuses is said in its sentence, with the dialog staying.

import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { api, call } from '../api.ts';
import type { Model, Provider, Slot } from '../api.ts';
import { addClosed, addedBody, addedSaid, asksOf, beginsOn, broughtBy, comingOf, doneBy, invitedInto, invitesOn, takenBy, thereSaid, waysOffered } from '../agentadded.ts';
import type { Way } from '../agentadded.ts';
import { held } from '../chosen.ts';
import { namesOf } from '../envsheld.ts';
import { nameOf } from '../external.ts';
import { TYPED, choices, offered } from '../models.ts';
import { shown } from '../narrowed.ts';
import { workspaces } from '../spaces.ts';
import { hashed } from '../kept.ts';
import { sameScopes, sectionKey, settingsOf } from '../membersettings.ts';
import type { Of } from '../membersettings.ts';
import { adapters, agents, channels, me, meta, permissionModes, projects, providers, refreshAgents, showSide, toast } from '../store.ts';
import { Modal } from '../ui.tsx';
import { listed } from '../workspaces.ts';
import { AgentPick } from './agentpick.tsx';
import { CalledSaid, InviteFields, InviteForm, Invitation, OUTSIDE_SAID, firstAsking, invite } from './external.tsx';
import { MemberSettings, useSet } from './membersettings.tsx';
import type { Asking, Invited } from './external.tsx';

/** The two an agent added beside a channel may be, chosen at the dialog's head: each in a few words, and in a sentence under the choice once it is chosen. */
const WAYS: [Way, string, string, string][] = [
  [
    'runs',
    'Run by this factory',
    'On a model of your choosing, started here.',
    'An agent works from a slot: a place with its own credential, tools and permissions. Name one that exists to add it here, or a new one to create it.',
  ],
  [
    'outside',
    'From outside',
    'Somebody runs it themselves, and it joins with an invitation.',
    'An agent from outside is a program somebody runs for themselves, here or on another machine, under a name of its own. This factory starts nothing for it and gives it no work: it reads what is said where it is, and takes its cards up itself. Name one that is here already to add it, or a new one to invite it.',
  ],
];

/** What is said under the choice on the Agents tab, where the agent is a new one and is put in no channel. */
const PAGE_SAID: Record<Way, string> = {
  runs: 'An agent works from a slot: a place with its own credential, tools and permissions. This makes a new one, in no channel: it is brought into one from the channel’s members.',
  outside: OUTSIDE_SAID,
};

/** The choice at a dialog's head of which an agent is, as cards: one each of `ways`, by a few words. */
function WayChoice({ ways, way, onChoose }: { ways: readonly Way[]; way: Way; onChoose: (way: Way) => void }) {
  return (
    <div class="add-agent-ways" role="radiogroup" aria-label="Which agent it is">
      {WAYS.filter(([one]) => ways.includes(one)).map(([one, title, means]) => (
        <label key={one} class={`add-agent-way${way === one ? ' chosen' : ''}`} data-way={one}>
          <input type="radio" name="add-agent-way" value={one} checked={way === one} onChange={() => onChoose(one)} />
          <span class="add-agent-way-words">
            <strong>{title}</strong>
            <span class="muted">{means}</span>
          </span>
        </label>
      ))}
    </div>
  );
}

/**
 * The Agents tab's one dialog for an agent: added, where this factory
 * runs it (`AddAgent` with no channel), or invited, where it is from
 * outside (`InviteForm`), with its invitation shown once. Each way is
 * offered only to whoever may take it, and with one there is no choice.
 */
export function AddOrInviteAgent({ onClose }: { onClose: () => void }) {
  const ways = waysOffered(me.value?.principal.scopes ?? []);
  const [way, setWay] = useState<Way>(ways[0] ?? 'runs');
  // The caret begins in the name, and stays on the choice once somebody has made one.
  const [chose, setChose] = useState(false);
  const [made, setMade] = useState<Invited | null>(null);
  if (made) return <Invitation made={made} onClose={onClose} />;
  return (
    <Modal onClose={onClose}>
      <div class="stack add-or-invite-agent" data-way={way}>
        <h2>Add or invite an agent</h2>
        {ways.length > 1 && (
          <WayChoice
            ways={ways}
            way={way}
            onChoose={(one) => {
              setChose(true);
              setWay(one);
            }}
          />
        )}
        <p class="muted add-agent-said">{PAGE_SAID[way]}</p>
        {way === 'runs' ? <AddAgent key="runs" onClose={onClose} focus={!chose} /> : <InviteForm key="outside" onMade={setMade} onClose={onClose} focus={!chose} />}
      </div>
    </Modal>
  );
}

/** One invited beside a channel, with what became of bringing it in: shown once, by whoever opened the dialog. */
export interface InvitedHere {
  invited: Invited;
  refused: string | null;
}

/**
 * Adds an agent: to `channel`, giving it a slot if it has none; or, with
 * no channel, a new one that works in a project of the workspace showing
 * or in none, and is in no channel. Beside a channel it is the agent's
 * part of the dialog that adds to a channel (`addmember.tsx`), which has
 * the dialog, its heading and the choice of who it is: this is what is
 * under those, and an invitation it makes is handed to it to show
 * (`onInvited`). `focus` is whether the caret begins in its name, which it
 * does only where the dialog began on it.
 */
export function AddAgent({ channel, onClose, focus = true, onInvited }: { channel?: string; onClose: () => void; focus?: boolean; onInvited?: (made: InvitedHere) => void }) {
  const page = channel === undefined;
  const known = agents.value.filter((slot) => slot.active);
  // Beside a channel, each way is offered only to whoever the daemon would let take it (`waysOffered`), as on the page.
  const mine = me.value?.principal.scopes ?? [];
  const ways = page ? (['runs'] as Way[]) : waysOffered(mine);
  const applies = ways.includes('runs');
  const [way, setWay] = useState<Way>(ways[0] ?? 'runs');
  // The name a first agent is offered is one the factory runs: it is not offered to one from outside.
  const [name, setName] = useState(known.length === 0 && way === 'runs' ? 'builder' : '');
  const [program, setProgram] = useState<Program>({ adapter: adapters.value.fallback, model: '', mode: '', instructions: '' });
  const { adapter, model, instructions } = program;
  const mode = modeOf(adapter, program.mode);
  const input = useRef<HTMLInputElement>(null);
  useLayoutEffect(() => {
    if (focus) input.current?.focus();
  }, []);
  // Beside a channel: which it is, what that comes to with the name typed, and what an invitation is made with.
  const coming = comingOf(way, name, known);
  const existing = !page && coming.kind === 'there' ? known.find((slot) => slot.name === coming.slot.name) : undefined;
  const asks = page ? { program: true, invitation: false } : asksOf(coming);
  const here = channels.value.find((one) => one.id === channel);
  const [asking, setAsking] = useState<Asking>(() => firstAsking(invitesOn(here?.project, projects.value)));
  // What may be set of it beside a channel: what it may ask of the factory, by area, sent with the add or the invitation.
  const outsideNow = coming.kind === 'invited' || (coming.kind === 'nothing' && coming.way === 'outside');
  const usualOf = outsideNow ? (meta.value?.external_scopes ?? meta.value?.agent_scopes ?? []) : (meta.value?.agent_scopes ?? []);
  const of: Of = existing ? { kind: 'agent', coming: 'there', slot: existing } : { kind: 'agent', coming: outsideNow ? 'invited' : 'new', usual: usualOf };
  const sections = page ? [] : settingsOf(of, me.value?.principal.scopes ?? []);
  const holding = sections.find((section) => section.section === 'holds');
  const [scopes, setScopes] = useSet(sectionKey(of), holding?.section === 'holds' ? holding.scopes : []);
  const sends = holding?.section === 'holds' && holding.sent;
  // The name a first agent is offered is one the factory runs: it is not offered to one from outside.
  const choose = (one: Way) => {
    if (one === 'outside' && known.length === 0 && name === 'builder') setName('');
    setWay(one);
    setError('');
  };

  // From the page: the projects of the workspace the panel is of, beginning on the one the page is narrowed to.
  const offeredProjects = listed(projects.value, workspaces.value, held.value);
  const [project, setProject] = useState(() => beginsOn(shown.value, offeredProjects));
  const environments = page ? namesOf(project) : [];
  const [under, setUnder] = useState('');
  const taken = page ? takenBy(name, agents.value) : null;
  const closed = page ? addClosed(taken?.name ?? null, environments, under) : null;
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const called = (id: string) => projects.value.find((one) => one.id === id)?.name ?? id;

  const submit = async (event: Event) => {
    event.preventDefault();
    if (!page) {
      if (busy || coming.kind === 'nothing') return;
      setBusy(true);
      setError('');
      const into = here ? hashed(here) : `#${channel}`;
      try {
        if (coming.kind === 'invited') {
          const invited = await invite(nameOf(name), sends ? { ...asking, scopes } : asking);
          // Listed from now, as invited, among the channel's members too once it is brought in.
          void refreshAgents();
          // Invited, it is shown whatever becomes of bringing it in: it is not shown again.
          let refused: string | null = null;
          try {
            await api.addPerson(channel, `agent:${invited.agent.name}`);
          } catch (err) {
            refused = err instanceof Error ? err.message : String(err);
          }
          onInvited?.({ invited, refused });
          return;
        }
        // What was changed of one that is there is given it first, through its own route, as the Tools panel gives it.
        if (existing && sends && !sameScopes(scopes, existing.scopes)) await api.giveAgent(existing.name, scopes);
        if (existing && broughtBy(existing, applies) === 'member') await api.addPerson(channel, `agent:${existing.name}`);
        else
          await api.bringIn(
            channel,
            existing
              ? { name: existing.name }
              : { name: name.trim(), adapter, model: model || undefined, instructions: instructions || undefined, permission_mode: mode || undefined, scopes: sends ? scopes : undefined },
          );
        toast(`${existing?.name ?? nameOf(name)} is now in ${into}`);
        onClose();
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
        setBusy(false);
      }
      return;
    }
    if (busy || closed) return;
    setBusy(true);
    setError('');
    try {
      const body = addedBody({ name, adapter, model, instructions, mode, project, environments, under });
      // A daemon from before says no environment of an agent.
      const slot = await call<Slot & { environment?: string }>('POST', '/agents', body);
      toast(addedSaid(slot.name, slot.project ? called(slot.project) : null, slot.environment || null));
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setBusy(false);
    }
  };

  const form = (
    <form class={`stack add-agent${asks.invitation ? ' invite-agent' : ''}`} onSubmit={(event) => void submit(event)}>
      {!page && ways.length > 1 && <WayChoice ways={ways} way={way} onChoose={choose} />}
      {!page && <p class="muted add-agent-said">{WAYS.find(([one]) => one === way)![3]}</p>}
      {page ? (
        <label>
          Name
          <input ref={input} required value={name} onInput={(event) => setName(event.currentTarget.value)} />
        </label>
      ) : (
        <AgentPick value={name} onInput={setName} here={new Set(here?.members.map((member) => member.actor) ?? [])} outside={way === 'outside'} field={input} />
      )}
      {!page && coming.kind === 'invited' && <CalledSaid typed={name} />}
      {page && (
        <WhereFields
          project={project}
          under={under}
          offered={offeredProjects}
          environments={environments}
          onProject={(one) => {
            setProject(one);
            setUnder('');
          }}
          onUnder={setUnder}
        />
      )}
      {existing && <p class="muted add-agent-there">{thereSaid(existing, Date.now())}</p>}
      {asks.invitation && <InviteFields asking={asking} onChange={setAsking} holds={false} />}
      {asks.program && <ProgramFields value={program} onChange={setProgram} onClose={onClose} />}
      {!page && <MemberSettings sections={sections} scopes={scopes} onScopes={setScopes} />}
      {error && <p class="error-text">{error}</p>}
      {closed && !error && <p class="faint add-agent-closed">{closed}</p>}
      <div class="row modal-foot">
        <button type="button" onClick={onClose}>
          Cancel
        </button>
        <button class="primary" type="submit" disabled={page ? busy || !!closed || !name.trim() : busy || (asks.invitation && coming.kind !== 'invited')}>
          {page ? 'Add agent' : doneBy(coming)}
        </button>
      </div>
    </form>
  );
  // On the page it is drawn inside the dialog that adds or invites one, which has its heading and what is said of it.
  return form;
}

/** What runs an agent and what it is told: its program, its model, how much it may do unasked and its standing instructions. `mode` is empty for the usual one. */
export interface Program {
  adapter: string;
  model: string;
  mode: string;
  instructions: string;
}

/** How much an agent on `adapter` may do unasked, of what its program can be held to: `asked` where that is one, else the usual, changing files where it works. */
export function modeOf(adapter: string, asked: string): string {
  const modes = modesOf(adapter);
  if (modes.some((one) => one.name === asked)) return asked;
  return (modes.find((mode) => mode.name === 'edit') ?? modes[0])?.name ?? '';
}

const modesOf = (adapter: string) => {
  const chosen = providers.value.find((provider) => provider.name === adapter);
  return permissionModes.value.filter((mode) => chosen?.permissions?.includes(mode.name) ?? true);
};

/**
 * The fields of what runs an agent, as the dialog that adds one asks them
 * and an agent's setup in the drawer changes them (`agentedit.tsx`).
 * `closed` says why none of it may be changed, and `fullClosed` why it may
 * not be let do anything unasked.
 */
export function ProgramFields({ value, onChange, onClose, closed, fullClosed }: { value: Program; onChange: (next: Program) => void; onClose?: () => void; closed?: string | null; fullClosed?: string | null }) {
  const { adapter, model, instructions } = value;
  const chosen = providers.value.find((provider) => provider.name === adapter);
  const modes = modesOf(adapter);
  const mode = modeOf(adapter, value.mode);
  const shut = !!closed;
  // A model belongs to what offers it; another provider starts from its own default.
  const pick = (name: string) => onChange({ ...value, adapter: name, model: '', mode: '' });
  return (
    <>
      <label>
        Runs on
        <select value={adapter} disabled={shut} title={closed ?? undefined} onChange={(event) => pick(event.currentTarget.value)}>
          {providers.value.map((option) => (
            <option key={option.name} value={option.name}>
              {option.title}
              {option.kind === 'builtin' ? ' (offline, no model needed)' : ''}
              {option.available ? '' : ' (its program is not installed here)'}
            </option>
          ))}
        </select>
      </label>
      {chosen?.kind === 'builtin' && providers.value.length === 1 && (
        <p class="muted">
          Real agents come as extension packages, and none is installed yet.{' '}
          <button
            type="button"
            class="ghost link"
            onClick={() => {
              onClose?.();
              showSide('extensions', false);
            }}
          >
            Install one from Extensions
          </button>
        </p>
      )}
      {chosen && chosen.kind !== 'builtin' && <ModelPicker provider={chosen} value={model} onChange={(next) => onChange({ ...value, model: next })} closed={closed} />}
      {chosen && chosen.kind !== 'builtin' && modes.length > 0 && (
        <label>
          May do, unasked
          <select value={mode} disabled={shut} title={closed ?? undefined} onChange={(event) => onChange({ ...value, mode: event.currentTarget.value })}>
            {modes.map((one) => (
              <option key={one.name} value={one.name} disabled={one.name === 'full' && !!fullClosed && mode !== 'full'} title={one.name === 'full' ? (fullClosed ?? undefined) : undefined}>
                {one.name}: {one.means}
              </option>
            ))}
          </select>
          {modes.length < permissionModes.value.length && (
            <span class="muted">{chosen.title} cannot be held to less: run for one request, its program asks nobody's leave.</span>
          )}
        </label>
      )}
      <label>
        Standing instructions (optional)
        <textarea value={instructions} disabled={shut} title={closed ?? undefined} onInput={(event) => onChange({ ...value, instructions: event.currentTarget.value })} />
      </label>
    </>
  );
}

/** Which project an agent works in, of `offered`, and where that project has environments which one it works under. */
export function WhereFields(props: {
  project: string;
  under: string;
  offered: readonly { id: string; name: string }[];
  environments: readonly string[];
  onProject: (project: string) => void;
  onUnder: (under: string) => void;
  closed?: string | null;
}) {
  const shut = !!props.closed;
  return (
    <>
      <label>
        Project
        <select aria-label="The project it works in" value={props.project} disabled={shut} title={props.closed ?? undefined} onChange={(event) => props.onProject(event.currentTarget.value)}>
          <option value="">No project</option>
          {props.offered.map((one) => (
            <option key={one.id} value={one.id}>
              {one.name}
            </option>
          ))}
        </select>
      </label>
      {props.environments.length > 0 && (
        <label>
          Environment
          {/* Nothing is chosen until somebody chooses: an agent of this project works under one of them. */}
          <select aria-label="The environment it works under" value={props.under} disabled={shut} title={props.closed ?? undefined} onChange={(event) => props.onUnder(event.currentTarget.value)}>
            <option value="" disabled>
              Choose an environment
            </option>
            {props.environments.map((one) => (
              <option key={one} value={one}>
                {one}
              </option>
            ))}
          </select>
          <span class="muted">What its program is handed of the environment: what is marked there for agents.</span>
        </label>
      )}
    </>
  );
}

/** What is said under an invitation made beside a channel, of where the agent is. */
export function InvitedInto({ made, channel }: { made: InvitedHere; channel: string }) {
  return <p class={made.refused === null ? 'invited-into' : 'invited-into error-text'}>{invitedInto(made.invited.agent.name, channel, made.refused)}</p>;
}

/**
 * Choosing a model the way the provider's own tool offers them: its list,
 * with its usual one first. Typing a name stays possible, for a model the
 * list does not have yet, and is all there is for a provider that lists none.
 */
function ModelPicker({ provider, value, onChange, closed }: { provider: Provider; value: string; onChange: (model: string) => void; closed?: string | null }) {
  const shut = !!closed;
  const [models, setModels] = useState<Model[] | null>(null);
  const [trouble, setTrouble] = useState('');
  const [typing, setTyping] = useState(false);
  const lists = provider.models !== 'none' && provider.available;

  const ask = (fresh: boolean) => {
    setModels(null);
    setTrouble('');
    api
      .models(provider.name, fresh)
      .then(setModels)
      .catch((err) => setTrouble(err instanceof Error ? err.message : String(err)));
  };
  useEffect(() => {
    setTyping(false);
    if (lists) ask(false);
  }, [provider.name, lists]);

  if (!lists || trouble || typing) {
    return (
      <label>
        Model (optional)
        <input value={value} disabled={shut} title={closed ?? undefined} placeholder={`${provider.title}'s own choice`} onInput={(event) => onChange(event.currentTarget.value)} />
        {trouble && <span class="muted">Its models could not be listed ({trouble}), so name one or leave it empty.</span>}
        {typing && (
          <button
            type="button"
            class="ghost link"
            onClick={() => {
              setTyping(false);
              onChange('');
            }}
          >
            Choose from its list instead
          </button>
        )}
      </label>
    );
  }
  if (models === null) {
    return (
      <label>
        Model
        <select disabled>
          <option>Asking {provider.title} which models it offers</option>
        </select>
      </label>
    );
  }
  const options = choices(models, value);
  const note = options.find((option) => option.value === value)?.note;
  return (
    <label>
      Model
      <select
        disabled={shut}
        title={closed ?? undefined}
        value={offered(models, value) ? value : TYPED}
        onChange={(event) => {
          if (event.currentTarget.value === TYPED) setTyping(true);
          else onChange(event.currentTarget.value);
        }}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
        <option value={TYPED}>Another model</option>
      </select>
      {note && <span class="muted">{note}</span>}
    </label>
  );
}
