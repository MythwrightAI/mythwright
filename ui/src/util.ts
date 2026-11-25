// Small pure helpers: time, sizes, names.

import { timeIn, whenIn, worn } from './clock.ts';
import type { Setting } from './clock.ts';

/** A time as the reader has it written: `10:07 PM MDT` in their zone, or `22:07 MDT` on a clock of twenty-four hours (`clock.ts`). */
export function clock(ms: number, setting: Setting = worn.value): string {
  return timeIn(ms, setting);
}

/** The time today, `Mon 10:07 PM MDT` this week, `3 Nov 10:07 PM MDT` otherwise, each day the reader's. */
export function when(ms: number, now = Date.now(), setting: Setting = worn.value): string {
  return whenIn(ms, setting, now);
}

/** `850ms`, `4.2s`, `3m 05s`, `2h 10m`. */
export function duration(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return '-';
  if (ms < 1000) return `${Math.round(ms)}ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(ms < 10_000 ? 1 : 0)}s`;
  if (ms < 3_600_000) return `${Math.floor(ms / 60_000)}m ${String(Math.floor((ms % 60_000) / 1000)).padStart(2, '0')}s`;
  return `${Math.floor(ms / 3_600_000)}h ${String(Math.floor((ms % 3_600_000) / 60_000)).padStart(2, '0')}m`;
}

/** An amount of something that is not a length of time, as the daemon says what it is of: money with its sign, anything else its number; whole once it is large, and never fewer than two places below one. */
export function amount(value: number, unit: string): string {
  if (!Number.isFinite(value)) return '-';
  const size = Math.abs(value);
  const number = size >= 100 ? Math.round(value).toLocaleString('en-US') : size >= 1 ? value.toFixed(2) : value.toFixed(4).replace(/0{1,2}$/, '');
  return unit === 'usd' ? '$' + number : number;
}

export function ago(ms: number, now = Date.now()): string {
  const passed = now - ms;
  if (passed < 45_000) return 'just now';
  if (passed < 3_600_000) return `${Math.round(passed / 60_000)}m ago`;
  if (passed < 86_400_000) return `${Math.round(passed / 3_600_000)}h ago`;
  return `${Math.round(passed / 86_400_000)}d ago`;
}

export function percent(fraction: number): string {
  if (!Number.isFinite(fraction)) return '-';
  const value = fraction * 100;
  return `${value >= 10 || value === 0 ? value.toFixed(0) : value.toFixed(1)}%`;
}

export function megabytes(mb: number): string {
  return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${Math.round(mb)} MB`;
}

export function count(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 10_000) return `${Math.round(n / 1000)}k`;
  return n.toLocaleString();
}

/** How much a search found, in a few words. As many as were asked for may not be all there are. */
export function found(hits: number, asked: number): string {
  if (hits === 0) return 'No results';
  if (hits >= asked) return `${asked} or more results`;
  return hits === 1 ? '1 result' : `${hits} results`;
}

export function initials(name: string): string {
  const words = name.trim().split(/[\s\-_]+/).filter(Boolean);
  const letters = words.length > 1 ? `${words[0]![0]}${words[1]![0]}` : name.trim().slice(0, 2);
  return letters.toUpperCase();
}

/** `agent`, `user` or `system`, from `agent:builder`, `user:acct_1`, `system`. */
export function actorKind(actor: string): 'agent' | 'bot' | 'user' | 'system' {
  if (actor.startsWith('agent:')) return 'agent';
  if (actor.startsWith('bot:')) return 'bot';
  if (actor.startsWith('user:')) return 'user';
  return 'system';
}

/** The folder a path is in; the empty string for one at the top. */
export function parentOf(path: string): string {
  const slash = path.lastIndexOf('/');
  return slash < 0 ? '' : path.slice(0, slash);
}

/**
 * Where a copy of a file goes: beside it, called what it is called and
 * `-copy`, with a number where that is taken too. `taken` are the paths
 * there are. What a file's name ends in is kept, so that a copy of a
 * document is a document.
 */
export function copyOf(path: string, taken: string[]): string {
  const slash = path.lastIndexOf('/') + 1;
  const [folder, name] = [path.slice(0, slash), path.slice(slash)];
  const dot = name.lastIndexOf('.');
  const [stem, ending] = dot > 0 ? [name.slice(0, dot), name.slice(dot)] : [name, ''];
  for (let nth = 1; ; nth += 1) {
    const copy = `${folder}${stem}-copy${nth > 1 ? `-${nth}` : ''}${ending}`;
    if (!taken.includes(copy)) return copy;
  }
}

/** A size in bytes, as people say it. */
export function fileSize(bytes: number): string {
  if (bytes < 1000) return `${bytes} B`;
  if (bytes < 1_000_000) return `${(bytes / 1000).toFixed(bytes < 10_000 ? 1 : 0)} kB`;
  return `${(bytes / 1_000_000).toFixed(bytes < 10_000_000 ? 1 : 0)} MB`;
}

export function classes(...names: (string | false | null | undefined)[]): string {
  return names.filter(Boolean).join(' ');
}

/** A step's id without its `for_each` index: `deploy[2]` is drawn on `deploy`. */
export function baseStep(step: string): string {
  const bracket = step.indexOf('[');
  return bracket < 0 ? step : step.slice(0, bracket);
}

export function show(value: unknown): string {
  if (value === null || value === undefined) return '';
  if (typeof value === 'string') return value;
  return JSON.stringify(value, null, 2);
}

/** Whether a JSON value is worth showing: not null, and not an empty container. */
export function hasContent(value: unknown): boolean {
  if (value === null || value === undefined || value === '') return false;
  if (Array.isArray(value)) return value.length > 0;
  if (typeof value === 'object') return Object.keys(value as object).length > 0;
  return true;
}

/** `list` with `item` moved to sit before `before`, or to the end when there is no such place. */
export function reorder<T>(list: T[], item: T, before: T | null): T[] {
  if (!list.includes(item) || item === before) return list;
  const rest = list.filter((entry) => entry !== item);
  const at = before === null ? -1 : rest.indexOf(before);
  if (at < 0) return [...rest, item];
  return [...rest.slice(0, at), item, ...rest.slice(at)];
}

/** How an agent stands: in the middle of a turn, ready to be asked, or not running at all. */
export type AgentState = 'working' | 'idle' | 'stopped';

/** What an agent is doing now, from whether it is in a turn and whether it has a slot that is active. */
export function agentState(working: boolean, slot: { active: boolean } | undefined): AgentState {
  if (!slot?.active) return 'stopped';
  return working ? 'working' : 'idle';
}

/**
 * Whether a file is a blueprint by what it is called: its name ends in one
 * of the endings a blueprint's file may have (`fleet.bp`, and `fleet.bp.yml`
 * for an editor's sake), with something before it. Never by `.yml` alone: a
 * project is full of YAML that is not a blueprint.
 */
export function isNamedBlueprint(path: string, endings: string[]): boolean {
  const name = path.slice(path.lastIndexOf('/') + 1);
  return endings.some((ending) => ending !== '' && name.length > ending.length + 1 && name.endsWith(`.${ending}`));
}
