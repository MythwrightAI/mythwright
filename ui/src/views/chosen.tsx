// The foot of a side panel: which workspace the panel is of, said and
// chosen there, and Help beside it.
//
// The panel is bounded by one workspace: its projects are what is
// listed, each as a project always was, and its channels and theirs. So
// the workspace is not a row among the projects but what the whole panel
// is of, and it is said where the panel ends: a strip that stays at the
// foot while the lists scroll above it, the same under Resources and
// under Channels, with one choice for both. At its left is the
// workspace: a square that stands for it (the first letter of its name:
// a workspace has no picture yet), its name and a small arrow, one
// button. Pressed, its menu says the workspace again with how many
// projects it has; says who is signed in; offers every workspace by
// name, each with how many wait unread in its channels; makes a new one;
// and then, set apart at its foot, opens Settings at the part for the
// workspace showing (where it is called something else and taken away),
// says where somebody is invited for whoever may invite, and signs out.
//
// Beside it, while anything waits unread in a channel of another
// workspace, is how many: a button of its own, which shows the workspace
// where the most wait.
//
// A factory always has a workspace, so the strip always says one once
// they are read. Until then its left says nothing and is nothing to
// press: an empty list that was never read says nothing of what there
// is. At its right is Help (`help.tsx`), which is always there.
//
// Which workspace is showing is one person's own (`chosen.ts`). Choosing
// changes nothing else: what is open stays open, and what somebody
// folded, grouped and pinned is as they left it.
//
// Its menu is the one the rows of the panel open (`RowMenu`), opened
// upward: it lies over the page and pushes nothing about, stays while
// the pointer crosses to it, and is worked by the keyboard.

import { useRef } from 'preact/hooks';
import { held, hold, mostOutOfSight, outOfSight, waiting } from '../chosen.ts';
import { counted } from '../help.ts';
import { workspaces } from '../spaces.ts';
import { me, openTab, projects, settingsPart, signOut } from '../store.ts';
import { listed, sameHeld } from '../workspaces.ts';
import type { Held } from '../workspaces.ts';
import { offer, offeredBy, putAway } from './grouped.tsx';
import type { Offered, Said } from './grouped.tsx';
import { Help } from './help.tsx';
import { Face, signs } from './signs.tsx';
import { forSpace } from './spaces.tsx';

/**
 * What is offered, from its head down: the workspace showing, with what
 * it holds; who is signed in; each choice; a new one; and set apart at
 * the foot, Settings, inviting for whoever may, and signing out.
 * Calling the one showing something else and taking it away are in
 * Settings, under the workspace (`spacesettings.tsx`).
 */
function offered(now: Held): (Offered | Said)[] {
  const all = workspaces.value;
  const showing = now.kind === 'space' ? (all.find((one) => one.id === now.id) ?? null) : null;
  const choice = (to: Held, label: string): Offered => {
    const shown = sameHeld(to, now);
    // What waits in what is showing is said in the list itself, where it is.
    return { label, chosen: shown, count: shown ? 0 : waiting(to), mark: <Face name={label} />, act: () => hold(to) };
  };
  const lines: (Offered | Said)[] = [
    {
      said: (
        <span class="menu-head">
          <Face name={showing?.name ?? ''} none={!showing} />
          <span class="menu-head-said">
            <strong class="truncate">{showing?.name ?? ''}</strong>
            <small class="truncate">{counted(listed(projects.value, all, now).length, 'project', 'projects')}</small>
          </span>
        </span>
      ),
    },
    { said: <span class="menu-quiet truncate">Signed in as {me.value?.principal.name}</span>, apart: true },
    ...all.map((one) => choice({ kind: 'space', id: one.id }, one.name)),
    ...forSpace(),
  ];
  const settings = (part: 'workspace' | 'people') => () => {
    settingsPart.value = part;
    openTab({ kind: 'settings' });
  };
  // Opened from here it shows what is of the workspace this menu is of.
  lines.push({ label: 'Settings', mark: signs.gear(), apart: true, act: settings('workspace') });
  // For whoever may invite somebody as things stand (the daemon says: the factory's owner, on a plan that has more than one person).
  if (me.value?.can_invite) lines.push({ label: 'Invite members', mark: signs.envelope(), act: settings('people') });
  lines.push({ label: 'Log out', mark: signs.door(), act: () => void signOut() });
  return lines;
}

/** The workspace at the left of the foot: what the panel is of, and the way to another. */
function SpacePick() {
  const button = useRef<HTMLButtonElement>(null);
  const all = workspaces.value;
  const now = held.value;
  const name = now.kind === 'space' ? (all.find((one) => one.id === now.id)?.name ?? '') : '';
  const out = outOfSight.value;
  const open = offeredBy(button.current);
  const unread = out > 0 ? `, and ${out} unread in ${out === 1 ? 'a channel' : 'channels'} of another` : '';
  return (
    <button
      ref={button}
      class="space-pick"
      aria-haspopup="menu"
      aria-expanded={open}
      aria-label={`Workspace: ${name}${unread}`}
      title={`This panel is of the workspace ${name}: its projects, and the channels that belong to it or to them. Which one it is of is your own choice, kept in this browser.`}
      onClick={(event) => {
        if (open) return putAway();
        const at = event.currentTarget.getBoundingClientRect();
        offer({ x: at.left, y: at.top - 4 }, offered(now), { up: true, wide: Math.max(at.width, 256), label: 'Workspace', from: event.currentTarget });
      }}
    >
      <Face name={name} />
      <span class="space-said">
        <strong class="truncate">{name}</strong>
      </span>
      <span class="space-turn">{signs.chevron()}</span>
    </button>
  );
}

/**
 * How many wait unread in channels the panel leaves out, which are those
 * of another workspace: choosing never hides that somebody spoke. It is
 * a button of its own beside the workspace: pressed, the panel is of the
 * workspace where the most of them wait, which is one press to what was
 * said; the menu beside it says how many wait in each.
 */
function OutOfSight() {
  const out = outOfSight.value;
  const most = mostOutOfSight.value;
  const where = workspaces.value.find((one) => one.id === most);
  if (out <= 0 || !where) return null;
  return (
    <button
      class="space-out-press"
      aria-label={`${out} unread in ${out === 1 ? 'a channel' : 'channels'} of another workspace: show ${where.name}`}
      title={`${out} unread in ${out === 1 ? 'a channel' : 'channels'} this panel leaves out. Press to show ${where.name}, where ${out === 1 ? 'it waits' : 'the most wait'}.`}
      onClick={() => hold({ kind: 'space', id: where.id })}
    >
      <span class="space-out">{out > 99 ? '99+' : out}</span>
    </button>
  );
}

/** The foot of a side panel that lists by workspace: which one it is of with the way to another, and Help. */
export function SpaceFoot() {
  return (
    <div class="space-foot">
      {/* Until the workspaces are read there is nothing to say here, and nothing to press: Help keeps its place at the right. */}
      {held.value.kind === 'space' ? <SpacePick /> : <span class="grow" />}
      <OutOfSight />
      <Help />
    </div>
  );
}
