// What somebody keeps to hand, at the top of the Resources panel: what
// they pinned, and under it what they lately opened.
//
// Each line is a thing as its tab shows it: its mark and what the tab is
// called, and, where two lines read alike, whose each is, fainter.
// Pressed, it shows its tab. Both lists are of the workspace the panel is
// of, as the lists under them are, with what is the factory's own in
// every one; a section with nothing in it is not drawn.
//
// What was lately opened holds still while the panel is pointed at, or
// the keyboard is in the list: a line that is pressed is the latest from
// that moment, and would jump to the top from under the pointer; and a
// thing opened from the lists below would come in at the top and push
// those lists down from under it. It is put right when the pointer has
// left the panel.
//
// What is in either is one person's own (`latelykept.ts`), and what the
// lists are worked out from is `lately.ts`.

import { signal } from '@preact/signals';
import { useContext, useEffect, useRef, useState } from 'preact/hooks';
import { held } from '../chosen.ts';
import { heldStill, pinnedNow, projectOf, recentAll, recentNow, said } from '../lately.ts';
import type { Line } from '../lately.ts';
import { knownNow, lately, pinning, removeFromRecents } from '../latelykept.ts';
import { workspaces } from '../spaces.ts';
import { active, openTab, projects } from '../store.ts';
import type { Tab } from '../tab.ts';
import { tabKey } from '../tab.ts';
import { classes } from '../util.ts';
import { names } from '../inplace.ts';
import { newTabLines } from './newtab.tsx';
import { sameHeld } from '../workspaces.ts';
import { tabTitle } from '../workbench.tsx';
import { More, offer } from './grouped.tsx';
import { whatIs } from './running.tsx';
import { Section } from './navtree.tsx';
import { SECTIONS } from '../folds.ts';
import { Sought } from './panelseek.tsx';
import { ofKept, shows } from '../panelsought.ts';

/**
 * One thing kept to hand: pressed, it shows its tab; its menu pins it or
 * takes its pin away, and one of Recents is taken out of them. The list
 * held still under the pointer leaves out at once what was taken out of
 * it (`heldStill`), since that can no longer be listed.
 */
function Kept({ line, recent }: { line: Line; recent?: boolean }) {
  const { thing, key, title, whose } = line;
  const current = active.value === key;
  const items = () => [pinning(thing), ...(recent ? [{ label: 'Remove from Recents', act: () => removeFromRecents(thing) }] : []), ...newTabLines(thing)];
  return (
    // Each in a place of its own, as a project's row is in its tree: a row directly in the list is what a project is found by.
    <div class="lately-thing" data-thing={key}>
      <div
        class={classes('nav-row lately-line', current && 'current')}
        onContextMenu={(event) => {
          event.preventDefault();
          offer({ x: event.clientX, y: event.clientY }, items());
        }}
      >
        <button class="nav-item" aria-current={current} data-names={names(thing)} title={whose ? `${title}, of ${whose}` : title} onClick={() => openTab(thing)}>
          <span class="glyph">{tabTitle(thing).glyph}</span>
          <span class="truncate">{title}</span>
          {whose && <span class="lately-whose">{whose}</span>}
        </button>
        <More label={`More for ${whose ? `${title} of ${whose}` : title}`} items={items} />
      </div>
    </div>
  );
}

// Whether the pointer is on the panel the lists are in. A finger does not point: it presses, and the panel is then as it was left.
const pointed = signal(false);
/** What the list of a panel that holds these is given, so that they hold still while it is pointed at. */
export const overPanel = {
  onPointerEnter: (event: PointerEvent) => {
    if (event.pointerType === 'mouse') pointed.value = true;
  },
  onPointerLeave: () => {
    pointed.value = false;
  },
};

/** Pinned, and then Recents: each drawn only where it has something. */
export function Lately() {
  const [kept, known, now, all] = [lately.value, knownNow.value, held.value, workspaces.value];
  // Something running is listed once whose it is was read, and what reads that is its being drawn somewhere. Its tab
  // used to be where: but the tabs are each workspace's own, and one that was opened in another workspace's is on no
  // page here. So what is kept of it asks, once for an id as anything that draws one does.
  for (const thing of [...kept.pinned, ...kept.recent]) if (thing.kind === 'instance') whatIs(thing.id);
  const pinned = pinnedNow(kept, known, now, all);
  // The keyboard is in the list where what has it is marked as having it; a line that was pressed by the pointer has it unmarked.
  const [within, setWithin] = useState(false);
  // A panel that is put away while it is pointed at is pointed at no longer.
  useEffect(() => () => void (pointed.value = false), []);
  // What the list showed when somebody began to point at the panel, or came into the list by keyboard; nothing while nobody is there.
  const was = useRef<string[] | null>(null);
  const latest = recentNow(kept, known, now, all);
  // Another workspace chosen is another list: it is not held to what the one before it showed.
  const of = useRef(now);
  if (!sameHeld(of.current, now)) was.current = null;
  of.current = now;
  if (!pointed.value && !within) was.current = null;
  else was.current ??= latest.map(tabKey);
  const recent = was.current ? heldStill(was.current, recentAll(kept, known, now, all)) : latest;
  const named = (id: string) => projects.value.find((project) => project.id === id)?.name ?? id;
  const whose = (thing: Tab) => {
    const project = projectOf(thing, known);
    return project ? named(project) : null;
  };
  // Whose a line is, is said where another of either list reads the same.
  const lines = said([...pinned, ...recent], (thing) => tabTitle(thing).title, whose);
  // What is typed in the panel's box leaves out each line that does not say it (`panelseek.tsx`), after the list was held still.
  const found = useContext(Sought);
  const pinnedLines = lines.slice(0, pinned.length).filter((line) => shows(found, ofKept.pinned(line.key)));
  const recentLines = lines.slice(pinned.length).filter((line) => shows(found, ofKept.recent(line.key)));
  return (
    <>
      {/* Each folds as the panel's other parts do, and is held open while what was typed in the panel's box is found in it. */}
      {pinnedLines.length > 0 && (
        <Section at={SECTIONS.pinned} label="Pinned" held={found !== null} under={{ class: 'lately lately-pinned' }}>
          {pinnedLines.map((line) => (
            <Kept key={line.key} line={line} />
          ))}
        </Section>
      )}
      {recentLines.length > 0 && (
        <Section
          at={SECTIONS.recents}
          label="Recents"
          held={found !== null}
          under={{
            class: 'lately lately-recent',
            onFocusIn: (event) => setWithin((event.target as Element).matches(':focus-visible')),
            onFocusOut: (event) => !event.currentTarget.contains(event.relatedTarget as Node | null) && setWithin(false),
          }}
        >
          {recentLines.map((line) => (
            <Kept key={line.key} line={line} recent />
          ))}
        </Section>
      )}
    </>
  );
}
