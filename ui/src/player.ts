// What plays a blueprint's graph a frame at a time.
//
// A frame says which tiles are lit and how (an event arriving at a
// handler, a condition asked, a step being done, done, failed, a new
// text taken, ended, or only pointed at), which line of what is being
// read it is of, and how long it is shown. Whatever is played hands frames and nothing else: a preview
// works them out from a text (`preview.ts`), and a replay of a recorded
// run would make them from what its events said, each lasting as long as
// it took. So nothing here knows a summary, a run or an event: which
// frame is showing after so long is a function of the frames alone, and
// what draws them (`views/player.tsx`) asks it once a frame of the
// browser's and never on a timer. Where the playhead is (`Head`) is kept by
// whatever plays, and each thing somebody does to it (Play, Pause, a frame
// before or after, the track, a line pressed, a key) is a function of it
// below, so that what is shown is the playhead's and not the clock's.

/** How a tile is lit in a frame. */
export type Lit = 'arrives' | 'asks' | 'runs' | 'done' | 'failed' | 'taken' | 'ended' | 'shown';

export interface Frame {
  /** The tiles lit in it, by node id, each with how; a tile that is not named is not lit. */
  tiles: Readonly<Record<string, Lit>>;
  /** The line of what is being read it is of, as indexes down from the first level (`[1, 0]`: the first line under the second); none for the whole. */
  line: readonly number[] | null;
  /** What is said over the graph while it is shown, as the text writes it: a handler's condition. */
  says: string | null;
  /** How long it is shown before the next, in milliseconds. */
  lasts: number;
}

/** How each way a tile is lit is marked on its corner, for whoever does not tell the colours apart, and said under the graph. */
export const LIT: [Lit, string, string][] = [
  ['arrives', '↓', 'what it hears arrives'],
  ['asks', '?', 'whether to go on is asked'],
  ['runs', '▸', 'being done'],
  ['done', '✓', 'done'],
  ['failed', '✕', 'failed'],
  ['taken', '↻', 'its new text taken'],
  ['ended', '■', 'ended'],
  ['shown', '●', 'what a line is about'],
];

/** The mark of each way a tile is lit. */
export const MARKS: Record<Lit, string> = Object.fromEntries(LIT.map(([lit, mark]) => [lit, mark])) as Record<Lit, string>;

/** How a frame is played round: from the first again once the last has been shown, after a rest. */
export interface Round {
  again: boolean;
  /** How long the last frame is held over its own time before the first comes again, in milliseconds. */
  rest: number;
}

/** Played once, and the last frame kept: as a replay of a run is. */
export const ONCE: Round = { again: false, rest: 0 };

/**
 * Which frame is showing once `elapsed` milliseconds have been played:
 * each for its own time, in order; round again from the first after the
 * last has been held for `rest`, or the last kept where it is not played
 * round. Nothing for no frames.
 */
export function frameAt(frames: readonly Pick<Frame, 'lasts'>[], elapsed: number, round: Round): number | null {
  if (frames.length === 0) return null;
  const whole = frames.reduce((sum, frame) => sum + Math.max(0, frame.lasts), 0) + Math.max(0, round.rest);
  let at = Math.max(0, elapsed);
  if (round.again && whole > 0) at %= whole;
  for (let index = 0; index < frames.length; index += 1) {
    at -= Math.max(0, frames[index]!.lasts);
    if (at < 0) return index;
  }
  return frames.length - 1;
}

/**
 * What the frames come to when nothing is to move: every tile any of them
 * lit, as the last of them that lit it left it, at once and of no line.
 * It is what is shown where less movement is asked for.
 */
export function stillOf(frames: readonly Frame[]): Frame {
  const tiles: Record<string, Lit> = {};
  for (const frame of frames) Object.assign(tiles, frame.tiles);
  return { tiles, line: null, says: null, lasts: 0 };
}

/** Whether a frame is of a line, or of one under it. */
export function isOf(frame: Pick<Frame, 'line'> | null, line: readonly number[]): boolean {
  if (!frame?.line || frame.line.length < line.length) return false;
  return line.every((index, depth) => frame.line![depth] === index);
}

/**
 * Where the playhead is, and whether it moves by itself: which frame
 * shows (`at`), or, where nothing was played or stepped to and nothing is
 * to move, none: what the frames come to, still (`stillOf`). Whatever
 * plays frames keeps one, and each thing done to it (Play, Pause, a step,
 * the track, a line pressed) is a function of it and the frames below,
 * so that what is shown is said by the playhead and not by the clock.
 */
export interface Head {
  at: number | null;
  playing: boolean;
}

/** Where a part's frames begin when it comes to be read: from the first, played once; or, where nothing is to move or there is nothing to play, still. */
export const begun = (still: boolean, count: number): Head => (still || count === 0 ? { at: null, playing: false } : { at: 0, playing: true });

/** When a frame begins, in milliseconds from the first. */
export function startOf(frames: readonly Pick<Frame, 'lasts'>[], index: number): number {
  return frames.slice(0, Math.max(0, index)).reduce((sum, frame) => sum + Math.max(0, frame.lasts), 0);
}

/**
 * Which frame shows once `elapsed` milliseconds have been played from the
 * beginning of frame `from`, played once: and whether that is over, which
 * it is once the last has been shown for its own time. The last is then
 * held.
 */
export function playedFrom(frames: readonly Pick<Frame, 'lasts'>[], from: number, elapsed: number): Head {
  if (frames.length === 0) return { at: null, playing: false };
  const all = startOf(frames, frames.length);
  const total = startOf(frames, from) + Math.max(0, elapsed);
  if (total >= all) return { at: frames.length - 1, playing: false };
  return { at: frameAt(frames, total, ONCE), playing: true };
}

/**
 * Play and Pause, one button: playing, it pauses where it is; paused
 * part of the way, it goes on from the frame it shows; at the last frame,
 * or still with none shown, it plays again from the first, once.
 */
export function pressedPlay(count: number, head: Head): Head {
  if (count === 0) return { at: null, playing: false };
  if (head.playing) return { at: head.at, playing: false };
  if (head.at === null || head.at >= count - 1) return { at: 0, playing: true };
  return { at: head.at, playing: true };
}

/**
 * The frame before or after, paused there: never past either end. From
 * none shown (still), the next is the first and the one before the last.
 */
export function stepTo(count: number, head: Head, by: number): Head {
  if (count === 0) return { at: null, playing: false };
  if (head.at === null) return { at: by > 0 ? 0 : count - 1, playing: false };
  return { at: Math.min(count - 1, Math.max(0, head.at + by)), playing: false };
}

/** The frame under a point of the track, which is so many frames side by side, each as wide as the next: paused there. */
export function trackAt(count: number, x: number, width: number): Head {
  if (count === 0 || width <= 0) return { at: null, playing: false };
  return { at: Math.min(count - 1, Math.max(0, Math.floor((x / width) * count))), playing: false };
}

/** The first frame of a line, or of one under it: where pressing the line puts the playhead. None where no frame is of it. */
export function lineAt(frames: readonly Pick<Frame, 'line'>[], line: readonly number[]): number | null {
  const at = frames.findIndex((frame) => isOf(frame, line));
  return at < 0 ? null : at;
}

/** What a key pressed where frames are played does to the playhead: Space plays and pauses, the arrows and `,` and `.` step. Nothing for any other key. */
export function keyed(key: string, count: number, head: Head): Head | null {
  if (key === ' ') return pressedPlay(count, head);
  if (key === 'ArrowLeft' || key === ',') return stepTo(count, head, -1);
  if (key === 'ArrowRight' || key === '.') return stepTo(count, head, 1);
  return null;
}
