// The agents of the factory, on the Agents tab (`agents.tsx`), held to
// the projects of what is chosen there (`only`).
//
// By project: a band for each project that has any, in the order the
// projects are listed, and a last one for those that belong to none
// (`agentbands.ts`). A line is an agent: its name, what it runs on, what
// it is doing now, its permissions (what it may ask of the factory) in a
// few words, and at the end a menu of what can be done with it, its
// setup first (`agentedit.tsx`), then its history, made by the one
// function the side panel's Members asks too (`agentacts.tsx`). Over them, for whoever may change agents, one button
// that adds one this factory runs or invites one from outside, by one
// dialog that asks first which, as a channel's does (`AddOrInviteAgent`
// in `addagent.tsx`).
//
// What each is doing is worked out from what the daemon says of agents'
// turns (`doing.ts`): the latest of that read once when the list is
// drawn, with whatever is said while the reading is on its way, and each
// thing said after. Nothing is asked again.
//
// An agent's permissions are shown here and changed elsewhere: beside its
// channel, in the Tools panel, or in its fleet's row. Pressed, the few
// words open the whole by area in one panel over the page.

import { createPortal } from 'preact/compat';
import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { banded } from '../agentbands.ts';
import { api } from '../api.ts';
import type { FleetEvent, Slot } from '../api.ts';
import { Arriving } from '../arrived.ts';
import { NOTHING, doingNow, heard, turnsNow } from '../doing.ts';
import type { TurnsNow } from '../doing.ts';
import { placed } from '../icons.ts';
import { brieflyHeld } from '../scopes.ts';
import { agents, channels, me, meta, projects } from '../store.ts';
import { Empty, useEvents } from '../ui.tsx';
import { classes, when } from '../util.ts';
import { More } from './grouped.tsx';
import { HoldsTable } from './holds.tsx';
import { agentActs } from './agentacts.tsx';
import { calledOf } from '../kept.ts';
import { runsOnListed } from './who.tsx';
import { EXTERNAL, externalDoing, isExternal } from '../external.ts';
import { useInHand } from './external.tsx';
import { AddOrInviteAgent } from './addagent.tsx';
import { waysOffered } from '../agentadded.ts';

const TURNS = ['agent.turn.started', 'agent.turn.finished'];
/** How many of the latest things said of turns are read to begin with: the most the daemon hands at once. */
const READ = 1000;
/** What is said of turns while the reading of them is on its way. */
const arriving = new Arriving<FleetEvent>();

/** Which turn each agent has in hand and when each last ended one: read once, and followed. */
function useTurnsNow(): TurnsNow {
  const [now, setNow] = useState<TurnsNow>(NOTHING);
  useEffect(() => {
    let left = false;
    void arriving
      .during('turns', () => api.recentEvents({ topics: TURNS.join(','), limit: READ }))
      .then(([read, arrived]) => !left && setNow(turnsNow(read.events, arrived)))
      .catch(() => undefined);
    return () => {
      left = true;
    };
  }, []);
  useEvents(TURNS, (event) => {
    arriving.arrive('turns', event);
    setNow((before) => heard(before, event));
  });
  return now;
}

/** `only` is the projects shown, nothing where everything is; `none` is what an empty list says of what is shown, and `called` whose they are. */
export function AgentList({ only = null, none = null, called = 'Agents' }: { only?: readonly string[] | null; none?: string | null; called?: string }) {
  const turns = useTurnsNow();
  // One the factory does not start has no turns: what it has in hand is the cards given to it.
  const inHand = useInHand(agents.value);
  // Adding one this factory runs, and inviting one from outside, each offered to whoever the daemon would let.
  const ways = waysOffered(me.value!.principal.scopes);
  const [adding, setAdding] = useState(false);
  const bands = banded(agents.value, projects.value, only);
  const channelName = (id: string) => {
    const found = channels.value.find((one) => one.id === id);
    return found ? calledOf(found) : null;
  };
  return (
    <div class="panel agents-listed">
      <div class="panel-head">
        <h2 class="grow">{called}</h2>
        <span class="muted">{ways.length > 0 ? 'Add one here or from a channel’s members. Each works from its own slot.' : 'Each works from its own slot.'}</span>
        {ways.length > 0 && (
          <button type="button" class="add-or-invite-agent" onClick={() => setAdding(true)}>
            Add or invite an agent
          </button>
        )}
      </div>
      {/* What each part of a line is, said once over all of them where a line is laid across; in a narrow panel a line says it of itself. */}
      {bands.length > 0 && (
        <div class="agent-heads" aria-hidden="true">
          <span>Agent</span>
          <span>Runs on</span>
          <span>Now</span>
          <span>Permissions</span>
          <span />
        </div>
      )}
      {bands.length === 0 ? (
        <Empty title={none ? 'No agents here' : 'No agents yet'}>{none && <span>{none}</span>}</Empty>
      ) : (
        bands.map((band) => (
          <section key={band.project ?? ''} class="agents-of" data-of={band.project ?? ''} aria-label={`Agents of ${band.name}`}>
            <h3>
              <span class="truncate">{band.name}</span>
              <span class="agents-count">{band.agents.length === 1 ? '1 agent' : `${band.agents.length} agents`}</span>
            </h3>
            <ul>
              {band.agents.map((slot) => {
                const outside = isExternal(slot);
                const doing = outside ? externalDoing(slot, inHand[slot.name] ?? [], Date.now()) : doingNow(slot.active, turns.going[slot.name], turns.ended[slot.name], Date.now(), channelName);
                const runs = outside ? EXTERNAL : runsOnListed(slot.name);
                const items = agentActs(slot);
                const ended = turns.ended[slot.name];
                return (
                  <li key={slot.name} class="agent-line" data-agent={slot.name}>
                    <strong class="agent-name truncate" title={slot.name}>
                      {slot.name}
                    </strong>
                    <span class="agent-runs truncate" data-external={outside || undefined} title={outside ? 'External: this factory starts no program for it' : runs ? `Runs on ${runs}` : undefined}>
                      {runs}
                    </span>
                    <span class="agent-doing" data-doing={doing.state} title={outside ? 'What it has in hand is the cards given to it' : doing.state === 'idle' && ended ? `It last ended a turn: ${when(ended)}` : undefined}>
                      <span class={classes('standing', doing.state)} aria-hidden="true" />
                      <span class="truncate">{doing.words}</span>
                    </span>
                    <MayAsk slot={slot} />
                    <span class="agent-acts">
                      <More label={`More that can be done with ${slot.name}`} items={() => items} keys />
                    </span>
                  </li>
                );
              })}
            </ul>
          </section>
        ))
      )}
      {adding && <AddOrInviteAgent onClose={() => setAdding(false)} />}
    </div>
  );
}

const WIDE = 340;

/**
 * An agent's permissions, which are what it may ask of the factory: a
 * few words, and pressed the whole by area, in one panel over the page
 * under what was pressed. Nothing is changed here.
 */
function MayAsk({ slot }: { slot: Slot }) {
  const [open, setOpen] = useState(false);
  const opener = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const [place, setPlace] = useState<{ left: number; top: number; most: number } | null>(null);
  // The usual for one the factory does not start is what one is invited with: its own inbox beside the rest.
  const usual = (isExternal(slot) ? meta.value?.external_scopes : undefined) ?? meta.value?.agent_scopes ?? [];
  const brief = brieflyHeld(slot.scopes, usual, meta.value?.scopes ?? []);
  const width = Math.min(WIDE, window.innerWidth - 16);
  const put = () => {
    const at = opener.current?.getBoundingClientRect();
    const whole = panel.current?.scrollHeight;
    if (!at || whole === undefined) return;
    // It is under what was pressed or over it, whichever has more room, and no taller than that room: where neither holds it whole it
    // scrolls in itself, and never lies over what opened it.
    const most = Math.max(window.innerHeight - at.bottom - 12, at.top - 12, 120);
    const height = Math.min(whole, most);
    setPlace({ ...placed(at, { width, height }, { width: window.innerWidth, height: window.innerHeight }), most });
  };
  // Put where it will be before it is seen: nothing is drawn in one place and then another.
  useLayoutEffect(() => {
    if (open) put();
    else setPlace(null);
  }, [open, slot.scopes.join(' ')]);
  useEffect(() => {
    if (!open) return;
    const away = (event: Event) => {
      const target = event.target as Node | null;
      // A press on what opened it is that button's to answer.
      if (!panel.current?.contains(target) && !opener.current?.contains(target)) setOpen(false);
    };
    const key = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      setOpen(false);
      opener.current?.focus();
    };
    // The window resized, or what is under the panel scrolled: it is put where what opened it now is.
    const moved = (event: Event) => {
      if (event.type === 'scroll' && event.target instanceof Node && panel.current?.contains(event.target)) return;
      put();
    };
    window.addEventListener('pointerdown', away, true);
    window.addEventListener('keydown', key);
    window.addEventListener('resize', moved);
    window.addEventListener('scroll', moved, true);
    return () => {
      window.removeEventListener('pointerdown', away, true);
      window.removeEventListener('keydown', key);
      window.removeEventListener('resize', moved);
      window.removeEventListener('scroll', moved, true);
    };
  }, [open]);
  return (
    <>
      <button
        ref={opener}
        type="button"
        class="ghost agent-may"
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-label={`Permissions of ${slot.name}: ${brief}`}
        title={`Permissions, which are what it may ask of the factory: ${brief}`}
        onClick={() => setOpen(!open)}
      >
        <span class="agent-may-word" aria-hidden="true">
          Permissions
        </span>
        <span class="truncate">{brief}</span>
      </button>
      {open &&
        createPortal(
          <div
            ref={panel}
            class="agent-holds"
            role="dialog"
            aria-label={`Permissions of ${slot.name}`}
            style={{ width: `${width}px`, left: `${place?.left ?? 0}px`, top: `${place?.top ?? 0}px`, maxHeight: place ? `${place.most}px` : undefined, visibility: place ? 'visible' : 'hidden' }}
          >
            <strong class="truncate">Permissions of {slot.name}</strong>
            <p class="faint holds-where">What it may ask of the factory, by area.</p>
            <HoldsTable scopes={slot.scopes} />
            <p class="faint holds-where">It is changed from Edit… in its line's menu, beside a channel the agent is in under Channel in the Tools panel, and for one of a fleet in the row of the fleet's agents.</p>
          </div>,
          document.body,
        )}
    </>
  );
}
