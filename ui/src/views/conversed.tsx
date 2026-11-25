// A conversation, as the page draws what is its own: the mark at the
// inbox's head that writes one and the dialog it opens, what stands
// where a channel's name does at the head of its page, and opening it
// as a channel.
//
// A conversation is a channel kept to its members that says it is one
// (`conversed.ts`), so its page is a channel's page, opened as a tab
// from its line in the inbox. It is written in one asking: who it is
// to, what it is about and what is said go to the daemon together, and
// what the daemon refuses (an agent that may be in no private channel,
// somebody who is gone) is said in its own sentence with the dialog
// staying as it was written. What was written is kept while the dialog
// is away, until it is sent.

import { signal } from '@preact/signals';
import { useEffect, useState } from 'preact/hooks';
import { api, call } from '../api.ts';
import type { Channel, Member, Message } from '../api.ts';
import { held } from '../chosen.ts';
import { candidates, found, isConversation, mayOpenAsChannel, mayWrite, places, written } from '../conversed.ts';
import type { Candidate, Place } from '../conversed.ts';
import { calledOf, mayLeave } from '../kept.ts';
import { agents, attempt, channels, me, openTab, projects, toast } from '../store.ts';
import { workspaces } from '../spaces.ts';
import { Modal, useFocus } from '../ui.tsx';
import { actorKind, classes } from '../util.ts';
import type { Offered } from './grouped.tsx';
import { signs } from './signs.tsx';

const asks = {
  write: (asked: { to: string[]; subject?: string; text: string }) => call<{ channel: Channel; message: Message }>('POST', '/inbox/conversations', asked),
  asChannel: (id: string, asked: { name?: string; workspace?: string; project?: string }) =>
    call<{ channel: Channel; workspace: string | null }>('POST', `/inbox/conversations/${encodeURIComponent(id)}/channel`, asked),
};

/** Whether the dialog that writes one is open. */
const composing = signal(false);
/** What is being written, kept while the dialog is away. */
const draft = signal<{ to: Candidate[]; subject: string; text: string }>({ to: [], subject: '', text: '' });
/** The conversation being asked about opening as a channel, by id. */
const opening = signal<string | null>(null);

/** What the daemon said, as a page says it: without the marks a terminal reads commands by. */
const said = (refused: unknown) => (refused instanceof Error ? refused.message : String(refused)).replaceAll('`', '');

/** At the inbox's head: writes a conversation, for whoever may. */
export function ComposeMark() {
  if (!mayWrite(me.value?.principal.scopes ?? [])) return null;
  return (
    <button class="ghost icon brand-act inbox-compose" aria-label="Write a message" title="Write a message to somebody" onMouseDown={(event) => event.stopPropagation()} onClick={() => (composing.value = true)}>
      {signs.pencil()}
    </button>
  );
}

/** The dialogs of a conversation, drawn once in the frame. */
export function AskConversed() {
  const channel = opening.value && channels.value.find((one) => one.id === opening.value);
  return (
    <>
      {composing.value && <Compose onClose={() => (composing.value = false)} />}
      {channel && isConversation(channel) && <AsChannel key={channel.id} channel={channel} onClose={() => (opening.value = null)} />}
    </>
  );
}

/** Writes a conversation: who it is to, what it is about, and what is first said. */
function Compose({ onClose }: { onClose: () => void }) {
  const [people, setPeople] = useState<Member[] | null>(null);
  const [typed, setTyped] = useState('');
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const first = useFocus<HTMLInputElement>();
  useEffect(() => void attempt(() => api.people()).then((read) => setPeople(read ?? [])), []);
  const { to, subject, text } = draft.value;
  const keep = (next: Partial<typeof draft.value>) => (draft.value = { ...draft.value, ...next });
  const chosen = new Set(to.map((one) => one.actor));
  const offered = found(candidates(people ?? [], agents.value, me.value?.actor ?? ''), typed).filter((one) => !chosen.has(one.actor));
  const add = (one: Candidate) => {
    keep({ to: [...to, one] });
    setTyped('');
  };
  const asked = written(
    to.map((one) => one.actor),
    subject,
    text,
  );
  const send = async (event: Event) => {
    event.preventDefault();
    if (!asked || busy) return;
    setBusy(true);
    setWrong(null);
    try {
      const sent = await asks.write(asked);
      // It is shown at once: what lists the channels reads it again when it hears of it.
      if (!channels.value.some((one) => one.id === sent.channel.id)) channels.value = [...channels.value, sent.channel];
      draft.value = { to: [], subject: '', text: '' };
      onClose();
      openTab({ kind: 'channel', id: sent.channel.id });
    } catch (refused) {
      // Nothing was made: what was written stays, to be put right.
      setBusy(false);
      setWrong(said(refused));
    }
  };
  return (
    <Modal onClose={onClose}>
      <h2>New message</h2>
      <form class="stack conversed-compose" onSubmit={send}>
        <div class="conversed-field">
          <label for="conversed-to">To</label>
          <div class="conversed-to">
            {to.map((one) => (
              <span key={one.actor} class="conversed-chosen">
                <span class="truncate">{one.name}</span>
                <button type="button" class="ghost icon" aria-label={`Take ${one.name} off`} onClick={() => keep({ to: to.filter((other) => other.actor !== one.actor) })}>
                  {signs.cross()}
                </button>
              </span>
            ))}
            <input
              id="conversed-to"
              ref={first}
              value={typed}
              placeholder={to.length === 0 ? 'A person or an agent' : 'Somebody else'}
              autocomplete="off"
              onInput={(event) => setTyped(event.currentTarget.value)}
              onKeyDown={(event) => {
                // Enter takes the first that is offered, and does not send what is half written.
                if (event.key !== 'Enter') return;
                event.preventDefault();
                if (typed.trim() && offered[0]) add(offered[0]);
              }}
            />
          </div>
          <div class="conversed-offered" role="listbox" aria-label="Who it can be written to">
            {people === null ? (
              <p class="faint">Looking up who is here</p>
            ) : offered.length === 0 ? (
              <p class="faint">{typed.trim() ? 'Nobody here is called that.' : to.length > 0 ? 'Everybody here is in it.' : 'Nobody else is here yet.'}</p>
            ) : (
              offered.map((one) => (
                <button key={one.actor} type="button" role="option" aria-selected="false" class="conversed-one" onClick={() => add(one)}>
                  <span class="grow truncate">{one.name}</span>
                  <span class="faint">{one.kind}</span>
                </button>
              ))
            )}
          </div>
        </div>
        <label>
          Subject
          <input value={subject} maxLength={200} placeholder="Left empty, it is called by its first words" onInput={(event) => keep({ subject: event.currentTarget.value })} />
        </label>
        <label>
          Message
          <textarea required rows={5} value={text} onInput={(event) => keep({ text: event.currentTarget.value })} />
        </label>
        <p class="faint conversed-note">Only those it is to and you will read it. It is in the inbox of each, and is answered there.</p>
        {wrong && (
          <p role="alert" class="kept-wrong">
            {wrong}
          </p>
        )}
        <div class="row modal-foot" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" type="submit" disabled={busy || !asked}>
            Send
          </button>
        </div>
      </form>
    </Modal>
  );
}

/** Where a channel's name is at the head of its page: an envelope, and what the conversation is about. */
export function Subject({ channel }: { channel: Channel }) {
  return (
    <h1 class="conversed-subject">
      <span class="kept-lock" role="img" aria-label="Conversation" title="A conversation: read by whoever it is to and whoever wrote it, and nobody else">
        {signs.envelope()}
      </span>
      <span class="truncate">{calledOf(channel)}</span>
    </h1>
  );
}

/** Who is signed in, as what they are offered goes by. */
function asker() {
  const who = me.value;
  return who ? { actor: who.actor, scopes: who.principal.scopes, owner: who.account?.role === 'owner' } : null;
}

/** What is done with a conversation, for whoever pressing a line does something for: under Channel in the Tools panel beside it. */
export function conversationItems(channel: Channel): Offered[] {
  const by = asker();
  if (!by || !isConversation(channel)) return [];
  const items: Offered[] = [];
  if (mayOpenAsChannel(channel, by)) items.push({ label: 'Open as a channel…', mark: signs.out(), act: () => (opening.value = channel.id) });
  if (mayLeave(channel, by)) {
    const leave = async () => {
      if (await attempt(() => api.leaveChannel(channel.id))) toast(`You have left ${calledOf(channel)}`);
    };
    items.push({ label: 'Leave', mark: signs.door(), apart: items.length > 0, act: () => void leave() });
  }
  return items;
}

const sameTo = (one: Place['to'], other: Place['to']) => JSON.stringify(one) === JSON.stringify(other);

/** Asks what a conversation is to be called as a channel and what it belongs to, saying what goes with it. */
function AsChannel({ channel, onClose }: { channel: Channel; onClose: () => void }) {
  const showing = held.value.kind === 'space' ? held.value.id : null;
  const offered = places(workspaces.value, projects.value, showing);
  const [name, setName] = useState(calledOf(channel));
  const [to, setTo] = useState<Place['to'] | null>(offered[0]?.to ?? null);
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const input = useFocus<HTMLInputElement>();
  const within = channel.members.filter((member) => actorKind(member.actor) !== 'bot');
  const open = async (event: Event) => {
    event.preventDefault();
    setBusy(true);
    setWrong(null);
    try {
      await asks.asChannel(channel.id, { name: name.trim() || undefined, ...(to ?? {}) });
      toast(`${name.trim() || calledOf(channel)} is a channel now`);
      onClose();
    } catch (refused) {
      setBusy(false);
      setWrong(said(refused));
    }
  };
  return (
    <Modal onClose={onClose}>
      <h2>Open as a channel?</h2>
      <form class="stack" onSubmit={open}>
        <p style="margin:0">
          Everything said in it so far goes with it. It will be a private channel, listed among the channels of{' '}
          {within.length === 1 ? 'you alone' : `the ${within.length} in it: ${within.map((member) => member.name).join(', ')}`}. It cannot be made a conversation again.
        </p>
        <label>
          Name
          <input ref={input} required maxLength={200} value={name} onInput={(event) => setName(event.currentTarget.value)} />
        </label>
        {offered.length > 0 && (
          <fieldset class="belongs">
            <legend>It belongs to</legend>
            {offered.map((one) => (
              <label key={JSON.stringify(one.to)} class="belongs-choice">
                <input type="radio" name="conversed-belongs" checked={to !== null && sameTo(to, one.to)} onChange={() => setTo(one.to)} />
                <span class="grow truncate">{one.label}</span>
                <span class="faint">{one.kind}</span>
              </label>
            ))}
          </fieldset>
        )}
        {wrong && (
          <p role="alert" class="kept-wrong">
            {wrong}
          </p>
        )}
        <div class="row modal-foot" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class={classes('primary')} type="submit" disabled={busy}>
            Open as a channel
          </button>
        </div>
      </form>
    </Modal>
  );
}
