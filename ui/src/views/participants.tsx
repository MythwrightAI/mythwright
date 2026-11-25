// Who is in a channel and adding to them, or inviting into it: the list
// the participants in the channel's head open, which ends in one line that
// adds somebody (Add a person, agent or bot, or what of those is offered),
// and the one dialog that line opens, Add or invite channel member.
//
// There are three kinds of member, chosen at the dialog's head. An agent
// works from a slot, on a model, or is a program from outside that is
// invited (`addagent.tsx`). A person has an account on this installation,
// or is invited to make one. A bot is a blueprint taking part under a
// name of its own: it answers by itself, with no model behind it. Who is
// offered which, and what is said, is `memberadded.ts`; the daemon holds
// each, and what it refuses is said in its sentence with the dialog
// staying.

import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Channel, Member } from '../api.ts';
import { isExternal } from '../external.ts';
import type { Listed } from '../given.ts';
import { covers } from '../holds.ts';
import { hashed, mayAdd } from '../kept.ts';
import { handleOf } from '../mentions.ts';
import { KINDS, beginsAs, invitedSaid, inviting, kindsOffered, lineCalled, nobodySaid } from '../memberadded.ts';
import type { Kind } from '../memberadded.ts';
import { settingsOf } from '../membersettings.ts';
import { agents, me, openTab, toast } from '../store.ts';
import { actorKind, classes, initials } from '../util.ts';
import { Modal } from '../ui.tsx';
import { AddAgent, InvitedInto } from './addagent.tsx';
import type { InvitedHere } from './addagent.tsx';
import { Invitation, copy } from './external.tsx';
import { MemberSettings } from './membersettings.tsx';
import { signs } from './signs.tsx';
import { runsOn } from './who.tsx';

/** What the dialog is called. */
const CALLED = 'Add or invite channel member';

/** Who is signed in, as what is offered goes by. */
function asker() {
  const who = me.value;
  return who ? { actor: who.actor, scopes: who.principal.scopes, owner: who.account?.role === 'owner' } : null;
}

/** The kinds the one signed in is offered beside `channel`: none, and there is no line that adds. */
function offeredHere(channel: Channel): Kind[] {
  const by = asker();
  return by ? kindsOffered(channel, by, me.value?.can_invite ?? false) : [];
}

/** How people, agents and bots are told apart in a few words. One the factory does not start runs on nothing it knows of. */
function kindOf(member: Member): string {
  const kind = actorKind(member.actor);
  if (kind === 'agent') {
    const name = member.actor.slice('agent:'.length);
    if (isExternal(agents.value.find((slot) => slot.name === name))) return 'agent (external)';
    return ['agent', runsOn(name)].filter(Boolean).join(' · ');
  }
  if (kind === 'bot') return 'bot';
  return member.actor === me.value?.actor ? 'human · you' : 'human';
}

/**
 * Who is in the channel, in its head: a few faces, one over the edge of
 * the next, and how many there are. Pressing it lists them all, over the
 * page and moving nothing, and the list ends, set apart under them, in
 * the one line that adds somebody (`lineCalled`), which opens the one
 * dialog, for whoever may add anybody; nobody else has the line.
 *
 * The arrows, Home and End go through what can be pressed in the list,
 * which is that line: who is in it has nothing to press, and the keyboard
 * passes them by, as a menu passes by a line that only says something.
 * Opened from the keyboard, the list has it at the line; Escape puts the
 * list away and gives it back to what opened it.
 */
export function Participants({ members, channel }: { members: Member[]; channel: Channel }) {
  const [open, setOpen] = useState<false | 'pointer' | 'keys'>(false);
  const [adding, setAdding] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  const opener = useRef<HTMLButtonElement>(null);
  const line = useRef<HTMLButtonElement>(null);
  const called = lineCalled(offeredHere(channel));
  // Listened to from the frame the list is drawn in, which the line is reached from at once; opened from the
  // keyboard, the list has it at its line as it is drawn.
  useLayoutEffect(() => {
    if (!open) return;
    if (open === 'keys') line.current?.focus();
    const away = (event: Event) => {
      if (event instanceof KeyboardEvent) {
        if (event.key === 'Escape') {
          const had = box.current?.contains(document.activeElement);
          setOpen(false);
          if (had) opener.current?.focus();
          return;
        }
        if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key) && line.current) {
          event.preventDefault();
          line.current.focus();
        }
        return;
      }
      if (!box.current?.contains(event.target as Node)) setOpen(false);
    };
    window.addEventListener('pointerdown', away);
    window.addEventListener('keydown', away);
    return () => {
      window.removeEventListener('pointerdown', away);
      window.removeEventListener('keydown', away);
    };
  }, [open]);
  // People first: theirs are the faces, while there are any.
  const order = { user: 0, agent: 1, bot: 2, system: 3 };
  const all = [...members].sort((a, b) => order[actorKind(a.actor)] - order[actorKind(b.actor)]);
  const face = (member: Member) => <span class={classes('avatar', 'face', actorKind(member.actor))}>{initials(member.name)}</span>;
  return (
    <div class="menu-anchor" ref={box}>
      <button
        ref={opener}
        class="ghost participants"
        aria-haspopup="menu"
        aria-expanded={!!open}
        aria-label={all.length === 1 ? '1 participant' : `${all.length} participants`}
        onClick={(event) => setOpen(open ? false : event.detail === 0 ? 'keys' : 'pointer')}
      >
        <span class="faces" aria-hidden="true">
          {all.slice(0, 3).map((member) => (
            <span key={member.actor}>{face(member)}</span>
          ))}
        </span>
        <span>
          {all.length}
          {/* The word is left out where there is no room for it: the faces say what is counted. */}
          <span class="roomy">{all.length === 1 ? ' participant' : ' participants'}</span>
        </span>
      </button>
      {open && (
        <div class="menu people" role="menu" aria-label="Participants">
          {/* Who is in it scrolls where they are many, and the line that adds stays in view under them. */}
          <div class="people-list" role="none">
            {all.map((member) => (
              // The name, and under it in fewer words what they are: a long name wraps whole within the list, which
              // is never wider than the window and never scrolls sideways.
              <div key={member.actor} class="person" role="menuitem" aria-disabled="true">
                {face(member)}
                <span class="person-said">
                  <strong class="person-name">{member.name}</strong>
                  <span class="faint person-kind">{kindOf(member)}</span>
                </span>
              </div>
            ))}
          </div>
          {called && (
            <button
              ref={line}
              type="button"
              class="people-add"
              role="menuitem"
              aria-haspopup="dialog"
              onClick={() => {
                setOpen(false);
                setAdding(true);
              }}
            >
              <span class="people-add-mark" aria-hidden="true">
                {signs.plus()}
              </span>
              <span>{called}</span>
            </button>
          )}
        </div>
      )}
      {adding && <AddChannelMember channel={channel} onClose={() => setAdding(false)} />}
    </div>
  );
}

/**
 * The dialog: who it is, chosen at its head, and under that what that one
 * asks. It begins on `begins` where that is offered (a channel with no
 * agent asks for one), else on the first that is.
 */
export function AddChannelMember({ channel, onClose, begins }: { channel: Channel; onClose: () => void; begins?: Kind }) {
  const offered = offeredHere(channel);
  const [kind, setKind] = useState<Kind | null>(() => beginsAs(offered, begins));
  // The caret begins in what the first kind asks; once another is chosen it stays on the choice.
  const [chosen, setChosen] = useState(false);
  const [invited, setInvited] = useState<InvitedHere | null>(null);
  if (invited) {
    return (
      <Invitation made={invited.invited} onClose={onClose}>
        <InvitedInto made={invited} channel={hashed(channel)} />
      </Invitation>
    );
  }
  const now = kind && offered.includes(kind) ? kind : beginsAs(offered);
  return (
    <Modal onClose={onClose}>
      <h2>{CALLED}</h2>
      {offered.length > 1 && (
        <div class="add-agent-ways add-member-kinds" role="radiogroup" aria-label="Who it is">
          {KINDS.filter(([one]) => offered.includes(one)).map(([one, title, means]) => (
            <label key={one} class={`add-agent-way${now === one ? ' chosen' : ''}`} data-kind={one}>
              <input
                type="radio"
                name="add-member-kind"
                value={one}
                checked={now === one}
                onChange={() => {
                  setChosen(true);
                  setKind(one);
                }}
              />
              <span class="add-agent-way-words">
                <strong>{title}</strong>
                <span class="muted">{means}</span>
              </span>
            </label>
          ))}
        </div>
      )}
      {now === 'agent' && <AddAgent key="agent" channel={channel.id} onClose={onClose} focus={!chosen} onInvited={setInvited} />}
      {now === 'person' && <AddPerson key="person" channel={channel} onClose={onClose} focus={!chosen} />}
      {now === 'bot' && <AddBot key="bot" channel={channel} onClose={onClose} focus={!chosen} />}
    </Modal>
  );
}

/** The caret put in `ref` as a part is first drawn, where it is to begin there. */
function useBegins<T extends HTMLElement>(focus: boolean) {
  const ref = useRef<T>(null);
  useLayoutEffect(() => {
    if (focus) ref.current?.focus();
  }, []);
  return ref;
}

/**
 * Adds someone who has an account here, chosen among those who are not in
 * the channel, with what they may ask of the factory under them; and, for
 * whoever may invite, makes an invitation to register for somebody who
 * has none, shown this once.
 */
function AddPerson({ channel, onClose, focus }: { channel: Channel; onClose: () => void; focus: boolean }) {
  const [people, setPeople] = useState<Member[] | null>(null);
  // What each account holds is read by whoever manages the factory, and by nobody else.
  const [accounts, setAccounts] = useState<Listed[] | null>(null);
  const [picked, setPicked] = useState('');
  const [code, setCode] = useState('');
  const [wrong, setWrong] = useState('');
  const [busy, setBusy] = useState(false);
  const first = useBegins<HTMLInputElement>(focus);
  const by = asker();
  const mine = by?.scopes ?? [];
  useEffect(() => {
    api
      .people()
      .then(setPeople)
      .catch((err) => {
        setPeople([]);
        setWrong(err instanceof Error ? err.message : String(err));
      });
    if (covers(mine, 'admin')) void api.accounts().then((answer) => setAccounts(answer.accounts as Listed[]), () => setAccounts([]));
  }, []);
  const adds = by ? mayAdd(channel, by) : false;
  const here = new Set(channel.members.map((member) => member.actor));
  const others = (people ?? []).filter((person) => !here.has(person.actor));
  const nobody = people === null ? null : nobodySaid(people.length, others.length);
  const invites = inviting(me.value?.can_invite ?? false, mine);
  const chosen = others.find((person) => person.actor === picked) ?? others[0] ?? null;
  const account = chosen ? (accounts?.find((one) => `user:${one.id}` === chosen.actor) ?? null) : null;
  const sections = chosen ? settingsOf({ kind: 'person', name: chosen.name, account }, mine) : [];
  const add = async (event: Event) => {
    event.preventDefault();
    if (!chosen || busy) return;
    setBusy(true);
    setWrong('');
    try {
      await api.addPerson(channel.id, chosen.actor);
      toast(`${chosen.name} is now in ${hashed(channel)}`);
      onClose();
    } catch (err) {
      setWrong(err instanceof Error ? err.message : String(err));
      setBusy(false);
    }
  };
  const makeInvitation = async () => {
    setWrong('');
    try {
      setCode((await api.invite()).code);
    } catch (err) {
      setWrong(err instanceof Error ? err.message : String(err));
    }
  };
  return (
    <form class="stack add-person" onSubmit={(event) => void add(event)}>
      {adds && (
        <>
          <p class="muted">Anyone with an account on this installation can be added to {hashed(channel)}.</p>
          {people === null ? (
            <p class="faint">Looking up who has an account here</p>
          ) : nobody ? (
            <p>{nobody}</p>
          ) : (
            <div class="add-people" role="radiogroup" aria-label="Who is added">
              {others.map((person, index) => (
                <label key={person.actor} class={`add-person-line${chosen?.actor === person.actor ? ' chosen' : ''}`} data-actor={person.actor}>
                  <input ref={index === 0 ? first : undefined} type="radio" name="add-person" value={person.actor} checked={chosen?.actor === person.actor} onChange={() => setPicked(person.actor)} />
                  <span class="truncate">{person.name}</span>
                </label>
              ))}
            </div>
          )}
          {chosen && <MemberSettings sections={sections} scopes={[]} onScopes={() => {}} onAccount={(changed) => setAccounts((accounts ?? []).map((one) => (one.id === changed.id ? changed : one)))} />}
        </>
      )}
      {invites !== 'nothing' && (
        <div class="stack add-person-invite">
          <strong>Someone who has no account yet</strong>
          {invites === 'invite' ? (
            code ? (
              <>
                <p>Their invitation, shown this once:</p>
                <div class="invite-shown">
                  <code class="code-box invite-code">{code}</code>
                  <button type="button" onClick={() => void copy(code, 'The invitation')}>
                    Copy
                  </button>
                </div>
                <p class="muted invited-person">{invitedSaid(hashed(channel), adds)}</p>
              </>
            ) : (
              <>
                <p class="muted">They join this installation with an invitation: a code that works once. Make one and give it to them.</p>
                <div class="row">
                  <button type="button" onClick={() => void makeInvitation()}>
                    Make an invitation
                  </button>
                </div>
              </>
            )
          ) : (
            <>
              <p class="muted">The free plan is for one person. Inviting others needs the premium plan, which is chosen in Settings.</p>
              <div class="row">
                <button
                  type="button"
                  onClick={() => {
                    onClose();
                    openTab({ kind: 'settings' });
                  }}
                >
                  Open Settings
                </button>
              </div>
            </>
          )}
        </div>
      )}
      {wrong && (
        <p role="alert" class="error-text">
          {wrong}
        </p>
      )}
      <div class="row modal-foot">
        <button type="button" onClick={onClose}>
          {chosen && adds ? 'Cancel' : 'Done'}
        </button>
        {chosen && adds && (
          <button class="primary" type="submit" disabled={busy}>
            Add {chosen.name}
          </button>
        )}
      </div>
    </form>
  );
}

/** The blueprint that ships for a bot of the simplest kind. */
const BOT = 'std/bot';

/**
 * Adds a bot: applies the bot blueprint to this channel. A bot of another
 * kind is any blueprint that joins a channel and answers messages in it,
 * applied like any other.
 */
function AddBot({ channel, onClose, focus }: { channel: Channel; onClose: () => void; focus: boolean }) {
  const [name, setName] = useState('');
  const [run, setRun] = useState('echo "You said: $MESSAGE"');
  const [wrong, setWrong] = useState('');
  const [busy, setBusy] = useState(false);
  const input = useBegins<HTMLInputElement>(focus);
  const handle = handleOf(name);
  const taken = channel.members.some((member) => member.actor === `bot:${handle}` || member.actor === `agent:${handle}`);
  const submit = async (event: Event) => {
    event.preventDefault();
    if (busy) return;
    setBusy(true);
    setWrong('');
    try {
      await api.apply({
        blueprint: BOT,
        // One instance for each bot in each channel; adding it again changes that one.
        key: `bot-${handle}-${channel.name}`,
        project: channel.project ?? undefined,
        params: { channel: channel.name, name: name.trim(), run },
      });
      toast(`${name.trim()} is joining ${hashed(channel)}. Write @${handle} to ask it.`);
      onClose();
    } catch (err) {
      setWrong(err instanceof Error ? err.message : String(err));
      setBusy(false);
    }
  };
  return (
    <form class="stack add-bot" onSubmit={(event) => void submit(event)}>
      <p class="muted">
        A bot is a blueprint taking part in the channel: it answers by itself, the same way every time, with no model behind it. This
        one (<code>{BOT}</code>) answers, when someone writes @name, with what a command prints.
      </p>
      <label>
        Name
        <input ref={input} required value={name} placeholder="uptime" onInput={(event) => setName(event.currentTarget.value)} />
        {handle && <span class="muted">{taken ? `There is already an @${handle} in this channel.` : `People will address it as @${handle}.`}</span>}
      </label>
      <label>
        Answers with what this command prints
        <textarea class="mono" rows={2} required value={run} onInput={(event) => setRun(event.currentTarget.value)} />
        <span class="muted">
          It runs on this device each time the bot is mentioned. The message reaches it as <code>$MESSAGE</code> and its author as{' '}
          <code>$AUTHOR</code>; what people write is never part of the command itself.
        </span>
      </label>
      <p class="muted">
        It runs as an instance of the blueprint, listed under Activity; stopping that instance takes the bot out of the channel. A
        blueprint of your own that joins a channel and answers in it is a bot too.
      </p>
      {wrong && (
        <p role="alert" class="error-text">
          {wrong}
        </p>
      )}
      <div class="row modal-foot">
        <button type="button" onClick={onClose}>
          Cancel
        </button>
        <button class="primary" type="submit" disabled={busy || !handle || taken}>
          Add bot
        </button>
      </div>
    </form>
  );
}
