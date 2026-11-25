// What plays frames on a page: the drawn side of `player.ts`.
//
// It is handed frames and a playhead (`Head`), and moves the playhead on
// while it plays: it asks the browser once a frame of the browser's how
// long it has played (`requestAnimationFrame`, and never a timer), from
// the beginning of the frame the playhead is at, and stops at the end,
// holding the last frame. Paused, it does nothing: what shows is where the
// playhead is, put there by whoever pressed something (`Controls`). Where
// less movement is asked for nothing plays by itself (`useStill`), and
// whoever draws the frames shows what they come to (`stillOf`) until
// somebody steps to one; what a key does there is `keyed`. A preview hands it what a text comes to; a replay
// of a recorded run would hand it frames of what that run's events said.

import { useEffect, useRef, useState } from 'preact/hooks';
import { playedFrom, pressedPlay, stepTo, trackAt } from '../player.ts';
import type { Frame, Head } from '../player.ts';

/** Moves the playhead on while it plays, telling `onHead` each time it is at another frame, and once more as it comes to the end. */
export function usePlaying(frames: readonly Frame[], head: Head, onHead: (head: Head) => void): void {
  const told = useRef(onHead);
  told.current = onHead;
  useEffect(() => {
    if (!head.playing || head.at === null || frames.length === 0) return;
    // From the beginning of the frame it is at: each frame shown begins the count again from its own beginning.
    const from = head.at;
    let begun: number | null = null;
    let asked = 0;
    const step = (now: number) => {
      begun ??= now;
      const next = playedFrom(frames, from, now - begun);
      if (!next.playing || next.at !== from) return told.current(next);
      asked = requestAnimationFrame(step);
    };
    asked = requestAnimationFrame(step);
    return () => cancelAnimationFrame(asked);
  }, [frames, head.at, head.playing]);
}

const STILL = '(prefers-reduced-motion: reduce)';

/** Whether less movement is asked for, followed as that changes. */
export function useStill(): boolean {
  const asked = typeof window === 'undefined' ? null : window.matchMedia(STILL);
  const [still, setStill] = useState(asked?.matches ?? false);
  useEffect(() => {
    if (!asked) return;
    const heard = () => setStill(asked.matches);
    asked.addEventListener('change', heard);
    heard();
    return () => asked.removeEventListener('change', heard);
  }, []);
  return still;
}

/** A small drawing for a control, in the text's colour. */
const Glyph = ({ d }: { d: string }) => (
  <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
    <path d={d} fill="currentColor" />
  </svg>
);
const PLAY = 'M3 1.5v9l7.5-4.5z';
const PAUSE = 'M2.5 1.5h2.5v9H2.5zM7 1.5h2.5v9H7z';
const BEFORE = 'M1.5 1.5h2v9h-2zM10.5 1.5v9L4 6z';
const AFTER = 'M8.5 1.5h2v9h-2zM1.5 1.5v9L8 6z';

/**
 * Play and Pause, the frame before and after, and a track of the frames
 * with the playhead on it, pressed or dragged to a frame: each a function
 * of the playhead (`player.ts`), handed back to whoever keeps it. The
 * keys are heard where the frames are drawn, by `keyed`.
 */
export function Controls({ count, head, onHead }: { count: number; head: Head; onHead: (head: Head) => void }) {
  const track = useRef<HTMLDivElement>(null);
  const held = useRef(false);
  const at = (event: PointerEvent) => {
    const box = track.current?.getBoundingClientRect();
    if (box) onHead(trackAt(count, event.clientX - box.left, box.width));
  };
  const shown = head.at === null ? null : head.at + 1;
  return (
    <div class="canvas-player" role="group" aria-label="What is played">
      <button class="icon" aria-label="Previous frame" title="The frame before (← or ,)" disabled={count === 0} onClick={() => onHead(stepTo(count, head, -1))}>
        <Glyph d={BEFORE} />
      </button>
      <button class="icon" aria-label={head.playing ? 'Pause' : 'Play'} title={head.playing ? 'Pause (Space)' : 'Play it from here, or from the first once it is over (Space)'} disabled={count === 0} onClick={() => onHead(pressedPlay(count, head))}>
        <Glyph d={head.playing ? PAUSE : PLAY} />
      </button>
      <button class="icon" aria-label="Next frame" title="The frame after (→ or .)" disabled={count === 0} onClick={() => onHead(stepTo(count, head, 1))}>
        <Glyph d={AFTER} />
      </button>
      <div
        ref={track}
        class="player-track"
        role="slider"
        tabIndex={count === 0 ? -1 : 0}
        aria-label="Frames"
        aria-valuemin={count === 0 ? 0 : 1}
        aria-valuemax={count}
        aria-valuenow={shown ?? 0}
        aria-valuetext={shown === null ? 'all of it, still' : `frame ${shown} of ${count}`}
        onPointerDown={(event) => {
          if (count === 0) return;
          event.preventDefault();
          track.current?.setPointerCapture(event.pointerId);
          held.current = true;
          at(event);
        }}
        onPointerMove={(event) => held.current && at(event)}
        onPointerUp={() => (held.current = false)}
        onPointerCancel={() => (held.current = false)}
        onKeyDown={(event) => {
          // Home and End to its ends; the arrows are heard where the frames are drawn.
          if (event.key !== 'Home' && event.key !== 'End') return;
          event.preventDefault();
          onHead({ at: count === 0 ? null : event.key === 'Home' ? 0 : count - 1, playing: false });
        }}
      >
        {Array.from({ length: count }, (_, index) => (
          <span key={index} class={head.at === null ? 'player-still' : index === head.at ? 'player-now' : index < head.at ? 'player-past' : undefined} />
        ))}
      </div>
      <span class="player-count" aria-hidden="true">
        {count === 0 ? '' : shown === null ? `all ${count}` : `${shown}/${count}`}
      </span>
    </div>
  );
}
