// Choosing the icon a table's column is drawn with.
//
// Whoever may save a view of a table may choose its columns' icons. The
// first line of a column's menu has the column's mark as a button
// (`views/headmenu.tsx`), and pressing it opens a panel under the
// column's head: somewhere to type, which has the caret and narrows the
// icons by name and by the words each is found by; the icons, which
// scroll inside the panel; and, where one was chosen, what takes the
// choice away.
//
// The panel lies over the page and is no part of its layout, so nothing
// moves when it opens, and it stays until an icon is chosen, something
// outside it is pressed, or Escape. It is put under the head that opened
// it and kept wholly in the window, at a phone's width too. An icon is
// the field's, for everybody (`icons.ts`): choosing one asks the daemon,
// and every page that shows the table follows what it then says.

import { createPortal } from 'preact/compat';
import { useEffect, useId, useMemo, useRef, useState } from 'preact/hooks';
import { calledIcon, found, mayChoose, moved, placed } from '../icons.ts';
import { chooseIcon, iconSet, wantIcons } from '../iconset.ts';
import { attempt, collections, me, refreshCollections } from '../store.ts';
import { heading } from '../table.ts';
import type { Field } from '../table.ts';
import { classes } from '../util.ts';
import { IconDrawing } from './mark.tsx';

/** A table whose columns' icons can be chosen, and what is done once one has been, where the page keeps the table itself. */
export interface Choosing {
  project: string;
  name: string;
  then?: () => void;
}

/** The table, where whoever is signed in may choose its columns' icons; nothing, where they may not or no table was said. */
export function useChoosing(of: Choosing | undefined): Choosing | null {
  if (!of) return null;
  const table = (collections.value[of.project] ?? []).find((one) => one.name === of.name);
  return mayChoose(me.value?.principal.scopes ?? [], table ?? { name: of.name }) ? of : null;
}

/**
 * The icons to choose from for a column, opened from the column's menu
 * and put under its head (`opener`): one chosen, or the choice taken
 * away, asks the daemon and puts the panel away.
 */
export function IconPick({ of, field, opener, onClose }: { of: Choosing; field: Field; opener: { current: HTMLElement | null }; onClose: () => void }) {
  const close = () => {
    onClose();
    // The caret goes back to the head it was opened from, and the page stays where it is.
    opener.current?.focus({ preventScroll: true });
  };
  const choose = async (icon: string | null) => {
    close();
    if ((await attempt(() => chooseIcon(of.project, of.name, field.name, icon))) === undefined) return;
    // Every page hears of it and reads the table again; this one does not wait to be told.
    void refreshCollections(of.project);
    of.then?.();
  };
  return <IconPicker opener={opener} called={heading(field)} chosen={field.icon_chosen ?? null} onChoose={(icon) => void choose(icon)} onClose={close} />;
}

/** How many icons are drawn to begin with, and how many more each time the end of them is scrolled to. */
const DRAWN = 240;
/**
 * How wide and tall the panel is, where the window has the room: tall
 * enough that the icons shown are whole rows of them, eight under a
 * pointer and seven of the larger ones under a finger (`icons.css` says
 * how tall what is typed in, a row and the foot are).
 */
const WIDE = 320;
const tall = () => (window.matchMedia('(pointer: coarse)').matches ? 394 : 366);

interface PickerProps {
  /** What opened it, which it is put under. */
  opener: { current: HTMLElement | null };
  /** What the column is called. */
  called: string;
  /** The icon somebody chose for the column, if one was. */
  chosen: string | null;
  /** An icon by its name; or none, to take the choice away. */
  onChoose: (icon: string | null) => void;
  onClose: () => void;
}

function IconPicker({ opener, called, chosen, onChoose, onClose }: PickerProps) {
  const id = useId();
  const panel = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const grid = useRef<HTMLDivElement>(null);
  const [typed, setTyped] = useState('');
  const [marked, setMarked] = useState(0);
  // Whether the mark was moved up or down since something was typed: until then the arrows to either side are the caret's.
  const [aimed, setAimed] = useState(false);
  const [drawn, setDrawn] = useState(DRAWN);
  const where = () => {
    const size = { width: Math.min(WIDE, window.innerWidth - 16), height: Math.min(tall(), window.innerHeight - 16) };
    const at = opener.current?.getBoundingClientRect() ?? { left: 8, top: 8, bottom: 8 };
    return { ...size, ...placed(at, size, { width: window.innerWidth, height: window.innerHeight }) };
  };
  const [place, setPlace] = useState(where);

  const set = iconSet.value;
  useEffect(() => wantIcons(), []);
  const names = useMemo(() => (set ? found(set, typed) : []), [set, typed]);
  const at = Math.min(marked, Math.max(0, names.length - 1));

  useEffect(() => {
    input.current?.focus({ preventScroll: true });
    const inside = (target: EventTarget | null) => target instanceof Node && (panel.current?.contains(target) || opener.current?.contains(target));
    // What opened it closes it by its own press; anything else outside it does here.
    const away = (event: Event) => {
      if (!inside(event.target)) onClose();
    };
    const key = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose();
    };
    // The window resized, or what is under the panel scrolled: it is put where its opener now is.
    const put = (event: Event) => {
      if (event.type === 'scroll' && event.target instanceof Node && panel.current?.contains(event.target)) return;
      setPlace(where());
    };
    window.addEventListener('pointerdown', away, true);
    window.addEventListener('keydown', key);
    window.addEventListener('resize', put);
    window.addEventListener('scroll', put, true);
    return () => {
      window.removeEventListener('pointerdown', away, true);
      window.removeEventListener('keydown', key);
      window.removeEventListener('resize', put);
      window.removeEventListener('scroll', put, true);
    };
  }, []);

  // The icon that is marked is kept in sight as the mark moves.
  useEffect(() => {
    const box = grid.current;
    const one = box?.querySelector<HTMLElement>('[aria-selected="true"]');
    if (!box || !one) return;
    const top = one.offsetTop - box.offsetTop;
    if (top < box.scrollTop) box.scrollTop = top - 4;
    else if (top + one.offsetHeight > box.scrollTop + box.clientHeight) box.scrollTop = top + one.offsetHeight - box.clientHeight + 4;
  }, [at, drawn]);

  /** How many icons there are to a row, as they are laid out now. */
  const columns = () => {
    const all = grid.current?.querySelectorAll<HTMLElement>('[role="option"]') ?? [];
    let count = 0;
    for (const one of all) {
      if (one.offsetTop !== all[0]!.offsetTop) break;
      count += 1;
    }
    return Math.max(1, count);
  };
  const onKey = (event: KeyboardEvent) => {
    if (event.key === 'Enter') {
      event.preventDefault();
      const name = names[at];
      if (name) onChoose(name);
      return;
    }
    const upOrDown = event.key === 'ArrowDown' || event.key === 'ArrowUp';
    if (!upOrDown && !(aimed && (event.key === 'ArrowLeft' || event.key === 'ArrowRight'))) return;
    event.preventDefault();
    const next = moved(at, event.key, columns(), names.length);
    setAimed(true);
    setMarked(next);
    // The mark may go past what is drawn: more of them are drawn then.
    if (next >= drawn) setDrawn((Math.floor(next / DRAWN) + 1) * DRAWN);
  };

  return createPortal(
    <div ref={panel} class="iconpick" role="dialog" aria-label={`Icon of ${called}`} style={`left:${place.left}px;top:${place.top}px;width:${place.width}px;height:${place.height}px`}>
      <input
        ref={input}
        class="iconpick-filter"
        type="text"
        role="combobox"
        aria-label={`Find an icon for ${called}`}
        aria-expanded="true"
        aria-autocomplete="list"
        aria-controls={`${id}-icons`}
        aria-activedescendant={names[at] ? `${id}-${names[at]}` : undefined}
        placeholder="Find an icon"
        autocomplete="off"
        spellcheck={false}
        value={typed}
        onInput={(event) => {
          setTyped(event.currentTarget.value);
          setMarked(0);
          setAimed(false);
          setDrawn(DRAWN);
          if (grid.current) grid.current.scrollTop = 0;
        }}
        onKeyDown={onKey}
      />
      <div
        ref={grid}
        id={`${id}-icons`}
        class="iconpick-grid"
        role="listbox"
        aria-label="Icons"
        onScroll={(event) => {
          // More are drawn as the end of those that are is come to.
          const box = event.currentTarget;
          if (drawn < names.length && box.scrollTop + box.clientHeight > box.scrollHeight - 160) setDrawn(drawn + DRAWN);
        }}
      >
        {set &&
          names.slice(0, drawn).map((name, index) => (
            <button
              key={name}
              id={`${id}-${name}`}
              type="button"
              role="option"
              class={classes('iconpick-one', name === chosen && 'chosen')}
              aria-selected={index === at}
              aria-label={calledIcon(name)}
              title={calledIcon(name)}
              tabIndex={-1}
              // The caret stays where one types: a press chooses and takes it nowhere.
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => onChoose(name)}
            >
              <IconDrawing name={name} nodes={set[name]![0]} size={18} />
            </button>
          ))}
        {!set && <div class="iconpick-none faint">Reading the icons</div>}
        {set && names.length === 0 && <div class="iconpick-none faint">No icon is called that, or found by it.</div>}
      </div>
      <div class="iconpick-foot">
        <span class="grow truncate" aria-live="polite">
          {names[at] ? calledIcon(names[at]) : ''}
        </span>
        {set && <span class="faint">{names.length === 1 ? '1 icon' : `${names.length} icons`}</span>}
        {chosen && (
          <button type="button" class="ghost" title="Remove the choice: the column is drawn as its table says" onClick={() => onChoose(null)}>
            Remove
          </button>
        )}
      </div>
    </div>,
    document.body,
  );
}
