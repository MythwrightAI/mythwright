// Choosing from options shown as tags.
//
// What is chosen sits in the field as tags, each with a cross that takes
// it out. Under it, while the field has the caret, are all the options:
// pressing one chooses it or lets it go, typing narrows them, and where
// more may be made, typing something that is not there offers to make it.
// Where their order matters, an option is dragged by its grip, or moved
// with Alt and an arrow, and what is chosen follows the order they are in.
//
// The options lie over what is under the field and do not push it down:
// a button further down must not move from under the pointer as the
// options open or close. So whatever this is put in must not cut off what
// reaches past its edge.

import { useEffect, useRef, useState } from 'preact/hooks';
import { add, begin, colour, making, matching, move, only, toggle, value } from './pills.ts';
import type { Item, Picked } from './pills.ts';
import { classes } from './util.ts';

export interface SelectProps {
  /** What is being chosen, said to whoever cannot see the field's own label. */
  label: string;
  /** What is chosen to begin with. */
  chosen: Item[];
  /** What else there is to choose from. */
  options?: Item[];
  /** Several may be chosen. Otherwise choosing one lets the other go. */
  many?: boolean;
  /** More may be made by typing them. */
  makes?: boolean;
  /** Where one is chosen, it may be let go and nothing chosen in its place. */
  clears?: boolean;
  /** Their order matters, and can be changed. */
  ordered?: boolean;
  /** Start with the caret in it, and so with its options open. */
  focused?: boolean;
  onChange: (chosen: Item[]) => void;
}

function Tag({ item, onRemove }: { item: Item; onRemove?: () => void }) {
  return (
    <span class={`tag tag-${colour(item)}`}>
      <span class="truncate">{String(item)}</span>
      {onRemove && (
        // Not to take the caret out of the field, which would close what is under it.
        <button type="button" class="tag-x" aria-label={`Remove ${item}`} onMouseDown={(event) => event.preventDefault()} onClick={onRemove}>
          ×
        </button>
      )}
    </span>
  );
}

export function Select(props: SelectProps) {
  const [picked, setPicked] = useState<Picked>(() => begin(props.chosen, props.options));
  const [typed, setTyped] = useState('');
  const [open, setOpen] = useState(false);
  const [marked, setMarked] = useState(0);
  const [dragged, setDragged] = useState<number | null>(null);
  // Whether the mark was moved with the arrows since the options opened.
  // Until it is, Enter with nothing typed is the form's, not this field's:
  // a dialog that opens with the caret here must not lose an option to it.
  const [aimed, setAimed] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLDivElement>(null);
  // The caret is leaving because an answer was given, or given up with Escape: what is still written is not one.
  const settled = useRef(false);

  useEffect(() => {
    if (props.focused) input.current?.focus();
  }, []);

  const change = (next: Picked) => {
    setPicked(next);
    props.onChange(value(next));
  };
  const shown = matching(picked, typed);
  const made = props.makes ? making(picked, typed) : null;
  // The rows that can be marked: each option shown, then the one that would be made.
  const rows = shown.length + (made ? 1 : 0);
  const at = Math.min(marked, Math.max(rows - 1, 0));
  const isChosen = (item: Item) => picked.chosen.some((one) => String(one) === String(item));
  // Their order is changed among all of them, so not while only some are shown.
  const sortable = !!props.ordered && typed.trim() === '';

  const choose = (item: Item) => {
    if (props.many) change(toggle(picked, item));
    else {
      change(only(picked, item));
      settled.current = true;
      input.current?.blur();
    }
    setTyped('');
  };
  const make = () => {
    change(props.many ? add(picked, typed) : only(add(picked, typed), typed.trim()));
    setTyped('');
    setMarked(picked.options.length);
    // Where one is chosen, the one that was made is the answer, as one that was pressed is.
    if (!props.many) {
      settled.current = true;
      input.current?.blur();
    }
  };
  const shift = (from: number, to: number) => {
    if (to < 0 || to >= picked.options.length) return;
    change(move(picked, from, to));
    setMarked(to);
  };

  // Dragging an option by its grip: it goes where the pointer is among the rows.
  const drag = (event: PointerEvent, from: number) => {
    const grip = event.currentTarget as HTMLElement;
    const rowsEl = list.current;
    if (!rowsEl) return;
    event.preventDefault();
    grip.setPointerCapture(event.pointerId);
    let current = from;
    let now = picked;
    setDragged(from);
    const onMove = (moved: PointerEvent) => {
      const row = rowsEl.querySelector<HTMLElement>('.select-option');
      if (!row) return;
      const height = row.getBoundingClientRect().height || 1;
      const to = Math.max(0, Math.min(now.options.length - 1, Math.floor((moved.clientY - rowsEl.getBoundingClientRect().top) / height)));
      if (to === current) return;
      now = move(now, current, to);
      current = to;
      setPicked(now);
      setDragged(to);
      setMarked(to);
    };
    const onUp = () => {
      grip.removeEventListener('pointermove', onMove);
      grip.removeEventListener('pointerup', onUp);
      grip.removeEventListener('pointercancel', onUp);
      setDragged(null);
      props.onChange(value(now));
    };
    grip.addEventListener('pointermove', onMove);
    grip.addEventListener('pointerup', onUp);
    grip.addEventListener('pointercancel', onUp);
  };

  return (
    <div
      class={classes('select', open && 'open')}
      ref={root}
      onFocusIn={() => {
        settled.current = false;
        setOpen(true);
      }}
      onFocusOut={(event) => {
        if (!root.current?.contains(event.relatedTarget as Node | null)) {
          // Where one is taken and more may be made, a word that was written and left is the answer, as it is
          // of any text that is written: the caret going elsewhere is not giving it up. Read from the field
          // itself, which has every letter whether or not this has been drawn again since.
          const written = (input.current?.value ?? '').trim();
          if (props.makes && !props.many && !settled.current && written !== '') change(only(add(picked, written), written));
          setOpen(false);
          setTyped('');
          setAimed(false);
        }
      }}
    >
      <div class="select-field" onMouseDown={(event) => event.target === event.currentTarget && (event.preventDefault(), input.current?.focus())}>
        {value(picked).map((item) => (
          <Tag key={String(item)} item={item} onRemove={props.many || props.clears ? () => change(toggle(picked, item)) : undefined} />
        ))}
        <input
          ref={input}
          role="combobox"
          aria-label={props.label}
          aria-expanded={open}
          value={typed}
          placeholder={value(picked).length === 0 ? (props.makes ? 'Choose, or type to add' : 'Choose') : ''}
          onInput={(event) => {
            setTyped(event.currentTarget.value);
            setMarked(0);
          }}
          onKeyDown={(event) => {
            if (event.key === 'Escape' && open) {
              // The dialog this is in closes on Escape too; this one is for what is open here.
              event.stopPropagation();
              settled.current = true;
              input.current?.blur();
            } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
              event.preventDefault();
              const step = event.key === 'ArrowDown' ? 1 : -1;
              setAimed(true);
              if (event.altKey && sortable && at < shown.length) shift(at, at + step);
              else if (rows > 0) setMarked((at + step + rows) % rows);
            } else if (event.key === 'Enter' && (typed.trim() !== '' || aimed)) {
              if (rows === 0) return;
              event.preventDefault();
              if (at < shown.length) choose(shown[at]!);
              else make();
            } else if (event.key === 'Backspace' && typed === '' && props.many) {
              const last = value(picked).at(-1);
              if (last !== undefined) change(toggle(picked, last));
            }
          }}
        />
      </div>
      {open && (
        <div class="select-menu">
          <div class="select-hint">{props.makes ? 'Select an option or create one' : 'Select an option'}</div>
          <div ref={list} role="listbox" aria-label={`${props.label} options`}>
            {shown.map((item, index) => (
              <div
                key={String(item)}
                class={classes('select-option', index === at && 'marked', index === dragged && 'dragged')}
                role="option"
                aria-selected={isChosen(item)}
                // Not to take the caret out of the field.
                onMouseDown={(event) => event.preventDefault()}
                onMouseMove={() => dragged === null && setMarked(index)}
                onClick={() => choose(item)}
              >
                {props.ordered && (
                  <span
                    class={classes('select-grip', !sortable && 'still')}
                    aria-hidden="true"
                    title="Drag to put it somewhere else in the order"
                    onPointerDown={(event) => sortable && drag(event, index)}
                    onClick={(event) => event.stopPropagation()}
                  >
                    ⠿
                  </span>
                )}
                <Tag item={item} />
                {isChosen(item) && <span class="select-check">✓</span>}
              </div>
            ))}
            {made && (
              <div class={classes('select-option', at === shown.length && 'marked')} role="option" aria-selected="false" onMouseDown={(event) => event.preventDefault()} onMouseMove={() => setMarked(shown.length)} onClick={make}>
                <span class="select-create">Create</span>
                <Tag item={made} />
              </div>
            )}
            {rows === 0 && <div class="select-none">Nothing matches.</div>}
          </div>
        </div>
      )}
    </div>
  );
}
