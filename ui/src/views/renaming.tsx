// A channel called something else, as the page offers it: the line of a
// channel's menu, set at its top as Slack sets it, and the small dialog
// it opens, with what the channel is called in a field, saved on Enter.
//
// What is worked out is `renamed.ts`; the daemon holds what renaming is
// (`talk::renamed`), and what it refuses is said in its sentence in the
// dialog, which stays with what was typed. Nothing is changed here when
// the daemon answers: the channel is read again where it is said to be
// renamed (`channel.renamed` in the store), on this page as on any other.

import { signal } from '@preact/signals';
import { useState } from 'preact/hooks';
import type { Channel } from '../api.ts';
import { calledOf, hashed } from '../kept.ts';
import { MOST, mayRename, renameChannel, renaming } from '../renamed.ts';
import { channels, me } from '../store.ts';
import { Modal, useFocus } from '../ui.tsx';
import type { Offered } from './grouped.tsx';
import { signs } from './signs.tsx';

/** The channel somebody is renaming, until they save or put the dialog away. */
const renamingOf = signal<string | null>(null);

/** What the daemon said, as a page says it: without the marks a terminal reads commands by. */
const said = (refused: unknown) => (refused instanceof Error ? refused.message : String(refused)).replaceAll('`', '');

/** The line of a channel's menu that renames it: only for whoever may. */
export function renameItems(channel: Channel): Offered[] {
  const who = me.value;
  if (!who || !mayRename(channel, { actor: who.actor, scopes: who.principal.scopes })) return [];
  return [{ label: 'Rename…', mark: signs.pencil(), act: () => (renamingOf.value = channel.id) }];
}

/** The dialog that renames a channel, while one is being renamed. */
export function AskRename() {
  const id = renamingOf.value;
  const channel = id ? channels.value.find((one) => one.id === id) : undefined;
  if (!channel) return null;
  return <Rename key={channel.id} channel={channel} onClose={() => (renamingOf.value = null)} />;
}

/** Asks what a channel is to be called: its name, or of a private one what it is called. */
function Rename({ channel, onClose }: { channel: Channel; onClose: () => void }) {
  const [typed, setTyped] = useState(calledOf(channel));
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const input = useFocus<HTMLInputElement>();
  const now = renaming(channel, typed);
  const save = async (event: Event) => {
    event.preventDefault();
    if (!now.called || !now.changes || busy) return;
    setBusy(true);
    setWrong(null);
    try {
      await renameChannel(channel.id, now.called);
      onClose();
    } catch (refused) {
      setBusy(false);
      setWrong(said(refused));
    }
  };
  return (
    <Modal onClose={onClose}>
      <h2>Rename {hashed(channel)}</h2>
      <form class="stack rename-form" onSubmit={save}>
        <label>
          {channel.kept ? 'Name' : 'Channel name'}
          <input
            ref={input}
            value={typed}
            maxLength={channel.kept ? MOST : undefined}
            aria-describedby="rename-says"
            onInput={(event) => {
              setTyped(event.currentTarget.value);
              setWrong(null);
            }}
          />
        </label>
        <p id="rename-says" class="faint" style="margin:0;font-size:calc(12px * var(--size))">
          {now.says ??
            (channel.kept
              ? 'Only its members see what it is called. Another channel may be called the same.'
              : 'Lowercase words joined by dashes. Its old name is free for another channel once it is saved.')}
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
          <button class="primary" type="submit" disabled={busy || !now.called || !now.changes}>
            Save
          </button>
        </div>
      </form>
    </Modal>
  );
}
