// What is asked of a card's tabs from elsewhere in its drawer: from its
// Details, that its Comments be shown with the box having the keyboard,
// where what it waits on is answered; and from a question in its
// Comments, that the tab of the table whose row asked be shown, where
// what the question names is marked. The tabs keep which shows
// (`views/cardtabs.tsx`); these only ask, a card at a time.

import { signal } from '@preact/signals';

/** A tab of a card asked for: the card, and the tab (`@comments`, or a table's name). */
export const tabAsked = signal<{ card: string; tab: string } | null>(null);

/** The tab of a card's Comments, as its tabs keep it. */
export const COMMENTS = '@comments';

/** Shows `card`'s Comments with the box something is said on it in having the keyboard: where what it asks is answered. */
export function answerIn(card: string): void {
  tabAsked.value = { card, tab: COMMENTS };
}

/** Shows `card`'s tab of the table `table`. */
export function showCardTab(card: string, table: string): void {
  tabAsked.value = { card, tab: table };
}
