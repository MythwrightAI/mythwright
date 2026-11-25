// An agent added from the Agents tab (`views/addagent.tsx`, the dialog
// a channel adds one with): which project it begins on, what keeps it
// from being added as things stand, what is sent and what is said once it
// is. And beside a channel, where the dialog asks first whether the agent
// is one this factory runs or one from outside, which is invited: what
// the dialog comes to as it stands, what it asks for each, and what is
// said once it is done. It holds nothing of the page, so that its tests
// can read it.
//
// The daemon holds every rule (`Agents::ensure`): who may add one
// (`agents:write`), that a project with environments says which one it
// works under (`Agents::none_said`), that the project is there. What is
// closed here is only what the dialog can tell before it asks, and one
// thing the daemon would not refuse: a name that is an agent's already,
// which `POST /api/agents` would take as changing that one. Adding is for
// a new one; one that is there is brought into a channel from the
// channel, and moved or changed from its line.

import { isExternal, nameOf, standingOf } from './external.ts';
import { covers } from './holds.ts';
import type { Outside } from './external.ts';
import type { Shown } from './narrowing.ts';

/**
 * The project the dialog begins on: the one the page is narrowed to,
 * where it is among those offered, and otherwise none (the empty text).
 */
export function beginsOn(shown: Shown, offered: { id: string }[]): string {
  if (shown.kind !== 'projects' || !shown.project) return '';
  const project = shown.project;
  return offered.some((one) => one.id === project) ? project : '';
}

/** The agent a typed name comes to where there is one called so already, stopped or not; nothing where it is new. */
export function takenBy<A extends { name: string }>(typed: string, agents: A[]): A | null {
  const called = nameOf(typed);
  return (called && agents.find((one) => one.name === called)) || null;
}

/**
 * Why it cannot be added as it stands, in a sentence, or nothing: a name
 * that is an agent's already, and, in a project that has environments,
 * none of them chosen. A name not yet written is no reason to say
 * anything: the button is closed until there is one.
 */
export function addClosed(taken: string | null, environments: string[], under: string): string | null {
  if (taken) return `There is already an agent called ${taken}. It is brought into a channel from the channel, and moved from its line here.`;
  if (environments.length > 0 && !environments.includes(under)) return 'Choose the environment it works under: this project has some, and an agent of it works under one of them.';
  return null;
}

/** What is said of it once it is added. */
export function addedSaid(name: string, project: string | null, under: string | null): string {
  if (!project) return `${name} is added, and works in no project.`;
  return under ? `${name} is added, and works in ${project} under ${under}.` : `${name} is added, and works in ${project}.`;
}

/** What the dialog asks, as it is sent. */
export interface Asked {
  name: string;
  adapter: string;
  model: string;
  instructions: string;
  mode: string;
  /** The project's id, or the empty text for none. */
  project: string;
  /** The project's environments as the page read them. */
  environments: string[];
  under: string;
}

/**
 * What is sent to make it (`POST /api/agents`): what is said and nothing
 * else. An environment is sent only where its project has any: one with
 * none is asked nothing of it, and none is never said on purpose for one
 * that has some (the empty text would be taken as none).
 */
export function addedBody(asked: Asked): Record<string, string> {
  const body: Record<string, string> = { name: asked.name.trim(), adapter: asked.adapter };
  if (asked.model) body.model = asked.model;
  if (asked.instructions) body.instructions = asked.instructions;
  if (asked.mode) body.permission_mode = asked.mode;
  if (asked.project) body.project = asked.project;
  if (asked.project && asked.environments.length > 0 && asked.under) body.environment = asked.under;
  return body;
}

/**
 * Which an agent added beside a channel is, as the head of the dialog
 * asks: one this factory runs (`runs`), made with what its program is
 * asked; or one from outside (`outside`), a program somebody runs for
 * themselves, which is invited and joins with its invitation.
 */
export type Way = 'runs' | 'outside';

/**
 * What the channel's dialog comes to as it stands: nothing yet, with no
 * name, which asks what the way chosen asks; an agent that is there, which is brought in by its name with
 * nothing else asked, whichever way was chosen, of either kind; a new one
 * this factory runs; or a new one from outside, invited. An agent that
 * was stopped is not one that is there: one of the factory's is made
 * again as it always was, and one from outside is invited again, which
 * lets it in again.
 */
export type Coming = { kind: 'nothing'; way: Way } | { kind: 'there'; slot: Outside } | { kind: 'new' } | { kind: 'invited' };

export function comingOf(way: Way, typed: string, agents: readonly Outside[]): Coming {
  const called = nameOf(typed);
  if (!called) return { kind: 'nothing', way };
  const slot = agents.find((one) => one.active && one.name === called);
  if (slot) return { kind: 'there', slot };
  return way === 'outside' ? { kind: 'invited' } : { kind: 'new' };
}

/**
 * Which of the two the Agents tab's one dialog offers whoever opens it,
 * by what the daemon would let them do: one this factory runs is added by
 * applying `std/agent` as them (`agents:write` and `blueprints:run`), and
 * one from outside is invited (`agents:write`). With one there is no
 * choice to make, and with neither there is no dialog.
 */
export function waysOffered(scopes: readonly string[]): Way[] {
  // The same for the channel's dialog (`kindsOffered`), where an agent is offered at all only where one of these is.
  const held = [...scopes];
  const ways: Way[] = [];
  if (covers(held, 'agents:write') && covers(held, 'blueprints:run')) ways.push('runs');
  if (covers(held, 'agents:write')) ways.push('outside');
  return ways;
}

/** What the dialog asks under the name for what it comes to: what a program is run with, or what an invitation is made with, or neither. */
export function asksOf(coming: Coming): { program: boolean; invitation: boolean } {
  const way = coming.kind === 'nothing' ? coming.way : coming.kind === 'new' ? 'runs' : coming.kind === 'invited' ? 'outside' : null;
  return { program: way === 'runs', invitation: way === 'outside' };
}

/** What the button that does it says. */
export function doneBy(coming: Coming): string {
  return coming.kind === 'invited' || (coming.kind === 'nothing' && coming.way === 'outside') ? 'Invite' : 'Add agent';
}

/**
 * How one that is there is brought into the channel: one from outside as
 * a member is (`POST /channels/<id>/members`, which changes nothing else
 * of it, the project it is listed with among that); one this factory runs
 * as it always was, by its name to the route that makes one, which
 * applies its text as whoever asks, where they may apply (`applies`), and
 * as a member is where they may not, since that route would refuse them.
 */
export function broughtBy(slot: Outside, applies: boolean): 'member' | 'name' {
  return isExternal(slot) || !applies ? 'member' : 'name';
}

/** What is said of one that is there, in the dialog, before it is brought in. */
export function thereSaid(slot: Outside & { project?: string | null }, now: number): string {
  if (isExternal(slot)) return `${slot.name} is an agent from outside (${standingOf(slot, now)}). Adding it brings it into this channel, and nothing else of it changes.`;
  return `${slot.name} already has a slot (${slot.adapter}${slot.project ? `, working on ${slot.project}` : ''}).`;
}

/** The project one invited beside a channel begins on: the channel's, where it is offered, and otherwise none. */
export function invitesOn(channel: string | null | undefined, offered: { id: string }[]): string {
  return channel && offered.some((one) => one.id === channel) ? channel : '';
}

/**
 * What is said under its invitation of where it is: in the channel,
 * listed as invited until it joins; or, where the daemon would not bring
 * it in after it was invited, that, in its sentence, and how it is
 * brought in later. The invitation is made either way, and is shown.
 */
export function invitedInto(name: string, channel: string, refused: string | null): string {
  if (refused === null) return `${name} is in ${channel}, listed as invited until it joins.`;
  const said = /[.!?]$/.test(refused.trim()) ? refused.trim() : `${refused.trim()}.`;
  return `${name} is invited, and was not brought into ${channel}: ${said} It is brought in from the channel's members, by its name.`;
}
