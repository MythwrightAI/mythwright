// An archived channel, as the page draws it: the lines of a channel's
// menu that archive it and give it back, the question asked before it is
// archived, the Archived part at the foot of the side panel's channels,
// and what the channel's own page says in its head and where something
// would be written.
//
// What is worked out is `archived.ts`; the daemon holds what archiving
// is (`talk::archived`), and what it refuses is said in its sentence
// where it was asked. Nothing here lets go of anything kept of a channel:
// an archived channel is still one, and given back it stands in its
// group, pinned where it was pinned, as it stood.

import { signal } from '@preact/signals';
import type { ComponentChildren } from 'preact';
import { useEffect, useRef, useState } from 'preact/hooks';
import type { Channel } from '../api.ts';
import { archiveChannel, archivedSays, isArchived, mayArchive, unarchiveChannel } from '../archived.ts';
import { hashed } from '../kept.ts';
import { attempt, channels, me, toast, turn, turned } from '../store.ts';
import { Modal, Twist } from '../ui.tsx';
import { CHANNELS } from '../folds.ts';
import type { Offered } from './grouped.tsx';
import { signs } from './signs.tsx';
import { holds, ofChannels, shows } from '../panelsought.ts';
import type { Found } from '../panelsought.ts';

/** The channel somebody asked to archive, until they say yes or no. */
const archiving = signal<string | null>(null);

/** What the daemon said, as a page says it: without the marks a terminal reads commands by. */
const said = (refused: unknown) => (refused instanceof Error ? refused.message : String(refused)).replaceAll('`', '');

/** Whether whoever is signed in is offered archiving `channel`, or giving it back. */
function offered(channel: Channel): boolean {
  const who = me.value;
  return !!who && mayArchive(channel, { actor: who.actor, scopes: who.principal.scopes });
}

/** Gives `channel` back, saying so. */
export async function unarchive(channel: Channel): Promise<void> {
  if (await attempt(() => unarchiveChannel<Channel>(channel.id))) toast(`${hashed(channel)} is back`);
}

/** The line of a channel's menu that archives it, or gives back one that is archived: only for whoever may. */
export function archiveItems(channel: Channel): Offered[] {
  if (!offered(channel)) return [];
  if (isArchived(channel)) return [{ label: 'Unarchive', mark: signs.archive(), apart: true, act: () => void unarchive(channel) }];
  return [{ label: 'Archive…', mark: signs.archive(), apart: true, act: () => (archiving.value = channel.id) }];
}

/** What is asked before a channel is archived. */
export function AskArchive() {
  const id = archiving.value;
  const channel = id ? channels.value.find((one) => one.id === id) : undefined;
  if (!channel) return null;
  return <Archive key={channel.id} channel={channel} onClose={() => (archiving.value = null)} />;
}

/** Asks whether a channel is to be archived, saying what that does and how it is given back. */
function Archive({ channel, onClose }: { channel: Channel; onClose: () => void }) {
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const archive = async () => {
    setBusy(true);
    setWrong(null);
    try {
      await archiveChannel(channel.id);
      onClose();
    } catch (refused) {
      setBusy(false);
      setWrong(said(refused));
    }
  };
  return (
    <Modal onClose={onClose}>
      <h2>Archive {hashed(channel)}?</h2>
      <div class="stack">
        <p style="margin:0">
          Whoever reads it goes on reading it, and finds it by search, with everything said in it. Nobody says anything there, and no agent or bot is asked anything there, until it is given back.
        </p>
        <p class="faint" style="margin:0;font-size:12px">
          It is listed under Archived at the foot of the channels{channel.kept ? ', and stays private' : ''}. Whoever may write in it gives it back from there, as it was.
        </p>
        {wrong && (
          <p role="alert" class="kept-wrong">
            {wrong}
          </p>
        )}
        <div class="row" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" type="button" disabled={busy} onClick={() => void archive()}>
            Archive channel
          </button>
        </div>
      </div>
    </Modal>
  );
}

/** Where the Archived part's fold is kept: folded until somebody unfolds it. */
const UNFOLDED = CHANNELS.archived;

/**
 * The Archived part at the foot of the workspace's channels: folded until
 * it is unfolded, each channel drawn by `one` as every row of the panel
 * is, and held still while the pointer is on it or the keyboard in it,
 * so that a channel archived meanwhile does not come in under the
 * pointer: it is listed once the pointer has left.
 */
export function ArchivedPart(props: { channels: Channel[]; one: (channel: Channel, under: string) => ComponentChildren; found: Found | null }) {
  const [within, setWithin] = useState(false);
  const [pointed, setPointed] = useState(false);
  const was = useRef<string[] | null>(null);
  useEffect(() => () => setPointed(false), []);
  if (!pointed && !within) was.current = null;
  else was.current ??= props.channels.map((channel) => channel.id);
  const by = new Map(props.channels.map((channel) => [channel.id, channel]));
  const listed = was.current ? was.current.flatMap((id) => by.get(id) ?? []) : props.channels;
  if (listed.length === 0 || !shows(props.found, ofChannels.archived)) return null;
  const hold = holds(props.found, ofChannels.archived);
  const open = hold || turned.value.has(UNFOLDED);
  return (
    <div
      class="grouped archived-part"
      onPointerEnter={(event) => event.pointerType === 'mouse' && setPointed(true)}
      onPointerLeave={() => setPointed(false)}
      onFocusIn={(event) => setWithin((event.target as Element).matches(':focus-visible'))}
      onFocusOut={(event) => !event.currentTarget.contains(event.relatedTarget as Node | null) && setWithin(false)}
    >
      <div class="nav-row group-row">
        <button class="nav-twist" aria-label={open ? 'Fold Archived' : 'Unfold Archived'} aria-expanded={open} aria-disabled={hold || undefined} onClick={() => !hold && turn(UNFOLDED)}>
          <Twist open={open} />
        </button>
        <button class="nav-item group-name" title="Archived: read as before, and said nothing in until given back" aria-disabled={hold || undefined} onClick={() => !hold && turn(UNFOLDED)}>
          <span class="glyph">{signs.archive(13)}</span>
          <span class="truncate">Archived</span>
          {!open && <span class="hint">{listed.length}</span>}
        </button>
      </div>
      {open && <div class="tree">{listed.map((channel) => props.one(channel, ofChannels.archived))}</div>}
    </div>
  );
}

/** Said at the head of an archived channel's page, with Unarchive for whoever may. */
export function ArchivedHead({ channel }: { channel: Channel }) {
  if (!isArchived(channel)) return null;
  return (
    <span class="archived-head" role="status">
      <span class="archived-mark" title="Archived: read as before, and said nothing in until given back">
        {signs.archive(13)}
        <span class="archived-word">Archived</span>
      </span>
      {offered(channel) && (
        <button class="ghost" onClick={() => void unarchive(channel)}>
          Unarchive
        </button>
      )}
    </span>
  );
}

/**
 * Where something would be written in an archived channel: one line that
 * says it is archived, with nothing to type in, and Unarchive for whoever
 * may, which on a phone is where it is offered, the head having no room.
 */
export function ArchivedLine({ channel }: { channel: Channel }) {
  return (
    <div class="archived-line" role="note">
      {signs.archive(14)}
      <span class="archived-said">{archivedSays(channel)}</span>
      {offered(channel) && (
        <button class="ghost" onClick={() => void unarchive(channel)}>
          Unarchive
        </button>
      )}
    </div>
  );
}
