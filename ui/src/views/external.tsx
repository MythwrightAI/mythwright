// An agent the factory does not start, where the client draws one: the
// dialog that invites it, what is said of it where it is read about, the
// cards it has in hand for the list in Settings, and what its history is
// in the drawer.
//
// It is invited by whoever may add agents: a name, the project it is
// listed with, what it may ask of the factory by area (the table the
// Tools panel draws, with nothing offered that whoever invites does not
// hold), and how long the invitation works. The daemon answers the
// invitation once, and it is shown once, with the command that hands it
// over: put away, it is not there to be shown again, and another is made
// by inviting the agent again, from the one dialog that adds or invites
// one (`addagent.tsx`). What it asks beside the name, and how the
// invitation is shown, are parts a channel's dialog that adds an agent
// asks too (`addagent.tsx`), where one from outside is added there.
//
// What it is doing is not worked out from turns, since it has none: it
// is the cards given to it that are in hand, read from the board of the
// project it is listed with, once, and again when something is said of a
// card there (`external.ts`). Nothing is read where no external agent is.

import type { ComponentChildren } from 'preact';
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'preact/hooks';
import { call, api } from '../api.ts';
import type { Board, FleetEvent, Slot } from '../api.ts';
import { coalesce } from '../events.ts';
import { EXTERNAL, LASTS, didOf, givenTo, inHand, isExternal, joinCommand, nameOf, offeredScopes, standingOf, within } from '../external.ts';
import type { Standing } from '../external.ts';
import { scopesAfter } from '../holds.ts';
import { calledOf } from '../kept.ts';
import { channels, hasBoard, me, meta, projects, refreshAgents, toast } from '../store.ts';
import { Empty, Field, Modal, useEvents } from '../ui.tsx';
import { clock, when } from '../util.ts';
import { AgentHolds, HoldsTable } from './holds.tsx';

/** An agent as the daemon lists one that it does not start: with how it stands. */
export type External = Slot & { external?: Standing | null };

/** What the daemon answers an invitation with: the agent, and the invitation, once. */
export interface Invited {
  agent: Slot;
  invitation: string;
  expires: number;
}

/** What an invitation is made with, beside the agent's name. */
export interface Asking {
  /** The project it is listed with, or the empty text for none. */
  project: string;
  scopes: string[];
  lasts: string;
}

/** Makes an invitation (`POST /api/agent/invite`). */
export const invite = (name: string, asking: Asking) =>
  call<Invited>('POST', '/agent/invite', { name, project: asking.project || null, scopes: asking.scopes, expires: asking.lasts });

/**
 * What an invitation begins being made with, listed with `project`: what
 * one is invited with where nothing is said, what an agent usually holds
 * and its own inbox, which is where what names it waits for it (a daemon
 * from before says only the first), as far as whoever invites holds it;
 * and a day.
 */
export function firstAsking(project: string): Asking {
  const mine = me.value?.principal.scopes ?? [];
  return { project, scopes: offeredScopes(meta.value?.external_scopes ?? meta.value?.agent_scopes ?? [], mine), lasts: '1d' };
}

/** Puts text on the clipboard, and says so. */
export async function copy(text: string, what: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    toast(`${what} copied.`);
  } catch {
    toast(`${what} could not be copied from here: select it and copy it.`);
  }
}

/** What a typed name will be called, where that is not what was typed. */
export function CalledSaid({ typed }: { typed: string }) {
  const called = nameOf(typed);
  if (!typed || called === typed) return null;
  return <p class="faint invite-called">It will be called {called || 'nothing: a name has a letter or a digit in it'}.</p>;
}

/**
 * What an invitation is made with, beside the name: the project it is
 * listed with, what it may ask of the factory by area, and how long the
 * invitation works. The Agents tab's dialog asks it, and a channel's
 * where an agent from outside is added.
 */
export function InviteFields({ asking, onChange, holds = true }: { asking: Asking; onChange: (asking: Asking) => void; holds?: boolean }) {
  const mine = me.value?.principal.scopes ?? [];
  const areas = meta.value?.scopes ?? [];
  return (
    <>
      <Field label="Project">
        <select value={asking.project} onChange={(event) => onChange({ ...asking, project: event.currentTarget.value })} aria-label="The project it is listed with">
          <option value="">No project</option>
          {projects.value.map((one) => (
            <option key={one.id} value={one.id}>
              {one.name}
            </option>
          ))}
        </select>
      </Field>
      {/* Beside a channel what it may ask is one of what may be set of it there (`membersettings.ts`), drawn under these. */}
      {holds && (
        <Field label="What it may ask of the factory">
          <HoldsTable scopes={asking.scopes} giver={mine} onChange={(area, verbs) => onChange({ ...asking, scopes: scopesAfter(asking.scopes, areas, area, verbs) })} />
        </Field>
      )}
      <Field label="The invitation works for">
        <select value={asking.lasts} onChange={(event) => onChange({ ...asking, lasts: event.currentTarget.value })} aria-label="How long the invitation works">
          {LASTS.map(([said, words]) => (
            <option key={said} value={said}>
              {words}
            </option>
          ))}
        </select>
      </Field>
    </>
  );
}

/**
 * An invitation, shown this once, with the command that hands it over:
 * put away, it is not there to be shown again. `children` is what more
 * is said of where the agent is.
 */
export function Invitation({ made, onClose, children }: { made: Invited; onClose: () => void; children?: ComponentChildren }) {
  const join = joinCommand(meta.value?.cli ?? '', window.location.origin);
  return (
    <Modal onClose={onClose}>
      <div class="stack invite-agent invited-agent" data-agent={made.agent.name}>
        <h2>{made.agent.name} is invited</h2>
        {children}
        <p>Its invitation, shown this once:</p>
        <div class="invite-shown">
          <code class="code-box invite-code">{made.invitation}</code>
          <button type="button" onClick={() => void copy(made.invitation, 'The invitation')}>
            Copy
          </button>
        </div>
        <p>Whoever is to be that agent runs this where its program runs, and gives it the invitation when it asks:</p>
        <div class="invite-shown">
          <code class="code-box invite-join">{join}</code>
          <button type="button" onClick={() => void copy(join, 'The command')}>
            Copy
          </button>
        </div>
        <p class="faint">
          It lets that agent in once, within {within(made.expires, Date.now())}. Pass it on as you would a password. It will not be shown again: once this is closed, another is made by inviting {made.agent.name} again.
        </p>
        <div class="row modal-foot">
          <button type="button" class="primary" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    </Modal>
  );
}

/** What an agent from outside is, in a sentence, wherever one is invited. */
export const OUTSIDE_SAID =
  'For a program somebody runs for themselves, here or on another machine, which this factory does not start. It is listed with the agents, is given cards, and what it does says its name. Nothing here gives it work.';

/**
 * What invites an agent from outside, on the Agents tab: its name, and
 * what it is invited with. The dialog it is in (`AddOrInviteAgent` in
 * `addagent.tsx`) shows the invitation it is handed (`onMade`), once.
 * `focus` is whether the caret begins in its name, which it does only
 * where the dialog began on it.
 */
export function InviteForm({ onMade, onClose, focus = true }: { onMade: (made: Invited) => void; onClose: () => void; focus?: boolean }) {
  const [name, setName] = useState('');
  const [asking, setAsking] = useState<Asking>(() => firstAsking(''));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const first = useRef<HTMLInputElement>(null);
  useLayoutEffect(() => {
    if (focus) first.current?.focus();
  }, []);
  const called = nameOf(name);

  const ask = async (event: Event) => {
    event.preventDefault();
    if (busy || !called) return;
    setBusy(true);
    setError('');
    try {
      const made = await invite(called, asking);
      // It is listed from now, as invited: the list follows what is said of agents, and is read here too.
      void refreshAgents();
      onMade(made);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setBusy(false);
    }
  };

  return (
    <form class="stack invite-agent" onSubmit={(event) => void ask(event)}>
      <Field label="Name">
        <input ref={first} value={name} onInput={(event) => setName(event.currentTarget.value)} placeholder="walls" aria-label="The agent's name" />
      </Field>
      <CalledSaid typed={name} />
      <InviteFields asking={asking} onChange={setAsking} />
      {error && <p class="error-text">{error}</p>}
      <div class="row modal-foot">
        <button type="button" onClick={onClose}>
          Cancel
        </button>
        <button type="submit" class="primary" disabled={busy || !called}>
          Invite
        </button>
      </div>
    </form>
  );
}

/** The boards of the projects that external agents are listed with, by project: read once, and again when something is said of a card there. */
function useBoards(wanted: string[]): Record<string, Board> {
  const [boards, setBoards] = useState<Record<string, Board>>({});
  const key = wanted.join(' ');
  const read = useMemo(() => {
    const one = (project: string) =>
      void api.board(project).then(
        (board) => setBoards((before) => ({ ...before, [project]: board })),
        () => undefined,
      );
    const soon: Record<string, () => void> = {};
    return {
      all: () => wanted.forEach(one),
      soon: (project: string) => (soon[project] ??= coalesce(() => one(project), 150))(),
    };
  }, [key]);
  useEffect(() => read.all(), [read]);
  useEvents(['board.item.*'], (event) => typeof event.data?.project === 'string' && wanted.includes(event.data.project) && read.soon(event.data.project), [read]);
  return boards;
}

/**
 * The cards each external agent has in hand, by the agent's name: those
 * given to it on the board of the project it is listed with. One that is
 * listed with no project, or whose project has no board, has none that
 * is known here.
 */
export function useInHand(slots: Slot[]): Record<string, { id: string; status: string; assignee: string | null }[]> {
  const outside = slots.filter((slot) => isExternal(slot) && slot.project && hasBoard(slot.project));
  const wanted = [...new Set(outside.map((slot) => slot.project!))].sort();
  const boards = useBoards(wanted);
  const held: Record<string, { id: string; status: string; assignee: string | null }[]> = {};
  for (const slot of outside) held[slot.name] = inHand(boards[slot.project!], slot.name);
  return held;
}

/**
 * An external agent, read about beside a channel: how it stands with the
 * factory, what it is listed with, and what it may ask, changed here as
 * any agent's in no row of a fleet is. Nothing of a program is said of
 * it, since none is started here.
 */
export function ExternalSeen({ slot }: { slot: External }) {
  const cli = meta.value?.cli ?? '';
  const boards = useBoards(slot.project && hasBoard(slot.project) ? [slot.project] : []);
  const cards = slot.project ? givenTo(boards[slot.project], slot.name) : [];
  const until = slot.external?.invited_until ?? null;
  return (
    <div class="context-about external-about" data-agent={slot.name}>
      <div class="row">
        <strong class="context-card-title context-name">{slot.name}</strong>
        <span class="grow" />
        <span class="faint">agent · {EXTERNAL}</span>
      </div>
      <div class="context-doing faint">{standingOf(slot, Date.now())}</div>
      <dl class="context-facts">
        <dt>Runs</dt>
        <dd>
          outside<span class="muted">: this factory starts no program for it and gives it no work</span>
        </dd>
        {!slot.external?.joined && slot.active && (
          <>
            <dt>Let in by</dt>
            <dd class="muted">{until !== null && until > Date.now() ? `the invitation made for it, within ${within(until, Date.now())}` : `an invitation: ${cli} agent invite ${slot.name}`}</dd>
          </>
        )}
        {slot.project && (
          <>
            <dt>Listed with</dt>
            <dd>{projects.value.find((one) => one.id === slot.project)?.name ?? slot.project}</dd>
          </>
        )}
        <dt>Added</dt>
        <dd class="muted">{when(slot.created)}</dd>
      </dl>
      <AgentHolds slot={slot} />
      <h3>Cards given to it</h3>
      {cards.length === 0 ? (
        <p class="faint">None that is not done{slot.project ? '' : ': it is listed with no project'}.</p>
      ) : (
        <ul class="external-cards">
          {cards.map((card) => (
            <li key={card.id}>
              <span class="external-card">{card.id}</span> <span class="muted truncate">{card.title}</span> <span class="faint">{card.status}</span>
            </li>
          ))}
        </ul>
      )}
      <p class="faint holds-where">A message that names it starts nothing: it reads what is said where it is, and takes its cards up itself.</p>
    </div>
  );
}

/** What is read of the journal for what an external agent did. */
const DID = ['board.item.moved', 'board.item.updated', 'board.item.created', 'record.commented', 'channel.message'];
const READ = 1000;

/**
 * An external agent's history, in the drawer: it has no turns, which is
 * said, and under that what it did of late as the journal says it, from
 * one reading of what whoever looks may be told. Nothing is kept for it.
 */
export function ExternalHistory({ slot }: { slot: string }) {
  const [events, setEvents] = useState<FleetEvent[] | null>(null);
  useEffect(() => {
    let left = false;
    api.recentEvents({ topics: DID.join(','), limit: READ }).then(
      (read) => !left && setEvents(read.events),
      () => !left && setEvents([]),
    );
    return () => {
      left = true;
    };
  }, [slot]);
  const channelName = (id: string) => {
    const found = channels.value.find((one) => one.id === id);
    return found ? calledOf(found) : null;
  };
  const did = events ? didOf(events, slot, channelName) : null;
  return (
    <div class="turns external-history" data-agent={slot}>
      <div class="turns-head">
        <p class="faint turns-of">
          {slot} is an external agent: this factory starts no program for it and asks it nothing, so it has no turns to list. What it did is said where it did it, on the cards it moved and in the channels it spoke in.
        </p>
      </div>
      {did === null ? (
        <div class="empty">Reading</div>
      ) : did.length === 0 ? (
        <Empty title="Nothing it did is among what was said lately" />
      ) : (
        <section class="turns-day" aria-label="What it did of late">
          <h3>What it did of late</h3>
          <ul class="external-did">
            {did.map((one) => (
              <li key={one.key}>
                <span class="faint" title={when(one.ts)}>
                  {clock(one.ts)}
                </span>
                <span>{one.words}</span>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
