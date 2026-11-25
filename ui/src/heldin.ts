// Which card an agent's turn is about, read from the conversation the turn
// is part of: the words the daemon holds a card's conversations by
// (`spent::CONVERSATIONS` in the `deciding` crate), so that what an agent
// is doing, what it is writing, what a turn cost a card and its history
// all say the same of it.

/**
 * A card an agent has in hand, and how: it `works` it or `plans` it, and
 * then has the card itself; or it `reviews` it, with others, and the card
 * stays where it is for each of them to look at.
 */
export interface Held {
  card: string;
  how: 'works' | 'plans' | 'reviews';
}

/**
 * What an agent has in hand, from the conversation its turn is part of:
 * a card's lane, its planning, or a seat at its review; one of several
 * pieces of work on a card at once (`contend:<card>:<way>`), which works
 * it; or a judge of those, alone or seated (`judge:<card>[:<seat>]`),
 * which looks at it as a reviewer does. Anything else an agent does (an
 * answer in a channel) is no card's.
 */
export function heldIn(thread: string | null | undefined): Held | null {
  const [kind, ...rest] = (thread ?? '').split(':');
  if (rest.length === 0 || rest[0] === '') return null;
  if (kind === 'lane') return { card: rest.join(':'), how: 'works' };
  if (kind === 'plan') return { card: rest.join(':'), how: 'plans' };
  // A rung of the chain that looks at a card that waits (`std/chain`).
  if (kind === 'chain') return { card: rest.join(':'), how: 'reviews' };
  // A seat at a review is the card and then which seat.
  if (kind === 'review' && rest.length > 1) return { card: rest.slice(0, -1).join(':'), how: 'reviews' };
  // A piece of work done beside others is the card and then which way; judges the card and then a seat, if any.
  if (kind === 'contend') return { card: rest[0] ?? '', how: 'works' };
  if (kind === 'judge') return { card: rest[0] ?? '', how: 'reviews' };
  return null;
}
