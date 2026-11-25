// Adding to a channel, or inviting into it: the one dialog the line at the
// foot of who is in a channel opens (`views/participants.tsx`), which asks first who it is (an agent, a
// person or a bot) and then what that one asks. What is offered to whom,
// what is said, and what the list of agents under the name says of each,
// are worked out here from plain values, so that the tests can read it.
//
// The daemon holds every rule: whoever may write in a channel adds to it
// (of one kept to its members, one of them: `may_write`), an agent is
// made or brought in by whoever may change agents, a bot is a blueprint
// applied (`blueprints:run`, and what `std/bot` lists under
// `permissions`), and an invitation to register is made by whoever
// manages the installation on a plan that has more than one person
// (`can_invite`). Here a choice is drawn only where it would not be
// refused as things stand.

import { wordsOf, says } from './blueprintlist.ts';
import { isExternal, nameOf } from './external.ts';
import { covers } from './holds.ts';
import { waysOffered } from './agentadded.ts';
import { mayAdd } from './kept.ts';
import type { Asks, Called } from './kept.ts';

/** Who is being added: one of three kinds. */
export type Kind = 'agent' | 'person' | 'bot';

/** Each kind as the dialog's head offers it: a word or two, and a few more under them. */
export const KINDS: [Kind, string, string][] = [
  ['agent', 'An agent', 'One this factory runs, or one from outside, invited.'],
  ['person', 'A person', 'Somebody with an account here, or invited to make one.'],
  ['bot', 'A bot', 'A blueprint that answers by itself, with no model.'],
];

/** What `std/bot` lists under `permissions`, which whoever applies it is to hold. */
export const BOT_TAKES = ['blueprints:run', 'channels', 'shell'];

/**
 * The kinds `by` is offered beside `channel`, in the order they are
 * drawn: an agent where they may add to the channel and change agents; a
 * person where they may add to the channel, or invite somebody to
 * register; a bot where they may add to the channel and apply what makes
 * one. None, and the head draws no button.
 */
export function kindsOffered(channel: Called, by: Asks, canInvite: boolean): Kind[] {
  const adds = mayAdd(channel, by);
  const offered: Kind[] = [];
  // An agent where either way of having one is the asker's (`waysOffered`): inviting takes less than adding one this factory runs.
  if (adds && waysOffered(by.scopes).length > 0) offered.push('agent');
  if (adds || canInvite) offered.push('person');
  if (adds && BOT_TAKES.every((scope) => covers(by.scopes, scope))) offered.push('bot');
  return offered;
}

/**
 * The line that adds somebody, at the foot of who is in a channel: it
 * names what may be added, a person, an agent and a bot in that order,
 * and only those offered. Nothing where nothing is: then there is no line.
 */
export function lineCalled(offered: Kind[]): string | null {
  const words = (['person', 'agent', 'bot'] as const).filter((one) => offered.includes(one));
  if (words.length === 0) return null;
  const said = words.length === 1 ? words[0]! : `${words.slice(0, -1).join(', ')} or ${words.at(-1)}`;
  return `Add ${words[0] === 'agent' ? 'an' : 'a'} ${said}`;
}

/** The kind the dialog begins on: the one asked for where it is offered, else the first offered. */
export function beginsAs(offered: Kind[], asked?: Kind): Kind | null {
  return asked && offered.includes(asked) ? asked : (offered[0] ?? null);
}

/**
 * What the person part says of somebody with no account yet: an
 * invitation to make, for whoever may invite; for whoever manages the
 * installation on a plan for one person, that the plan is why; and
 * nothing for anybody else, who may not invite whatever the plan.
 */
export function inviting(canInvite: boolean, scopes: string[]): 'invite' | 'plan' | 'nothing' {
  if (canInvite) return 'invite';
  return covers(scopes, 'admin') ? 'plan' : 'nothing';
}

/**
 * What is said once a person is invited, beside the code. An invitation
 * to register names no channel (the daemon keeps none with it), so they
 * are not brought in by registering: whoever invited them adds them.
 */
export function invitedSaid(channel: string, mayAddHere: boolean): string {
  const how = 'Give it to them with the address of this installation: they choose Register and enter it. It works once.';
  return mayAddHere
    ? `${how} Registering does not bring them into ${channel}: once they have registered, add them here.`
    : `${how} Registering does not bring them into ${channel}: somebody in it adds them once they have registered.`;
}

/** What is said where nobody can be added from those with an account. */
export function nobodySaid(people: number, others: number): string | null {
  if (others > 0) return null;
  return people <= 1 ? 'Nobody else has an account here yet.' : 'Everyone with an account here is already in this channel.';
}

/** As much of an agent as a line of the list under its name goes by. */
export interface Offerable {
  name: string;
  adapter: string;
  active: boolean;
  project?: string | null;
}

/** A line of the list under an agent's name: its name, and the few words said of it beside it. */
export interface AgentLine {
  name: string;
  said: string[];
}

/**
 * What a line says of an agent: the project it works in (or none), what
 * it runs on as it is said everywhere else (`runsOn`), or that it is
 * external where the factory does not start it, that it is stopped, and
 * that it is in this channel already. `project` is what the project is
 * called, where it has one the page knows of; `on` what it runs on, or
 * nothing where nothing is to be said.
 */
export function agentLine(slot: Offerable, about: { project: string | null; on: string | null; here: boolean }): AgentLine {
  const said = [about.project ?? (slot.project ? slot.project : 'no project')];
  if (isExternal(slot)) said.push('external');
  else if (about.on) said.push(about.on);
  if (!slot.active) said.push('stopped');
  if (about.here) said.push('in this channel');
  return { name: slot.name, said };
}

/**
 * The lines the list shows for what is typed: each that says every word
 * typed, in its name or in what is said beside it; those working first,
 * then those stopped, each in the order they were handed.
 */
export function agentsFound(lines: AgentLine[], typed: string): AgentLine[] {
  const words = wordsOf(typed);
  const found = lines.filter((line) => says(words, [line.name, ...line.said]));
  const stopped = (line: AgentLine) => line.said.includes('stopped');
  return [...found.filter((line) => !stopped(line)), ...found.filter(stopped)];
}

/**
 * The list's last line: what a name no agent has comes to, which is a
 * new one made, or invited from outside; for a name a stopped agent has,
 * that it is made, or invited, again. Nothing for a name a working agent
 * has, which is that agent, its own line saying so.
 */
export function lastLine(outside: boolean, typed: string, agents: readonly Offerable[]): string | null {
  const made = outside ? 'invited' : 'made';
  const called = nameOf(typed);
  if (!called) return `A name no agent has makes a new one, ${outside ? 'invited from outside' : 'run by this factory'}.`;
  const slot = agents.find((one) => one.name === called);
  if (slot?.active) return null;
  if (slot) return `${called} is stopped: it is ${made} again.`;
  return `No agent is called ${called}: a new one is ${made}.`;
}

/**
 * Where the list's mark goes for a key: down and up go round the lines,
 * the first from none; any other key leaves it where it is. None is
 * marked until a key moves the mark, so that Enter on a name no agent
 * has is the dialog's.
 */
export function marking(marked: number, count: number, key: string): number {
  if (count === 0) return -1;
  if (key === 'ArrowDown') return marked < 0 ? 0 : (marked + 1) % count;
  if (key === 'ArrowUp') return marked < 0 ? count - 1 : (marked + count - 1) % count;
  return Math.min(marked, count - 1);
}

/**
 * Where the list under a field goes, and how tall it may be: under the
 * field where it fits there, over it where it fits there instead, and
 * otherwise on the side with more room, as tall as that room and
 * scrolling in itself; never over the field, and never past the window
 * by less than `room`. `tall` is how tall it would be with nothing held.
 */
export function listPlaced(field: { left: number; top: number; bottom: number; width: number }, tall: number, window: { width: number; height: number }, room = 8): { left: number; top: number; width: number; most: number | null } {
  const gap = 4;
  const left = Math.max(room, Math.min(field.left, window.width - field.width - room));
  const under = window.height - field.bottom - gap - room;
  const over = field.top - gap - room;
  if (tall <= under) return { left, top: field.bottom + gap, width: field.width, most: null };
  if (tall <= over) return { left, top: field.top - gap - tall, width: field.width, most: null };
  if (under >= over) return { left, top: field.bottom + gap, width: field.width, most: Math.max(0, under) };
  return { left, top: room, width: field.width, most: Math.max(0, over) };
}
