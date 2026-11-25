// What runs in a channel, as its Channel tab of the Tools panel lists it:
// each thing applied whose `channel` names it (the daemon finds them,
// `GET /api/channels/{id}/running`, by the rule it goes by everywhere,
// `posts_in`), what may be done to each by whoever reads it, the params
// its form asks and what is sent to apply it again, what is asked
// before it is stopped, and which of what is said means reading again.
//
// Holds nothing of the page, so that its tests can read it.

import type { Fields, GraphParam, Instance } from './api.ts';
import { covers } from './holds.ts';

/** The channel as the daemon goes by it here: its id and its name. */
export interface Named {
  id: string;
  name: string;
}

/** Whether `value` names `channel`, as the daemon reads a `channel` param: its id or its name, with a `#` before it or not. */
export function namesChannel(value: unknown, channel: Named): boolean {
  if (typeof value !== 'string') return false;
  const said = value.replace(/^#+/, '');
  return said === channel.id || said === channel.name;
}

/** What whoever reads one may do to it: what started another thing is changed and stopped where that is, and anything only by whoever may apply. */
export function offeredOf(instance: Pick<Instance, 'parent'>, scopes: string[]): { change: boolean; stop: boolean } {
  const may = instance.parent === null && covers(scopes, 'blueprints:run');
  return { change: may, stop: may };
}

/** What a line says under its name: the key it is applied under where that is not its blueprint's name, its environment, and what started it. */
export function saidOf(instance: Pick<Instance, 'blueprint' | 'key' | 'environment' | 'parent'>): string {
  const parts = [];
  if (instance.key && instance.key !== instance.blueprint) parts.push(`as ${instance.key}`);
  if (instance.environment) parts.push(`under ${instance.environment}`);
  if (instance.parent !== null) parts.push('started by what runs above it');
  return parts.join(', ');
}

/**
 * The params the form asks, in the text's order: every one but those that
 * name this channel, which are what it is listed here by and are kept as
 * they are (`kept`), so that changing its settings never moves it.
 */
export function askedOf(params: GraphParam[], instance: Pick<Instance, 'params'>, channel: Named): { asked: GraphParam[]; kept: Fields } {
  const kept: Fields = {};
  const asked = params.filter((param) => {
    const value = instance.params[param.name];
    if (param.name !== 'channel' || value === undefined || !namesChannel(value, channel)) return true;
    kept[param.name] = value;
    return false;
  });
  return { asked, kept };
}

/**
 * What is sent to apply it again, through the door the apply drawer uses:
 * the same blueprint, key, project, environment and labels, with what the
 * form says and what was kept. The labels are what it was applied with,
 * which everything measured under it carries: applied without them it
 * would carry none from then on.
 */
export function againOf(instance: Pick<Instance, 'blueprint' | 'key' | 'project' | 'environment' | 'labels'>, values: Fields, kept: Fields) {
  return {
    blueprint: instance.blueprint,
    key: instance.key,
    project: instance.project,
    ...(instance.environment ? { environment: instance.environment } : {}),
    ...(Object.keys(instance.labels ?? {}).length > 0 ? { labels: instance.labels } : {}),
    params: { ...values, ...kept },
    wait: true,
  };
}

/** What is asked before it is stopped: what stops, and what stays. */
export function stopSaid(instance: Pick<Instance, 'blueprint' | 'key'>, channel: string): { title: string; says: string; stays: string } {
  const what = instance.key && instance.key !== instance.blueprint ? `${instance.blueprint} (${instance.key})` : instance.blueprint;
  return {
    title: `Stop ${what}?`,
    says: `It stops now, with everything it started under it, and says nothing more in ${channel}.`,
    stays: 'What it said there stays. It starts again only when it is applied again.',
  };
}

/** Whether something said of an instance means what runs here is read again: a thing applied by somebody, a thing changed, or one listed that ended. */
export function heardOf(topic: string, data: Record<string, unknown> | undefined, instance: string | undefined, listed: string[]): boolean {
  if (topic === 'instance.started') return (data?.parent ?? null) === null;
  if (topic === 'instance.updated') return true;
  if (topic === 'instance.finished') return instance !== undefined && listed.includes(instance);
  return false;
}
