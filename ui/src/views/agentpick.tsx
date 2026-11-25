// The name of an agent added beside a channel, and the list under it of
// the agents there are: each by its name, with where it works, what it
// runs on (or that it is external), whether it is stopped and whether it
// is in the channel already, in fainter words beside it (`agentLine` in
// `memberadded.ts`). It is the list the composer offers names in (the
// `mentions` look, a line a name over what is said of it), put under the
// field: it lies over the dialog and moves nothing, is placed on the
// screen under the field, or over it, and never over it (`listPlaced`), and is a finger's size on a
// phone as that list is. Typing narrows it; the arrows mark a line and
// Enter takes it; Escape puts it away before the dialog. A name no agent
// has is a new one, which its last line says. A browser's own list of
// suggestions held a name and nothing more, which is why this is not one.

import type { RefObject } from 'preact';
import { useLayoutEffect, useRef, useState } from 'preact/hooks';
import { agentLine, agentsFound, lastLine, listPlaced, marking } from '../memberadded.ts';
import { agents, projectName, projects } from '../store.ts';
import { classes } from '../util.ts';
import { runsOn } from './who.tsx';

/** The field, called Name, and its list. `here` is who is in the channel, by actor; `outside` whether a new one would be invited. */
export function AgentPick(props: { value: string; onInput: (value: string) => void; here: Set<string>; outside: boolean; field: RefObject<HTMLInputElement | null> }) {
  const { value, onInput, here, outside, field } = props;
  const [open, setOpen] = useState(false);
  const [marked, setMarked] = useState(-1);
  const list = useRef<HTMLDivElement>(null);
  const [at, setAt] = useState<{ left: number; top: number; width: number; most: number | null } | null>(null);
  const all = agents.value;
  const lines = agentsFound(
    all.map((slot) =>
      agentLine(slot, {
        project: slot.project && projects.value.some((one) => one.id === slot.project) ? projectName(slot.project) : null,
        on: runsOn(slot.name),
        here: here.has(`agent:${slot.name}`),
      }),
    ),
    value,
  );
  const last = lastLine(outside, value, all);
  const shown = open && (lines.length > 0 || last !== null);
  const mark = Math.min(marked, lines.length - 1);

  // Placed under the field, on the screen, before it is drawn where anybody sees it; again as it grows or the dialog scrolls.
  useLayoutEffect(() => {
    if (!shown) {
      setAt(null);
      return;
    }
    const place = () => {
      const box = field.current?.getBoundingClientRect();
      if (!box) return;
      // How tall it would be with nothing held: what it holds, to the most its style lets it be.
      const shown = list.current;
      let tall = 0;
      if (shown) {
        // Measured as wide as it is to be, since its lines wrap by that.
        const held = [shown.style.maxHeight, shown.style.width];
        shown.style.maxHeight = '';
        shown.style.width = `${box.width}px`;
        tall = Math.min(shown.scrollHeight + shown.offsetHeight - shown.clientHeight, parseFloat(getComputedStyle(shown).maxHeight) || Infinity);
        [shown.style.maxHeight, shown.style.width] = held as [string, string];
      }
      setAt(listPlaced(box, tall, { width: window.innerWidth, height: window.innerHeight }));
    };
    place();
    window.addEventListener('resize', place);
    window.addEventListener('scroll', place, true);
    return () => {
      window.removeEventListener('resize', place);
      window.removeEventListener('scroll', place, true);
    };
  }, [shown, lines.length, last, value]);

  const take = (name: string) => {
    onInput(name);
    setOpen(false);
    setMarked(-1);
  };

  return (
    <>
      <label>
        Name
        <input
          ref={field}
          required
          role="combobox"
          aria-autocomplete="list"
          aria-expanded={shown}
          aria-controls="agents-offered"
          aria-activedescendant={shown && mark >= 0 ? `agent-offered-${mark}` : undefined}
          autocomplete="off"
          value={value}
          onClick={() => setOpen(true)}
          onBlur={() => setOpen(false)}
          onInput={(event) => {
            onInput(event.currentTarget.value);
            setOpen(true);
            setMarked(-1);
          }}
          onKeyDown={(event) => {
            if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
              event.preventDefault();
              setOpen(true);
              setMarked(marking(open ? mark : -1, lines.length, event.key));
              return;
            }
            if (event.key === 'Enter' && shown && mark >= 0 && !event.isComposing) {
              event.preventDefault();
              take(lines[mark]!.name);
              return;
            }
            // The list first, then the dialog: kept from the dialog's own Escape, which it hears on the window.
            if (event.key === 'Escape' && shown) {
              event.preventDefault();
              event.stopPropagation();
              setOpen(false);
              setMarked(-1);
            }
          }}
        />
      </label>
      {shown && (
        <div
          ref={list}
          id="agents-offered"
          class="mentions agents-offered"
          role="listbox"
          aria-label="Agents there are"
          style={at ? { left: `${at.left}px`, top: `${at.top}px`, width: `${at.width}px`, ...(at.most === null ? {} : { maxHeight: `${at.most}px` }) } : { visibility: 'hidden' }}
          // A press anywhere in it keeps the caret in the field, so that the list is not put away under the pointer.
          onMouseDown={(event) => event.preventDefault()}
        >
          {lines.map((line, index) => (
            <button
              key={line.name}
              id={`agent-offered-${index}`}
              type="button"
              role="option"
              tabIndex={-1}
              data-agent={line.name}
              aria-selected={index === mark}
              class={classes(index === mark && 'marked')}
              // Before the field loses the caret, which a press would take.
              onMouseDown={(event) => {
                event.preventDefault();
                take(line.name);
              }}
              onMouseEnter={() => setMarked(index)}
            >
              <span class="asked-words">
                <strong class="asked-name">{line.name}</strong>
                <span class="asked-under">{line.said.join(' · ')}</span>
              </span>
            </button>
          ))}
          {last && <p class="faint agents-offered-new">{last}</p>}
        </div>
      )}
    </>
  );
}
