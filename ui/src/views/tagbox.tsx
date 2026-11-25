// The box something is said on a record in, with who can be named offered
// as an `@` is typed, as the channel's composer offers who is in it.
//
// Who is offered is whom being named tells (`taggable` in `mentions.ts`):
// each person with an account and each agent the factory does not start;
// in a topic's reply boxes (`among: 'topic'`, `topical`) every agent as
// well, those the factory starts said to answer when named.
// The daemon decides who a name is and whether they are told (only where
// they may read the record): this is only the typing of one. The list lies
// under the box, over what is under it, or over the box where the window
// has no room for it under (a box at the foot of a drawer), on the page's
// body so that nothing it is drawn in cuts it off, and moves nothing; the arrows go
// through it, Enter and Tab put the one marked in, and Escape puts it away
// before anything else hears the key.
//
// In a topic's reply boxes, `[[` typed offers the pages by title in the
// same list, and the page chosen is written as a link to it
// (`pagelinks.ts`), its title the words.

import { useEffect, useRef, useState } from 'preact/hooks';
import { complete, matching, offeredUnder, taggable, topical, typing } from '../mentions.ts';
import type { Mentionable } from '../mentions.ts';
import { completePage, pagesOffered, pageTyping } from '../pagelinks.ts';
import type { Offered } from '../pagelinks.ts';
import { usePeople } from '../peopleknown.ts';
import { topics, wantTopics } from '../topicsread.ts';
import { treeOfLines } from '../wikipage.ts';
import { agents, me, working } from '../store.ts';
import { agentState, classes } from '../util.ts';
import { besideBox, Floating } from './offered.tsx';
import { reads, runsOn } from './who.tsx';

const GLYPH = { agent: '◈', bot: '⚙', user: '●', system: '◆' };

/** What is said under somebody offered: on a record what being named does there, in a topic how each agent stands and what it runs on. */
function underOf(one: Mentionable, among: 'record' | 'topic'): string {
  if (among === 'record') return one.kind === 'agent' ? 'agent (external)' : one.name;
  if (one.kind !== 'agent') return offeredUnder(one);
  const outside = reads(one.handle);
  const slot = agents.value.find((agent) => agent.name === one.handle);
  const state = outside ? null : agentState(one.handle in working.value, slot);
  return offeredUnder(one, { state, on: outside ?? runsOn(one.handle), answers: !outside && state !== 'stopped' });
}

export function TagBox({
  value,
  onInput,
  label,
  placeholder,
  rows,
  among = 'record',
  page = null,
  onKeyDown,
}: {
  value: string;
  onInput: (text: string) => void;
  label: string;
  placeholder: string;
  rows: number;
  among?: 'record' | 'topic';
  /** In a topic's box, the topic: pages are offered as `[[` is typed, and it is not among them. */
  page?: string | null;
  /** A key the list did not take, while nobody is offered (Ctrl and Enter to send, say). */
  onKeyDown?: (event: KeyboardEvent) => void;
}) {
  const people = usePeople();
  const box = useRef<HTMLTextAreaElement>(null);
  const [at, setAt] = useState<{ from: number; typed: string } | null>(null);
  const [linking, setLinking] = useState<{ from: number; typed: string } | null>(null);
  const [marked, setMarked] = useState(0);
  const writer = me.value?.actor ?? null;
  const pages = among === 'topic';
  useEffect(() => {
    if (pages) wantTopics();
  }, [pages]);
  const offered: Mentionable[] = at ? matching(among === 'topic' ? topical(people, agents.value, writer) : taggable(people, agents.value, writer), at.typed).slice(0, 8) : [];
  const pagesShown: Offered[] = linking ? pagesOffered(topics.value, treeOfLines(topics.value), linking.typed, page) : [];
  const look = (area: HTMLTextAreaElement) => {
    const caret = area.selectionStart;
    const one = area.selectionStart === area.selectionEnd;
    const now = one ? typing(area.value, caret) : null;
    const link = one && pages ? pageTyping(area.value, caret) : null;
    if (now?.typed !== at?.typed || now?.from !== at?.from || link?.typed !== linking?.typed) setMarked(0);
    setAt(now);
    setLinking(link);
  };
  const put = (done: { text: string; caret: number }) => {
    const area = box.current;
    if (!area) return;
    // Put in at once, with the caret after what was put, and only then handed on: drawn again with the same text,
    // the box keeps its caret, where one put a moment later took it back from whatever was typed meanwhile.
    area.value = done.text;
    area.setSelectionRange(done.caret, done.caret);
    area.focus();
    onInput(done.text);
  };
  const choosePage = (one: Offered) => {
    const area = box.current;
    if (!area || !linking) return;
    setLinking(null);
    put(completePage(area.value, area.selectionStart, linking.from, one.page));
  };
  const count = offered.length + pagesShown.length;
  const choose = (one: Mentionable) => {
    const area = box.current;
    if (!area || !at) return;
    setAt(null);
    put(complete(area.value, area.selectionStart, at.from, one.handle));
  };
  return (
    <div class="tagbox">
      <textarea
        ref={box}
        rows={rows}
        value={value}
        aria-label={label}
        placeholder={placeholder}
        aria-autocomplete="list"
        aria-expanded={count > 0}
        onInput={(event) => {
          onInput(event.currentTarget.value);
          look(event.currentTarget);
        }}
        onClick={(event) => look(event.currentTarget)}
        onBlur={() => {
          setAt(null);
          setLinking(null);
        }}
        onKeyDown={(event) => {
          if (count === 0) return onKeyDown?.(event);
          if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
            event.preventDefault();
            const step = event.key === 'ArrowDown' ? 1 : count - 1;
            setMarked((now) => (now + step) % count);
          } else if (event.key === 'Enter' || event.key === 'Tab') {
            event.preventDefault();
            const nth = Math.min(marked, count - 1);
            if (nth < offered.length) choose(offered[nth]!);
            else choosePage(pagesShown[nth - offered.length]!);
          } else if (event.key === 'Escape') {
            // The list's alone: the drawer is not put away with it.
            event.preventDefault();
            event.stopPropagation();
            setAt(null);
            setLinking(null);
          }
        }}
      />
      {count > 0 && (
        // On the page's body, under the box or over it where the window has no room under it (`offered.tsx`): drawn in the
        // tools panel's card form it was cut off by what scrolls there.
        <Floating beside={() => (box.current ? besideBox(box.current.getBoundingClientRect(), 'under') : null)} class="mentions tag-offered" role="listbox" label={offered.length > 0 ? 'Name somebody' : 'Link a page'}>
          {offered.map((one, index) => (
            <button
              key={one.handle}
              type="button"
              role="option"
              aria-selected={index === marked}
              class={classes(index === marked && 'marked')}
              // Before the box loses the caret, which a click would take.
              onMouseDown={(event) => {
                event.preventDefault();
                choose(one);
              }}
              onMouseEnter={() => setMarked(index)}
            >
              <span class={classes('glyph', one.kind)}>{GLYPH[one.kind]}</span>
              <span class="asked-words">
                <strong class="asked-name">@{one.handle}</strong>
                <span class="asked-under">{underOf(one, among)}</span>
              </span>
            </button>
          ))}
          {pagesShown.map((one, index) => (
            <button
              key={one.page.id}
              type="button"
              role="option"
              aria-selected={offered.length + index === marked}
              class={classes(offered.length + index === marked && 'marked')}
              onMouseDown={(event) => {
                event.preventDefault();
                choosePage(one);
              }}
              onMouseEnter={() => setMarked(offered.length + index)}
            >
              <span class="glyph page">¶</span>
              <span class="asked-words">
                <strong class="asked-name">{one.page.title}</strong>
                <span class="asked-under">{one.wiki ? `in ${one.wiki}` : 'a page'}</span>
              </span>
            </button>
          ))}
        </Floating>
      )}
    </div>
  );
}
