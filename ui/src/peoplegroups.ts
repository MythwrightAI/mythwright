// Groups of people and agents, and what their members may ask beside
// what each holds itself (`people::groups`): what Settings' Groups draws,
// worked out from what was read. (`groups.ts` is a table's column groups.)
//
// The daemon holds every rule: whoever manages the factory changes a
// group, nobody gives by one what they do not hold, and an agent never
// changes a group it is in. This offers only what it would let through,
// and says why where it would not.

import { type Area, covers } from './holds.ts';

/** A group as `/api/groups` hands it; `holds` only to whoever manages the factory. */
export interface PeopleGroup {
  id: string;
  name: string;
  holds?: string[];
  members: string[];
  created: number;
}

/** Somebody who can be in a group: a person or an agent, by actor. */
export interface Joiner {
  actor: string;
  name: string;
  kind: 'person' | 'agent';
}

/** The most letters a group's name has: the daemon's `LONGEST_NAME`. */
export const LONGEST_NAME = 60;

/** Everybody who can be in a group: the people with an account, then the agents, each by name. */
export function joiners(people: { id: string; name: string }[], agents: { name: string }[]): Joiner[] {
  const byName = (one: Joiner, other: Joiner) => one.name.localeCompare(other.name);
  return [
    ...people.map((one) => ({ actor: `user:${one.id}`, name: one.name, kind: 'person' as const })).sort(byName),
    ...agents.map((one) => ({ actor: `agent:${one.name}`, name: one.name, kind: 'agent' as const })).sort(byName),
  ];
}

/** Who a member is, as it is drawn: by name where they are known, else as the daemon wrote them. */
export function memberOf(actor: string, known: Joiner[]): Joiner {
  const found = known.find((one) => one.actor === actor);
  if (found) return found;
  const [kind, name] = actor.includes(':') ? [actor.slice(0, actor.indexOf(':')), actor.slice(actor.indexOf(':') + 1)] : ['', actor];
  return { actor, name: kind === 'agent' ? name : actor, kind: kind === 'agent' ? 'agent' : 'person' };
}

/** Who may be put in `group`: everybody who is not in it already, in the order they are listed. */
export function addable(group: PeopleGroup, known: Joiner[]): Joiner[] {
  return known.filter((one) => !group.members.includes(one.actor));
}

/** Whether somebody who holds `mine` changes groups: whoever manages the factory. */
export function mayManage(mine: string[]): boolean {
  return covers(mine, 'admin');
}

/**
 * Whether whoever holds `mine` holds `scope`: an area written whole is
 * held where each thing done in it is, as the daemon has it, since a
 * table of areas writes one whose every part is chosen as the area.
 */
export function holdsAll(mine: string[], scope: string, areas: Area[]): boolean {
  if (covers(mine, scope)) return true;
  const area = areas.find((one) => one.area === scope);
  return !!area && area.verbs.length > 0 && area.verbs.every((verb) => covers(mine, `${scope}:${verb.verb}`));
}

/**
 * Why somebody who holds `mine` may not put anybody in `group`, if they
 * may not: putting somebody in a group gives them all it holds, and
 * nobody gives what they do not hold.
 */
export function addClosed(mine: string[], group: PeopleGroup, areas: Area[]): string | null {
  const beyond = (group.holds ?? []).find((scope) => !holdsAll(mine, scope, areas));
  return beyond ? `It holds ${beyond}, which you do not, and nobody gives what they do not hold.` : null;
}

/** Why `name` is no name for a group (`id` the group's own, where it is renamed), or nothing where it is one. */
export function nameRefused(name: string, groups: PeopleGroup[], id?: string): string | null {
  const said = name.trim();
  if (said === '') return 'A group is called something: say what.';
  if ([...said].length > LONGEST_NAME) return `A group's name is ${LONGEST_NAME} letters at most.`;
  const taken = groups.find((one) => one.id !== id && one.name.toLowerCase() === said.toLowerCase());
  return taken ? `There is a group called ${taken.name} already.` : null;
}

/** How many are in a group, in words. */
export function countSaid(group: PeopleGroup): string {
  const count = group.members.length;
  return count === 0 ? 'nobody in it yet' : count === 1 ? '1 member' : `${count} members`;
}

/** What a group holds, in words for its fold's line. */
export function holdsSaid(group: PeopleGroup): string {
  const holds = group.holds ?? [];
  return holds.length === 0 ? 'nothing beyond what each holds' : holds.join(', ');
}
