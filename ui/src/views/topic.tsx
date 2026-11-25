import { useLayoutEffect, useRef, useState } from 'preact/hooks';

/** How many lines of what a channel is for are drawn before it is cut. */
const LINES = 2;

/**
 * What a channel is for, under what it is called in its head: smaller and
 * fainter, wrapping within the head and cut to two lines. It is a line of
 * the head of its own (`topic.css`), so the head's first line, and all that
 * stands at its right, is where it is with or without it, and opened whole
 * it pushes down only what is under the head.
 *
 * Where it is cut it is a button that shows it whole and cuts it back, by a
 * press, a tap or the keyboard, and the pointer resting on it says it whole
 * in the client's tip (`tips.ts`) while it is cut; where it is not cut it is
 * plain words, with nothing to press and nothing said that is already there
 * to read. Whether it is cut is how many lines its words take at the width
 * the head has, asked again whenever that width changes.
 */
export function Topic({ topic }: { topic: string }) {
  const words = useRef<HTMLSpanElement>(null);
  const [cut, setCut] = useState(false);
  const [open, setOpen] = useState(false);
  useLayoutEffect(() => {
    const said = words.current;
    const line = said?.parentElement;
    if (!said || !line) return;
    const measure = () => {
      const high = parseFloat(getComputedStyle(said).lineHeight);
      // Its whole height, whether or not it is cut: a cut line keeps what it holds as what can be scrolled to.
      const longer = Number.isFinite(high) && said.scrollHeight > LINES * high + 1;
      setCut(longer);
      if (!longer) setOpen(false);
    };
    measure();
    const watch = new ResizeObserver(measure);
    watch.observe(line);
    return () => watch.disconnect();
  }, [topic, cut]);
  if (!topic.trim()) return null;
  const inner = (
    <span ref={words} class="topic-words">
      {topic}
    </span>
  );
  if (!cut) return <p class="channel-topic">{inner}</p>;
  return (
    <button
      type="button"
      class="channel-topic"
      aria-expanded={open}
      data-open={open ? '' : undefined}
      // Said whole as the pointer rests on it only while it is cut; opened, it is there to read.
      title={open ? undefined : topic}
      onClick={() => setOpen(!open)}
    >
      {inner}
    </button>
  );
}
