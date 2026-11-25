// What a channel offers its agents of discussions, in the client: read
// once for a channel and followed as it is offered and withdrawn
// (`channel.discussion.offered`, `.withdrawn`), nothing polled; the
// choice beside Send that asks the agents a message names to discuss it,
// drawn only while it names two or more who take part; a message's mark;
// and the line of a channel's menu that opens what it offers as a form,
// made from `std/discuss`'s own params (`Applying`, as any blueprint is
// applied with) and applied for that channel.

import { signal } from '@preact/signals';
import { useEffect } from 'preact/hooks';
import { api, call } from '../api.ts';
import type { Channel, Fields } from '../api.ts';
import { asksToDiscuss, discussionOf, markSaid } from '../discussing.ts';
import type { Offered as Discussing } from '../discussing.ts';
import { covers } from '../holds.ts';
import { attempt, drawer, me, toast } from '../store.ts';
import type { Drawer } from '../store.ts';
import { useEvents } from '../ui.tsx';
import { Applying } from './applying.tsx';
import { signs } from './signs.tsx';
import type { Offered } from './rowmenu.tsx';
import { Tag } from './tag.tsx';

/** What each channel offers, by its id, as it was last read. */
const offered = signal<Record<string, Discussing>>({});

async function read(channel: string): Promise<void> {
  const said = await call<Discussing>('GET', `/channels/${encodeURIComponent(channel)}/discussing`).catch(() => null);
  if (said) offered.value = { ...offered.value, [channel]: said };
}

/** What `channel` offers of discussions, read once and followed. */
export function useDiscussing(channel: string): Discussing | undefined {
  useEffect(() => {
    if (!offered.value[channel]) void read(channel);
  }, [channel]);
  useEvents(
    'channel.discussion.*',
    (event) => {
      if (event.data?.channel === channel) void read(channel);
    },
    [channel],
    () => void read(channel),
  );
  return offered.value[channel];
}

/**
 * Over a thread that is a discussion going or waiting for a person: how it
 * stands, and End discussion for whoever may write in the channel, which
 * ends it without answering it and calls off the turn in hand.
 */
export function DiscussionHead({ channel, root }: { channel: string; root: string }) {
  const discussing = useDiscussing(channel);
  const stands = discussionOf(discussing, root);
  if (!stands) return null;
  const may = covers(me.value?.principal.scopes ?? [], 'channels:write');
  const end = async () => {
    const ended = await attempt(() => call('POST', `/channels/${encodeURIComponent(channel)}/discussions/${encodeURIComponent(root)}/end`, {}));
    if (ended) {
      toast('The discussion is ended.');
      void read(channel);
    }
  };
  return (
    <div class="discussion-head" role="status">
      <span class="faint grow">{stands === 'going' ? 'Agents are talking this over here.' : 'This discussion waits for a person to decide.'}</span>
      {may && (
        <button type="button" class="ghost" onClick={() => void end()}>
          End discussion
        </button>
      )}
    </div>
  );
}

/** A message's mark, as a tag before its words: nothing for one that has none. */
export function MarkTag({ mark }: { mark?: string | null }) {
  const said = markSaid(mark);
  return said ? <Tag of={said} small /> : null;
}

/** The choice beside Send that asks the agents what is written names to discuss it, while it names two or more. */
export function DiscussSend({ channel, text, disabled, onDiscuss }: { channel: string; text: string; disabled: boolean; onDiscuss: () => void }) {
  const discussing = useDiscussing(channel);
  if (!asksToDiscuss(discussing, text)) return null;
  return (
    <button type="button" class="ghost discuss-send" disabled={disabled} data-tip="Send, asking the agents it names to talk it over in a thread" onClick={onDiscuss}>
      Ask them to discuss
    </button>
  );
}

/** What `std/discuss` is applied under for a channel. */
const keyOf = (channel: Channel) => `discuss-${channel.id}`;

/** The line of a channel's menu that opens what it offers of discussions: for whoever may apply and write there. */
export function discussionItems(channel: Channel): Offered[] {
  const scopes = me.value?.principal.scopes ?? [];
  if (!covers(scopes, 'blueprints:run') || !covers(scopes, 'channels:write')) return [];
  return [{ label: 'Discussions…', mark: signs.chat(), act: () => void openDiscussions(channel) }];
}

async function openDiscussions(channel: Channel): Promise<void> {
  const described = await attempt(() => api.blueprints());
  const discuss = described?.find((one) => one.name === 'std/discuss');
  if (!discuss) return;
  const live = (await attempt(() => api.instances({ live: true }))) ?? [];
  const running = live.find((one) => one.blueprint === 'std/discuss' && one.key === keyOf(channel));
  // The channel is the one it was opened from, and is not asked.
  const params = discuss.params.filter((param) => param.name !== 'channel');
  const kept = `discussions:${channel.id}`;
  const mine: Drawer = {
    title: `Discussions in ${channel.title ?? channel.name}`,
    body: () => (
      <Applying
        key={kept}
        kept={kept}
        params={params}
        known={running?.params ?? {}}
        onClose={() => drawer.value === mine && (drawer.value = null)}
        onApply={async (values: Fields) => {
          const body = { blueprint: 'std/discuss', key: keyOf(channel), project: null, params: { ...values, channel: channel.id }, wait: true };
          const applied = await attempt(() => api.apply(body));
          if (!applied) return false;
          toast(running ? 'The discussions here are said otherwise from now.' : 'The agents here talk things over from now.');
          void read(channel.id);
          return true;
        }}
      />
    ),
  };
  drawer.value = mine;
}
