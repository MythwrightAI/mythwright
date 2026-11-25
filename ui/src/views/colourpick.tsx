// Choosing a colour of a theme somebody makes, in a picker of the client's
// own (`picking.ts` is what it works out, with its cases).
//
// A browser's own picker is the system's dialog on some systems, and the
// page can neither change it nor put anything beside it; what somebody
// asked for was the colour as it was beside the colour being chosen. So
// the picker is a panel that lies over the page, opened from the colour's
// swatch, and is no part of the layout: nothing moves as it opens, and it
// stays until it is left. It holds the colour as it was and as it is now,
// side by side and each said in words; an area of how strong and how
// bright with a slider of hue, each moved by the pointer and by the arrows;
// the colour written out, taken once it reads as one; the theme's other
// colours and those of the themes that ship, to take in a press; and,
// where the colour chosen leaves the theme unreadable, the rule's own
// sentence, so that what it costs is seen before it is kept.
//
// Each change is handed on as it is made, as the browser's picker handed
// on each move, so that the form draws its samples in it at once. Done or
// Enter keeps it; Escape, or a press outside, hands back the colour as it
// was. Escape is the picker's alone while it is open: the dialog under it
// would close on the same key.

import { createPortal } from 'preact/compat';
import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { placed } from '../icons.ts';
import { phone } from '../phone.ts';
import { fromArea, fromSlider, hexOf, hsvOf, stepped, typedColour } from '../picking.ts';
import type { Hsv, Pick } from '../picking.ts';
import { lightness } from '../themes.ts';

/** How wide the picker is where the window has the room. */
const WIDE = 300;

export interface PickProps {
  /** What the colour is called, as its swatch says it. */
  called: string;
  /** The colour as it was when the picker was opened. */
  was: string;
  /** What is offered to pick in a press. */
  offered: { own: Pick[]; shipped: Pick[] };
  /** Why the theme as it now stands cannot be read, in the rule's own sentence; nothing where it can. */
  wrong: string | null;
  /** What opened it, which it is put under. */
  opener: { current: HTMLElement | null };
  /** Each colour as it is chosen. */
  onChange: (colour: string) => void;
  /** Kept, by Done or Enter. */
  onDone: () => void;
  /** Left by Escape or a press outside: the colour as it was is handed back first. */
  onBack: () => void;
}

/** Words drawn on a colour, dark on a light one and light on a dark one. */
const inkOn = (colour: string) => (lightness(colour) > 0.3 ? '#111111' : '#f4f4f4');

export function ColourPick({ called, was, offered, wrong, opener, onChange, onDone, onBack }: PickProps) {
  const panel = useRef<HTMLDivElement>(null);
  const area = useRef<HTMLDivElement>(null);
  const slider = useRef<HTMLDivElement>(null);
  const [hsv, setHsv] = useState<Hsv>(() => hsvOf(was));
  const now = hexOf(hsv);
  const [typed, setTyped] = useState(was);
  const [typedWrong, setTypedWrong] = useState<string | null>(null);
  const [place, setPlace] = useState<{ left: number; top: number; width: number } | null>(null);

  /** The colour moved to, from what is not the field: the field follows, and the form is told. */
  const moveTo = (next: Hsv) => {
    setHsv(next);
    const colour = hexOf(next);
    setTyped(colour);
    setTypedWrong(null);
    if (colour !== now) onChange(colour);
  };
  /** A colour taken whole, from the field, a pick or the colour as it was: the hue is kept where the colour has none. */
  const take = (colour: string, fromField = false) => {
    setHsv(hsvOf(colour, hsv.hue));
    if (!fromField) setTyped(colour);
    setTypedWrong(null);
    if (colour !== now) onChange(colour);
  };

  const back = () => {
    if (now !== was) onChange(was);
    onBack();
    opener.current?.focus({ preventScroll: true });
  };
  const done = () => {
    onDone();
    opener.current?.focus({ preventScroll: true });
  };
  // What is handed to the listeners on the window is the picker as it is drawn now.
  const latest = useRef({ back, done });
  latest.current = { back, done };

  // Put under what opened it, wholly in the window; on a phone it has the window's width. It stays where it was put as
  // it grows (what is said of a colour comes and goes as it is chosen), so that nothing moves under the pointer: what
  // no longer fits the window scrolls inside it, with Done kept in reach at its foot.
  const where = () => {
    const box = panel.current?.getBoundingClientRect();
    const width = phone.value || window.innerWidth < WIDE + 32 ? window.innerWidth - 16 : WIDE;
    const at = opener.current?.getBoundingClientRect() ?? { left: 8, top: 8, bottom: 8 };
    const height = Math.min(box?.height ?? 0, window.innerHeight - 16);
    return { width, ...placed(at, { width, height }, { width: window.innerWidth, height: window.innerHeight }) };
  };
  useLayoutEffect(() => setPlace(where()), []);
  // The keyboard is on the area once it is where it is to be: drawn out of sight to be measured, it could not have it.
  // Taken in the commit that shows it, before it is painted: after it, it was shown for a moment with the keyboard
  // still on the swatch, which a check found once in seven crowded.
  const placedYet = place !== null;
  useLayoutEffect(() => {
    if (placedYet) area.current?.focus({ preventScroll: true });
  }, [placedYet]);

  useEffect(() => {
    const inside = (target: EventTarget | null) => target instanceof Node && (panel.current?.contains(target) || opener.current?.contains(target));
    const away = (event: Event) => {
      if (!inside(event.target)) latest.current.back();
    };
    // Escape is the picker's: the dialog it lies over hears it after the window does, and never.
    const key = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      event.stopPropagation();
      latest.current.back();
    };
    const put = (event: Event) => {
      if (event.type === 'scroll' && event.target instanceof Node && panel.current?.contains(event.target)) return;
      setPlace(where());
    };
    window.addEventListener('pointerdown', away, true);
    window.addEventListener('keydown', key, true);
    window.addEventListener('resize', put);
    window.addEventListener('scroll', put, true);
    return () => {
      window.removeEventListener('pointerdown', away, true);
      window.removeEventListener('keydown', key, true);
      window.removeEventListener('resize', put);
      window.removeEventListener('scroll', put, true);
    };
  }, []);

  const dragged = (on: 'area' | 'hue') => (event: PointerEvent) => {
    if (event.type === 'pointermove' && !(event.buttons & 1)) return;
    if (event.type === 'pointerdown') {
      event.preventDefault();
      (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
      (event.currentTarget as HTMLElement).focus({ preventScroll: true });
    }
    const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
    moveTo(on === 'area' ? { ...hsv, ...fromArea(event.clientX, event.clientY, box) } : { ...hsv, hue: fromSlider(event.clientX, box) });
  };
  const keyed = (on: 'area' | 'hue') => (event: KeyboardEvent) => {
    const next = stepped(hsv, on, event.key, event.shiftKey);
    if (!next) return;
    event.preventDefault();
    moveTo(next);
  };
  // Enter keeps the colour from wherever it is pressed but a button, which does what it says, and the field while it holds no colour.
  const onKey = (event: KeyboardEvent) => {
    if (event.key !== 'Enter' || event.target instanceof HTMLButtonElement) return;
    event.preventDefault();
    if (event.target instanceof HTMLInputElement && 'wrong' in typedColour(event.target.value)) return;
    done();
  };

  const swatch = (pick: Pick) => (
    <button key={pick.colour} type="button" class="colourpick-pick" style={{ background: pick.colour }} aria-label={`${pick.says}, ${pick.colour}`} title={`${pick.says}, ${pick.colour}`} aria-pressed={pick.colour === now} onClick={() => take(pick.colour)} />
  );
  const percent = (part: number) => `${Math.round(part * 100)}%`;
  return createPortal(
    <div
      ref={panel}
      class="colourpick"
      role="dialog"
      aria-label={`Choosing ${called}`}
      style={place ? `left:${place.left}px;top:${place.top}px;width:${place.width}px;max-height:${Math.max(160, window.innerHeight - place.top - 8)}px` : `visibility:hidden;width:${WIDE}px`}
      onKeyDown={onKey}
    >
      <div class="colourpick-head">
        <strong>{called}</strong>
      </div>
      <div class="colourpick-split" role="group" aria-label="The colour as it was beside the colour now">
        <button type="button" class="colourpick-was" style={{ background: was, color: inkOn(was) }} aria-label={`Was ${was}: go back to it`} title="Go back to it" onClick={() => take(was)}>
          <span>Was</span>
          <code>{was}</code>
        </button>
        <div class="colourpick-now" style={{ background: now, color: inkOn(now) }} aria-label={`Now ${now}`} role="img">
          <span>Now</span>
          <code>{now}</code>
        </div>
      </div>
      <div
        ref={area}
        class="colourpick-area"
        tabIndex={0}
        role="slider"
        aria-label="Strength and brightness"
        aria-valuetext={`strength ${percent(hsv.strength)}, brightness ${percent(hsv.brightness)}`}
        aria-valuenow={Math.round(hsv.strength * 100)}
        style={{ background: `linear-gradient(to top, #000, rgba(0, 0, 0, 0)), linear-gradient(to right, #fff, hsl(${hsv.hue}, 100%, 50%))` }}
        onPointerDown={dragged('area')}
        onPointerMove={dragged('area')}
        onKeyDown={keyed('area')}
      >
        <i class="colourpick-thumb" style={{ left: percent(hsv.strength), top: percent(1 - hsv.brightness), background: now }} />
      </div>
      <div
        ref={slider}
        class="colourpick-hue"
        tabIndex={0}
        role="slider"
        aria-label="Hue"
        aria-valuemin={0}
        aria-valuemax={359}
        aria-valuenow={Math.round(hsv.hue)}
        onPointerDown={dragged('hue')}
        onPointerMove={dragged('hue')}
        onKeyDown={keyed('hue')}
      >
        <i class="colourpick-thumb" style={{ left: `${(Math.round(hsv.hue) / 359) * 100}%`, background: `hsl(${hsv.hue}, 100%, 50%)` }} />
      </div>
      <label class="colourpick-hex">
        <span class="faint">Written out</span>
        <input
          type="text"
          aria-label="Hex"
          value={typed}
          spellcheck={false}
          autocomplete="off"
          maxLength={16}
          aria-invalid={typedWrong !== null}
          onInput={(event) => {
            const text = event.currentTarget.value;
            setTyped(text);
            const read = typedColour(text);
            if ('colour' in read) take(read.colour, true);
            else setTypedWrong(read.wrong);
          }}
        />
      </label>
      {typedWrong && <p class="error-text colourpick-said">{typedWrong}</p>}
      {wrong && (
        <p class="error-text colourpick-said colourpick-unreadable" role="status">
          {wrong}
        </p>
      )}
      <div class="colourpick-picks" role="group" aria-label="This theme's other colours">
        <span class="faint">This theme</span>
        <div>{offered.own.map(swatch)}</div>
      </div>
      {offered.shipped.length > 0 && (
        <div class="colourpick-picks" role="group" aria-label="The colours of the themes that ship">
          <span class="faint">Themes that ship</span>
          <div>{offered.shipped.map(swatch)}</div>
        </div>
      )}
      <div class="colourpick-foot">
        <span class="faint grow">Escape puts it back.</span>
        <button type="button" class="primary" onClick={done}>
          Done
        </button>
      </div>
    </div>,
    document.body,
  );
}
