// Somebody read about: an agent, a person or a bot, as the Tools panel
// beside a channel draws them under Channel members, and as the drawer
// draws them where no channel is beside what was pressed (a member of the
// side panel's Members; whoever said something on a record). It came out of `context.tsx`, which draws it beside a
// channel, so that the two are one drawing.
//
// Where somebody is read about is one function (`readAbout`): beside the
// channel showing, in the Tools panel, where they are of it or are an
// agent; else in the drawer, which says nothing of a channel.

import { useEffect, useMemo, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Channel, Member, Message, Turn } from '../api.ts';
import { coalesce } from '../events.ts';
import { doingNow } from '../doing.ts';
import { heldIn } from '../heldin.ts';
import { hashed } from '../kept.ts';
import { fixedWords } from '../mentions.ts';
import { active, agents, channels, drawer, me, openTab, permissionModes, projectName, providers, showMember, speaker, working } from '../store.ts';
import { useEvents } from '../ui.tsx';
import { actorKind, classes, duration, when } from '../util.ts';
import { isExternal } from '../external.ts';
import { ExternalSeen } from './external.tsx';
import { AgentHolds, PersonHolds } from './holds.tsx';
import { BotDone } from './botdone.tsx';
import { BotHolds } from './botholds.tsx';
import { ManagerLine } from './channeldetails.tsx';
import { said as modelSaidOf, Sees, Standing } from './who.tsx';

/**
 * Shows somebody read about: beside the channel that is showing, in its
 * Tools panel, where they are in it or are an agent (which is read about
 * there wherever it is); anywhere else in the drawer. `name` is what they
 * are called where they were pressed, said where the page knows no other.
 */
export function readAbout(actor: string, name: string): void {
  const kind = actorKind(actor);
  if (kind === 'system') return;
  const shown = active.value.startsWith('channel:') ? channels.value.find((one) => `channel:${one.id}` === active.value) : undefined;
  if (shown && (kind === 'agent' || shown.members.some((one) => one.actor === actor))) return showMember(actor);
  const member: Member = { actor, name };
  drawer.value = { title: name, body: () => <Seen key={actor} member={member} /> };
}

/** Somebody read about in the drawer: as the Tools panel reads them, with nothing of a channel. */
function Seen({ member }: { member: Member }) {
  const kind = actorKind(member.actor);
  if (kind === 'agent') return <AgentSeen slot={member.actor.slice('agent:'.length)} />;
  if (kind === 'bot') return <BotSeen member={member} project={null} />;
  return <PersonSeen member={member} />;
}

/** The channels whoever reads this and somebody are both in: each pressed to open it. */
function Shared({ actor }: { actor: string }) {
  const mine = me.value?.actor;
  const both = channels.value.filter((one) => !one.archived && one.members.some((m) => m.actor === actor) && one.members.some((m) => m.actor === mine));
  if (actor === mine) return null;
  return (
    <>
      <h3>Channels you share</h3>
      {both.length === 0 ? (
        <p class="faint">None yet.</p>
      ) : (
        <div class="seen-shared">
          {both.map((one) => (
            <button key={one.id} type="button" class="ghost seen-channel" onClick={() => openTab({ kind: 'channel', id: one.id })}>
              <span class="truncate">{hashed(one)}</span>
            </button>
          ))}
        </div>
      )}
    </>
  );
}


/**
 * What an agent in the middle of a turn is said to be doing: on a card,
 * the card and what it does with it, as the Agents tab says it
 * (`doingNow`); else the last line its program printed.
 */
export function atWork(turn: { channel: string | null; thread?: string | null; text: string }): string {
  if (heldIn(turn.thread)) return doingNow(true, { channel: turn.channel, thread: turn.thread ?? null }, undefined, 0, () => null).words;
  return turn.text.trim().split('\n').pop() || 'working';
}
/**
 * An agent, read about: how it stands and what it is at, what it runs on
 * and may do, what it was told it is for, and what it was last asked.
 */
export function AgentSeen({ slot }: { slot: string }) {
  const kept = agents.value.find((one) => one.name === slot);
  const turn = working.value[slot];
  const [turns, setTurns] = useState<Turn[] | null>(null);
  const reload = useMemo(() => coalesce(() => void api.turns(slot).then(setTurns, () => setTurns([])), 200), [slot]);
  useEffect(() => {
    setTurns(null);
    reload();
  }, [reload]);
  // What it was asked changes as it is asked, and as it answers.
  useEvents(['agent.turn.**'], (event) => event.data?.slot === slot && reload(), [slot]);
  if (!kept) return <p class="faint context-agent-gone">There is no agent called {slot} any more. What it said is still under its name.</p>;
  // One the factory does not start has no program to say anything of.
  if (isExternal(kept)) return <ExternalSeen slot={kept} />;
  const provider = providers.value.find((one) => one.name === kept.adapter);
  const model = modelSaidOf(kept);
  const may = permissionModes.value.find((mode) => mode.name === kept.permission_mode);
  return (
    <div class="context-about">
      <div class="row">
        <Standing agent={slot} />
        <strong class="context-card-title context-name">{slot}</strong>
        <span class="grow" />
        <span class="faint">agent</span>
      </div>
      <div class={classes('context-doing', turn ? 'working' : 'faint')}>{turn ? atWork(turn) : kept.active ? 'idle' : 'not running'}</div>
      <dl class="context-facts">
        <dt>Runs on</dt>
        <dd>
          {provider?.title ?? kept.adapter}
          {(model || provider?.kind !== 'builtin') && <span class="muted"> · {model ?? 'its usual model'}</span>}
        </dd>
        <dt>May do</dt>
        <dd>
          {kept.permission_mode}
          {may && <span class="muted">: {may.means}</span>}
        </dd>
        <Sees slot={kept} builtin={provider?.kind === 'builtin'} />
        {kept.project && (
          <>
            <dt>Works on</dt>
            <dd>{projectName(kept.project)}</dd>
          </>
        )}
        {kept.web !== undefined && provider?.kind !== 'builtin' && (
          <>
            <dt>The web</dt>
            <dd>
              {kept.web ? 'may use it' : 'no'}
              <span class="muted">: {kept.web ? 'its program searches and fetches unasked' : 'its program is not let search or fetch'}</span>
            </dd>
          </>
        )}
        <dt>Added</dt>
        <dd class="muted">{when(kept.created)}</dd>
      </dl>
      <AgentHolds slot={kept} />
      <h3>What it was told it is for</h3>
      {kept.instructions ? <div class="context-card-body">{kept.instructions}</div> : <p class="faint">Nothing beyond what every agent here is told.</p>}
      <h3>What it was last asked</h3>
      {turns === null && <p class="faint">Reading</p>}
      {turns?.length === 0 && <p class="faint">It has not been asked anything yet.</p>}
      {turns?.slice(0, 8).map((one) => (
        <div key={one.id} class="context-turn">
          <div class="row">
            <span class={classes('badge', one.status === 'done' ? 'ok' : one.status)}>{one.status}</span>
            <span class="faint">
              {when(one.created)}
              {one.started && one.finished ? ` · ${duration(one.finished - one.started)}` : ''}
            </span>
          </div>
          <div class="context-turn-words">{one.error ?? (one.reply.trim() || 'No answer yet.')}</div>
        </div>
      ))}
    </div>
  );
}


/** What somebody said here lately, newest first: when, how it began, and for a bot which blueprint had it said. */
function SaidLately({ actor, messages }: { actor: string; messages: Message[] }) {
  const theirs = messages.filter((message) => message.author === actor).slice(-6).reverse();
  if (theirs.length === 0) return <p class="faint">Nothing here yet.</p>;
  return (
    <>
      {theirs.map((message) => {
        const said = message.said_by;
        return (
          <div key={message.id} class="context-turn">
            <div class="row">
              <span class="faint">{when(message.created)}</span>
              <span class="grow" />
              {/* The channel names no blueprint for what a bot says; here, where the bot is read about, is where it is. */}
              {said && (
                <button class="ghost link context-from" title={`Its step ${said.step}, in run ${said.run}`} onClick={() => openTab({ kind: 'run', id: said.run })}>
                  from {said.blueprint}
                </button>
              )}
            </div>
            <div class="context-turn-words">{message.body.trim() || 'Something attached.'}</div>
          </div>
        );
      })}
    </>
  );
}

/**
 * A bot, read about: what it is, what it takes when it is written to (the
 * factory's own, which takes commands), and what it said here lately with
 * which blueprint had it say each.
 */
export function BotSeen({ member, messages, onSay, project }: { member: Member; messages?: Message[]; onSay?: (words: string) => void; project: string | null }) {
  const handle = member.actor.slice('bot:'.length);
  const own = speaker.value?.name === handle ? speaker.value : null;
  return (
    <div class="context-about">
      <div class="row">
        <strong class="context-card-title">{member.name}</strong>
        <span class="grow" />
        <span class="faint">bot</span>
      </div>
      {own ? (
        <>
          <p class="muted">The factory's own voice in a channel: what its blueprints have to say is said by it. It has no model and guesses nothing. It takes the commands below, and does each as whoever asks, with what they hold.</p>
          {/* A project's channel is where that project is asked about: the daemon holds it to that, and this says so. */}
          {project && (
            <p class="muted">
              In this channel it is {projectName(project)}'s: what it is asked here is about that project, and it does nothing to another's.
            </p>
          )}
          <h3>Commands</h3>
          {own.commands.map((one) => (
            <button key={one.written} class="context-command" title={onSay ? 'Begin writing it' : undefined} disabled={!onSay} onClick={() => onSay?.(`@${handle} ${fixedWords(one.written)} `)}>
              <code>
                @{handle} {one.written}
              </code>
              <span class="muted">{one.does}</span>
            </button>
          ))}
        </>
      ) : (
        <>
          <p class="muted">A blueprint taking part in this channel under this name. It answers by itself, the same way every time, with no model behind it.</p>
          <p class="faint">Write @{handle} to ask it. Stopping its instance, under Activity, takes it out.</p>
        </>
      )}
      {/* The factory's own: what it may ask, and what it did lately anywhere the reader may read, as the records' histories say it. */}
      {own && (
        <>
          <BotHolds />
          <h3>What it did lately</h3>
          <BotDone />
        </>
      )}
      {/* Read about where no channel is beside it (`readAbout`), there is no "here" to say what it said in. */}
      {messages && (
        <>
          <h3>What it said here lately</h3>
          <SaidLately actor={member.actor} messages={messages} />
        </>
      )}
    </div>
  );
}

/** A person, read about: who they are here, and what they said here lately. */
export function PersonSeen({ channel, member, messages }: { channel?: Channel; member: Member; messages?: Message[] }) {
  const you = member.actor === me.value?.actor;
  return (
    <div class="context-about">
      <div class="row">
        <strong class="context-card-title">{member.name}</strong>
        <span class="grow" />
        <span class="faint">{you ? 'person · you' : 'person'}</span>
      </div>
      <p class="muted">{you ? `Signed in here as ${me.value?.principal.name}.` : 'Has an account on this installation.'}</p>
      {channel && <ManagerLine channel={channel} member={member} />}
      <PersonHolds you={you} />
      {messages ? (
        <>
          <h3>What they said here lately</h3>
          <SaidLately actor={member.actor} messages={messages} />
        </>
      ) : (
        <Shared actor={member.actor} />
      )}
    </div>
  );
}
