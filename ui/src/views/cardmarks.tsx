// What a card says of itself in a line, wherever it is drawn small: on the
// board in its column, and in the order by what comes first.

import type { Item } from '../api.ts';
import { asks, waitsFor } from '../table.ts';
import { Assignee } from './assignee.tsx';
import { Tag } from './tag.tsx';

/** Whether a card has anything to say under its title: where it has not, no line is kept for it. */
export function hasMarks(item: Item): boolean {
  return Boolean(asks(item.more) || waitsFor(item.more) || item.labels.length > 0 || item.assignee);
}

/** A card's marks: that something is asked on it, that it waits its turn, whose it is, and its labels. */
export function CardMarks({ item }: { item: Item }) {
  return (
    <>
      {/* A card waiting on an answer says so where it will be seen. */}
      {asks(item.more) && (
        <span class="badge warning" title="Something was asked on this card, and it is waiting on an answer. Open it to give one.">
          question
        </span>
      )}
      {/* And one whose turn has not come, with what it waits for. */}
      {waitsFor(item.more) && (
        <span class="badge plain" title={`This card waits its turn: ${waitsFor(item.more)}. It is taken when a card being worked is handed on, asks something or fails.`}>
          waiting
        </span>
      )}
      <span class="grow" />
      {/* Whom it is assigned to, a line of its own as a property: where the card is too narrow
          for one line the words go onto the next, and the name is never cut short. */}
      {item.assignee && <Assignee assignee={item.assignee} said class="card-assigned" />}
      {/* Each label in the colour its name comes to, the same on every card, on a line of their own
          under the card's id: beside the id, a card with three read as a line of four things. */}
      {item.labels.length > 0 && (
        <span class="card-tags">
          {item.labels.map((label) => (
            <Tag key={label} of={label} small />
          ))}
        </span>
      )}
    </>
  );
}

