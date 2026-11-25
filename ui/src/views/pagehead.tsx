// What stands at the right of a page's head, the same on every page of a
// thing: when it was last changed, what it is shared by, an address that
// opens it, its pin, and a gear that opens what else can be done with it.
//
// A page says what it is (`tab`) and hands the rest. Its address and its
// pin are that thing's and nothing is handed for them: the pin is the one
// the side panel lists under Pinned (`latelykept.ts`), so pinned here it
// is there at once, and the other way round. When it was changed, what it
// is shared by and the lines of its menu are the page's to say, and each
// is drawn only where something is behind it: a page with nothing to
// share has no Share, and no line of the menu is there that does nothing.
//
// It goes at the end of the head's first line, after what gives way, and
// ends that line: what the page had in its head before (its views, what
// adds a record) is on the line after it, at the right as it always was,
// so that what those open still opens toward the page and not off its
// left edge. A page with nothing after it says so (`last`).
//
// The gear is the one thing a head has to press for what else can be done
// with its page, so what a page once had a button of its own for is a line
// of the gear's menu (`more`: the tables that sit beside a table, what can
// be done to a board). And what such a line opens that is more than a
// line, how a view is laid out, hangs under the gear where the menu was
// (`hung`): from the line the head's first line ends in, at its right
// end, where it lies over the page and moves nothing.

import type { ComponentChildren } from 'preact';
import { useEffect, useRef, useState } from 'preact/hooks';
import { linkTo } from '../address.ts';
import { api } from '../api.ts';
import { edited as editedOf, lastOf, newest, saidOf } from '../edited.ts';
import type { Edited, Last } from '../edited.ts';
import { isPinned } from '../lately.ts';
import { lately, pinThing } from '../latelykept.ts';
import { narrowTo, openTab, projects, toast } from '../store.ts';
import type { Tab } from '../tab.ts';
import { useEvents } from '../ui.tsx';
import { offer, offeredBy, putAway } from './grouped.tsx';
import type { Offered, Said } from './grouped.tsx';
import { signs } from './signs.tsx';
import { FollowButton } from './followbutton.tsx';
import { followableOf } from '../following.ts';

/** How wide what the gear and Share open is. */
const WIDE = 252;

/** Puts some words where they can be pasted from, and says so; or says that it could not. */
export async function copy(text: string, done: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    toast(done);
  } catch {
    // A page that is not served from this device or over a sealed link is given no clipboard: the words are said, to be copied by hand.
    toast(`It could not be copied here: ${text}`, true);
  }
}

/** The project a thing is of, where it is of one. */
function projectOf(tab: Tab): string | null {
  if (tab.kind === 'project') return tab.id;
  return 'project' in tab && tab.project ? tab.project : null;
}

/**
 * When a table was last changed and by whom, as far as its page knows
 * (`edited.ts`): from its rows as the page read them and from when the
 * table itself was last written; who, from the last line of its newest
 * record's history, read once, and after that from each line as it is
 * said to have been written. Nothing until the rows are read.
 */
export function useEdited(project: string, name: string, own: number | undefined, rows: { id: string; updated: number }[] | null): Edited | null {
  const [read, setRead] = useState<Last | null>(null);
  const [heard, setHeard] = useState<Last | null>(null);
  const top = rows ? newest(rows) : null;
  const known = top !== null;
  useEffect(() => {
    setRead(null);
    setHeard(null);
    if (!top) return;
    let stale = false;
    // Whoever may not read its history is told nothing of who: when is still said.
    void api.history(top.id).then(
      (lines) => !stale && setRead(lastOf(lines)),
      () => {},
    );
    return () => {
      stale = true;
    };
  }, [project, name, known]);
  useEvents(
    ['collection.history'],
    (event) => {
      if (event.data?.project === project && event.data?.name === name && typeof event.data?.who === 'string') setHeard({ who: event.data.who, ts: event.ts });
    },
    [project, name],
  );
  return rows === null ? null : editedOf([own, top?.updated], [read, heard]);
}

/**
 * Tells whatever is open over the page (a column's menu, a view's layout)
 * what a press outside it would tell it. Asked before something else is
 * opened over the page by a key or by a line of a menu: it would otherwise
 * lie beside the other, and one Escape would be for both.
 */
export function pressOutside(): void {
  document.dispatchEvent(new Event('pointerdown', { bubbles: true }));
}

/** The lines of a menu: what is pressed, and what is only said. */
export type Lines = (Offered | Said)[];

/** Something a menu says that may be longer than a line: read whole, on as many lines as it takes. */
/**
 * What a page's head is called and what stands beside it (how many it
 * holds, where it is, whether it is trusted, how it is shown): one thing
 * on the head's first line, whose words stand on the title's baseline,
 * centred on the line as what is at its right (`PageActs`) is. Left as
 * the head's own items each was centred on the line by its box, and words
 * smaller than the title sat higher than it.
 */
export function Titled({ children }: { children: ComponentChildren }) {
  return <div class="head-title">{children}</div>;
}

export const note = (words: string, apart = false): Said => ({ said: <span class="menu-note">{words}</span>, apart });

/**
 * A tool of the page that opens a panel beside it (a document's comments,
 * details and history): drawn just after the gear, the last of the head's row, pressed while the panel is
 * open, with a count beside its drawing where it says one.
 */
export interface Tool {
  /** What it is called for whoever hears it read out. */
  label: string;
  title: string;
  mark: ComponentChildren;
  count?: number;
  pressed: boolean;
  onPress: () => void;
}

/** A mark of the client's own that stands for a kind of thing wherever it is listed, as a line's drawing. */
export const glyph = (mark: string) => <span class="glyph">{mark}</span>;

interface Props {
  /** What the page is of: its address and its pin are that thing's. */
  tab: Tab;
  /** When it was last changed and by whom, where the page knows; and what that leaves out, said to whoever points at it. */
  edited?: Edited | null;
  blind?: string;
  /** Something of it said in quiet words beside that, with more for whoever points at it: that a table is kept private. */
  quiet?: { words: string; more: string } | null;
  /** What it is shared by, where there is anything: the lines Share opens, as things stand when it is pressed. */
  share?: () => Lines | Promise<Lines>;
  /** The page's own lines of the gear's menu. */
  menu?: () => Lines;
  /** What the page once had buttons of its own for, after every line that opens something else: under what the menu had, and over when it was changed. */
  more?: () => Lines;
  /** What hangs under the gear while a line of its menu has opened it there, handed the gear, which has the keyboard again when it is put away. */
  hung?: (gear: HTMLButtonElement | null) => ComponentChildren;
  /** Nothing of the head comes after it on a line it would share: no line is begun for buttons that are not there. */
  last?: boolean;
  /** A panel the page opens beside itself, by a button just after the gear, at the end of the head's row. */
  tool?: Tool;
}

export function PageActs({ tab, edited, blind, quiet, share, menu, more, hung, last, tool }: Props) {
  const gear = useRef<HTMLButtonElement>(null);
  const sharing = useRef<HTMLButtonElement>(null);
  const pinned = isPinned(lately.value, tab);
  const link = linkTo(tab);
  const said = edited ? saidOf(edited) : null;
  const project = projectOf(tab);
  const called = project ? (projects.value.find((one) => one.id === project)?.name ?? project) : null;
  const copyLink = () => link && void copy(link, 'Copied. It opens this page for whoever is signed in to this factory here.');
  const opens = async (from: HTMLButtonElement | null, label: string, items: () => Lines | Promise<Lines>) => {
    if (!from) return;
    if (offeredBy(from)) return putAway();
    // Whatever else is open over the page (a column's menu, a view's layout, what hangs under the gear) is put away first.
    pressOutside();
    const offered = await items();
    const at = from.getBoundingClientRect();
    // As much wider as text is larger, so that a line whole at the usual size is whole at the largest; the menu keeps to the window.
    const size = Number.parseFloat(getComputedStyle(from).getPropertyValue('--size'));
    const wide = Math.min(Math.round(WIDE * Math.max(1, Number.isFinite(size) ? size : 1)), window.innerWidth - 8);
    offer({ x: at.right - wide - 2, y: at.bottom + 4 }, offered, { from, label, wide, fit: true });
  };
  const lines = (): Lines => {
    const all: Lines = [];
    if (link) all.push({ label: 'Copy link', mark: signs.link(), act: copyLink });
    all.push(...(menu?.() ?? []));
    if (project) {
      // What is running and what was measured, held to this thing's project as that project's own page holds them.
      const narrowed = (to: Tab) => () => {
        narrowTo(project);
        openTab(to);
      };
      // Each by the mark its line of the side panel has.
      all.push(
        { label: `Activity of ${called}`, mark: glyph('↻'), apart: all.length > 0, act: narrowed({ kind: 'activity' }) },
        { label: `Measurements of ${called}`, mark: glyph('◔'), act: narrowed({ kind: 'metrics' }) },
      );
    }
    all.push(...(more?.() ?? []));
    if (said) all.push(note(said.whole, all.length > 0));
    return all;
  };
  const hanging = hung?.(gear.current);
  // A board, a table, a library and a project are followed from their head; a channel and a page of a wiki from their own.
  const followable = followableOf(tab);
  return (
    <>
      <div class="head-acts">
        {said && (
          <span class="head-edited" data-at={edited?.at} title={`${said.whole}.${blind ? ` ${blind}` : ''}`}>
            {said.short}
          </span>
        )}
        {quiet && (
          <span class="head-quiet" title={quiet.more}>
            {quiet.words}
          </span>
        )}
        {followable && <FollowButton thing={followable} />}
        {share && (
          <button ref={sharing} type="button" class="ghost head-share" aria-haspopup="menu" aria-expanded={offeredBy(sharing.current)} title="What this is shared by" onClick={() => void opens(sharing.current, 'Share', share)}>
            {signs.share()}
            Share
          </button>
        )}
        {link && (
          <button type="button" class="ghost icon" aria-label="Copy link" title="Copy an address that opens this page" onClick={copyLink}>
            {signs.link()}
          </button>
        )}
        {link && (
          <button
            type="button"
            class="ghost icon head-pin"
            aria-label={pinned ? 'Unpin' : 'Pin'}
            aria-pressed={pinned}
            title={pinned ? 'Pinned at the top of Resources. Press to unpin.' : 'Pin to the top of Resources'}
            onClick={() => pinThing(tab, !pinned)}
          >
            {signs.pin(pinned)}
          </button>
        )}
        <button ref={gear} type="button" class="ghost icon" aria-label="More for this page" aria-haspopup="menu" aria-expanded={offeredBy(gear.current) || Boolean(hanging)} title="What else can be done with this" onClick={() => void opens(gear.current, 'More for this page', lines)}>
          {signs.gear()}
        </button>
        {tool && (
          <button type="button" class={tool.count ? 'ghost icon head-tool counted' : 'ghost icon head-tool'} aria-label={tool.label} aria-pressed={tool.pressed} title={tool.title} onClick={tool.onPress}>
            {tool.mark}
            {tool.count ? <span class="head-tool-count">{tool.count}</span> : null}
          </button>
        )}
      </div>
      {/* What hangs under the gear is the break's: it is as wide as the head and ends where the gear does. So a page that
          hangs something there has the break though nothing of its head comes after on that line: a line of no height. */}
      {(!last || hung) && <span class="head-break">{hanging}</span>}
      {!last && <span class="grow" />}
    </>
  );
}
