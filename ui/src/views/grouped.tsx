// The side panel's channels, in the groups a person put them in.
//
// A group is drawn as a branch of the list: its name, which folds it, and
// under it its channels. Channels in no group are listed above the groups,
// as they always were, so somebody who makes no group sees no difference.
// A channel is moved by its menu (the button at the end of its row, or the
// pointer's other button), or by being dragged onto a group's name; it is
// taken out of its group by the same menu (No group), or by being dragged
// onto the panel's head, which says Channels, or among those in none where
// there are any: each is there before anything is dragged, so
// nothing appears or moves under the pointer, and nothing stands in the
// place of the channels in no group where every channel is in a group. A
// channel that is pinned is listed at the top, before all of that, and
// where it otherwise is again when it is no longer pinned.
//
// What is drawn here is one person's own (`channelgroups.ts`): it is kept
// with their account, so that it is the same in every window and browser
// they sign in to (`arrangedkept.ts`), and the daemon acts on none of it.
// What is folded is each browser's own, and every branch starts folded, as
// everything of the Resources panel does (`folds.ts`): a folded branch says
// how many wait unread under it, so that nothing waiting is out of sight.

import { computed, effect, signal } from '@preact/signals';
import type { ComponentChildren } from 'preact';
import { useEffect, useState } from 'preact/hooks';
import { api, ApiError } from '../api.ts';
import type { Channel } from '../api.ts';
import { arrangedOf, LONGEST, madeSaid, pinned, placed, renamed, withGroup, withoutGroup } from '../channelgroups.ts';
import { groupingFrom, withGrouping } from '../arranged.ts';
import { arrange, arrangedRead, arrangement } from '../arrangedkept.ts';
import { CHANNELS, groupFoldsGone } from '../folds.ts';
import type { Drawn } from '../folds.ts';
import type { Group, Grouping } from '../channelgroups.ts';
import { held } from '../chosen.ts';
import { workspaces } from '../spaces.ts';
import { asking, channels, me, onChannelRemoved, projects, toast, turn, turned, turnedTo, unread } from '../store.ts';
import { covers } from '../holds.ts';
import { holdsChannel } from '../workspaces.ts';
import { Modal, Twist, useFocus } from '../ui.tsx';
import { classes } from '../util.ts';
import { hashed } from '../kept.ts';
import { trashed } from '../trash.ts';
import { saidDeleted } from './trash.tsx';
import { signs } from './signs.tsx';
import { withNewTab } from './newtab.tsx';
import { doorItems } from './keptdoor.tsx';
import { archiveItems, ArchivedPart, AskArchive } from './archivedpart.tsx';
import { AskRename, renameItems } from './renaming.tsx';
import { belongsItem, GatheredChannels } from './spaces.tsx';
import { channelItems, holds, ofChannels, shows } from '../panelsought.ts';
import type { Found, Item } from '../panelsought.ts';
import { amongChannels } from '../conversed.ts';
import { gathered } from '../workspaces.ts';

/** The groups and pins of channels whoever is signed in made, as the page draws them: kept with their account (`arrangedkept.ts`). */
export const grouping = computed<Grouping>(() => groupingFrom(arrangement.value));

// A group that is not there any more takes its fold with it, once the account's groups were read: its id is given
// to the next group made, which would otherwise find itself unfolded. What is folded is read and not followed here.
effect(() => {
  const ids = grouping.value.groups.map((group) => group.id);
  if (!arrangedRead.value) return;
  const gone = new Set(groupFoldsGone(turned.peek(), ids));
  if (gone.size > 0) turnedTo([...turned.peek()].filter((one) => !gone.has(one)));
});

/** Changes the groups and pins, as a function of how they are when the change is made. */
function keep(change: (now: Grouping) => Grouping): void {
  void arrange((now) => withGrouping(now, change(groupingFrom(now))));
}

/** The group a channel goes into that is being made from that group's menu, while the dialog that makes it is open. */
export const madeInto = signal<string | null>(null);

/**
 * A channel made from a group's menu (`space` being the workspace the
 * dialog made it in): put in that group through the arrangement, at its
 * end, as dragging it there puts it; the group unfolded so that it is in
 * view; and where it went said where the group would not draw it.
 */
export function putMade(channel: Channel, group: string, space: string): void {
  const into = grouping.peek().groups.find((one) => one.id === group) ?? null;
  keep((now) => placed(now, channel.id, group));
  if (into && !turned.peek().has(CHANNELS.group(group))) turn(CHANNELS.group(group));
  const now = held.peek();
  const elsewhere = now.kind === 'space' && space && space !== now.id ? (workspaces.peek().find((one) => one.id === space)?.name ?? null) : null;
  const said = madeSaid(hashed(channel), into?.name ?? null, elsewhere);
  if (said) toast(said);
}

/** What is asked of a group by name: a new one, perhaps for a channel to go straight into, or another name for one there is. */
export const askingGroup = signal<{ channel?: string; rename?: string } | null>(null);

// The menu a row opens, and every menu like it, is `rowmenu.tsx`: offered from here under the names it always had.
export { offer, offeredBy, putAway, RowMenu } from './rowmenu.tsx';
export type { Offered, Said } from './rowmenu.tsx';
import { offer, offeredBy, putAway } from './rowmenu.tsx';
import type { Offered } from './rowmenu.tsx';

/** The button at the end of a row that opens what can be done with it. */
export function More(props: { label: string; items: () => Offered[]; keys?: boolean }) {
  return (
    <button
      class="ghost icon row-more"
      aria-label={props.label}
      // Pointed at it says nothing: a tip at a row's end lay over the panel (`tips.ts`).
      data-untipped
      aria-haspopup="menu"
      onClick={(event) => {
        const from = event.currentTarget;
        // One that gives the menu the keyboard (`keys`) is what opened it: pressed again it puts it away, and has the keyboard back after.
        if (props.keys && offeredBy(from)) return putAway();
        const at = from.getBoundingClientRect();
        offer({ x: at.left, y: at.bottom + 2 }, props.items(), props.keys ? { from, label: props.label } : {});
      }}
    >
      ⋯
    </button>
  );
}

/** The channel somebody asked to have taken away, until they say yes or no. */
const removing = signal<Channel | null>(null);

// A channel that was taken away is not drawn, and the browser keeps nothing of it (`copyOf`); where it was put is
// kept with the account and passed over, so that one given back from the Trash is where it was.
onChannelRemoved((id) => {
  if (removing.value?.id === id) removing.value = null;
});

/** What is asked before something of the side panel's is done: what a group is to be called, and whether a channel is really to go. */
export function AskGroup() {
  const asked = askingGroup.value;
  const going = removing.value;
  return (
    <>
      {asked && <GroupName asked={asked} />}
      {going && <RemoveChannel key={going.id} channel={going} />}
      <AskArchive />
      <AskRename />
    </>
  );
}

/** Asks whether a channel is to be moved to the Trash, saying what goes with it and for how long it is kept. It is everybody's that goes. */
function RemoveChannel({ channel }: { channel: Channel }) {
  // What the daemon said when something that is running posts there: then it is asked once more, to go all the same.
  const [held, setHeld] = useState<string | null>(null);
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [days, setDays] = useState<number | null | undefined>(undefined);
  useEffect(() => void trashed().then((read) => setDays(read.days)).catch(() => setDays(30)), []);
  const close = () => (removing.value = null);
  const remove = async () => {
    setBusy(true);
    setWrong(null);
    try {
      const gone = await api.removeChannel(channel.id, held !== null);
      close();
      saidDeleted(gone, hashed(channel));
    } catch (refused) {
      setBusy(false);
      if (refused instanceof ApiError && refused.status === 409) setHeld(refused.message);
      else setWrong(refused instanceof Error ? refused.message : String(refused));
    }
  };
  return (
    <Modal onClose={close}>
      <h2>Move {hashed(channel)} to the Trash?</h2>
      <div class="stack">
        <p style="margin:0" class="trash-asked">
          Its messages go with it, and everything attached to them, for everyone in it.{keptFor(days, channel.kept ? ' to the same members' : '')}
        </p>
        {held && (
          <p role="alert" style="margin:0">
            {/* The daemon says it in the command line's words: here, what posts there, and what can be done about it from here. */}
            {(held.split('. ')[0] ?? held).replaceAll('`', '')}. It can be stopped first, under Activity. Removed all the same, what it says here next fails.
          </p>
        )}
        {wrong && (
          <p role="alert" class="faint" style="margin:0">
            {wrong}
          </p>
        )}
        <div class="row" style="justify-content:flex-end">
          <button type="button" onClick={close}>
            Cancel
          </button>
          <button class="danger" type="button" disabled={busy || days === undefined} onClick={() => void remove()}>
            {held ? 'Move it all the same' : 'Move to Trash'}
          </button>
        </div>
      </div>
    </Modal>
  );
}

function GroupName({ asked }: { asked: { channel?: string; rename?: string } }) {
  const was = grouping.value.groups.find((group) => group.id === asked.rename)?.name ?? '';
  const [name, setName] = useState(was);
  const input = useFocus<HTMLInputElement>();
  const close = () => (askingGroup.value = null);
  const submit = (event: Event) => {
    event.preventDefault();
    const rename = asked.rename;
    if (rename) {
      keep((now) => renamed(now, rename, name));
    } else {
      keep((now) => {
        const made = withGroup(now, name);
        if (!made) return now;
        return asked.channel ? placed(made.grouping, asked.channel, made.id) : made.grouping;
      });
      // A group somebody has just made is unfolded, so that a channel they put straight into it is still in view:
      // the arrangement is drawn at once, so it is there to be found by what it is called.
      const made = withGroup(grouping.peek(), name)?.id;
      if (made && !turned.peek().has(CHANNELS.group(made))) turn(CHANNELS.group(made));
    }
    close();
  };
  return (
    <Modal onClose={close}>
      <h2>{asked.rename ? 'Rename group' : 'New group'}</h2>
      <form class="stack" onSubmit={submit}>
        <label>
          Name
          <input ref={input} required maxLength={LONGEST} value={name} placeholder="Work" onInput={(event) => setName(event.currentTarget.value)} />
        </label>
        <p class="faint" style="margin:0;font-size:calc(12px * var(--size))">A group is your own way of listing the channels. It is kept with your account, in every browser you sign in to, and nobody else sees it.</p>
        <div class="row" style="justify-content:flex-end">
          <button type="button" onClick={close}>
            Cancel
          </button>
          <button class="primary" type="submit">
            {asked.rename ? 'Rename' : 'Create'}
          </button>
        </div>
      </form>
    </Modal>
  );
}

/** The button in the panel's head that makes a group. */
export function NewGroup() {
  return (
    <button class="ghost icon brand-act" aria-label="New group" title="New group: list some channels together, your own way" onClick={() => (askingGroup.value = {})}>
      <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round" aria-hidden="true">
        <path d="M1.5 4h4.3l1.4 1.6h7.3v7.9h-13z" />
        <path d="M8 8v3.4M6.3 9.7h3.4" />
      </svg>
    </button>
  );
}

// What a channel that is being dragged is carried as.
const DRAGGED = 'application/x-channel';
// What the top of the list is called among the places a channel can be let go. No group is called this: theirs are `g` and a number.
const PINNED = 'pinned';
// What the panel's head is called among them: let go there, a channel is in no group.
const HEAD = 'head';

/** The channels in no group as the panel gathers them, for what the box finds among them. */
function gatheredOf(loose: Channel[]) {
  const found = gathered(loose, workspaces.value, projects.value, held.value);
  return { loose: found.loose, projects: found.projects };
}

/** The branches the panel draws, for Expand all (`folding.tsx`): what is pinned, the groups, each project's channels and what is archived, and every channel it lists under them. */
export function channelsDrawn(): Drawn {
  const now = held.value;
  const listedHere = (channel: Channel) => !channel.archived && holdsChannel(now, channel, workspaces.value, projects.value);
  const { pinned: top, loose, groups } = arrangedOf(channels.value, grouping.value, listedHere);
  const gatheredHere = gatheredOf(loose);
  const archived = archivedOf();
  return {
    pinned: top.length > 0,
    groups: groups.map(({ group }) => group.id),
    projects: gatheredHere.projects.map(({ project }) => project.id),
    archived: archived.length > 0,
    channels: [...top, ...gatheredHere.loose, ...gatheredHere.projects.flatMap((one) => one.channels), ...groups.flatMap((one) => one.channels), ...archived].map((channel) => channel.id),
  };
}

/** The archived channels of the workspace the panel is of, wherever somebody put them: under the Archived part alone. */
function archivedOf(): Channel[] {
  return gathered(channels.value, workspaces.value, projects.value, held.value).archived;
}

// What a dragged channel is over: a group's id, the empty text for those in no group, `PINNED` or `HEAD`.
const over = signal<string | null>(null);

/**
 * Somewhere a dragged channel can be let go: in a group, with `null` among
 * those in no group, at the top, or on the panel's head, which takes it out
 * of its group. Nothing is drawn or moved for the dragging itself: what is
 * under the pointer when a drag begins is still under it, and what it is
 * over is outlined, which moves nothing.
 */
function takes(group: string | null) {
  // Coming over it is heard as well as being over it: the browser says the second only now and then.
  const hears = (event: DragEvent) => {
    if (!event.dataTransfer?.types.includes(DRAGGED)) return;
    event.preventDefault();
    over.value = group ?? '';
  };
  return {
    onDragEnter: hears,
    onDragOver: hears,
    onDragLeave: (event: DragEvent) => {
      if (!(event.currentTarget as HTMLElement).contains(event.relatedTarget as Node | null)) over.value = null;
    },
    onDrop: (event: DragEvent) => {
      const channel = event.dataTransfer?.getData(DRAGGED);
      over.value = null;
      if (!channel) return;
      event.preventDefault();
      keep((now) => (group === PINNED ? pinned(now, channel, true) : placed(now, channel, group === HEAD ? null : group)));
    },
  };
}

/**
 * The Channels panel's head as somewhere a channel is let go to leave its
 * group: what it is handed to listen with, and whether a
 * channel is over it now, which the head is outlined for.
 */
export function outOfGroup() {
  return { takes: takes(HEAD), over: over.value === HEAD };
}

/** What can be done with a channel: pinned to the top or not, each group it could be in, none, and a new one; and, by the owner, taken away. */
/** The line of a channel's menu that moves it to the Trash, set apart as what takes something away. */
const trashLine = (channel: Channel): Offered => ({ label: 'Move to Trash…', mark: signs.bin(), look: 'grave', apart: true, act: () => (removing.value = channel) });

/** What the question says of how long a channel is kept, and to whom it comes back: nothing until that is read. */
function keptFor(days: number | null | undefined, to: string): string {
  if (days === undefined) return '';
  if (days === 0) return ' It is not kept: it cannot be given back.';
  const back = ` and given back from there whole${to}.`;
  return days === null ? ` It is kept in the Trash until somebody deletes it for good,${back}` : ` It is kept in the Trash for ${days === 1 ? 'a day' : `${days} days`},${back}`;
}

function forChannel(channel: Channel): Offered[] {
  // An archived one is in no group and pinned nowhere while it is archived: what it is offered is giving it back, what it
  // belongs to and its taking away. Its groups and pin are kept as they were, for when it is given back.
  if (channel.archived) {
    const belongs = belongsItem(channel);
    const owner = me.value?.principal.scopes.some((scope) => scope === 'admin' || scope === '*');
    return withNewTab({ kind: 'channel', id: channel.id }, [
      ...archiveItems(channel),
      ...(belongs ? [belongs] : []),
      ...(owner ? [trashLine(channel)] : []),
    ]);
  }
  const { groups, of, pinned: top } = grouping.value;
  const isPinned = top.includes(channel.id);
  const into = (group: string | null) => () => keep((now) => placed(now, channel.id, group));
  const items: Offered[] = [
    // Called something else by whoever may write in it: first, as Slack has it.
    ...renameItems(channel),
    { label: isPinned ? 'Unpin' : 'Pin to the top', act: () => keep((now) => pinned(now, channel.id, !isPinned)) },
    // While it is pinned it is listed at the top, so no group is marked as where it is.
    ...groups.map((group) => ({ label: group.name, chosen: !isPinned && of[channel.id] === group.id, act: into(group.id) })),
  ];
  if (of[channel.id] && !isPinned) items.push({ label: 'No group', act: into(null) });
  items.push({ label: 'New group…', act: () => (askingGroup.value = { channel: channel.id }) });
  // What it belongs to is the factory's and not this person's: said by whoever may change a channel.
  const belongs = belongsItem(channel);
  if (belongs) items.push(belongs);
  // Its door: made private by somebody in it, opened again by the owner, left by whoever is in it.
  items.push(...doorItems(channel));
  // Archived by whoever may write in it, and given back the same.
  items.push(...archiveItems(channel));
  // The owner's to do, and asked about first: it is not only theirs that goes.
  if (me.value?.principal.scopes.some((scope) => scope === 'admin' || scope === '*')) {
    items.push(trashLine(channel));
  }
  return withNewTab({ kind: 'channel', id: channel.id }, items);
}

function forGroup(group: Group): Offered[] {
  // A channel made here goes in this group, by the dialog the panel's `+` opens: offered to whoever may make one.
  const making = covers(me.value?.principal.scopes ?? [], 'channels:write');
  const make = (): void => {
    madeInto.value = group.id;
    asking.value = 'channel';
  };
  return [
    ...(making ? [{ label: 'New channel…', act: make }] : []),
    { label: 'Rename…', act: () => (askingGroup.value = { rename: group.id }) },
    { label: 'Remove group', act: () => keep((now) => withoutGroup(now, group.id)) },
  ];
}

/**
 * The channels, those in no group first and then each group. `row` draws
 * one channel, with what it is handed at the end of its row.
 */
/**
 * What the panel lists of the channels, as it is arranged: held to a
 * workspace, only what that lists is here, wherever somebody put it:
 * pinned, in a group or in none. A group all of whose channels the choice
 * leaves out is not drawn, and is as it was when they are listed again;
 * one with nothing in it at all is there still, to put something in. An
 * archived channel is listed under the Archived part at the foot and
 * nowhere else, its group and pin kept.
 */
function channelsListed() {
  const now = held.value;
  const listedHere = (channel: Channel) => !channel.archived && holdsChannel(now, channel, workspaces.value, projects.value);
  const { pinned: top, loose, groups } = arrangedOf(channels.value, grouping.value, listedHere);
  return { top, loose, groups, archived: archivedOf() };
}

/** The channels as the Channels panel's box looks through them (`panelsought.ts`). */
export function channelItemsNow(): Item[] {
  const { top, loose, groups, archived } = channelsListed();
  return channelItems({ pinned: top, groups, ...gatheredOf(loose), archived });
}

/**
 * The channels, directly in the Channels panel's list. What is typed in
 * the panel's box (`found`, worked out by the panel)
 * leaves out what does not say it, and holds open what something found is
 * under (`panelseek.tsx`); nothing more is read for it.
 */
export function GroupedChannels(props: { row: (channel: Channel, more: ComponentChildren) => ComponentChildren; found: Found | null }) {
  const { top, loose, groups, archived } = channelsListed();
  const found = props.found;
  const one = (channel: Channel, under: string) => {
    if (!shows(found, ofChannels.channel(under, channel.id))) return null;
    return (
      <div
        key={channel.id}
        class="grouped-channel"
        draggable={!channel.archived}
        onDragStart={(event) => {
          event.dataTransfer?.setData(DRAGGED, channel.id);
          if (event.dataTransfer) event.dataTransfer.effectAllowed = 'move';
        }}
        onDragEnd={() => (over.value = null)}
        onContextMenu={(event) => {
          event.preventDefault();
          offer({ x: event.clientX, y: event.clientY }, forChannel(channel));
        }}
      >
        {props.row(channel, <More label={`More for ${hashed(channel)}`} items={() => forChannel(channel)} />)}
      </div>
    );
  };
  // A branch of the list: its name, which folds it, and what is under it.
  const branch = (at: string, name: string, inside: Channel[], more: ComponentChildren, said: string) => {
    // It starts folded, as everything of the panel does: what is kept is that it was unfolded.
    const key = at === PINNED ? CHANNELS.pinned : CHANNELS.group(at);
    const id = at === PINNED ? ofChannels.pinned : ofChannels.group(at);
    if (!shows(found, id)) return null;
    const hold = holds(found, id);
    const open = hold || turned.value.has(key);
    const waiting = inside.reduce((sum, channel) => sum + (unread.value[channel.id] ?? 0), 0);
    return (
      <div key={at} class={classes('grouped', at === PINNED && 'pinned', over.value === at && 'drop')} {...takes(at)}>
        <div class="nav-row group-row">
          <button class="nav-twist" aria-label={open ? `Fold ${name}` : `Unfold ${name}`} aria-expanded={open} aria-disabled={hold || undefined} onClick={() => !hold && turn(key)}>
            <Twist open={open} />
          </button>
          <button class="nav-item group-name" title={said} aria-disabled={hold || undefined} onClick={() => !hold && turn(key)}>
            <span class="truncate">{name}</span>
            {!open && waiting ? <span class="count">{waiting}</span> : null}
          </button>
          {more}
        </div>
        {open && (
          <div class="tree">
            {inside.map((channel) => one(channel, id))}
            {inside.length === 0 && <span class="nav-who faint">nothing in it yet</span>}
          </div>
        )}
      </div>
    );
  };
  return (
    <>
      {top.length > 0 && branch(PINNED, 'Pinned', top, null, 'Pinned: the channels you keep at the top')}
      {/* Where a channel is let go to be in no group: among those that are in none, where there are any, and the panel's head always (`outOfGroup`). Nothing stands in their place where every channel is in a group. */}
      <div class={classes('grouped-loose', over.value === '' && 'drop')} {...takes(null)}>
        <GatheredChannels channels={loose} one={one} found={found} />
      </div>
      {groups.map(({ group, channels: inside }) =>
        branch(group.id, group.name, inside, <More label={`More for ${group.name}`} items={() => forGroup(group)} />, `${group.name}: a group of your own`),
      )}
      <ArchivedPart channels={archived} one={one} found={found} />
      {amongChannels(channels.value).length === 0 && !found && (
        <button class="nav-item" onClick={() => (asking.value = 'channel')}>
          <span class="glyph">+</span>
          <span>Create the first channel</span>
        </button>
      )}
    </>
  );
}
