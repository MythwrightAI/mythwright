// The daemon's API: one function per thing the client asks for, and the
// shapes of what comes back. Nothing here keeps state.

import type { Uses } from './uses.ts';
import type { Profile } from './profiles.ts';
import type { Carried, Field, Guarded, Row, Saved, Sits, View } from './table.ts';
import type { Area } from './holds.ts';
import type { Start } from './starts.ts';
import type { Seen } from './sees.ts';
import type { Turn } from './agenthistory.ts';
export type { Turn } from './agenthistory.ts';
export type { Start } from './starts.ts';
import type { Beside, Comparison, Counts, Moved, TextsSetting } from './comparing.ts';
export type { Beside, BesideRow, Comparison, Counts, Moved, Way } from './comparing.ts';
import type { Run } from './gave.ts';
import type { Within } from './blueprint/within.ts';
export type { Within, WithinKey, WithinWord } from './blueprint/within.ts';
export type { Run, Step } from './gave.ts';
import type { Diagnostic, Graph, GraphParam } from './graph/shapes.ts';
export type { Diagnostic, Graph, GraphEdge, GraphHook, GraphNode, GraphParam, GraphWatch, Pos } from './graph/shapes.ts';

export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
export type Fields = Record<string, Json>;

export interface Meta {
  name: string;
  id: string;
  cli: string;
  blueprint_ext: string;
  /** Every ending a blueprint's file may have, the usual first; a daemon from before says only the one. */
  blueprint_exts?: string[];
  version: string;
  /** The build of the client the daemon is serving now; it changes without the daemon restarting. */
  client_build: string | null;
  device: string;
  /** What this factory's profile is called, when it has one; a daemon from before profiles says nothing. */
  profile?: string | null;
  needs_setup: boolean;
  signed_in: boolean;
  /** What a new project may start with, in the order it is given; a daemon from before says nothing. */
  project_starts?: Start[];
  /** What can be held, by area, with what may be done in each; a daemon from before says nothing. */
  scopes?: Area[];
  /** What an agent holds where its slot says nothing else. */
  agent_scopes?: string[];
  /** What an agent the factory does not start is invited with where nothing is said. */
  external_scopes?: string[];
  /** When it began draining to stop and the latest it stops, where it was asked to (`ui/src/drained.ts`). */
  draining?: { since: number; until: number } | null;
  /** What every page says across its top, to whoever is signed in (`ui/src/across.ts`): nothing where it says nothing. */
  across?: unknown;
}
export interface Account {
  id: string;
  email: string;
  name: string;
  role: string;
  tier: string;
  created: number;
}
export interface Principal {
  kind: string;
  id: string;
  name: string;
  scopes: string[];
}
export interface Me {
  principal: Principal;
  actor: string;
  account: Account | null;
  tier: string;
  can_invite: boolean;
  /** Whether the account has a password to sign in with besides a link: an owner the factory made has none until they set one. */
  password?: boolean;
}
/** The factory's own bot in a channel, and the commands it takes: how each is written, and what it does. */
export interface Speaker {
  name: string;
  usual: string;
  commands: { written: string; does: string }[];
  /** The most it may ask where somebody narrowed it; nothing for what its commands reach (`reaches`). A daemon from before says neither. */
  holds?: string[] | null;
  reaches?: string[];
}

export interface Project {
  id: string;
  name: string;
  path: string;
  brownfield: boolean;
  /** Whether a file of its own may stand in for a blueprint that ships. One made of a folder that was already there is not, until the factory's owner says so. */
  trusted: boolean;
  created: number;
  /** Whose its folder is: one whoever added it said, or one the factory made for it since nobody said one. A daemon from before says nothing of it. */
  folder?: 'given' | 'own';
  /** Whether its folder is a git repository of its own, which has a Git page. A daemon from before says nothing of it. */
  repository?: boolean;
}
export interface Entry {
  path: string;
  name: string;
  dir: boolean;
  size: number;
  modified: number;
}
export interface Member {
  actor: string;
  name: string;
  /** Whether they manage the channel (`managed.ts`): said only where it is so. */
  manages?: boolean;
}
export interface Channel {
  id: string;
  name: string;
  project: string | null;
  topic: string;
  created: number;
  members: Member[];
  /** Kept to its members: read by whoever is in it and nobody else. Its `name` is then its own id, and it is called its `title`. */
  kept?: boolean;
  title?: string;
  /** Of a kept one: its agents are asked nowhere else. */
  alone?: boolean;
  /** Of a kept one: it is a conversation (`conversed.ts`), listed in its members' inboxes and not among the channels. */
  conversation?: boolean;
  /** When it was archived and by whom (`archived.ts`): read as before, and nothing said there. */
  archived?: { at: number; by: string; name: string };
}
/** Something attached to a message: a picture, a document, anything. */
export interface Attachment {
  id: string;
  name: string;
  /** What kind of thing it is, as a media type. */
  media: string;
  size: number;
  /** Where it is a file of a project posted as that file: which, and what it is now as the reader may read it (`posted.ts`). */
  file?: { project: string; path: string; state?: 'there' | 'gone' | 'kept' };
}
export interface Message {
  id: string;
  channel: string;
  author: string;
  author_name: string;
  body: string;
  reply_to: string | null;
  /** For an agent's message: the model that wrote it, when that is known. */
  model?: string | null;
  /** The message this one is said under, when it is part of a thread. */
  thread?: string | null;
  /** How many messages have been said under this one. */
  replies?: number;
  /** What is attached to it. */
  attachments?: Attachment[];
  /** For one a blueprint's step said: which, and in which run. The name on it is the bot's it was said as. */
  said_by?: { blueprint: string; instance: string; run: string; step: string; detail?: string; about_run?: string } | null;
  /** What its author marked it as: a plan, or something for agents to discuss (`kernel::discussed::MARKS`). */
  mark?: string | null;
  created: number;
}
export interface Item {
  id: string;
  project: string;
  number: number;
  title: string;
  body: string;
  status: string;
  assignee: string | null;
  labels: string[];
  /** The day it is wanted by, `2026-11-30`, where its board keeps one. */
  due?: string | null;
  /** How it came to be where it is, as its board wrote it with the move: where from, the kind of actor, what was said, how many moves. */
  moved?: string[];
  /** What else its board keeps of it: the fields its blueprint declared beyond a card's own, where the card has one. */
  more?: Record<string, unknown>;
  position: number;
  created_by: string;
  created: number;
  updated: number;
}
/** A line of a record's history: an event that happened to it, as it was put into words then. */
export interface HistoryLine {
  seq: number;
  ts: number;
  topic: string;
  /** Who did it, as the journal has them: `agent:builder`. */
  actor: string | null;
  /** Who did it, by name. */
  who: string;
  said: string;
  /** What the event said of itself: for a line that is something said on a card, the words and whose they are. */
  data?: Record<string, unknown> | null;
}
export interface Board {
  project: string;
  columns: string[];
  items: Item[];
}
/** Records a blueprint declared for a project: what they are made of and where they are kept. */
export interface Collection extends Sits, Guarded {
  project: string;
  name: string;
  fields: Field[];
  /** The ways it is laid out, each under a name: the blueprint's, and the ones people `saved`; and for each the fields it leaves out, where it says. */
  views: View[];
  /** `local`: this installation's database. `files`: one file a record, in the project. `service`: a store service a blueprint hosts, at `at`. `documents`: the Markdown files of the folder `at` of the project, as they are. */
  store: 'local' | 'files' | 'service' | 'documents' | 'mockups';
  /** For one kept by a store service: the address the service answers at. For a library: the folder its documents are in. */
  at?: string | null;
  /** The instance whose blueprint declared it. */
  owner: string | null;
}
export interface Slot {
  name: string;
  adapter: string;
  model: string | null;
  project: string | null;
  instructions: string;
  scopes: string[];
  permission_mode: string;
  /** Whether its program is started behind a wall, shown its work and little else of the device. */
  wall?: boolean;
  /** What else it is shown there, to read and to change. */
  reads?: string[];
  changes?: string[];
  /** What it may do with its project's repository there: `off`, `read`, `commit` or `write`. */
  git?: string;
  /** What its program sees of this device as walls are held while the daemon runs, in its words (`ui/src/sees.ts`). */
  sees?: Seen;
  /** What its model is called, as far as the daemon knows without asking its program. */
  model_name?: string | null;
  /** Whether its program may use the web unasked; a daemon from before says nothing. */
  web?: boolean;
  /** The key of what is running that brings it up and counts it (a fleet, by its row), where something does. */
  held_by?: string;
  active: boolean;
  created: number;
  updated: number;
}
/** Something an agent can run on: an installed extension, a configured program, or the built-in assistant. */
export interface Provider {
  name: string;
  title: string;
  description: string;
  kind: 'extension' | 'command' | 'builtin';
  /** Whether its program is installed on the daemon's device. */
  available: boolean;
  /** How its models are known: asked of its program, listed by its package, or not at all. */
  models: 'asked' | 'listed' | 'none';
  /** The permission modes an agent on it can be held to. */
  permissions?: string[];
}
/** How much an agent may do unasked: `read`, `edit` or `full`, and what each means. */
export interface PermissionMode {
  name: string;
  means: string;
}
/** An extension package, installed or on offer. */
/** A secret the factory keeps, by name, and what names it. What it is, nothing says. */
export interface Secret {
  name: string;
  /** When it was set, and nothing for one that is named somewhere and was never set. */
  set: number | null;
  set_by: string | null;
  kept: boolean;
  named: {
    agents: { name: string; project: string | null; variable: string; active: boolean }[];
    services: { name: string; instance: string; project: string | null; variable: string }[];
    instances: { id: string; blueprint: string; key: string | null; project: string | null }[];
  };
}

export interface Extension {
  name: string;
  title: string;
  description: string;
  /** The program it runs (none, for a package that names none), and whether that is installed on the daemon's device. */
  program: string;
  available: boolean;
  /** What the package says to someone who does not have its program. */
  install: string;
  /** Whether agents can run on it. */
  agent: boolean;
  /** The actions it brings, each a way its program is run, described as every action is. */
  actions: ActionSpec[];
  /** The blueprints it brings, by the names they are applied by. */
  blueprints: string[];
  /** Installed, and not the one of its name that came with this version; `differs` says where (`came.ts`). */
  behind?: boolean;
  differs?: { words: string };
}
export interface Model {
  id: string;
  label: string;
  note: string;
  /** The one the provider uses when none is named. */
  default: boolean;
}
/** A word that may be written, and what it means. */
export interface Choice {
  name: string;
  help: string;
}
/** What may be written after a field's colon. */
export type ValueKind =
  | { kind: 'text' | 'flag' | 'number' | 'duration' | 'expression' | 'action' | 'decision' | 'blueprint' | 'agent' | 'device' | 'topic' | 'steps' | 'capabilities' | 'data' | 'any' }
  | { kind: 'one_of'; of: Choice[] }
  | { kind: 'shape' | 'map_of' | 'list_of'; of: string }
  | { kind: 'arguments'; of: string | null };
export interface LanguageField {
  name: string;
  value: ValueKind;
  help: string;
}
/** A mapping with a closed set of fields: a step, a hook, the file itself. */
export interface LanguageShape {
  name: string;
  what: string;
  /** What a bare value in its place means, when one is allowed. */
  short: ValueKind | null;
  /** Fields that stand in for each other: one of each group, never two. */
  alternatives: string[][];
  fields: LanguageField[];
}
export interface ActionSpec {
  name: string;
  summary: string;
  /** `within`: what an argument says is written inside it; not there from a daemon older than this client. */
  args: { name: string; type: string; required: boolean; help: string; within?: Within }[];
  /** Takes arguments beyond those listed. */
  open: boolean;
  capability: string | null;
  outputs: [string, string][];
  idempotent: boolean;
}
/** One kind of event the daemon emits, and what it carries. */
export interface EventSpec {
  topic: string;
  summary: string;
  fields: { name: string; type: string; always: boolean; help: string }[];
  /** Carries more than is listed, under names of its emitter's own. */
  open: boolean;
  /** Kept, so that a handler can hear it. One that is not is only passed on. */
  kept: boolean;
}
/**
 * The blueprint language as the daemon describes it. The editor colours
 * and offers from this, and knows nothing about blueprints of its own.
 */
export interface BlueprintLanguage {
  /** Every mapping there is. The first is the file itself. */
  shapes: LanguageShape[];
  actions: ActionSpec[];
  /** The events a handler can hear. Not there from a daemon older than this client. */
  events?: EventSpec[];
  functions: { name: string; usage: string }[];
  /** The names an expression may start with, and what may follow each. */
  names: { name: string; meaning: string; members: Choice[] }[];
  /** What may follow `steps.<id>` and `self`. */
  step: Choice[];
  /** The words a failure hook may answer with. */
  decisions: string[];
}
/** A blueprint that can be called, with what it is started with and gives back. */
export interface Described {
  name: string;
  /** The file it is, within the project, when it is the project's own. One with none ships with the product, or came with an extension. */
  file: string | null;
  description: string | null;
  params: GraphParam[];
  outputs: string[];
}
export interface Checked {
  ok: boolean;
  diagnostics: Diagnostic[];
  name: string | null;
  graph: Graph | null;
  /** The text in plain words, where it checks (`summary.ts`): of the same text as the graph. */
  summary?: import('./summary.ts').Summary | null;
  revision: string;
}
/** The text an instance is behind, and how it would come to run it. */
export interface Behind {
  /** The revision of the text now there for its blueprint. */
  revision: string;
  /** What that text calls its own version, where it says. */
  version: string | null;
  /** `in_place`: where it is, with nothing run again, when what started it or it itself is applied again. `carried`: where it is, with what the text says under `on_update` done in place of its steps, when what started it or it itself is applied again. `again`: only by being applied again itself, which runs its steps again. `unfit`: not as the text stands, which does not check here. */
  takes: 'in_place' | 'carried' | 'again' | 'unfit';
  /** The same as a sentence, with what to do. */
  why: string;
  /** What moved between the text it runs and the one now there, a few words for each thing. */
  changed?: string[];
}
export interface Instance {
  id: string;
  blueprint: string;
  /** Which text it runs. */
  revision: string;
  /** What that text calls its own version, where it says. */
  version?: string | null;
  /** Said of one that is alive when the text now there for its blueprint is not the one it runs. */
  behind?: Behind | null;
  params: Fields;
  state: Fields;
  status: 'starting' | 'active' | 'done' | 'failed' | 'stopped';
  parent: string | null;
  key: string;
  target: string | null;
  project: string | null;
  environment?: string;
  /** What everything measured under it also carries, as the step that started it said with `labels:`. */
  labels: Record<string, string>;
  holds: string[];
  error: string | null;
  created: number;
  updated: number;
}
export interface InstanceDetail {
  instance: Instance;
  source: string;
  runs: Run[];
  children: Instance[];
  graph: Graph | null;
}
export interface Applied {
  instance: string;
  /** Which instance that is, as everything that names one says it: the blueprint it runs and its key. A daemon from before says neither. */
  blueprint?: string; key?: string;
  run: string | null;
  created: boolean;
  changed: boolean;
  /** For an instance that was there and now runs something else: how it came to it. Its steps ran again, it was carried over (`on_update`), or it took the text where it was and nothing ran. */
  took?: 'again' | 'carried' | 'in_place';
  /** How the run it started went, when it was waited for. */
  finished?: { status: string; error: string | null } | null;
  /** What became of the blueprints that ship among those it reaches, where they are other than it was applied with: left as they were, or taken. */
  reached?: { taken: Moved[]; left: Moved[] };
  /** What the check says of the text short of refusing it, said again by the apply, which went ahead: among them a handler that will hear nothing where the text was applied (`unheard`). */
  warnings?: Diagnostic[];
  /** What the daemon had to say of the apply beside its outcome: a file of the project's own passed over, or what the project lacks that this cannot work without (a fleet, and no board), with the command that gives it. */
  said?: string[];
}
/** One of the definitions an instance has been given: what it was applied with, by whom, and whether it held. */
export interface Version {
  /** Counted from 1, in the order they were applied. */
  version: number;
  revision: string;
  source: string;
  params: Fields;
  by: string;
  by_name: string;
  applied: number;
  /** `reverted`: the update failed and was undone, and the instance went back to the one before. */
  outcome: 'applied' | 'reverted';
}
export interface LogLine {
  line: number;
  ts: number;
  stream: string;
  text: string;
}
export interface Service {
  /** The project its instance was applied for, when it was for one. */
  project?: string | null;
  name: string;
  instance: string;
  status: 'starting' | 'healthy' | 'unhealthy' | 'exited' | 'stopped' | 'failed';
  pid: number;
  port: number;
  restarts: number;
  last_exit: number | null;
  reason: string | null;
  since: number;
  command: string;
}
export interface Router {
  project?: string | null;
  name: string;
  instance: string;
  host: string;
  listen: number;
  backends: { target: string; weight: number; connections: number; failures: number }[];
}
export interface Resources {
  cpus: number;
  load1: number | null;
  total_mem_mb: number;
  free_mem_mb: number;
  free_disk_mb: number | null;
  os: string;
  arch: string;
  host: string;
}
/** Another device of the fleet, as this one (its hub) knows it. */
export interface Device {
  name: string;
  connected: boolean;
  since: number | null;
  last_seen: number | null;
  info: {
    version: string;
    os: string;
    arch: string;
    cpus: number;
    load1: number | null;
    free_mem_mb: number;
    free_disk_mb: number | null;
    instances: number;
    services: { name: string; status: string; port: number }[];
  } | null;
  /** The short name of the key it proved when it was let in; none while it is only invited. */
  id: string | null;
  /** When its invitation stops working, while it has yet to use it. */
  invited: number | null;
}
/** This device's link to the hub of a fleet it has joined. */
export interface HubLink {
  url: string;
  /** The short name of the key the hub proves. */
  id: string;
  connected: boolean;
  since: number;
  hub: string | null;
  device: string | null;
  error: string | null;
  failures: number;
  retry_at: number | null;
}
export interface Fleet {
  device: { name: string; id: string; resources: Resources | null; address: string };
  instances: Instance[];
  services: Service[];
  routers: Router[];
  agents: Slot[];
  devices: Device[];
  hub: HubLink | null;
  /** The number of the last event there was when this was read. */
  last_seq: number;
  /** The place to follow events from after this reading: handed back as it is, never read. */
  cursor: string;
}
export interface Hit {
  reference: string;
  kind: 'file' | 'blueprint' | 'message' | 'card' | 'document' | 'record' | 'topic' | 'project' | 'channel' | 'person' | 'agent' | 'run';
  project: string | null;
  title: string;
  snippet: string;
  /** Where it is, in words, where that says more than its project: a row's table, a page's wiki, a document's library. */
  place?: string | null;
  /**
   * What opens it: a file's or a document's `path` (a document's with the `line` and the words of the heading nearest
   * what matched),
   * a message's `channel` and `id`, a card's `id`, a row's `id`, `at` and `table` (and `reply`, the number something
   * said on a topic was said at), a project's `id`, a channel's `channel`, somebody's `actor` and `name`, a blueprint
   * applied's `instance` and a run's `run`.
   */
  link: { path?: string; line?: number; heading?: string | null; channel?: string; id?: string; at?: string; table?: string; reply?: number; actor?: string; name?: string; instance?: string; run?: string };
}
export interface Progress {
  phase: 'idle' | 'scanning' | 'indexing';
  done: number;
  total: number;
  docs: number;
}
export interface Stats {
  count: number;
  errors: number;
  error_rate: number;
  avg: number;
  /** Every value added up. */
  sum: number;
  min: number;
  max: number;
  p50: number;
  p95: number;
}
export interface Series {
  name: string;
  /** What its values are of, where they are not a length of time: `usd`, `tokens`. */
  unit?: string;
  labels: Record<string, string>;
  total: Stats;
  points: [number, Stats][];
}
export interface Credential {
  id: string;
  kind: string;
  subject: string;
  name: string;
  scopes: string[];
  created: number;
  revoked: number | null;
  /** The key it was made with, which revokes it with itself. */
  made_with?: string | null;
}
export interface FleetEvent {
  seq: number;
  ts: number;
  topic: string;
  source?: { device?: string; instance?: string; run?: string; step?: string; actor?: string };
  data: any;
}

/** A refusal from the daemon, in its own words. */
export class ApiError extends Error {
  status: number;
  kind: string;
  constructor(status: number, kind: string, message: string) {
    super(message);
    this.status = status;
    this.kind = kind;
  }
}

/**
 * What the daemon's refusals mean for the page as a whole: the session is no
 * longer good, or the daemon does not have a route this client expects, which
 * happens when the client was updated and the daemon has not been.
 */
export const session = { onExpired: () => {}, onBehind: () => {} };

/**
 * The key this page was given when its person signed in, which goes with
 * everything it asks. A browser sends the session's cookie to every port of
 * the machine and with any link followed there, so the daemon takes the
 * cookie only together with this, which is kept where no other page on the
 * machine can read it: storage is a page's own by its port, and a cookie is
 * not.
 */
const PAGE_KEY = 'page-key';

function pageKey(): Record<string, string> {
  try {
    const key = typeof localStorage === 'undefined' ? null : localStorage.getItem(PAGE_KEY);
    return key ? { 'x-page-key': key } : {};
  } catch {
    return {};
  }
}

// What this page says in the address of what its browser fetches by itself
// (a picture, something attached, the stream of events), beside the cookie:
// such a request cannot carry the key, and the cookie, which a browser hands
// to whatever else listens on the machine, reads nothing alone. It is given
// with the key and is not the key.
const PAGE_SHOWS = 'page-shows';

function keepPageKey(key: string | null | undefined, shows?: string | null): void {
  try {
    if (typeof localStorage === 'undefined') return;
    if (key) localStorage.setItem(PAGE_KEY, key);
    else localStorage.removeItem(PAGE_KEY);
    if (key && shows) localStorage.setItem(PAGE_SHOWS, shows);
    else localStorage.removeItem(PAGE_SHOWS);
  } catch {
    /* storage that is refused: the page signs in again when it is opened again */
  }
}

function pageShows(): string | null {
  try {
    return typeof localStorage === 'undefined' ? null : localStorage.getItem(PAGE_SHOWS);
  } catch {
    return null;
  }
}

/** An address the browser fetches by itself, saying which page asks. */
export function shown(address: string): string {
  const shows = pageShows();
  if (!shows) return address;
  return `${address}${address.includes('?') ? '&' : '?'}by=${encodeURIComponent(shows)}`;
}

export async function call<T>(method: string, path: string, body?: unknown): Promise<T> {
  let response: Response;
  try {
    response = await fetch(`/api${path}`, {
      method,
      headers: { ...pageKey(), ...(body === undefined ? {} : { 'content-type': 'application/json' }) },
      body: body === undefined ? undefined : JSON.stringify(body),
      credentials: 'same-origin',
    });
  } catch {
    throw new ApiError(0, 'unavailable', 'The daemon is not answering.');
  }
  const text = await response.text();
  let parsed: any = null;
  try {
    parsed = text ? JSON.parse(text) : null;
  } catch {
    /* not JSON: an error page from something in between */
  }
  if (!response.ok) {
    if (response.status === 401 && !path.startsWith('/auth/')) session.onExpired();
    if (parsed?.error?.kind === 'no_such_route') session.onBehind();
    const message = parsed?.error?.message ?? parsed?.error ?? parsed?.message ?? `The request failed (${response.status}).`;
    throw new ApiError(response.status, parsed?.error?.kind ?? parsed?.kind ?? 'error', String(message));
  }
  return parsed as T;
}

/** Sends a file as it is to `path` (`/records/<id>/attachments`, a channel's), named by its name, saying what it is. */
export async function upload<T>(path: string, file: File): Promise<T> {
  let response: Response;
  try {
    response = await fetch(`/api${path}${query({ name: file.name || 'pasted' })}`, {
      method: 'POST',
      headers: { ...pageKey(), ...(file.type ? { 'content-type': file.type } : {}) },
      body: file,
      credentials: 'same-origin',
    });
  } catch {
    throw new ApiError(0, 'unavailable', 'The daemon is not answering.');
  }
  const parsed = await response.json().catch(() => null);
  if (!response.ok) {
    const large = response.status === 413 ? `${file.name} is too large to attach.` : null;
    throw new ApiError(response.status, parsed?.error?.kind ?? 'error', large ?? String(parsed?.error?.message ?? `It could not be attached (${response.status}).`));
  }
  return parsed as T;
}

function query(params: Record<string, string | number | boolean | string[] | undefined | null>): string {
  // A word that is several things is said once for each (`project=a&project=b`).
  const parts = Object.entries(params)
    .flatMap(([key, value]) => (Array.isArray(value) ? value : [value]).map((one) => [key, one] as const))
    .filter(([, value]) => value !== undefined && value !== null && value !== '')
    .map(([key, value]) => `${key}=${encodeURIComponent(String(value))}`);
  return parts.length ? `?${parts.join('&')}` : '';
}

const id = encodeURIComponent;

/** How many a search is asked for. As many as that found may not be all there are. */
export const SEARCH_LIMIT = 30;

export const api = {
  meta: () => call<Meta>('GET', '/meta'),
  /** The factories on this device that have a profile, and where each can be opened from this page. */
  profiles: () => call<{ profiles: Profile[] }>('GET', '/profiles').then((r) => r.profiles),
  me: () => call<Me>('GET', '/me'),
  // Signed in as a page: the session is the browser's cookie, and the key
  // that goes with it is this page's to keep.
  register: (body: { email: string; name: string; password: string; invite?: string }) =>
    call<{ account: Account; page?: string; shows?: string }>('POST', '/auth/register', { ...body, page: true }).then((signed) => {
      keepPageKey(signed.page, signed.shows);
      return signed;
    }),
  login: (body: { email: string; password: string }) =>
    call<{ account: Account; page?: string; shows?: string }>('POST', '/auth/login', { ...body, page: true }).then((signed) => {
      keepPageKey(signed.page, signed.shows);
      return signed;
    }),
  /** A link that signs somebody in once (`mw open`), traded for this page's session. */
  link: (once: string) =>
    call<{ account: Account; page?: string; shows?: string }>('POST', '/auth/link', { once, page: true }).then((signed) => {
      keepPageKey(signed.page, signed.shows);
      return signed;
    }),
  /** For a page that signed in before it was given what it shows with its key: asks for it, once. */
  learnShows: async (): Promise<void> => {
    const key = pageKey()['x-page-key'];
    if (!key || pageShows()) return;
    const answer = await call<{ shows: string }>('GET', '/auth/shows').catch(() => null);
    if (answer?.shows) keepPageKey(key, answer.shows);
  },
  logout: () => call<unknown>('POST', '/auth/logout', {}).finally(() => keepPageKey(null)),
  /** Ends every session of whoever asks, wherever each was begun: how many, and how many of their keys go on. */
  logoutEverywhere: () => call<{ sessions: number; keys: number }>('POST', '/auth/logout', { everywhere: true }).finally(() => keepPageKey(null)),

  projects: () => call<{ projects: Project[] }>('GET', '/projects').then((r) => r.projects),
  /** The factory's own bot: what it is called, and what it takes when it is written to. */
  speaker: () => call<Speaker>('GET', '/system/speaker'),
  /** Adds a project, with what it is to start with by name, each with what its blueprint is handed where it says, or a row of one's own (`true`: whatever the factory's default starts with unless said; `{ setup, rows }`: of a setup it keeps), each applied as whoever is signed in; what it could not be given is said in `set_up.left`. */
  createProject: (name: string, setUp: unknown[] | { setup: string; rows: unknown[] } | boolean, workspace?: string, path?: string) =>
    call<Project & { set_up?: { left?: string[] } & Record<string, string | string[] | undefined> }>('POST', '/projects', { name, set_up: setUp, workspace, path }),
  tree: (project: string) => call<{ entries: Entry[] }>('GET', `/projects/${id(project)}/tree`).then((r) => r.entries),
  /** Has the daemon look at a project's folder for what was changed outside it. */
  scanProject: (project: string) => call<{ changes: unknown[] }>('POST', `/projects/${id(project)}/scan`, {}),
  readFile: (project: string, path: string) =>
    call<{ content: string }>('GET', `/projects/${id(project)}/file${query({ path })}`).then((r) => r.content),
  /** Whether a path is a file of a project the reader may read, as its entry, with nothing of it read: refused as reading it would be. */
  fileThere: (project: string, path: string) => call<Entry>('GET', `/projects/${id(project)}/file${query({ path, head: true })}`),
  writeFile: (project: string, path: string, content: string) =>
    call<unknown>('PUT', `/projects/${id(project)}/file`, { path, content }),
  /** A file that is not there yet; refused where anything is there under its name. */
  newFile: (project: string, path: string, content: string) => call<Entry>('PUT', `/projects/${id(project)}/file`, { path, content, new: true }),
  /** An empty folder, with the folders it is in; refused where anything is there under its name. */
  makeFolder: (project: string, path: string) => call<Entry>('POST', `/projects/${id(project)}/folders`, { path }),
  removeFile: (project: string, path: string) =>
    call<unknown>('DELETE', `/projects/${id(project)}/file${query({ path })}`),
  initFleet: (project: string) => call<unknown>('POST', `/projects/${id(project)}/fleet`, {}),

  /** The language itself, as an editor needs it. It does not change while the daemon runs. */
  language: () => call<BlueprintLanguage>('GET', '/blueprints/language'),
  /** The blueprints that can be called from a project: its own, the ones installed extensions bring, and the ones that ship. */
  blueprints: (project?: string | null) =>
    call<{ blueprints: Described[] }>('GET', `/blueprints${query({ project: project ?? undefined })}`).then((answer) => answer.blueprints),
  check: (source: string, project?: string | null, is?: { path?: string; ships?: boolean }) =>
    call<Checked>('POST', '/blueprints/check', { source, project: project ?? undefined, ...is }),
  /** A blueprint's text laid out the one way, as its project says; `null` for one that does not read. */
  layOut: (source: string, project?: string | null, path?: string) =>
    call<{ formatted: string | null }>('POST', '/blueprints/format', { source, project: project ?? undefined, path }).then((r) => r.formatted),
  apply: (body: { project?: string | null; environment?: string; path?: string; source?: string; blueprint?: string; key?: string; params?: Fields; labels?: Record<string, string>; force?: boolean; take?: boolean; wait?: boolean }) =>
    call<Applied>('POST', '/apply', body),
  instances: (params: { live?: boolean; project?: string | string[]; limit?: number } = {}) =>
    call<{ instances: Instance[] }>('GET', `/instances${query(params)}`).then((r) => r.instances),
  instance: (instance: string) => call<InstanceDetail>('GET', `/instances/${id(instance)}`),
  /** Every definition an instance has been given, the latest first. */
  versions: (instance: string) => call<{ versions: Version[] }>('GET', `/instances/${id(instance)}/versions`).then((answer) => answer.versions),
  stopInstance: (instance: string) => call<unknown>('POST', `/instances/${id(instance)}/stop`, {}),
  /** Applies an instance again as it is: the text now there, with what it was started with. How it came to that text is `took` in the answer: it took it where it is and nothing ran, it was carried over (the text says `on_update`), or its steps ran again. */
  applyAgain: (instance: string) => call<Applied>('POST', `/instances/${id(instance)}/apply`, { wait: true }),
  /** How what is running comes by new texts, and how a taken one lands; changing it is the owner's. */
  texts: () => call<TextsSetting>('GET', '/system/texts'),
  setTexts: (asked: { come?: string; land?: string }) => call<TextsSetting>('POST', '/system/texts', asked),
  /** The comparisons there are, the oldest first; of one project where one is given. */
  comparisons: (project?: string) =>
    call<{ comparisons: Comparison[] }>('GET', `/comparisons${query({ project })}`).then((answer) => answer.comparisons),
  /** One comparison, and every measurement taken under it by the way it was taken under. */
  comparison: (name: string) => call<{ comparison: Comparison; measured: Series[]; beside: Beside }>('GET', `/comparisons/${id(name)}`),
  compare: (asked: { name: string; project?: string | null; of: string; per?: string; also?: string[]; counts?: Counts; ways: { name: string; share?: number; blueprint?: string; source?: string; with?: Fields; prompts?: Record<string, string> }[] }) =>
    call<Comparison>('POST', '/comparisons', asked),
  /** Ends one; `goes` is the way everything goes from then on, as it is written where none. */
  endComparison: (name: string, goes?: string) => call<Comparison>('POST', `/comparisons/${id(name)}/end`, { goes: goes ?? null }),
  runs: (params: { instance?: string; project?: string | string[]; limit?: number } = {}) =>
    call<{ runs: Run[] }>('GET', `/runs${query(params)}`).then((r) => r.runs),
  run: (run: string) => call<Run>('GET', `/runs/${id(run)}`),
  stepLog: (run: string, step: string) =>
    call<{ lines: LogLine[] }>('GET', `/runs/${id(run)}/steps/${id(step)}/log`).then((r) => r.lines),

  channels: () => call<{ channels: Channel[] }>('GET', '/channels').then((r) => r.channels),
  /** Makes a channel: a project's, or the own of the workspace said; with neither, the usual workspace's own. */
  createChannel: (name: string, project?: string, workspace?: string, kept = false, alone = false) =>
    call<Channel>('POST', '/channels', { name, project, workspace, kept, alone }),
  /** The door of a channel kept to its members: an open one kept, a kept one opened again, somebody leaving or an agent taken out, and whether its agents are asked nowhere else. */
  keepChannel: (channel: string) => call<Channel>('POST', `/channels/${id(channel)}/keep`, {}),
  openChannel: (channel: string, name?: string) => call<Channel>('POST', `/channels/${id(channel)}/open`, { name }),
  leaveChannel: (channel: string, agent?: string) => call<{ channel: string; left: string }>('POST', `/channels/${id(channel)}/leave`, { agent }),
  aloneChannel: (channel: string, alone: boolean) => call<Channel>('PUT', `/channels/${id(channel)}/alone`, { alone }),
  /** Takes a channel away with what was said in it; `force`, although something that is running says it posts there. */
  removeChannel: (id: string, force = false) => call<{ id: string; name: string; trash?: string | null; kept?: number | null }>('DELETE', `/channels/${encodeURIComponent(id)}${force ? '?force=true' : ''}`),
  messages: (channel: string) =>
    call<{ messages: Message[] }>('GET', `/channels/${id(channel)}/messages${query({ limit: 200 })}`).then((r) => r.messages),
  /** A message and what has been said under it, oldest first. */
  thread: (channel: string, message: string) =>
    call<{ messages: Message[] }>('GET', `/channels/${id(channel)}/messages${query({ thread: message })}`).then((r) => r.messages),
  /** Says something in a channel, or with `thread`, under one of its messages; with what was put there to go with it. */
  post: (channel: string, body: string, thread?: string, attachments: string[] = [], mark?: string) =>
    call<Message>('POST', `/channels/${id(channel)}/messages`, { body, thread, attachments, mark }),
  /** Puts a file where the next message in a channel can take it with it. The file is sent as it is. */
  attach: (channel: string, file: File) => upload<Attachment>(`/channels/${id(channel)}/attachments`, file),
  /** An attachment that is a page, as it is to be shown: its own HTML with a start put ahead of it. */
  view: async (attachment: string): Promise<string> => {
    const response = await fetch(`/api/attachments/${id(attachment)}/view`, { credentials: 'same-origin', headers: pageKey() }).catch(() => null);
    if (!response?.ok) throw new ApiError(response?.status ?? 0, 'error', 'This page could not be read.');
    return response.text();
  },
  /** Where the file of an attachment is fetched from. */
  attachmentUrl: (attachment: string) => shown(`/api/attachments/${id(attachment)}`),
  /** The people with an account on this installation. */
  people: () => call<{ people: Member[] }>('GET', '/people').then((answer) => answer.people),
  /** Adds a person who has an account here to a channel. */
  addPerson: (channel: string, actor: string) => call<Channel>('POST', `/channels/${id(channel)}/members`, { actor }),
  bringIn: (channel: string, body: { name: string; adapter?: string; model?: string; instructions?: string; permission_mode?: string; scopes?: string[] }) =>
    call<{ agent: Slot; channel: Channel }>('POST', `/channels/${id(channel)}/agents`, body),

  board: (project: string) => call<Board>('GET', `/board/${id(project)}`),
  /** A board's columns arranged, left to right: kept as the board's own through its blueprint applied again. */
  boardColumns: (project: string, columns: string[]) => call<{ columns: string[] }>('POST', `/board/${id(project)}/columns`, { columns }),
  /**
   * A project's board and where its cards are kept, or `null` when no
   * blueprint has made it one. What the project has is asked first, so that
   * having no board, which is how every project starts, is not an error.
   */
  boardIfAny: async (project: string): Promise<{ board: Board; store: Collection['store']; table: Collection } | null> => {
    const declared = (await api.collections(project)).find((one) => one.name === 'board');
    return declared ? { board: await api.board(project), store: declared.store, table: declared } : null;
  },
  /** What blueprints have declared for a project. */
  collections: (project: string) =>
    call<{ collections: Collection[] }>('GET', `/collections${query({ project })}`).then((answer) => answer.collections),
  /** The records of one collection, in the order they were made: what a field that names one is chosen from. */
  records: (project: string, collection: string) =>
    call<{ records: Row[] }>('GET', `/collections/${id(project)}/${id(collection)}/records`).then((answer) => answer.records),
  /** What has happened to a record, oldest first. It is there whether or not the record still is. */
  history: (record: string) => call<{ history: HistoryLine[] }>('GET', `/records/${id(record)}/history`).then((answer) => answer.history),
  /** Saves a view of a collection: a new one, or the one of that name (or of the name it had, `was`) as it now is. */
  saveView: (project: string, collection: string, view: Saved & Partial<Carried>) =>
    call<Collection>('POST', `/collections/${id(project)}/${id(collection)}/views`, view),
  removeView: (project: string, collection: string, view: string) =>
    call<Collection>('DELETE', `/collections/${id(project)}/${id(collection)}/views/${id(view)}`),
  addItem: (project: string, body: { title: string; body?: string; status?: string }) =>
    call<Item>('POST', `/board/${id(project)}/items`, body),
  /** `note` is what is said as a card is moved: why one in review is sent back. It goes with the move and into the card's history. */
  /** `fields` is what else its board keeps of a card, each by its name; nothing after a name clears it. */
  changeItem: (
    item: string,
    body: Partial<Pick<Item, 'title' | 'body' | 'status' | 'labels'>> & { assignee?: string | null; due?: string | null; before?: string; note?: string; fields?: Record<string, unknown> },
  ) => call<Item>('PATCH', `/board/items/${id(item)}`, body),
  /** One record as it is kept, a card included: what it holds, each field by its name. */
  record: (record: string) => call<Row>('GET', `/records/${id(record)}`),
  /** Adds a record to a collection that is not a board: a document to a library, say. */
  addRecord: (project: string, collection: string, fields: Record<string, unknown>) =>
    call<Row>('POST', `/collections/${id(project)}/${id(collection)}/records`, { fields }),
  /** Changes what a record that is not a card holds, each field by its name. */
  setRecord: (record: string, fields: Record<string, unknown>) => call<Row>('PATCH', `/records/${id(record)}`, { fields }),
  /** Takes a record that is not a card away. */
  removeRecord: (record: string) => call<unknown>('DELETE', `/records/${id(record)}`),
  /**
   * A way in to a mockup's own files, for showing it in a frame: an address that lets its holder read that one
   * folder for a while and is no credential. `address` is where the mockup is opened outside the client.
   */
  way: (mockup: string) => call<{ path: string; address: string; expires: number }>('POST', '/mockups/way', { id: mockup }),
  /** Says something on a record, a card included. It is kept in the record's history; what follows from it is its blueprint's to say. */
  /** `open` marks what is said public; without it, it is private. `on` is the few words of a document it is about (`anchors.ts`). */
  say: (record: string, body: string, open = false, on?: object) =>
    call<Row>('POST', `/records/${id(record)}/comments`, { body, ...(open ? { public: true } : {}), ...(on ? { anchor: on } : {}) }),
  removeItem: (item: string) => call<unknown>('DELETE', `/board/items/${id(item)}`),

  agents: () =>
    call<{ agents: Slot[]; adapters: string[]; default_adapter: string; providers: Provider[]; permissions?: PermissionMode[] }>('GET', '/agents'),
  /** Says what an agent holds, whole, and nothing else of it: one that is stopped stays stopped. */
  giveAgent: (name: string, scopes: string[]) => call<Slot>('PATCH', `/agents/${id(name)}`, { scopes }),
  /** A document as the page it would be published as. Its text is sent, so the preview is of what is in the editor, saved or not. */
  pagePreview: (source: string, name: string) =>
    call<{ html: string; title: string | null }>('POST', '/pages/preview', { source, name }).then((answer) => answer.html),
  /** What each project uses of the blueprints there are: those applied there that are alive, and its own files. */
  blueprintsUsed: () => call<{ projects: Record<string, Uses> }>('GET', '/blueprints/used').then((answer) => answer.projects),
  /** The text of a blueprint that is not a file in a project: one that ships with the product. */
  blueprintSource: (name: string) =>
    call<{ name: string; source: string }>('GET', `/blueprints/source${query({ name })}`).then((answer) => answer.source),
  /** The factory's secrets by name, with who set each and what names it: the owner's to ask. No route gives a value out. */
  secrets: () => call<{ secrets: Secret[] }>('GET', '/secrets?named=true').then((answer) => answer.secrets),
  /** Sets a secret, or sets it again over what was there. */
  setSecret: (name: string, value: string) => call<{ name: string }>('PUT', `/secrets/${encodeURIComponent(name)}`, { value }),
  removeSecret: (name: string) => call<{ ok: boolean }>('DELETE', `/secrets/${encodeURIComponent(name)}`),
  /** Installed packages, and those that came with the product and are not installed. */
  extensions: () => call<{ extensions: Extension[]; offered: Extension[] }>('GET', '/extensions'),
  /** Installs one that came with the product, by name, or any other from its manifest's text. */
  installExtension: (what: { name: string } | { source: string }) => call<Extension>('POST', '/extensions', what),
  removeExtension: (name: string) => call<{ ok: boolean }>('DELETE', `/extensions/${encodeURIComponent(name)}`),
  updateExtension: (name: string) => call<Extension & { updated: boolean }>('POST', `/extensions/${encodeURIComponent(name)}/update`, {}),
  /** The models a provider offers. Asking its program can take a few seconds the first time. */
  models: (provider: string, fresh = false) =>
    // Asking the program again changes what the daemon remembers, so it is not a read.
    call<{ models: Model[] }>(fresh ? 'POST' : 'GET', `/providers/${encodeURIComponent(provider)}/models`).then((answer) => answer.models),
  stopAgent: (name: string) => call<unknown>('DELETE', `/agents/${id(name)}`),
  turns: (name: string) => call<{ turns: Turn[] }>('GET', `/agents/${id(name)}/turns`).then((r) => r.turns),

  fleet: () => call<Fleet>('GET', '/fleet'),
  serviceLog: (name: string) =>
    call<{ lines: LogLine[] }>('GET', `/services/${id(name)}/log${query({ tail: 300 })}`).then((r) => r.lines),
  recentEvents: (params: { topics?: string; instance?: string; run?: string; limit?: number }) =>
    call<{ events: FleetEvent[]; last_seq: number; cursor: string }>('GET', `/events/recent${query(params)}`),

  /** What matches some words: everywhere, or with `project` only in that project. */
  search: (q: string, project?: string, kind?: Hit['kind']) =>
    call<{ hits: Hit[]; index: Progress }>('GET', `/search${query({ q, kind, project, limit: SEARCH_LIMIT })}`),
  searchStatus: () => call<Progress>('GET', '/search/status'),
  reindex: () => call<unknown>('POST', '/search/reindex', {}),
  metrics: (params: { name: string; since?: string; by?: string; step?: string; labels?: string }) =>
    call<{ series: Series[] }>('GET', `/metrics${query(params)}`).then((r) => r.series),
  /** The labels blueprints gave to what was measured, each with its values: what there is to compare by. */
  metricLabels: (params: { since?: string; labels?: string }) =>
    call<{ labels: Record<string, string[]> }>('GET', `/metrics/labels${query(params)}`).then((r) => r.labels),

  /** Puts an example that came with the program among a project's own blueprints; the same asked twice. */
  addExample: (project: string, example: string) => call<unknown>('POST', `/projects/${id(project)}/examples`, { example }),
  devices: () => call<{ this: string; id: string; devices: Device[]; hub: HubLink | null }>('GET', '/devices'),
  inviteDevice: (name: string, at?: string) =>
    call<{ name: string; invitation: string; expires: number; join: string; note: string | null }>('POST', '/devices', at ? { name, at } : { name }),
  removeDevice: (name: string) => call<unknown>('DELETE', `/devices/${id(name)}`),
  joinFleet: (invitation: string) => call<{ hub: HubLink }>('POST', '/gateway/join', { invitation }),
  leaveFleet: () => call<unknown>('POST', '/gateway/leave', {}),

  accounts: () => call<{ accounts: Account[]; tier: string }>('GET', '/accounts'),
  invite: (email?: string) => call<{ code: string }>('POST', '/invites', { email }),
  setTier: (tier: string) => call<{ tier: string }>('POST', '/tier', { tier }),
  credentials: () => call<{ credentials: Credential[] }>('GET', '/credentials').then((r) => r.credentials),
  /** A key that holds what whoever asks does: no scopes are named, so none is left out. */
  issueKey: (name: string) => call<{ token: string; name: string; scopes: string[] }>('POST', '/credentials', { name }),
};
