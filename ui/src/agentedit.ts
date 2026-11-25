// An agent's whole setup, as Edit… on the Agents tab opens it in the
// drawer (`views/agentedit.tsx`): which kind of setup an agent has, what
// its form starts with, what each part of it is closed by for whoever
// reads it, and what a save sends. It holds nothing of the page, so that
// its tests can read it.
//
// The daemon holds every rule (`Agents::ensure`, `byhand`): what is
// closed here is only what it would refuse as things stand, said before
// anybody presses Save.

import type { Fields } from './api.ts';
import { covers } from './holds.ts';

/** What an agent's settings are kept by, and so where they are changed. */
export type Setup =
  /** Added by hand: its own `std/agent`, applied again with the form's params. */
  | { kind: 'hand' }
  /** One of a fleet's: its row of a table of the project, which the fleet follows. */
  | { kind: 'row'; table: string; record: string }
  /** Brought up by something running that is no row of a table: changed where that is applied. */
  | { kind: 'brought'; by: string }
  /** One this factory does not start: what it may ask, and its project. */
  | { kind: 'outside' };

/** The parts of a slot a setup is read from. */
export interface SetupSlot {
  name: string;
  adapter: string;
  model: string | null;
  project: string | null;
  instructions: string;
  scopes: string[];
  permission_mode: string;
  wall?: boolean;
  reads?: string[];
  changes?: string[];
  git?: string;
  web?: boolean;
  outlives?: boolean;
  signins?: string[];
  secrets?: Record<string, string>;
  environment?: string;
  held_by?: string;
  active: boolean;
}

/**
 * Which setup `slot` has: one from outside, one something running holds
 * (`held_by`, the key of what counts it), which is a row of one of its
 * project's `tables` where that key ends in the row's id
 * (`role-shop-agents-2`), and otherwise one added by hand.
 */
export function setupOf(slot: SetupSlot, tables: readonly string[], external: boolean): Setup {
  if (external) return { kind: 'outside' };
  const by = slot.held_by;
  if (!by) return { kind: 'hand' };
  const project = slot.project;
  if (project) {
    for (const table of tables) {
      const found = new RegExp(`(?:^|-)(${escaped(project)}-${escaped(table)}-\\d+)$`).exec(by);
      if (found) return { kind: 'row', table, record: found[1]! };
    }
  }
  return { kind: 'brought', by };
}

const escaped = (text: string): string => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/** What the form of an agent added by hand holds, as it is written. */
export interface Written {
  adapter: string;
  model: string;
  /** How much it may do unasked: `read`, `edit` or `full`. */
  permission: string;
  instructions: string;
  wall: boolean;
  /** What else it may read and change behind its wall, a line each. */
  reads: string;
  changes: string;
  git: string;
  web: boolean;
  outlives: boolean;
  /** What it draws its sign-in from, a line each, in order. */
  signins: string;
  /** The keys it is handed, `VARIABLE=name` a line each. */
  secrets: string;
  holds: string[];
  project: string;
  environment: string;
  /** What a project's own fork of `std/agent` takes beside these, by its param, as written. */
  more: Record<string, string>;
}

/** The form as the slot stands, with what a fork of its text takes beside (`more`). */
export function writtenOf(slot: SetupSlot, more: Record<string, string> = {}): Written {
  return {
    adapter: slot.adapter,
    model: slot.model ?? '',
    permission: slot.permission_mode,
    instructions: slot.instructions,
    wall: slot.wall ?? true,
    reads: (slot.reads ?? []).join('\n'),
    changes: (slot.changes ?? []).join('\n'),
    git: slot.git || 'read',
    web: slot.web ?? false,
    outlives: slot.outlives ?? false,
    signins: (slot.signins ?? []).join('\n'),
    secrets: Object.entries(slot.secrets ?? {})
      .map(([variable, name]) => `${variable}=${name}`)
      .join('\n'),
    holds: [...slot.scopes],
    project: slot.project ?? '',
    environment: slot.environment ?? '',
    more,
  };
}

/** The lines of a box that holds one thing a line, trimmed, with the empty ones left out. */
export function linesOf(text: string): string[] {
  return text
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line !== '');
}

/** The params of `std/agent` its form says by name: what a fork takes beside these is `more`. */
export const OWN_PARAMS = ['name', 'role', 'rank', 'instructions', 'adapter', 'model', 'permission', 'holds', 'wall', 'reads', 'changes', 'git', 'secrets', 'signins', 'outlives', 'web', 'joins', 'leave'];

/** What a save sends to `POST /api/agents`: the name, where it works, and everything else as the text's params. */
export function bodyOf(name: string, written: Written, more: Fields = {}): { name: string; project?: string; environment?: string; params: Fields } {
  const on = (yes: boolean) => (yes ? 'on' : 'off');
  const params: Fields = {
    ...more,
    instructions: written.instructions,
    adapter: written.adapter,
    model: written.model,
    permission: written.permission,
    holds: written.holds,
    wall: on(written.wall),
    reads: linesOf(written.reads),
    changes: linesOf(written.changes),
    git: written.git,
    web: on(written.web),
    outlives: on(written.outlives),
    signins: linesOf(written.signins),
    secrets: linesOf(written.secrets),
  };
  return { name, project: written.project || undefined, environment: written.environment || undefined, params };
}

/** Whether two forms say the same, what is laid out another way in a box of lines included. */
export function sameWritten(one: Written, other: Written): boolean {
  const said = (written: Written) =>
    JSON.stringify({
      ...written,
      reads: linesOf(written.reads),
      changes: linesOf(written.changes),
      signins: linesOf(written.signins),
      secrets: linesOf(written.secrets),
      holds: [...written.holds].sort(),
    });
  return said(one) === said(other);
}

/** Who is reading: their kind, who they are and what they hold. */
export interface Reader {
  kind: string;
  id: string;
  scopes: string[];
}

/** Why each part of an agent's form is closed to whoever reads it, or nothing where it is open. */
export interface Closed {
  /** The whole of it: nothing of it is theirs to change. */
  all: string | null;
  /** Letting it do anything unasked (`full`). */
  full: string | null;
  /** Showing it more of the device: no wall, more to read or change, the web, going on without the daemon. Turning these off is anybody's who may change it. */
  device: string | null;
  /** What it signs in with and the keys it is handed. */
  secrets: string | null;
}

const takes = (scope: string, what: string) => `${what} takes ${scope}, which you do not hold.`;

/**
 * What of an agent's setup is closed to `reader`, by the rules the daemon
 * holds: an agent never changes its own slot; changing one takes
 * `agents:write`, and applying `std/agent` for one added by hand
 * `blueprints:run`; nobody gives what they do not hold, what it holds
 * already included; one that is handed secrets, runs freely or sees more
 * than its work is changed in anything only by whoever could have set it
 * so; and turning on what shows it more takes `shell`.
 */
export function closedOf(slot: SetupSlot, setup: Setup, reader: Reader): Closed {
  const mine = reader.scopes;
  const has = (scope: string) => covers(mine, scope);
  const all = (() => {
    if (reader.kind === 'agent' && reader.id === slot.name)
      return 'An agent never changes its own slot: what it may do is set for it by a person, or by an agent that holds what it gives.';
    if (setup.kind === 'brought') return `It is brought up by ${setup.by}, and is set up there: change it where that is applied.`;
    if (!has('agents:write')) return takes('agents:write', 'Changing an agent');
    if (setup.kind === 'hand' && !has('blueprints:run')) return takes('blueprints:run', 'An agent added by hand is changed by applying std/agent again as you, which');
    const beyond = slot.scopes.filter((scope) => !has(scope));
    if (beyond.length > 0) return `It holds what you do not (${beyond.join(', ')}), and nobody gives what they do not hold: whoever holds that changes it.`;
    if (setup.kind === 'outside') return null;
    if ((slot.signins?.length || Object.keys(slot.secrets ?? {}).length) && !has('secrets:use'))
      return takes('secrets:use', 'It is handed secrets, so changing anything of it');
    if (slot.permission_mode === 'full' && !has('shell')) return takes('shell', 'It may do anything unasked, so changing anything of it');
    if (slot.wall === false && !has('shell')) return takes('shell', 'It works with no wall, so changing anything of it');
    if ((slot.reads?.length || slot.changes?.length) && !has('shell')) return takes('shell', 'It is shown more of this device than its work, so changing anything of it');
    return null;
  })();
  return {
    all,
    full: has('shell') ? null : takes('shell', 'Letting it do anything unasked'),
    device: has('shell') ? null : takes('shell', 'Showing its program more of this device'),
    secrets: has('secrets:use') ? null : takes('secrets:use', 'Handing it a sign-in or a key'),
  };
}

/** What is said over the form of an agent that is stopped, where saving it brings it in. */
export function stoppedSaid(slot: { active: boolean }): string | null {
  return slot.active ? null : 'It is stopped. Saved, it is brought in again, set up as this says.';
}

/** What a save of one from outside sends: what changed of what it holds and its project, and nothing where nothing did. */
export function outsideChange(slot: { scopes: string[]; project: string | null }, holds: string[], project: string): { scopes?: string[]; project?: string | null } | null {
  const change: { scopes?: string[]; project?: string | null } = {};
  if ([...holds].sort().join(' ') !== [...slot.scopes].sort().join(' ')) change.scopes = holds;
  if ((slot.project ?? '') !== project) change.project = project || null;
  return Object.keys(change).length > 0 ? change : null;
}
