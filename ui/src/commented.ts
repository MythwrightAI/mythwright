// Comments on a record, as a list of records and a document's own page
// show them, with nothing of the page in it: what a comment's mark is
// called, which event says another comment was written, which of many
// comments are shown before the rest are asked for, and which of several
// readings of a count is kept.
//
// A comment is a line of the record's history that is something said
// (`comments` in `journey.ts`). How many each record of a table has is
// asked of the daemon once for the table, and again when a line of
// something said is written for one of its records: a count has no ids
// to go by, so it is read again and never added to.

/** The topics a line of history that is something said was written for; the second is what the first was called before anything but a card could be spoken on. */
const SAID = ['record.commented', 'board.item.commented'];

/** What says a line was added to a record's history. The line is written with it, so a reading asked for once it is heard holds the line. */
export const WRITTEN = 'collection.history';

/**
 * Whether an event says that something said was written into the history
 * of a record of the table `name` of `project`. What says only that it
 * was said (`record.commented`) comes before the line is written, and a
 * count read then would not hold it.
 */
export function commentedOn(event: { topic: string; data?: Record<string, unknown> }, project: string, name: string): boolean {
  const data = event.data ?? {};
  return event.topic === WRITTEN && data.project === project && data.name === name && typeof data.topic === 'string' && SAID.includes(data.topic);
}

/** What a comment mark is called, for whoever hears it read out: `3 comments on Install`, or `Comment on Install` where there is none. */
export function markCalled(count: number, title: string): string {
  if (count <= 0) return `Comment on ${title}`;
  return `${count} ${count === 1 ? 'comment' : 'comments'} on ${title}`;
}

/**
 * Keeps the reading that was asked for last. Of two readings of one thing
 * on their way the earlier asked may come in later, and says how the
 * thing was: it is let go, since the later was asked after whatever
 * prompted it was already so.
 */
export class Latest {
  private asked = 0;

  /** Reads, and hands back what was read, or nothing where another reading was asked for meanwhile. */
  async read<T>(read: () => Promise<T>): Promise<{ read: T } | null> {
    const mine = ++this.asked;
    const got = await read();
    return mine === this.asked ? { read: got } : null;
  }
}
