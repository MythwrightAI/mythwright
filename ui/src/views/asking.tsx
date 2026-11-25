// Who can be asked in a channel, behind one button among the composer's
// tools: the agents and the bots, each with how it stands just now. Choosing
// one begins asking it. The button says, by a mark, when one of them is at
// work, so that is known without the list being opened.

import { useEffect, useRef, useState } from 'preact/hooks';
import { type Mentionable, under } from '../mentions.ts';
import { agents, speaker, working } from '../store.ts';
import { agentState, classes } from '../util.ts';
import { besideTool, Floating } from './offered.tsx';
import { reads, runsOn, Standing } from './who.tsx';

// Not to take the caret out of the box, which a press on a button would.
const stay = (event: Event) => event.preventDefault();

export function Asking({ those, onAsk }: { those: Mentionable[]; onAsk: (handle: string) => void }) {
  const [open, setOpen] = useState(false);
  const menu = useRef<HTMLSpanElement>(null);
  // The list is on the page's body (`offered.tsx`): a press in it is not one outside.
  const list = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const away = (event: Event) => {
      const inside = menu.current?.contains(event.target as Node) || list.current?.contains(event.target as Node);
      if (event instanceof KeyboardEvent ? event.key === 'Escape' : !inside) setOpen(false);
    };
    window.addEventListener('pointerdown', away);
    window.addEventListener('keydown', away);
    return () => {
      window.removeEventListener('pointerdown', away);
      window.removeEventListener('keydown', away);
    };
  }, [open]);
  if (those.length === 0) return null;
  const busy = those.filter((one) => one.kind === 'agent' && one.handle in working.value).length;
  const says = busy === 0 ? 'Ask an agent or a bot' : busy === 1 ? 'Ask an agent or a bot. One is working' : `Ask an agent or a bot. ${busy} are working`;
  return (
    <span class="asking" ref={menu}>
      <button type="button" class={classes('ghost icon asking-button', open && 'on')} aria-label={says} data-tip={says} aria-haspopup="menu" aria-expanded={open} onMouseDown={stay} onClick={() => setOpen(!open)}>
        @{busy > 0 && <span class="asking-mark" aria-hidden="true" />}
      </button>
      {open && (
        // Over what is above it, moving nothing, and over whatever the composer is drawn in: the tools are at the foot of the window.
        <Floating beside={() => besideTool(menu.current?.querySelector('.asking-button') ?? null)} own={list} class="asking-menu" role="menu" label="Agents and bots in this channel">
          {those.map((one) => {
            // One the factory does not start is not asked: it is said to read what is said, and how it stands is not a turn's to say.
            const outside = one.kind === 'agent' ? reads(one.handle) : null;
            const state = one.kind === 'agent' && !outside ? agentState(one.handle in working.value, agents.value.find((slot) => slot.name === one.handle)) : null;
            const on = one.kind === 'agent' ? (outside ?? runsOn(one.handle)) : null;
            return (
              <button
                key={one.handle}
                type="button"
                role="menuitem"
                class="ghost"
                onMouseDown={stay}
                onClick={() => {
                  setOpen(false);
                  onAsk(one.handle);
                }}
              >
                {one.kind === 'agent' ? <Standing agent={one.handle} /> : <span class="asking-bot" aria-hidden="true">◇</span>}
                {/* What is said of it is under its name and whole: beside the name it was cut off where the list is narrow. */}
                <span class="asked-words">
                  <strong class="asked-name">{one.handle}</strong>
                  <span class="asked-under">{under(one, { state, on, takes: speaker.value?.name === one.handle })}</span>
                </span>
              </button>
            );
          })}
        </Floating>
      )}
    </span>
  );
}
