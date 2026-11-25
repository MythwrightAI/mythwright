// The one menu there is at a time: what a row of the side panel opens,
// and every menu like it (the foot's, Help's, a page's gear). It lies over
// the page from the frame, and not in what opened it: the side panel
// scrolls, and would cut off what reached past its edge.
//
// A line may open a menu of its own beside it (`sub`): on the pointer
// resting on it, a press, Enter or the right arrow. It is put beside the
// line on the side that has room (`besideAt` in `submenu.ts`), stays while
// the pointer crosses other lines on its way there, and the left arrow or
// Escape puts it away and gives the keyboard back to its line. Its lines
// are known by where they are, as the menu's are: a workspace may be
// called what a line is.

import { signal } from '@preact/signals';
import type { ComponentChildren } from 'preact';
import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { besideAt, towards } from '../submenu.ts';
import type { Point } from '../submenu.ts';
import { classes } from '../util.ts';

export interface Offered {
  label: string;
  /** One of several of which one is how things are. The one that is, is marked, and choosing it changes nothing. */
  chosen?: boolean;
  /** How many wait unread under it, said beside its name where there are any. */
  count?: number;
  /** The first of another kind of thing than those above it: a line is drawn between. */
  apart?: boolean;
  /** A small drawing before its name. Where any line of a menu has one, every line keeps the room for one, so that the names begin together. */
  mark?: ComponentChildren;
  /** How it is drawn where that is not as the rest are: something that takes away, or one of a list joined down its left (its first, one between, its last). */
  look?: 'grave' | 'joined-first' | 'joined' | 'joined-last';
  /** A few faint words at the end of its line: what it would open, or how much it is of. */
  hint?: string;
  /** Faint words under its name, read whole on as many lines as they take: what it is, where that is longer than a hint. */
  under?: string;
  /** A switch, and whether it is on now. Pressed, it does what it does and the menu stays, saying how it then is. */
  on?: () => boolean;
  /** It is offered and cannot be pressed as things stand: why not, said to whoever points at it. */
  closed?: string;
  /** How many steps in it is drawn, as one of a list that is a tree (the folders of a project's outline): nothing for none. */
  indent?: number;
  /** What it opens beside itself, as a menu of its own: what there was when the menu was opened. Its own `act` is not asked. */
  sub?: (Offered | Said)[];
  act: () => void;
}

/** A line of a menu that only says something: there is nothing to press, and the keyboard passes it by. */
export interface Said {
  said: ComponentChildren;
  /** The first of another kind of thing than those above it: a line is drawn between. */
  apart?: boolean;
}

/** How a menu is opened where it is not a row's: upward from where it is, as wide as what opened it, called something, and with the keyboard. */
interface Opened {
  /** It lies above where it is, its foot there: for what is at the foot of the page. */
  up?: boolean;
  /** At least this wide: as wide as what opened it. */
  wide?: number;
  /** What it is a menu of, for whatever reads the page out. */
  label?: string;
  /** What opened it, which has the keyboard again when it is put away; pressed again, it puts the menu away. */
  from?: HTMLElement;
  /** It is kept on the page by how tall it is drawn, and not by how many lines it has: for one whose lines are not all as tall. */
  fit?: boolean;
}

type Shown = { x: number; y: number; items: (Offered | Said)[] } & Opened;

const menu = signal<Shown | null>(null);

export function offer(at: { x: number; y: number }, items: (Offered | Said)[], how: Opened = {}): void {
  menu.value = { ...at, items, ...how };
}

/** Whether the menu that is open was opened by this. */
export function offeredBy(from: HTMLElement | null): boolean {
  // Read whatever is asked, so that whoever asks while it draws is drawn again when a menu comes or goes.
  const by = menu.value?.from;
  return from !== null && by === from;
}

/** Puts the menu away. */
export function putAway(): void {
  menu.value = null;
}

/** The least room under what opened a fitted menu in which it stays there and scrolls in itself, where it is taller than that. */
const UNDER = 320;
/** How long the pointer rests on a line before what it opens is opened. */
const OPEN_AFTER = 80;
/** How long what a line opened stays while the pointer is on another line: the way across to it, which may pass over a line or two. */
const STAY_FOR = 350;

/** Which line opened the menu beside it, of which menu, and whether the keyboard goes into it. */
type Beside = { of: Shown; at: number; keys: boolean };

const buttons = (box: HTMLElement | null) => [...(box?.querySelectorAll<HTMLButtonElement>(':scope > button') ?? [])];

/** The menu a row of the side panel opened, drawn once in the frame. */
export function RowMenu() {
  const shown = menu.value;
  const drawn = useRef<HTMLDivElement>(null);
  const besideDrawn = useRef<HTMLDivElement>(null);
  // How tall one that is fitted was drawn, once it was; and how often a switch of it was pressed, to say how it then is.
  const [tall, setTall] = useState<number | null>(null);
  const [, setPressed] = useState(0);
  const [beside, setBeside] = useState<Beside | null>(null);
  const opened = beside && beside.of === shown ? beside : null;
  // Read by what listens for keys, which is made once for a menu.
  const now = useRef<Beside | null>(null);
  now.current = opened;
  const last = useRef<Point | null>(null);
  const timers = useRef<{ open?: ReturnType<typeof setTimeout>; shut?: ReturnType<typeof setTimeout> }>({});
  const clear = (which: 'open' | 'shut') => {
    clearTimeout(timers.current[which]);
    timers.current[which] = undefined;
  };
  useEffect(() => () => (clear('open'), clear('shut')), []);
  useLayoutEffect(() => setTall(shown?.fit ? (drawn.current?.offsetHeight ?? null) : null), [shown]);
  const lineOf = (at: number) => drawn.current?.children[at] as HTMLElement | undefined;
  const openBeside = (at: number, keys: boolean) => {
    clear('open');
    clear('shut');
    if (shown) setBeside({ of: shown, at, keys });
  };
  const shutBeside = (focusLine: boolean) => {
    clear('open');
    clear('shut');
    const was = now.current;
    setBeside(null);
    if (focusLine && was) (lineOf(was.at) as HTMLButtonElement | undefined)?.focus();
  };
  // Put beside its line before it is drawn, so that it is never seen anywhere else; and given the keyboard where it was opened by it.
  useLayoutEffect(() => {
    const box = besideDrawn.current;
    const line = opened ? lineOf(opened.at) : undefined;
    if (!box || !line || !drawn.current) return;
    const place = besideAt(drawn.current.getBoundingClientRect(), line.getBoundingClientRect(), { width: box.offsetWidth, height: box.scrollHeight }, { width: window.innerWidth, height: window.innerHeight });
    box.style.left = `${place.left}px`;
    box.style.top = `${place.top}px`;
    box.style.maxHeight = `${place.most}px`;
    box.dataset.side = place.side;
    box.style.visibility = '';
    if (opened?.keys) (buttons(box).find((one) => one.getAttribute('aria-disabled') !== 'true') ?? buttons(box)[0])?.focus();
  }, [opened?.of, opened?.at, opened?.keys]);
  useEffect(() => {
    if (!shown) return;
    const within = () => [...(drawn.current?.querySelectorAll('button') ?? [])];
    // Opened by something that keeps the keyboard, it takes the keyboard: at what is chosen (a switch that is on is not that), or at its first.
    if (shown.from) (within().find((one) => one.getAttribute('role') === 'menuitemradio' && one.getAttribute('aria-checked') === 'true') ?? within()[0])?.focus();
    const away = (event: Event) => {
      const target = event.target as HTMLElement | null;
      // A press on what opened it is that thing's to answer: it puts the menu away itself.
      if (!target?.closest?.('.row-menu') && !shown.from?.contains(target)) menu.value = null;
    };
    const onKey = (event: KeyboardEvent) => {
      const side = now.current;
      const inSide = !!side && !!besideDrawn.current?.contains(document.activeElement);
      // One Escape puts one thing away, and it is heard first, by `besideFirst`.
      if (event.key === 'ArrowLeft' && inSide) {
        event.preventDefault();
        return shutBeside(true);
      }
      if (event.key === 'ArrowRight' && !inSide) {
        const line = (document.activeElement as HTMLElement | null)?.closest?.('.row-menu > *');
        const index = line && drawn.current?.contains(line) ? [...drawn.current.children].indexOf(line) : -1;
        if (index >= 0 && (shown.items[index] as Offered | undefined)?.sub) {
          event.preventDefault();
          openBeside(index, true);
        }
        return;
      }
      // The arrows go through what it offers, and round again; Home and End to its ends.
      const step = event.key === 'ArrowDown' ? 1 : event.key === 'ArrowUp' ? -1 : 0;
      if (!step && event.key !== 'Home' && event.key !== 'End') return;
      const all = inSide ? buttons(besideDrawn.current) : within();
      if (all.length === 0) return;
      event.preventDefault();
      // Gone on from in the menu, what a line opened beside it is put away.
      if (side && !inSide) shutBeside(false);
      const at = all.indexOf(document.activeElement as HTMLButtonElement);
      const next = event.key === 'Home' ? 0 : event.key === 'End' ? all.length - 1 : at < 0 ? (step > 0 ? 0 : all.length - 1) : (at + step + all.length) % all.length;
      all[next]!.focus();
    };
    // Heard before anything else hears the key (the frame puts a drawer away on it): the menu it was opened from stays.
    // And the menu itself is put away before anything under it hears the key: a drawer open beside the page went with it.
    const besideFirst = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.stopPropagation();
      event.preventDefault();
      if (now.current) return shutBeside(true);
      menu.value = null;
      shown.from?.focus();
    };
    window.addEventListener('pointerdown', away, true);
    window.addEventListener('keydown', besideFirst, true);
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('pointerdown', away, true);
      window.removeEventListener('keydown', besideFirst, true);
      window.removeEventListener('keydown', onKey);
    };
  }, [shown]);
  if (!shown) return null;
  // Kept on the page, whatever corner it was opened in.
  const width = Math.max(232, (shown.wide ?? 0) + 4);
  const left = Math.max(4, Math.min(shown.x, window.innerWidth - width));
  // One that is fitted and taller than the room under where it was opened stays there, under what opened it, and scrolls in
  // itself, while there is room enough to read it in: moved up the page it lay over the very thing that opened it. With less
  // room than that it is moved up, as any menu is.
  const room = window.innerHeight - shown.y - 8;
  const under = Boolean(shown.fit) && tall !== null && tall > room && room >= UNDER;
  const top = under ? Math.max(4, shown.y) : Math.max(4, Math.min(shown.y, window.innerHeight - (shown.fit && tall !== null ? tall + 8 : 30 * shown.items.length + 16)));
  // One that opens upward stands on where it was opened, and scrolls in itself where there is not the height for all of it.
  const place = shown.up
    ? `left:${left}px;bottom:${window.innerHeight - shown.y}px;max-height:${Math.max(120, shown.y - 8)}px;overflow-y:auto`
    : `left:${left}px;top:${top}px${shown.fit ? `;max-height:${under ? room : window.innerHeight - 8}px;overflow-y:auto` : ''}`;
  const size = shown.wide ? `;min-width:${Math.min(shown.wide, window.innerWidth - 8)}px;max-width:${Math.min(Math.max(shown.wide, 228), window.innerWidth - 8)}px` : '';
  const done = (item: Offered) => {
    menu.value = null;
    shown.from?.focus();
    if (!item.chosen) item.act();
  };
  // The pointer over the menu while a line of it has another open beside it: moving on its way there across other lines, what
  // was opened stays; resting on another line, or moving off that way, it goes a moment later. Over its own line it stays.
  const crossing = (event: PointerEvent) => {
    const was = last.current;
    last.current = { x: event.clientX, y: event.clientY };
    if (event.pointerType === 'touch' || !now.current || !besideDrawn.current) return;
    if (lineOf(now.current.at)?.contains(event.target as Node)) return clear('shut');
    // On its way the moment begins again with each move, so that it goes where the pointer stops short of it.
    if (was && towards(was, last.current, besideDrawn.current.getBoundingClientRect(), (besideDrawn.current.dataset.side ?? 'over') as 'right' | 'left' | 'over')) clear('shut');
    timers.current.shut ??= setTimeout(() => shutBeside(false), STAY_FOR);
  };
  const opensBeside = opened ? (shown.items[opened.at] as Offered | undefined) : undefined;
  return (
    <>
      <div class="row-menu" role="menu" aria-label={shown.label} ref={drawn} style={place + size} onPointerMove={(event) => crossing(event)}>
        {/* By where each is: two things a menu offers may be called the same, and what it offers is what there was when it was opened. */}
        {shown.items.map((item, at) => (
          <Line
            key={at}
            item={item}
            marked={shown.items.some((one) => 'label' in one && one.mark !== undefined)}
            open={opened?.at === at}
            onRest={(pointer) => {
              if (pointer === 'touch') return;
              if (!('label' in item) || !item.sub) return;
              clear('shut');
              clear('open');
              timers.current.open = setTimeout(() => openBeside(at, false), OPEN_AFTER);
            }}
            onLeave={() => clear('open')}
            onPress={async (keys) => {
              if (!('label' in item) || item.closed) return;
              if (item.sub) return openBeside(at, keys);
              // A switch is pressed where it is: the menu stays, and says how it is once that is known.
              if (item.on) {
                await item.act();
                setPressed((times) => times + 1);
                return;
              }
              done(item);
            }}
          />
        ))}
      </div>
      {opened && opensBeside?.sub && (
        <div
          key={opened.at}
          class="row-menu row-beside"
          role="menu"
          aria-label={opensBeside.label.replace(/…$/, '')}
          ref={besideDrawn}
          style="visibility:hidden;left:0;top:0;overflow-y:auto"
          onPointerEnter={() => clear('shut')}
        >
          {opensBeside.sub.map((item, at) => (
            <Line
              key={at}
              item={item}
              marked={opensBeside.sub!.some((one) => 'label' in one && one.mark !== undefined)}
              open={false}
              onRest={() => clear('shut')}
              onLeave={() => {}}
              onPress={(_keys) => {
                if (!('label' in item) || item.closed) return;
                done(item);
              }}
            />
          ))}
        </div>
      )}
    </>
  );
}

/** One line of a menu: something it only says, or something to press. */
function Line(props: { item: Offered | Said; marked: boolean; open: boolean; onRest: (pointer: string) => void; onLeave: () => void; onPress: (keys: boolean) => void }) {
  const { item } = props;
  if ('said' in item)
    return (
      <div class={classes('menu-said', item.apart && 'apart')} role="presentation" onPointerEnter={(event) => props.onRest(event.pointerType)}>
        {item.said}
      </div>
    );
  return (
    <button
      class={classes(item.apart && 'apart', item.look && `menu-${item.look}`, item.under && 'menu-two')}
      role={item.on ? 'menuitemcheckbox' : item.chosen === undefined || item.sub ? 'menuitem' : 'menuitemradio'}
      aria-checked={item.sub ? undefined : item.on ? item.on() : item.chosen}
      aria-haspopup={item.sub ? 'menu' : undefined}
      aria-expanded={item.sub ? props.open : undefined}
      aria-disabled={item.closed ? true : undefined}
      title={item.closed}
      aria-label={item.count ? `${item.label}: ${item.count} unread` : undefined}
      onPointerEnter={(event) => props.onRest(event.pointerType)}
      onPointerLeave={props.onLeave}
      // A press by the keyboard (Enter, Space) is a click with no pointer: what it opens beside it then has the keyboard.
      onClick={(event) => props.onPress(event.detail === 0)}
    >
      {props.marked && (
        <span class="menu-sign" aria-hidden="true">
          {item.mark}
        </span>
      )}
      {item.indent ? <span class="menu-indent" aria-hidden="true" style={`width:calc(${item.indent * 12}px * var(--size))`} /> : null}
      {item.under ? (
        <span class="menu-lines">
          <span class="truncate">{item.label}</span>
          <span class="menu-under">{item.under}</span>
        </span>
      ) : (
        <span class="truncate">{item.label}</span>
      )}
      {item.hint ? <span class="hint">{item.hint}</span> : null}
      {item.on ? <span class="switch" aria-hidden="true" /> : null}
      {item.count ? <span class="waits">{item.count > 99 ? '99+' : item.count}</span> : null}
      {/* Drawn beside the name and not part of it: what it is called is what is read out. */}
      {item.chosen && (
        <span class="mark" aria-hidden="true">
          ✓
        </span>
      )}
      {item.sub && (
        <span class="menu-opens" aria-hidden="true">
          ›
        </span>
      )}
    </button>
  );
}
