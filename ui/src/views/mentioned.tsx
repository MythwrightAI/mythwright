// A name said with `@` in what somebody wrote, drawn as who it names.
//
// Where the name is somebody known (a person with an account, an agent,
// the factory's bot: `namedBy` in `mentions.ts`), it is underlined, and
// pointed at or reached by the keyboard it shows the card the client shows
// of anybody (`Who`); whoever cannot see the card is told who it is in its
// name (`whoIs`). A name of nobody stays the words it was. Pressing it
// shows them in a channel, where there is one to show them in, and does
// nothing elsewhere.

import type { ComponentChildren } from 'preact';
import { namedBy, saidIn, whoIs } from '../mentions.ts';
import type { Named } from '../mentions.ts';
import { peopleKnown } from '../peopleknown.ts';
import { active, agents, speaker, working } from '../store.ts';
import { agentState } from '../util.ts';
import { reads, runsOn, Who } from './who.tsx';

/** Somebody named, underlined, with who they are shown as they are pointed at or have the keyboard. */
function Mention({ one, written }: { one: Named; written: string }) {
  const outside = one.kind === 'agent' ? reads(one.handle) : null;
  const slot = agents.value.find((agent) => agent.name === one.name);
  const about = {
    state: one.kind === 'agent' && !outside && slot ? agentState(one.name in working.value, slot) : null,
    on: one.kind === 'agent' ? (outside ?? runsOn(one.name)) : null,
    takes: one.kind === 'bot',
  };
  // A person or the bot is shown under Channel in the tools panel, which is only beside a channel.
  const still = one.kind !== 'agent' && !active.value.startsWith('channel:');
  return (
    <Who member={{ actor: one.actor, name: one.name }} class="mention" says={`${written}: ${whoIs(one, about)}`} still={still}>
      {written}
    </Who>
  );
}

/**
 * Words with each `@name` of somebody known drawn as a mention, and the
 * rest as `rest` draws it (as they are, unless said). Reads who is known
 * as it is drawn, so a name said before its person was known is drawn
 * once they are.
 */
export function withMentions(words: string, rest: (text: string) => ComponentChildren = (text) => text): ComponentChildren[] {
  const pieces = saidIn(words);
  if (pieces.length === 1 && typeof pieces[0] === 'string') return [rest(words)];
  const known = { people: peopleKnown.value, agents: agents.value, bot: speaker.value?.name };
  // Words around a name of nobody are drawn as one, as they would be with no `@` in them.
  const drawn: ComponentChildren[] = [];
  let plain = '';
  for (const piece of pieces) {
    const one = typeof piece === 'string' ? null : namedBy(piece.handle, known);
    if (typeof piece === 'string' || one === null) {
      plain += typeof piece === 'string' ? piece : piece.written;
      continue;
    }
    if (plain !== '') drawn.push(rest(plain));
    plain = '';
    drawn.push(<Mention one={one} written={piece.written} />);
  }
  if (plain !== '') drawn.push(rest(plain));
  return drawn;
}
