// The side panel's workspaces: the panel is of one at a time, and the
// channels in no group of a person's own are gathered by what they
// belong to.
//
// A workspace is the factory's (`workspaces.ts`): it is the same for
// everybody, and is changed by whoever may add a project. A factory
// always has one, every project is in one and every channel is a
// project's or a workspace's own, so nothing here offers a project no
// workspace or a channel that belongs to nothing: each is put in
// another, and a workspace that holds something is taken away with
// where that goes. Which one the panel is
// of is the reader's own, chosen at the panel's foot (`chosen.tsx`): its
// projects are listed as projects always were, each directly in the
// list, and no row stands over them for the workspace. What a person
// arranged for themselves is drawn as it was: their groups, pins and
// folds are theirs and stay in the browser (`grouped.tsx`), and only the
// channels they put in no group are gathered here.

import { effect, signal } from '@preact/signals';
import type { ComponentChildren } from 'preact';
import { useState } from 'preact/hooks';
import { ApiError } from '../api.ts';
import type { Channel, Project } from '../api.ts';
import { held, hold } from '../chosen.ts';
import { hashed } from '../kept.ts';
import { covers } from '../holds.ts';
import { pinning } from '../latelykept.ts';
import { me, projects, toast, turn, turned, unread } from '../store.ts';
import { Modal, Twist, useFocus } from '../ui.tsx';
import { forgetWorkspaces, readWorkspaces, spaces, workspaces } from '../spaces.ts';
import { gathered, listed, removeClosed, spaceOf, usualOf } from '../workspaces.ts';
import type { Workspace } from '../workspaces.ts';
import { channelPlaces, channelQuestion, projectPlaces, projectQuestion } from '../moveto.ts';
import type { Place, Question } from '../moveto.ts';
import { More, offer } from './grouped.tsx';
import type { Offered, Said } from './grouped.tsx';
import { signs } from './signs.tsx';
import { newTabLines } from './newtab.tsx';
import { Turning } from './turning.tsx';
import { CHANNELS } from '../folds.ts';
import { TrashProject } from './trash.tsx';
import { forgetLine, projectLines } from './outline.tsx';
import { holds as heldBy, ofChannels, shows } from '../panelsought.ts';
import type { Found } from '../panelsought.ts';

// Read once it is known who is signed in, and let go of when nobody is.
effect(() => {
  if (me.value) void readWorkspaces();
  else forgetWorkspaces();
});

type Asked =
  // A new one: for a project to go straight into, or (`shown`) for the side panel to be of once it is made.
  | { kind: 'new'; project?: string; shown?: boolean }
  | { kind: 'remove'; workspace: Workspace }
  // Asked before something is moved: what it says, and what moves it once somebody says yes.
  | { kind: 'move'; question: Question; move: () => Promise<unknown>; refusal?: (said: string, status: number | null) => string }
  | { kind: 'trash'; project: Project };

/** What is being asked about a workspace or a channel's belonging, until it is answered. */
const asking = signal<Asked | null>(null);

const holds = (scope: string) => covers(me.value?.principal.scopes ?? [], scope);

/** Does it, and says in a toast why it could not be done. */
async function done(work: () => Promise<unknown>): Promise<boolean> {
  try {
    await work();
    return true;
  } catch (refused) {
    toast(refused instanceof Error ? refused.message : String(refused), true);
    return false;
  }
}

/**
 * The lines of what something can be moved to, from its places
 * (`moveto.ts`): where it is is marked and is nothing to press, a heading
 * only says, and a new workspace is set apart at the end.
 */
function placesMenu(places: Place[], to: (place: Exclude<Place, { kind: 'heading' } | { kind: 'new' }>) => void, made: () => void): (Offered | Said)[] {
  return places.map((place) => {
    if (place.kind === 'heading') return { said: place.said, apart: true };
    if (place.kind === 'new') return { label: 'New workspace…', mark: signs.plus(), apart: true, act: made };
    return { label: place.name, chosen: place.here, closed: place.here ? 'It is here already' : undefined, act: () => to(place) };
  });
}

/** Asks a question before something of another kind is moved to a workspace (a wiki with its pages, `wikimoved.tsx`): the same question, with Move it doing `move`. */
export function askMoving(question: Question, move: () => Promise<unknown>): void {
  asking.value = { kind: 'move', question, move };
}

/** Asks the question before a project is put in a workspace. */
function askMoveProject(project: Project, to: { id: string; name: string }): void {
  asking.value = { kind: 'move', question: projectQuestion(project.name, to.name), move: () => spaces.place(project.id, to.id) };
}

/**
 * What can be done with a project from its row: pinned to the top of the
 * panel or no longer, which is anybody's own to do; and, for whoever may
 * add a project, moved: one line that opens beside it the workspaces it
 * could be in, and a new one. It is always in one, so there is no line
 * that takes it out of all; what takes it away is the line that moves it
 * to the Trash.
 */
function forProject(project: Project): Offered[] {
  // A folder of its arrangement made, and it called something else, for whoever may arrange it (`outline.tsx`).
  const items: Offered[] = [pinning({ kind: 'project', id: project.id }), ...projectLines(project), ...newTabLines({ kind: 'project', id: project.id })];
  if (!holds('projects:write')) return items;
  const sub = placesMenu(
    projectPlaces(project.id, workspaces.value),
    (place) => askMoveProject(project, place),
    () => (asking.value = { kind: 'new', project: project.id }),
  );
  const move: Offered = { label: 'Move to…', apart: true, sub, act: () => {} };
  // Set apart at the foot, as what takes something away: behind a question that says what happens.
  const away: Offered = { label: 'Move to Trash…', mark: signs.bin(), look: 'grave', apart: true, act: () => (asking.value = { kind: 'trash', project }) };
  return [...items, move, ...forgetLine(project), away];
}

/**
 * What is done with a workspace from the menu at the panel's foot, for
 * whoever may add a project: a new one is made, under the workspaces
 * there are, and the panel is of it then. Calling the one showing
 * something else and taking it away are in Settings, under the
 * workspace (`spacesettings.tsx`).
 */
export function forSpace(): Offered[] {
  if (!holds('projects:write')) return [];
  return [{ label: 'New workspace…', mark: signs.plus(), act: () => (asking.value = { kind: 'new', shown: true }) }];
}

/** Asks whether a workspace is to be taken away, and where what is in it goes (`RemoveSpace`). */
export function askRemove(workspace: Workspace): void {
  asking.value = { kind: 'remove', workspace };
}

/**
 * The line of a channel's menu that moves it to what it is to belong to,
 * for whoever may change a channel: beside it each workspace and, under
 * a heading, each project, what it belongs to now marked. It always
 * belongs to one, so there is no line for nothing.
 */
export function belongsItem(channel: Channel): Offered | null {
  if (!holds('channels:write')) return null;
  const all = workspaces.value;
  const was = channel.project ? (projects.value.find((one) => one.id === channel.project)?.name ?? null) : null;
  const sub = placesMenu(
    channelPlaces(channel, all, projects.value),
    (place) => {
      const to = place.kind === 'project' ? ({ kind: 'project', name: place.name, space: spaceOf(place.id, all)?.name ?? null } as const) : ({ kind: 'workspace', name: place.name } as const);
      asking.value = {
        kind: 'move',
        question: channelQuestion(hashed(channel), to, was),
        move: () => spaces.belongs(channel.id, place.kind === 'project' ? { project: place.id } : { workspace: place.id }),
        // The daemon says it in the command line's words: here, without them.
        refusal: (said, status) => (status === 409 ? `${(said.split('. Stop it first')[0] ?? said).replaceAll('`', '')}. It can be stopped first, under Activity.` : said),
      };
    },
    () => {},
  );
  return { label: 'Move to…', sub, act: () => {} };
}

/** A branch of the channels: a project's name, which folds it, how many wait unread under it while it is folded, and its channels. */
function Branch(props: { at: string; name: string; said: string; waiting: number; held?: boolean; children: ComponentChildren }) {
  // Held open by what was typed in the panel's box where something under it was found, whatever its fold was left as.
  const hold = !!props.held;
  // It starts folded, as everything of the panel does: what is kept is that it was unfolded.
  const open = hold || turned.value.has(props.at);
  return (
    <div class="spaced">
      <div class="nav-row space-row">
        <button class="nav-twist" aria-label={open ? `Fold ${props.name}` : `Unfold ${props.name}`} aria-expanded={open} aria-disabled={hold || undefined} onClick={() => !hold && turn(props.at)}>
          <Twist open={open} />
        </button>
        <button class="nav-item space-name" title={props.said} aria-disabled={hold || undefined} onClick={() => !hold && turn(props.at)}>
          <span class="truncate">{props.name}</span>
          {!open && props.waiting ? <span class="count">{props.waiting}</span> : null}
        </button>
      </div>
      {open && <div class="tree">{props.children}</div>}
    </div>
  );
}

/**
 * The projects the side panel is of: those of the workspace it is of
 * (every one, until the workspaces are read), each as a project always
 * was listed. `tree` draws one project,
 * with what it is handed at the end of its row.
 */
export function ProjectsListed(props: { tree: (project: Project, more: ComponentChildren) => ComponentChildren }) {
  const now = held.value;
  const shown = listed(projects.value, workspaces.value, now);
  return (
    <>
      {shown.map((project) => props.tree(project, <More label={`More for ${project.name}`} items={() => forProject(project)} />))}
      {now.kind === 'space' && shown.length === 0 && <span class="nav-who faint">no project in this workspace yet</span>}
    </>
  );
}

/**
 * The channels a person put in no group of their own, gathered by what
 * they belong to: the own channels of the workspace the panel is of
 * first, under no heading, then under each of its projects that has any.
 * `one` draws a channel's row.
 */
export function GatheredChannels(props: { channels: Channel[]; one: (channel: Channel, under: string) => ComponentChildren; found: Found | null }) {
  const found = gathered(props.channels, workspaces.value, projects.value, held.value);
  const waiting = (inside: Channel[]) => inside.reduce((sum, channel) => sum + (unread.value[channel.id] ?? 0), 0);
  return (
    <>
      {found.loose.map((channel) => props.one(channel, ofChannels.loose))}
      {found.projects
        .filter(({ project }) => shows(props.found, ofChannels.project(project.id)))
        .map(({ project, channels: inside }) => (
          <Branch
            key={project.id}
            at={CHANNELS.project(project.id)}
            name={project.name}
            said={`${project.name}: the channels of this project`}
            waiting={waiting(inside)}
            held={heldBy(props.found, ofChannels.project(project.id))}
          >
            {inside.map((channel) => props.one(channel, ofChannels.project(project.id)))}
          </Branch>
        ))}
    </>
  );
}

/** What is asked before something of a workspace is done, or of what a channel belongs to. */
export function AskSpace() {
  const asked = asking.value;
  if (!asked) return null;
  const close = () => (asking.value = null);
  if (asked.kind === 'remove') return <RemoveSpace key={asked.workspace.id} workspace={asked.workspace} onClose={close} />;
  if (asked.kind === 'move') return <MoveTo key={asked.question.title} asked={asked} onClose={close} />;
  if (asked.kind === 'trash') return <TrashProject key={asked.project.id} project={asked.project} onClose={close} />;
  return <SpaceName asked={asked} onClose={close} />;
}

/** What a new workspace is to be called, perhaps for a project to go straight into. Another name for one there is, is given in Settings. */
function SpaceName({ asked, onClose }: { asked: { kind: 'new'; project?: string; shown?: boolean }; onClose: () => void }) {
  const [name, setName] = useState('');
  const [busy, setBusy] = useState(false);
  const input = useFocus<HTMLInputElement>();
  const submit = async (event: Event) => {
    event.preventDefault();
    if (busy) return;
    setBusy(true);
    let made: Workspace | null = null;
    const ok = await done(async () => {
      const one = await spaces.create(name);
      made = one;
      if (asked.shown) {
        // Made from the panel's foot, it is what the panel is of next: once it is read, since a choice of
        // something the reading does not list yet would be taken for one that is gone.
        await readWorkspaces();
        hold({ kind: 'space', id: one.id });
      }
    });
    if (!ok) return setBusy(false);
    // Made for a project, it is then asked, as any move is, whether the project is to go there.
    const project = asked.project ? projects.value.find((one) => one.id === asked.project) : undefined;
    if (project && made) askMoveProject(project, made);
    else onClose();
  };
  return (
    <Modal onClose={onClose}>
      <h2>New workspace</h2>
      <form class="stack" onSubmit={submit}>
        <label>
          Name
          <span class="turning-field">
            <input ref={input} required value={name} onInput={(event) => setName(event.currentTarget.value)} />
            {name === '' && <Turning />}
          </span>
        </label>
        <p class="faint" style="margin:0;font-size:calc(12px * var(--size))">
          A workspace is a name for the projects that belong together, for everybody here. It changes where they are listed, and nothing of who may read them.
        </p>
        <div class="row" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" type="submit" disabled={busy}>
            Create
          </button>
        </div>
      </form>
    </Modal>
  );
}

const counted = (count: number, one: string, many: string) => `${count} ${count === 1 ? one : many}`;

/**
 * Asks whether a workspace is to be taken away, saying what becomes of
 * what is in it: its projects and its own channels are always in a
 * workspace, so they go to another, which is chosen here. The usual one
 * is offered first and is what is chosen unless somebody says otherwise.
 * It cannot be undone, so the workspace's name is typed before the button
 * that removes opens; why the
 * button is closed is said under the field (`removeClosed`).
 */
function RemoveSpace({ workspace, onClose }: { workspace: Workspace; onClose: () => void }) {
  const others = workspaces.value.filter((one) => one.id !== workspace.id);
  const first = usualOf(others);
  const offered = first ? [first, ...others.filter((one) => one.id !== first.id)] : [];
  const [to, setTo] = useState(first?.id ?? '');
  const [typed, setTyped] = useState('');
  const [busy, setBusy] = useState(false);
  const [wrong, setWrong] = useState<string | null>(null);
  const field = useFocus<HTMLInputElement>();
  const closed = removeClosed(typed, workspace.name);
  const remove = async () => {
    if (busy || closed || offered.length === 0) return;
    setBusy(true);
    setWrong(null);
    try {
      // Said whether or not anything is seen in it: it may hold a channel that is kept from whoever asks.
      await spaces.remove(workspace.id, to || undefined);
      onClose();
    } catch (refused) {
      // What the daemon refuses is said where it was asked, and the question stays.
      setBusy(false);
      setWrong(refused instanceof Error ? refused.message : String(refused));
    }
  };
  const holds = workspace.projects.length + workspace.channels.length > 0;
  const inside = `${counted(workspace.projects.length, 'project', 'projects')} and ${counted(workspace.channels.length, 'channel', 'channels')}`;
  return (
    <Modal onClose={onClose}>
      <h2>Remove {workspace.name}?</h2>
      <form
        class="stack"
        onSubmit={(event) => {
          event.preventDefault();
          void remove();
        }}
      >
        {holds ? (
          <>
            <p style="margin:0">
              Only the workspace goes. Its {inside} are put in another workspace, and nothing of them is removed: every project, its board and its files, and every channel with what was said in it, stay as they are.
            </p>
            <label>
              They go to
              <select value={to} onChange={(event) => setTo(event.currentTarget.value)}>
                {offered.map((one) => (
                  <option key={one.id} value={one.id}>
                    {one.name}
                  </option>
                ))}
              </select>
            </label>
          </>
        ) : (
          <p style="margin:0">Only the workspace goes. It has no project and no channel of its own.</p>
        )}
        <label class="space-typed">
          Its name, typed to say it is this one
          <input
            ref={field}
            value={typed}
            autocomplete="off"
            spellcheck={false}
            aria-describedby="space-typed-why"
            onInput={(event) => {
              setTyped(event.currentTarget.value);
              setWrong(null);
            }}
          />
        </label>
        <p id="space-typed-why" class="faint space-typed-why" aria-live="polite">
          {closed ?? 'It cannot be undone.'}
        </p>
        {wrong && (
          <p role="alert" class="error-text" style="margin:0">
            {wrong}
          </p>
        )}
        <div class="row modal-foot" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="danger" type="submit" disabled={busy || offered.length === 0 || closed !== null} title={closed ?? undefined} aria-describedby="space-typed-why">
            Remove workspace
          </button>
        </div>
      </form>
    </Modal>
  );
}

/**
 * Asks whether something is to be moved, saying what moving it means
 * (`moveto.ts`). What the daemon refuses is said here in its sentence, and
 * the question stays; Cancel leaves everything as it was.
 */
function MoveTo({ asked, onClose }: { asked: Extract<Asked, { kind: 'move' }>; onClose: () => void }) {
  const [busy, setBusy] = useState(false);
  const [wrong, setWrong] = useState<string | null>(null);
  const move = async () => {
    setBusy(true);
    setWrong(null);
    try {
      await asked.move();
      onClose();
    } catch (refused) {
      setBusy(false);
      const said = refused instanceof Error ? refused.message : String(refused);
      setWrong(asked.refusal ? asked.refusal(said, refused instanceof ApiError ? refused.status : null) : said);
    }
  };
  return (
    <Modal onClose={onClose}>
      <h2>{asked.question.title}</h2>
      <div class="stack">
        <p style="margin:0">{asked.question.says}</p>
        {wrong && (
          <p role="alert" class="error-text" style="margin:0">
            {wrong}
          </p>
        )}
        <div class="row modal-foot" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" type="button" disabled={busy} onClick={() => void move()}>
            Move it
          </button>
        </div>
      </div>
    </Modal>
  );
}

/** The workspace a new project, or a channel that is no project's, is to be in: it is always in one, so one of those there are, and nothing is drawn until they are read. */
export function SpaceChoice(props: { value: string; onChange: (workspace: string) => void }) {
  if (workspaces.value.length === 0) return null;
  return (
    <label>
      Workspace
      <select value={props.value} onChange={(event) => props.onChange(event.currentTarget.value)}>
        {workspaces.value.map((one) => (
          <option key={one.id} value={one.id}>
            {one.name}
          </option>
        ))}
      </select>
    </label>
  );
}

/** Opens a project's menu where the pointer's other button was pressed. */
export function projectMenu(project: Project) {
  return (event: MouseEvent) => {
    event.preventDefault();
    offer({ x: event.clientX, y: event.clientY }, forProject(project));
  };
}
