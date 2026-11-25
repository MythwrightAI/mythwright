// Choosing a colour in the client's own picker (`views/colourpick.tsx`),
// as functions of plain values with their cases (`picking.test.ts`).
//
// A colour is chosen as its hue, how strong it is and how bright, which is
// what the picker's area and its slider are: the area is strength across
// and brightness up, the slider hue. What is kept and handed on is the
// colour written as `#rrggbb`, as everywhere a theme's colours are. The
// hue is the picker's own while it is open: a grey has none, and one
// taken back from its writing would jump the slider to red as soon as the
// area reached its edge, so a colour read from its writing keeps the hue
// it had where it says none.

import { COLOURS } from './themes.ts';
import type { Colours } from './themes.ts';

/** A colour as the picker holds it: its hue in degrees from 0 to 359, and how strong and how bright it is, each from 0 to 1. */
export interface Hsv {
  hue: number;
  strength: number;
  brightness: number;
}

const HEX = /^#[0-9a-f]{6}$/i;
const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));

/** The colour as the picker holds it, from its writing; `hue` is kept where the colour has none of its own (a grey, or black). */
export function hsvOf(colour: string, hue = 0): Hsv {
  const [r, g, b] = [1, 3, 5].map((at) => parseInt(colour.slice(at, at + 2), 16) / 255) as [number, number, number];
  const high = Math.max(r, g, b);
  const span = high - Math.min(r, g, b);
  if (span === 0 || high === 0) return { hue, strength: 0, brightness: high };
  const sixth = high === r ? ((g - b) / span + 6) % 6 : high === g ? (b - r) / span + 2 : (r - g) / span + 4;
  return { hue: (sixth * 60) % 360, strength: span / high, brightness: high };
}

/** The colour written as `#rrggbb`, from how the picker holds it. */
export function hexOf({ hue, strength, brightness }: Hsv): string {
  const h = (((hue % 360) + 360) % 360) / 60;
  const [s, v] = [clamp(strength, 0, 1), clamp(brightness, 0, 1)];
  const channel = (n: number) => {
    const k = (n + h) % 6;
    return v - v * s * Math.max(0, Math.min(k, 4 - k, 1));
  };
  return `#${[channel(5), channel(3), channel(1)].map((part) => Math.round(part * 255).toString(16).padStart(2, '0')).join('')}`;
}

/** Where a point of the area comes to, by where it is in the area's box: strength across, brightness up, each held to the box. */
export function fromArea(x: number, y: number, box: { left: number; top: number; width: number; height: number }): { strength: number; brightness: number } {
  return { strength: clamp((x - box.left) / Math.max(1, box.width), 0, 1), brightness: clamp(1 - (y - box.top) / Math.max(1, box.height), 0, 1) };
}

/** The hue a point of the slider comes to, by where it is across the slider's box. */
export function fromSlider(x: number, box: { left: number; width: number }): number {
  return Math.round(clamp((x - box.left) / Math.max(1, box.width), 0, 1) * 359);
}

/** How far a key moves what it is pressed on: a step, and with Shift ten of them. In the area a step is a hundredth; on the slider a degree. */
export const STEP = { area: 0.01, hue: 1, many: 10 };

/**
 * What a key does to the colour, pressed on the area or on the slider:
 * the arrows move by a step (ten with Shift), and on the slider Home and
 * End go to either end. A key that is none of those answers nothing, so
 * that it is left to the page.
 */
export function stepped(hsv: Hsv, on: 'area' | 'hue', key: string, shift = false): Hsv | null {
  const by = shift ? STEP.many : 1;
  if (on === 'area') {
    const strength = { ArrowLeft: -1, ArrowRight: 1 }[key];
    const brightness = { ArrowDown: -1, ArrowUp: 1 }[key];
    if (strength === undefined && brightness === undefined) return null;
    const round = (value: number) => Math.round(clamp(value, 0, 1) * 100) / 100;
    return {
      ...hsv,
      strength: round(hsv.strength + (strength ?? 0) * by * STEP.area),
      brightness: round(hsv.brightness + (brightness ?? 0) * by * STEP.area),
    };
  }
  if (key === 'Home') return { ...hsv, hue: 0 };
  if (key === 'End') return { ...hsv, hue: 359 };
  const hue = { ArrowLeft: -1, ArrowDown: -1, ArrowRight: 1, ArrowUp: 1 }[key];
  if (hue === undefined) return null;
  return { ...hsv, hue: clamp(Math.round(hsv.hue) + hue * by * STEP.hue, 0, 359) };
}

/** What is said under the hex field where what was typed there is not a colour. */
export const NOT_A_COLOUR = 'A colour is written as # and six of 0 to 9 and a to f, as #2fd4c4. This is not taken.';

/** What was typed in the hex field: the colour it is, written as the theme keeps it, or why it is not taken. Space about it is passed over. */
export function typedColour(text: string): { colour: string } | { wrong: string } {
  const said = text.trim();
  return HEX.test(said) ? { colour: said.toLowerCase() } : { wrong: NOT_A_COLOUR };
}

/** A colour to pick in a press: the colour and what it is, which is said to whoever points at it or cannot see it. */
export interface Pick {
  colour: string;
  says: string;
}

/**
 * The colours offered to pick in a press, for the colour `key` of a
 * theme's `colours`: the theme's other seven, each by what it is, and
 * then those of the themes that ship (`shipped`, each by its name), each
 * once and none that is among the theme's other seven already. The colour
 * being chosen is no part of what is said, so taking one changes nothing
 * of what is offered.
 */
export function picks(colours: Colours, key: keyof Colours, shipped: { name: string; colours: Colours }[]): { own: Pick[]; shipped: Pick[] } {
  const own = COLOURS.filter(([one]) => one !== key).map(([one, label]) => ({ colour: colours[one].toLowerCase(), says: label }));
  // Not the colour being chosen: what is offered stays as it is while the picker is open, whatever is taken from it.
  const seen = new Set(own.map((one) => one.colour));
  const others: Pick[] = [];
  for (const theme of shipped) {
    for (const [one, label] of COLOURS) {
      const colour = theme.colours[one].toLowerCase();
      if (seen.has(colour)) continue;
      seen.add(colour);
      others.push({ colour, says: `${theme.name}: ${label}` });
    }
  }
  return { own, shipped: others };
}
