// The door of a private channel, as the page offers it: the switch in
// the dialog that makes a channel, the lines of a channel's menu that
// make it private, open it again and leave it, what each asks first,
// and adding to a channel from where its members are listed.
//
// A private channel is one kept to its members: read by whoever is in
// it and nobody else, which is the daemon's to hold. Here a line is
// drawn only where pressing it does something (`kept.ts` says who is
// offered what), what cannot be undone by the same press is asked about
// first, and what the daemon refuses is said in its own sentence where
// it was asked.

import { signal } from '@preact/signals';
import { useEffect, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Channel, Member } from '../api.ts';
import { kindSaid } from '../external.ts';
import { calledOf, hashed, lastPerson, mayAdd, mayKeep, mayLeave, mayOpen } from '../kept.ts';
import type { Asks } from '../kept.ts';
import { agents, attempt, channels, me, toast } from '../store.ts';
import { Modal, useFocus } from '../ui.tsx';
import { actorKind, classes } from '../util.ts';
import type { Offered } from './grouped.tsx';
import { signs } from './signs.tsx';

/** What is being asked about a channel's door, if anything is. */
const asking = signal<{ kind: 'keep' | 'open' | 'add'; channel: string } | null>(null);

/** Who is signed in, as what they are offered goes by. */
function asks(): Asks | null {
  const who = me.value;
  return who ? { actor: who.actor, scopes: who.principal.scopes, owner: who.account?.role === 'owner' } : null;
}

/** What the daemon said, as a page says it: without the marks a terminal reads commands by. */
const said = (refused: unknown) => (refused instanceof Error ? refused.message : String(refused)).replaceAll('`', '');

/**
 * Whether a channel that is being made is private, and under that,
 * only once it is, whether its agents are asked nowhere else.
 */
export function PrivateChoice(props: { kept: boolean; alone: boolean; onKept: (kept: boolean) => void; onAlone: (alone: boolean) => void }) {
  return (
    <fieldset class="starts kept-choice">
      <label class="start-with">
        <span class="grow">
          Private
          <span class="muted">Only the people and agents in it can read it. Nobody else sees that it is there.</span>
        </span>
        <input type="checkbox" role="switch" class="switch" checked={props.kept} onChange={(event) => props.onKept(event.currentTarget.checked)} />
      </label>
      {props.kept && (
        <label class="start-with">
          <span class="grow">
            Agents here are asked nowhere else
            <span class="muted">An agent in it is given no work anywhere but here, for what is most closely held.</span>
          </span>
          <input type="checkbox" role="switch" class="switch" checked={props.alone} onChange={(event) => props.onAlone(event.currentTarget.checked)} />
        </label>
      )}
    </fieldset>
  );
}

/** The lines of a channel's menu that are of its door: there for whoever pressing them does something for. */
export function doorItems(channel: Channel): Offered[] {
  const by = asks();
  if (!by) return [];
  const items: Offered[] = [];
  if (mayKeep(channel, by)) items.push({ label: 'Make private…', mark: signs.lock(), apart: true, act: () => (asking.value = { kind: 'keep', channel: channel.id }) });
  if (mayOpen(channel, by)) items.push({ label: 'Open again…', mark: signs.globe(), apart: true, act: () => (asking.value = { kind: 'open', channel: channel.id }) });
  if (mayLeave(channel, by)) {
    const leave = async () => {
      if ((await attempt(() => api.leaveChannel(channel.id))) && channel.kept) toast(`You have left ${calledOf(channel)}`);
    };
    items.push({ label: 'Leave', mark: signs.door(), apart: items.length === 0, act: () => void leave() });
  }
  return items;
}

/** What is asked before something of a channel's door is done. */
export function AskDoor() {
  const asked = asking.value;
  const channel = asked && channels.value.find((one) => one.id === asked.channel);
  if (!asked || !channel) return null;
  const close = () => (asking.value = null);
  if (asked.kind === 'keep') return <Keep key={channel.id} channel={channel} onClose={close} />;
  if (asked.kind === 'open') return <OpenAgain key={channel.id} channel={channel} onClose={close} />;
  return <AddTo key={channel.id} channel={channel} onClose={close} />;
}

/** Asks whether an open channel is to be private, saying who then reads it. */
function Keep({ channel, onClose }: { channel: Channel; onClose: () => void }) {
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const keep = async () => {
    setBusy(true);
    setWrong(null);
    try {
      await api.keepChannel(channel.id);
      onClose();
    } catch (refused) {
      setBusy(false);
      setWrong(said(refused));
    }
  };
  const within = channel.members.filter((member) => actorKind(member.actor) !== 'bot');
  return (
    <Modal onClose={onClose}>
      <h2>Make #{channel.name} private?</h2>
      <div class="stack">
        <p style="margin:0">
          {within.length === 1
            ? 'Only you will read it from here, with what was said in it so far.'
            : `Only the ${within.length} in it will read it from here, with what was said in it so far: ${within.map((member) => member.name).join(', ')}.`}{' '}
          Nobody else will see that it is there, and its name is free for another channel.
        </p>
        <p class="faint" style="margin:0;font-size:calc(12px * var(--size))">
          What was said in it until now was said where anybody could read it. Only the factory's owner opens it again.
        </p>
        {wrong && (
          <p role="alert" class="kept-wrong">
            {wrong}
          </p>
        )}
        <div class="row" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" type="button" disabled={busy} onClick={() => void keep()}>
            Make private
          </button>
        </div>
      </div>
    </Modal>
  );
}

/** Asks what a private channel is to be called once it is open, and says who then reads it. */
function OpenAgain({ channel, onClose }: { channel: Channel; onClose: () => void }) {
  const [name, setName] = useState(calledOf(channel));
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const input = useFocus<HTMLInputElement>();
  const open = async (event: Event) => {
    event.preventDefault();
    setBusy(true);
    setWrong(null);
    try {
      await api.openChannel(channel.id, name.trim() || undefined);
      onClose();
    } catch (refused) {
      setBusy(false);
      // The daemon says how it is done from a terminal: here it is the name above.
      setWrong((said(refused).split(' (')[0] ?? '').replace(': say what this one is to be called once it is open', ': call this one something else'));
    }
  };
  return (
    <Modal onClose={onClose}>
      <h2>Open {calledOf(channel)} again?</h2>
      <form class="stack" onSubmit={open}>
        <p style="margin:0">Anybody who may read channels will read it, everything said in it while it was private with the rest.</p>
        <label>
          Name
          <input ref={input} required value={name} onInput={(event) => setName(event.currentTarget.value)} />
        </label>
        {wrong && (
          <p role="alert" class="kept-wrong">
            {wrong}
          </p>
        )}
        <div class="row" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" type="submit" disabled={busy}>
            Open again
          </button>
        </div>
      </form>
    </Modal>
  );
}

/**
 * Adds somebody who is there to a channel: the people with an account
 * here and the agents of this factory, but for those in it already. One
 * the daemon will not have in it is said why, in its own sentence, on
 * their line.
 */
function AddTo({ channel, onClose }: { channel: Channel; onClose: () => void }) {
  const [people, setPeople] = useState<Member[] | null>(null);
  const [refused, setRefused] = useState<Record<string, string>>({});
  useEffect(() => void attempt(() => api.people()).then((found) => setPeople(found ?? [])), []);
  const here = new Set(channel.members.map((member) => member.actor));
  const others: Member[] = [
    ...(people ?? []),
    ...agents.value.map((agent) => ({ actor: `agent:${agent.name}`, name: agent.name })),
  ].filter((one) => !here.has(one.actor));
  const add = async (one: Member) => {
    try {
      await api.addPerson(channel.id, one.actor);
      const { [one.actor]: _was, ...rest } = refused;
      setRefused(rest);
    } catch (why) {
      setRefused({ ...refused, [one.actor]: said(why) });
    }
  };
  return (
    <Modal onClose={onClose}>
      <h2>Add to {hashed(channel)}</h2>
      {channel.kept && <p class="muted">They will read everything said here so far.</p>}
      {people === null ? (
        <p class="faint">Looking up who is here</p>
      ) : others.length === 0 ? (
        <p>Everybody who is here is in this channel already.</p>
      ) : (
        <div class="stack kept-adding">
          {others.map((one) => (
            <div key={one.actor} class={classes('kept-add', refused[one.actor] && 'refused')}>
              <span class="grow truncate">{one.name}</span>
              <span class="faint">{kindSaid(one.actor, agents.value)}</span>
              <button aria-label={`Add ${one.name}`} onClick={() => void add(one)}>
                Add
              </button>
              {refused[one.actor] && <p role="alert">{refused[one.actor]}</p>}
            </div>
          ))}
        </div>
      )}
      <p class="faint kept-note">Somebody new (an agent, a bot, a person with no account yet) is added from the channel's head.</p>
      <div class="row" style="justify-content:flex-end">
        <button type="button" onClick={onClose}>
          Done
        </button>
      </div>
    </Modal>
  );
}

/** Under the list of a channel's members: adding to it, for whoever may; and of a private one, that it is. */
export function MembersDoor({ channel }: { channel: Channel }) {
  const by = asks();
  return (
    <div class="kept-members">
      {channel.kept && (
        <p class="faint kept-said">
          {signs.lock(13)}
          <span>
            Private: read by these and nobody else{channel.alone ? '. Its agents are asked nowhere else' : ''}.
          </span>
        </p>
      )}
      {by && mayAdd(channel, by) && (
        <button class="kept-add-member" onClick={() => (asking.value = { kind: 'add', channel: channel.id })}>
          Add member
        </button>
      )}
      {by && lastPerson(channel, by) && channel.members.some((member) => member.actor === by.actor) && (
        <p class="faint">You are the only person in it: add somebody before you leave.</p>
      )}
    </div>
  );
}

/** On an agent's page among a channel's members: taking it out of the channel, for whoever may. */
export function RemoveAgent({ channel, slot }: { channel: Channel; slot: string }) {
  const by = asks();
  const [wrong, setWrong] = useState<string | null>(null);
  if (!by || !mayAdd(channel, by) || !channel.members.some((member) => member.actor === `agent:${slot}`)) return null;
  const remove = async () => {
    try {
      await api.leaveChannel(channel.id, slot);
      toast(`${slot} is out of ${hashed(channel)}`);
    } catch (refused) {
      setWrong(said(refused));
    }
  };
  return (
    <div class="kept-remove">
      <button title={`Take ${slot} out of this channel${channel.kept ? ', with what it kept of it' : ''}`} onClick={() => void remove()}>
        Remove from channel
      </button>
      {wrong && <p role="alert">{wrong}</p>}
    </div>
  );
}
