// What a table a blueprint makes is made of, as its Summary draws it:
// its fields, each by its name with its kind and
// what else the daemon says of it, and what is said of the whole (what it
// sits beside, that its rows are not written by hand). Every word is the
// daemon's (`blueprint::summary::Model`).
//
// It is drawn three ways: unfolded under the row of what the blueprint
// makes; in a card over the page while the table's name is pointed at or
// has the keyboard, as a key of the editor says what it means (the card
// lies over the page on its body, moves nothing, stays as the pointer
// crosses to it and is put away by Escape); and, where a blueprint makes
// two tables or more, in a part of its own, Data model, where a field
// that names a record of another of them leads to that one's model.

import type { ComponentChildren } from 'preact';
import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import type { Model } from '../summary.ts';
import { namedBy } from '../summarylaid.ts';
import type { Modelled } from '../summarylaid.ts';
import { besideBox, Floating } from './offered.tsx';
import { Words } from './summarylines.tsx';

/** How long a card stays once the pointer has left what it is about, so that it can be crossed to. */
const LINGER_MS = 250;

/** Brings a table's model in the Data model part into view, with the keyboard on it. */
function showModel(from: Element, called: string): void {
  const shown = from.closest('.summary')?.querySelector<HTMLElement>(`[data-model="${CSS.escape(called)}"]`);
  if (!shown) return;
  shown.scrollIntoView({ block: 'nearest' });
  shown.focus({ preventScroll: true });
}

/** A model's fields and what is said of the whole. `tables` are those a field may lead to, in the Data model part. */
export function ModelView({ model, tables = [] }: { model: Model; tables?: Modelled[] }) {
  return (
    <div class="summary-model">
      {model.as_said && (
        <p class="summary-model-said">
          <Words text={model.as_said} />
        </p>
      )}
      {(model.fields?.length ?? 0) > 0 && (
        <dl class="summary-fields">
          {model.fields!.map((field) => {
            const leads = namedBy(field.names, tables);
            return (
              <div key={field.name} class="summary-field" data-field={field.name}>
                <dt>
                  <code>{field.name}</code>
                </dt>
                <dd>
                  {leads ? (
                    <button type="button" class="link summary-field-kind" title={`See the model of ${leads.name.replaceAll('`', '')}`} onClick={(event) => showModel(event.currentTarget, leads.called!)}>
                      <Words text={field.kind} />
                    </button>
                  ) : (
                    <span class="summary-field-kind">
                      <Words text={field.kind} />
                    </span>
                  )}
                  {(field.about ?? []).map((said) => (
                    <span key={said} class="summary-field-about">
                      {' '}
                      <Words text={said} />
                    </span>
                  ))}
                </dd>
              </div>
            );
          })}
        </dl>
      )}
      {(model.said ?? []).map((said) => (
        <p key={said} class="summary-model-said">
          <Words text={said} />
        </p>
      ))}
    </div>
  );
}

/**
 * A table's name in what a blueprint makes: pointed at, or reached by the
 * keyboard, its model in a card over the page; pressed, its model
 * unfolded under its row (`onPress`).
 */
export function ModelName({ name, model, open, onPress, children }: { name: string; model: Model; open: boolean; onPress: () => void; children: ComponentChildren }) {
  const anchor = useRef<HTMLButtonElement>(null);
  const [shown, setShown] = useState(false);
  const going = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const stay = () => clearTimeout(going.current);
  const show = () => {
    stay();
    setShown(true);
  };
  const leave = () => {
    stay();
    going.current = setTimeout(() => setShown(false), LINGER_MS);
  };
  useEffect(() => stay, []);
  // Listening from the commit that draws it, so that an Escape pressed as soon as it is there is its own.
  useLayoutEffect(() => {
    if (!shown) return;
    // Escape is the card's while it is shown: nothing under it hears it.
    const key = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      event.stopImmediatePropagation();
      stay();
      setShown(false);
    };
    window.addEventListener('keydown', key, true);
    return () => window.removeEventListener('keydown', key, true);
  }, [shown]);
  const said = name.replaceAll('`', '');
  return (
    <>
      <button
        ref={anchor}
        type="button"
        class="summary-words summary-model-name"
        aria-expanded={open}
        aria-label={`${said}, ${open ? 'its fields shown' : 'show its fields'}`}
        onMouseEnter={show}
        onMouseLeave={leave}
        onFocus={show}
        onBlur={leave}
        onClick={() => {
          stay();
          setShown(false);
          onPress();
        }}
      >
        {children}
      </button>
      {shown && (
        <Floating beside={() => (anchor.current ? besideBox(anchor.current.getBoundingClientRect(), 'under') : null)} class="summary-model-card" role="tooltip" label={`What ${said} is made of`}>
          <div class="summary-model-card-in" onMouseEnter={stay} onMouseLeave={leave}>
            <div class="summary-model-card-head">
              <Words text={name} />
            </div>
            <ModelView model={model} />
          </div>
        </Floating>
      )}
    </>
  );
}

/** The part Data model: every table a blueprint makes with fields, each with its model, where it makes two or more. */
export function DataModel({ tables }: { tables: Modelled[] }) {
  return (
    <section class="summary-part" data-part="model">
      <h2>
        Data model<span class="summary-count">{tables.length}</span>
      </h2>
      <ul class="summary-lines summary-models">
        {tables.map((table) => (
          <li key={table.called ?? table.name} class="summary-modelled" data-model={table.called ?? undefined} tabIndex={-1}>
            <h3 class="summary-model-head">
              <Words text={table.name} />
            </h3>
            <ModelView model={table.model} tables={tables} />
          </li>
        ))}
      </ul>
    </section>
  );
}
