// Who someone in a channel is: what an agent runs on, and the card shown
// when a person, an agent or a bot is pointed at.

import type { ComponentChildren } from 'preact';
import { useEffect, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Member, Slot } from '../api.ts';
import { handleOf } from '../mentions.ts';
import { agents, me, permissionModes, projectName, providers, refreshAgents, showAgent, showMember, speaker, working } from '../store.ts';
import { actorKind, agentState, classes, initials } from '../util.ts';
import { isExternal, standingOf } from '../external.ts';
import { signal } from '@preact/signals';
import { knownBy, modelSaid, readable } from '../modelsaid.ts';
import type { Lists } from '../modelsaid.ts';
import { sees } from '../sees.ts';

/**
 * What an agent runs on, in a few words: its model, by the name its provider
 * gives it, where that is known; otherwise the provider itself.
 */
export function runsOn(agent: string): string | null {
  const slot = agents.value.find((one) => one.name === agent);
  if (!slot) return null;
  const provider = providers.value.find((one) => one.name === slot.adapter);
  // A model nothing gives a name, which its program reported, is said in a readable form of its id (`modelsaid.ts`
  // says why there is often none); one somebody chose is said as they wrote it.
  const named = said(slot);
  if (named) return named;
  // The built-in assistant has no model behind it, and nothing to name.
  return provider?.kind === 'builtin' ? null : (provider?.title ?? slot.adapter);
}

/** What an agent's model is said as: its name, else what somebody chose as they wrote it, else what its program reported, readable. */
export function said(slot: Slot): string | null {
  const named = slot.model_name ?? slot.model;
  if (!named) return null;
  return named === slot.model ? named : readable(named);
}

/**
 * The same for a list of agents: what a provider with no model behind it
 * is called, where that is what an agent runs on; and nothing where all
 * there is to go by is the id of a package, its provider not being known
 * and no model named.
 */
export function runsOnListed(agent: string): string | null {
  const slot = agents.value.find((one) => one.name === agent);
  if (!slot) return null;
  const provider = providers.value.find((one) => one.name === slot.adapter);
  if (!provider && !slot.model_name && !slot.model) return null;
  return runsOn(agent) ?? provider?.title ?? null;
}

/**
 * What is said of an agent the factory does not start, where another is
 * said to be idle or at work and to run on something: nothing answers
 * for it, and what names it is there for it to read. Nothing, for any
 * other agent.
 */
export function reads(agent: string): string | null {
  return isExternal(agents.value.find((one) => one.name === agent)) ? 'external · reads what is said for itself' : null;
}

/** The providers already asked, in this page, which models they offer. */
const asked = new Set<string>();
/** What each provider asked said it offers: each model's id and what it calls it, by which a model a message names is said. */
const lists = signal<Lists>({});

/**
 * The model a message of `agent` says wrote it (`id`), by the name its
 * provider gives it where that is known (`modelSaid`), else as what the
 * agent runs on now where the message says none. The head of a message
 * and whether it reads as one with the one before go by this alone.
 */
export function modelOf(agent: string, id: string | null | undefined): string | null {
  return modelSaid(id, agents.value.find((one) => one.name === agent), lists.value, runsOn(agent));
}

/**
 * Finds out what each agent here runs on is called, where that is not known
 * yet: the usual model of a provider an agent is left to choose by, the
 * name a provider gives the model that was chosen for one, and the name of
 * a model a message of one says wrote it (`said`, the ids by agent). The
 * provider's program is asked once; the daemon remembers.
 */
export function learnModels(members: Member[], said: Record<string, string[]> = {}): void {
  for (const member of members) {
    if (actorKind(member.actor) !== 'agent') continue;
    const slot = agents.value.find((one) => one.name === member.name);
    const provider = slot && providers.value.find((one) => one.name === slot.adapter);
    if (!slot || !provider) continue;
    // Named already, unless the name is only what was typed to choose it (`otter`, where its provider would say which one that is),
    // or a message of it names a model that has no name here.
    const unnamed = (said[member.name] ?? []).some((id) => !knownBy(id, slot, lists.value));
    if (slot.model_name && slot.model_name !== slot.model && !unnamed) continue;
    if (provider.models === 'none' || !provider.available || asked.has(provider.name)) continue;
    asked.add(provider.name);
    void api
      .models(provider.name, false)
      .then((models) => {
        lists.value = { ...lists.value, [provider.name]: models.map(({ id, label }) => ({ id, label })) };
        return refreshAgents();
      })
      .catch(() => {});
  }
}

const CARD_WIDTH = 300;
/** The room between a card and what it is about: enough to be told apart, little enough to cross before it goes. */
const GAP = 12;
/** How long a card stays after the pointer leaves it or what it is about: long enough to cross from one to the other. */
const LINGER_MS = 180;
/** Roughly how tall a card is, to decide whether it fits under what it is about. */
const CARD_HEIGHT = 230;

/** Takes away the card that is showing, if one is: there is only ever one. */
let putAway: (() => void) | null = null;

/**
 * Wraps something that stands for a participant (a name, a picture), and
 * shows a card about them while it is pointed at or has the keyboard.
 * `still` is where pressing it does nothing: there is no channel for them
 * to be shown in (a name said in a topic, say), and it is no button.
 */
export function Who(props: { member: Member; children: ComponentChildren; class?: string; onPress?: () => void; says?: string | false; still?: boolean }) {
  const { member, children, class: name } = props;
  // Anybody, wherever they are named, is pressed to be read about, unless pressing them here does something else:
  // an agent wherever the reader is, and a bot or a person where there is a channel for them to be of.
  const kind = actorKind(member.actor);
  const slot = kind === 'agent' ? member.actor.slice('agent:'.length) : null;
  // An agent that was taken away is named where it said something, and is nothing to press: its card says it is gone.
  const there = slot !== null && agents.value.some((one) => one.name === slot);
  const about = slot ? (there ? () => showAgent(slot) : undefined) : kind === 'bot' || kind === 'user' ? () => showMember(member.actor) : undefined;
  const onPress = props.still ? undefined : (props.onPress ?? about);
  const says = props.says || (!props.onPress && about ? `About ${member.name}` : undefined);
  const anchor = useRef<HTMLSpanElement>(null);
  const [at, setAt] = useState<{ left: number; top?: number; bottom?: number } | null>(null);
  const show = () => {
    const box = anchor.current?.getBoundingClientRect();
    if (!box) return;
    // Kept inside the window, under what it is about, or above when there is no room under.
    const left = Math.max(8, Math.min(box.left, window.innerWidth - CARD_WIDTH - 8));
    if (box.bottom + GAP + CARD_HEIGHT > window.innerHeight) setAt({ left, bottom: window.innerHeight - box.top + GAP });
    else setAt({ left, top: box.bottom + GAP });
  };
  // The card stays while it is itself pointed at, so that it can be read
  // and copied from. There is a gap between it and what it is about, so it
  // goes a moment after the pointer leaves, not at once; coming back, or
  // on to the card, within that moment keeps it.
  const going = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const stay = () => clearTimeout(going.current);
  // A finger does not point before it presses, and does not leave: under
  // one, the card is shown by touching what it is about, and put away by
  // touching that again or anything else that is not the card. This is
  // whether the card was showing when a finger last came down on what it is
  // about, and `undefined` while what points is not a finger.
  const wasShown = useRef<boolean | undefined>(undefined);
  const pressed = useRef(false);
  const hide = useRef(() => {
    stay();
    setAt(null);
  }).current;
  const leave = () => {
    // A finger that was lifted has not left: a browser that says it has is saying where a pointer it does not have would be.
    if (wasShown.current !== undefined) return;
    stay();
    going.current = setTimeout(hide, LINGER_MS);
  };
  // One that is still on its way out goes at once when another is shown.
  const open = () => {
    stay();
    if (putAway && putAway !== hide) putAway();
    putAway = hide;
    show();
  };
  useEffect(
    () => () => {
      stay();
      if (putAway === hide) putAway = null;
    },
    [],
  );
  const showing = at !== null;
  useEffect(() => {
    if (!showing) return;
    const elsewhere = (event: PointerEvent) => {
      if ((event.pointerType === 'touch' || wasShown.current !== undefined) && !anchor.current?.contains(event.target as Node)) hide();
    };
    window.addEventListener('pointerdown', elsewhere);
    return () => window.removeEventListener('pointerdown', elsewhere);
  }, [showing]);
  const touch = (event: MouseEvent) => {
    // Only the press a finger made: the same press is not answered twice.
    if (!pressed.current || wasShown.current === undefined || (event.target as Element).closest('.who-card')) return;
    pressed.current = false;
    if (wasShown.current) hide();
    else open();
  };
  return (
    <span
      class={classes('who-anchor', name)}
      ref={anchor}
      tabIndex={0}
      onMouseEnter={open}
      onMouseLeave={leave}
      onFocus={open}
      onBlur={hide}
      onPointerDown={(event) => {
        pressed.current = event.pointerType === 'touch';
        wasShown.current = pressed.current ? showing : undefined;
      }}
      // Where pressing it does something, it is a button: to the pointer, to
      // the keyboard, and to whoever is told what is on the page. A press
      // on the card over it is not a press on it.
      role={onPress ? 'button' : undefined}
      aria-label={says}
      onClick={
        onPress
          ? (event) => {
              if ((event.target as Element).closest('.who-card')) return;
              hide();
              onPress();
            }
          : touch
      }
      onKeyDown={
        onPress &&
        ((event) => {
          if (event.target !== event.currentTarget || (event.key !== 'Enter' && event.key !== ' ')) return;
          event.preventDefault();
          hide();
          onPress();
        })
      }
    >
      {children}
      {at && (
        <div class="who-card" role="tooltip" style={{ left: at.left, top: at.top, bottom: at.bottom, width: CARD_WIDTH }}>
          <Card member={member} />
        </div>
      )}
    </span>
  );
}

function Card({ member }: { member: Member }) {
  const kind = actorKind(member.actor);
  const head = (what: string, state?: ComponentChildren) => (
    <div class="who-head">
      <div class={classes('avatar', kind)}>{kind === 'system' ? '◆' : initials(member.name)}</div>
      {/* The name has its line to itself and wraps; how it stands is beside what it is, under the name, where a long name cannot push it off the card. */}
      <div class="who-named">
        <strong class="who-name">{member.name}</strong>
        <div class="who-what">
          <span class="muted">{what}</span>
          {state}
        </div>
      </div>
    </div>
  );
  if (kind === 'agent') {
    const slot = agents.value.find((one) => one.name === member.name);
    if (!slot) return head('agent', <span class="badge plain">gone</span>);
    // One the factory does not start: nothing of a program is said of it, and it is asked nothing.
    if (isExternal(slot)) {
      return (
        <>
          {head('agent · external', <span class={classes('badge', slot.active ? 'idle' : 'failed')}>{standingOf(slot, Date.now())}</span>)}
          <dl>
            <dt>Runs</dt>
            <dd>
              outside<span class="muted">: this factory starts no program for it</span>
            </dd>
            {slot.project && (
              <>
                <dt>Listed with</dt>
                <dd>{projectName(slot.project)}</dd>
              </>
            )}
            <dt>Can use</dt>
            <dd class="muted">{slot.scopes.join(', ')}</dd>
          </dl>
          <p class="faint">A message that names it starts nothing: it reads what is said here for itself.</p>
        </>
      );
    }
    const provider = providers.value.find((one) => one.name === slot.adapter);
    const model = said(slot);
    const may = permissionModes.value.find((mode) => mode.name === slot.permission_mode);
    const busy = working.value[slot.name];
    return (
      <>
        {head('agent', <span class={classes('badge', busy ? 'ok' : slot.active ? 'idle' : 'failed')}>{busy ? 'working' : slot.active ? 'idle' : 'stopped'}</span>)}
        <dl>
          <dt>Runs on</dt>
          <dd>
            {provider?.title ?? slot.adapter}
            {(model || provider?.kind !== 'builtin') && <span class="muted"> · {model ?? 'its usual model'}</span>}
          </dd>
          <dt>May do</dt>
          <dd>
            {slot.permission_mode}
            {may && <span class="muted">: {may.means}</span>}
          </dd>
          <Sees slot={slot} builtin={provider?.kind === 'builtin'} />
          {slot.project && (
            <>
              <dt>Works on</dt>
              <dd>{projectName(slot.project)}</dd>
            </>
          )}
          <dt>Can use</dt>
          <dd class="muted">{slot.scopes.join(', ')}</dd>
        </dl>
        {slot.instructions && <p class="who-note">{slot.instructions.length > 220 ? `${slot.instructions.slice(0, 220)}…` : slot.instructions}</p>}
        <p class="faint">Write @{slot.name} to ask it.</p>
      </>
    );
  }
  if (kind === 'bot' && speaker.value?.name === member.actor.slice('bot:'.length)) {
    return (
      <>
        {head('bot')}
        <p class="who-note">The factory's own voice in a channel. It has no model: it takes commands, and does each as whoever asks.</p>
        <p class="faint">
          Write @{speaker.value.name} and one of its {speaker.value.commands.length} commands. Press its name to see them.
        </p>
      </>
    );
  }
  if (kind === 'bot') {
    return (
      <>
        {head('bot')}
        <p class="who-note">A blueprint taking part in this channel. It answers by itself, the same way every time, with no model behind it.</p>
        <p class="faint">Write @{handleOf(member.name)} to ask it. Stopping its instance, under Activity, takes it out.</p>
      </>
    );
  }
  if (kind === 'user') {
    const you = member.actor === me.value?.actor;
    return (
      <>
        {head(you ? 'human · you' : 'human')}
        <p class="faint">{you ? `Signed in here as ${me.value?.principal.name}.` : 'Has an account on this installation.'}</p>
      </>
    );
  }
  return (
    <>
      {head('a notice')}
      <p class="faint">Posted by a blueprint as it ran, not by anyone in the channel.</p>
    </>
  );
}

/** How an agent stands just now, as a mark beside its name: green and turning in the middle of a turn, yellow and still when it is idle, hollow when it is not running. */
export function Standing({ agent }: { agent: string }) {
  const state = agentState(agent in working.value, agents.value.find((slot) => slot.name === agent));
  const says = { working: 'working', idle: 'idle', stopped: 'not running' }[state];
  return <span class={classes('standing', state)} role="img" aria-label={says} title={says} />;
}

/**
 * What an agent's program sees of the device it runs on: its work and
 * little else, behind a wall, or all that whoever runs the daemon does.
 * Nothing is said of one that is run by the daemon itself and has no
 * program to keep in.
 */
export function Sees({ slot, builtin }: { slot: Slot; builtin: boolean }) {
  if (builtin) return null;
  const [what, how] = sees(slot);
  const more = [...(slot.reads ?? []), ...(slot.changes ?? [])];
  return (
    <>
      <dt>Sees</dt>
      <dd title={slot.sees?.walled && more.length > 0 ? `Also shown: ${more.join(', ')}` : undefined}>
        {what}
        <span class="muted">: {how}</span>
      </dd>
    </>
  );
}

