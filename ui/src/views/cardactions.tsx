// What a card's column offers to be pressed on it: on the board, that the
// card waits for one; where the card is opened, each as a button, or as
// pressed, by whom and when.
//
// The hold is the fleet's and the runner's, and this is one place it is
// pressed from: a press goes through the daemon's one door, as whoever is
// signed in here, and what is drawn follows what is said of each press
// (`board.item.pressed`, `board.item.held`) with nothing read again. The
// button is drawn only for whoever the daemon would let press it; the
// daemon asks the same and refuses in its own words (`cardactions.ts`).

import { useState } from 'preact/hooks';
import { call } from '../api.ts';
import type { Item } from '../api.ts';
import { after, comingOf, mayPress, offered, pressedOf, waitsFor, whoPressed } from '../cardactions.ts';
import type { Action } from '../cardactions.ts';
import { attempt, me } from '../store.ts';
import { useEvents } from '../ui.tsx';
import { ago } from '../util.ts';

/** What was said of a press, and of a card that waits for one. */
export const PRESSES = ['board.item.pressed', 'board.item.held'];

/** What a card's row keeps of what was pressed on it, where its board keeps that. */
export function keptOf(item: Item): unknown {
  return (item as Item & { pressed?: unknown }).pressed;
}

/** The card as it is once what was said of a press of it is taken in. */
export function taken(item: Item, topic: string, data: Record<string, unknown>, ts: number): Item {
  return { ...item, pressed: after(keptOf(item), topic, data, ts) } as Item;
}

/** On the board: what a card waits for in its column, where anything holds it there. */
export function Waiting({ item, actions }: { item: Item; actions: Action[] }) {
  const waits = waitsFor(actions, item.status, pressedOf(keptOf(item)), comingOf(item.moved));
  if (waits.length === 0) return null;
  const titles = waits.map((one) => one.title).join(', ');
  return (
    <span class="card-waits" title={`Nothing is begun for it here until ${titles} is pressed on it`}>
      Waits for {titles}
    </span>
  );
}

/**
 * Where a card is opened: each thing its column offers, pressed, with who
 * and when, and a way to take it back while the card is where it was
 * pressed; or the button, for whoever may press it; or who may, in words.
 */
export function CardActions({ item, actions }: { item: Item; actions: Action[] }) {
  const [kept, setKept] = useState<unknown>(keptOf(item));
  const [busy, setBusy] = useState('');
  useEvents(PRESSES, (event) => event.data?.id === item.id && setKept((was: unknown) => after(was, event.topic, event.data ?? {}, event.ts)), [item.id]);
  const coming = comingOf(item.moved);
  const here = offered(actions, item.status, pressedOf(kept), coming);
  if (here.length === 0) return null;
  const principal = me.value?.principal;
  const held = principal?.scopes ?? [];
  const kind = (me.value?.actor ?? '').split(':')[0] ?? '';
  const press = async (action: Action, back: boolean) => {
    setBusy(action.name);
    await attempt(() => call('POST', `/board/items/${encodeURIComponent(item.id)}/press`, { action: action.name, back }));
    setBusy('');
  };
  const waits = here.filter(({ action, press }) => action.holds && !press).map(({ action }) => action.title);
  return (
    <div class="card-actions" role="group" aria-label="What this column asks of it">
      <span class="faint">{waits.length > 0 ? `It waits here until ${waits.join(', ')} ${waits.length === 1 ? 'is' : 'are'} pressed` : `What ${item.status} asks of it`}</span>
      {here.map(({ action, press: done }) => {
        const may = mayPress(action, held, kind);
        return (
          <div class="card-action" key={action.id} data-action={action.name}>
            {done ? (
              <>
                <span class="card-action-done">
                  <strong>{action.title}</strong>
                  <span class="faint">
                    {' '}
                    pressed by {whoPressed(done, me.value?.actor ?? '')}, {ago(done.at)}
                  </span>
                </span>
                {may && done.coming === coming && (
                  <button type="button" class="ghost" disabled={busy === action.name} onClick={() => void press(action, true)}>
                    Take back
                  </button>
                )}
              </>
            ) : may ? (
              <button type="button" disabled={busy === action.name} onClick={() => void press(action, false)}>
                {action.title}
              </button>
            ) : (
              <span>
                <strong>{action.title}</strong>
                <span class="faint"> is pressed by {action.who === 'person' ? 'a person' : 'anyone who may change it'}{action.scope ? ` who holds ${action.scope}` : ''}</span>
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
}
