// A blueprint previewed, in the drawer beside its page.
//
// Over it the blueprint's graph; under that each part of its Summary as a
// section, in the daemon's words, one after another down a column that
// scrolls. The section at the top of the column is the one read, and the
// graph plays what that section is about (`preview.ts`), following the
// tiles it lights. Which section is at the top is told by the browser
// (`IntersectionObserver`), and what is played is played once a frame of
// the browser's (`player.tsx`): once, from its first frame, when a section
// first comes to be the one read, and then held at its last; never round
// again by itself. Each section keeps where its
// playhead is, so one scrolled away from and back to is as it was left.
// Under the graph are what plays and pauses it, steps a frame and shows
// the playhead on a track of its frames, and a line a frame is of is
// pressed to put the playhead at that line, paused. Nothing is played
// while the drawer is put away, and where less movement is asked for
// nothing plays by itself: each section shows what it lights, still, until
// somebody steps to a frame. It is drawn from the check's answer the page already
// has, so it follows the text as it is written, saved or not; a text that
// does not check has none, and what is wrong is said in the check's words.

import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import type { Checked, Diagnostic } from '../api.ts';
import { GraphView } from '../graph/view.tsx';
import { begun, isOf, keyed, lineAt, LIT, MARKS } from '../player.ts';
import type { Frame, Head, Lit } from '../player.ts';
import { sections } from '../preview.ts';
import type { Section } from '../preview.ts';
import type { SummaryLine } from '../summary.ts';
import { Controls, usePlaying, useStill } from './player.tsx';
import { Words } from './summarylines.tsx';

/** What the band near the column's top is, where the section read is: its top two fifths, short of its very edge, which a section scrolled past still touches. */
const BAND = '-1px 0px -60% 0px';

/** No frames, the same each time it is drawn: handed new ones, the player begins again. */
const NONE: Frame[] = [];

export interface PreviewProps {
  checked: Checked | null;
  problems: Diagnostic[];
}

/** A section's lines, each lit while what is played is of it; one a frame is of pressed to put the playhead there. */
function Lines({ lines, frame, frames, path, onLine }: { lines: readonly SummaryLine[]; frame: Frame | null; frames: readonly Frame[]; path: number[]; onLine: (at: number) => void }) {
  return (
    <ul class="preview-lines">
      {lines.map((line, index) => {
        const here = [...path, index];
        const lit = isOf(frame, here) && frame!.line!.length === here.length;
        // The line a lit one is under (a handler, while one of its steps is done) is marked more faintly.
        const within = !lit && isOf(frame, here);
        const first = lineAt(frames, here);
        return (
          <li key={index} class={`preview-line${lit ? ' preview-lit' : within ? ' preview-within' : ''}`} data-lit={lit ? '' : undefined} data-at={line.at ?? undefined} data-ending={line.ending ?? undefined}>
            {first === null ? (
              <span class="preview-words">
                <Words text={line.text} />
              </span>
            ) : (
              <button class="preview-words" aria-current={lit ? 'true' : undefined} title="See this on the graph" onClick={() => onLine(first)}>
                <Words text={line.text} />
              </button>
            )}
            {line.under.length > 0 && <Lines lines={line.under} frame={frame} frames={frames} path={here} onLine={onLine} />}
          </li>
        );
      })}
    </ul>
  );
}

/** Where each section's playhead is, with the frames it is of, which are told apart by what they say: others, it begins again. */
type Heads = Record<string, { of: string; head: Head }>;

/** Which section is at the top of the column, as the browser tells it. */
function useRead(column: { current: HTMLElement | null }, parts: readonly Section[]): [string | null, (kind: string) => void] {
  const [read, setRead] = useState<string | null>(null);
  const kinds = parts.map((part) => part.kind).join(' ');
  useEffect(() => {
    const root = column.current;
    if (!root) return;
    const crossing = new Set<string>();
    const seen = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          const kind = (entry.target as HTMLElement).dataset.part ?? '';
          if (entry.isIntersecting) crossing.add(kind);
          else crossing.delete(kind);
        }
        // The first that crosses the band, which is the one at its top edge (one scrolled past has left it); where
        // none does, the one read before stays read.
        const first = [...root.querySelectorAll<HTMLElement>(':scope > .preview-part')].find((one) => crossing.has(one.dataset.part ?? ''));
        if (first) setRead(first.dataset.part ?? null);
      },
      { root, rootMargin: BAND, threshold: 0 },
    );
    root.querySelectorAll(':scope > .preview-part').forEach((one) => seen.observe(one));
    return () => seen.disconnect();
  }, [kinds]);
  return [read, setRead];
}

export default function Preview({ checked, problems }: PreviewProps) {
  const graph = checked?.graph ?? null;
  const parts = useMemo(() => sections(graph, checked?.summary), [graph, checked?.summary]);
  const still = useStill();
  const column = useRef<HTMLDivElement>(null);
  const [read] = useRead(column, parts);
  const section = parts.find((part) => part.kind === read) ?? parts[0] ?? null;
  const frames = section?.frames ?? NONE;
  const kind = section?.kind ?? '';
  // What a section's frames are, as words: the same frames worked out again from the same text are the same.
  const of = useMemo(() => JSON.stringify(frames), [frames]);
  const [heads, setHeads] = useState<Heads>({});
  const kept = heads[kind];
  // A section first read, or whose frames are others now, is played once from its first frame, or still.
  const fresh = !kept || kept.of !== of;
  const head: Head = fresh ? begun(still, frames.length) : kept.head;
  const setHead = (next: Head) => kind && setHeads((all) => ({ ...all, [kind]: { of, head: next } }));
  useEffect(() => {
    if (section && fresh) setHead(begun(still, frames.length));
  }, [kind, of]);
  // Left, a section stops where it is, and is as it was left when it is read again.
  useEffect(
    () => () =>
      setHeads((all) => {
        const was = all[kind];
        return was?.head.playing ? { ...all, [kind]: { ...was, head: { ...was.head, playing: false } } } : all;
      }),
    [kind],
  );
  usePlaying(frames, head, setHead);
  const frame = !section ? null : head.at === null ? section.still : (frames[head.at] ?? section.still);
  const preview = useRef<HTMLDivElement>(null);
  // Opened by a press, the drawer has the keyboard, so that Space plays and pauses at once.
  useEffect(() => {
    const now = document.activeElement;
    if (!now || now === document.body || now.closest('.head-testing')) preview.current?.focus({ preventScroll: true });
  }, [!!section]);

  if (!checked) return <div class="empty">Reading the blueprint</div>;
  if (!graph || !checked.summary) {
    const errors = problems.filter((problem) => problem.severity === 'error');
    return (
      <div class="preview-none">
        <strong>This cannot be previewed until it checks</strong>
        <ul class="summary-wrong">
          {errors.map((problem, index) => (
            <li key={index}>
              {problem.span.start.line > 0 && <span class="faint">line {problem.span.start.line} </span>}
              <Words text={problem.message} />
            </li>
          ))}
        </ul>
      </div>
    );
  }
  const tiles = frame?.tiles ?? {};
  // What the graph follows: what is happening in the frame, or where nothing is (all of it done), all it lights.
  const happening = Object.keys(tiles).filter((id) => tiles[id] !== 'done');
  const followed = happening.length > 0 ? happening : Object.keys(tiles);
  // What the marks mean, of those the section lights.
  // Still with no frame stepped to, what it comes to; else every frame of it, so that the legend stays as it is played and stepped.
  const used = new Set<Lit>((still && head.at === null ? [section?.still].filter((one) => !!one) : frames).flatMap((one) => Object.values(one!.tiles)));
  const playing = !!section && head.playing && head.at !== null;
  const bring = (kind: string) => column.current?.querySelector<HTMLElement>(`:scope > .preview-part[data-part="${kind}"]`)?.scrollIntoView({ block: 'start', behavior: still ? 'auto' : 'smooth' });
  return (
    <div
      ref={preview}
      class="preview"
      tabIndex={-1}
      data-of={checked.summary.name}
      data-read={section?.kind}
      data-frame={head.at === null ? 'still' : head.at}
      data-frames={frames.length}
      data-playing={playing ? 'true' : 'false'}
      onKeyDown={(event) => {
        // A key a field or a button takes for itself is theirs: Space presses a button.
        const target = event.target as HTMLElement;
        if (event.ctrlKey || event.metaKey || event.altKey || target.closest('input, textarea, select, [contenteditable="true"]')) return;
        if (event.key === ' ' && target.closest('button')) return;
        const next = keyed(event.key, frames.length, head);
        if (!next) return;
        event.preventDefault();
        setHead(next);
      }}
    >
      <div class="preview-graph">
        <GraphView
          graph={graph}
          statuses={Object.fromEntries(Object.entries(tiles).map(([id, how]) => [id, `pv-${how}`]))}
          badges={Object.fromEntries(Object.entries(tiles).map(([id, how]) => [id, MARKS[how]]))}
          selected={null}
          onSelect={() => {}}
          focus={followed}
          scene={section?.kind}
          legend={['drag to move', 'scroll to zoom']}
          band={<Controls count={frames.length} head={head} onHead={setHead} />}
        />
      </div>
      {/* Its room is kept whatever is said in it, so that nothing under it moves as a frame says something. */}
      <div class="preview-now">
        {playing && <span key={`${section!.kind}:${head.at}`} class="preview-time" style={{ animationDuration: `${frames[head.at ?? 0]?.lasts ?? 0}ms` }} />}
        <span class="preview-says">
          {frame?.says ? (
            <>
              only when <code>{frame.says}</code>
            </>
          ) : (
            <span class="faint">{section ? section.title : ''}</span>
          )}
        </span>
        {used.size > 0 && (
          <ul class="preview-legend" aria-label="What each mark means">
            {LIT.filter(([how]) => used.has(how)).map(([how, mark, words]) => (
              <li key={how}>
                <span class={`preview-mark lit-${how}`}>{mark}</span> {words}
              </li>
            ))}
          </ul>
        )}
      </div>
      <div class="preview-parts" ref={column}>
        {parts.map((part) => (
          <section key={part.kind} class="preview-part" data-part={part.kind} aria-current={part === section ? 'true' : undefined}>
            <h3>
              <button class="preview-heading" onClick={() => bring(part.kind)} title="Read this part, and see what it does on the graph">
                {part.title}
              </button>
            </h3>
            <Lines
              lines={part.lines}
              frame={part === section ? frame : null}
              frames={part.frames}
              path={[]}
              onLine={(at) => {
                // Pressed in a part that is not the one read, it is brought to be read, and then put at that line.
                if (part !== section) bring(part.kind);
                setHeads((all) => ({ ...all, [part.kind]: { of: JSON.stringify(part.frames), head: { at, playing: false } } }));
              }}
            />
            {part.frames.length === 0 && <p class="preview-nothing faint">Nothing on the graph is lit for this part.</p>}
          </section>
        ))}
        {/* Room after the last part, so that it too can come to the top of the column and be read. */}
        <div class="preview-room" aria-hidden="true" />
      </div>
    </div>
  );
}
