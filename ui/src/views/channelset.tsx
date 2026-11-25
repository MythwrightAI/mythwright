// Everything that is set of a channel, in its tab of the Tools panel
// beside it (Channel), under who is in it: whether it is private or open,
// archiving it, opening a conversation as a channel,
// and what its agents talk over between them (`std/discuss`). A
// channel's head has no gear,
// and what it offered is here. Each line is the one the channel's row of
// the side panel offers, made by the same function, so the two never
// part: what is pressed asks what the row's line asks, and is there only
// for whoever pressing it does something for. What it is called is not
// among them: it is written in its Details, at the top of the tab
// (`channeldetails.tsx`), where the row's Rename… is not needed.

import type { Channel } from '../api.ts';
import { isConversation } from '../conversed.ts';
import { archiveItems } from './archivedpart.tsx';
import { conversationItems } from './conversed.tsx';
import { discussionItems } from './discussing.tsx';
import { doorItems } from './keptdoor.tsx';
import type { Offered } from './rowmenu.tsx';

/** What is set of `channel`, each line once, in the order somebody reaches for them. */
function offered(channel: Channel): Offered[] {
  const all = [
    ...(isConversation(channel) ? conversationItems(channel) : []),
    ...doorItems(channel),
    // A conversation's agents talk nothing over between them: it is opened as a channel first.
    ...(isConversation(channel) ? [] : discussionItems(channel)),
    ...archiveItems(channel),
  ];
  return all.filter((one, at) => all.findIndex((other) => other.label === one.label) === at);
}

/** The part of the Channel tab that sets things of it: nothing where the reader may set nothing. */
export function ChannelSettings({ channel }: { channel: Channel }) {
  const lines = offered(channel);
  if (lines.length === 0) return null;
  return (
    <section class="channel-settings" aria-label="Settings of this channel">
      <h3>Settings</h3>
      {lines.map((line) => (
        <button key={line.label} type="button" class="channel-setting" onClick={line.act}>
          {line.mark && <span class="channel-setting-mark">{line.mark}</span>}
          <span class="truncate grow">{line.label}</span>
        </button>
      ))}
    </section>
  );
}
