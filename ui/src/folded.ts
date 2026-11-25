// What a bot says again and again in a channel is folded under the latest
// of it: a status said every ten minutes by one blueprint's
// step is one message with "12 earlier updates" under it, and not a page
// of the same thing.
//
// Messages fold together where they follow one another in the channel and
// were each said by the same step of the same running blueprint, under the
// same name (`said_by`: its instance and its step; the run is another each
// time). A message somebody answered, or that names somebody with `@`, is
// never folded away: it stands by itself and ends what folds before it.
// What is folded is shown again by a press, which the tab keeps by the
// first message of what folds (`opened`), and a message that is asked for
// (a link to it) is never hidden: what holds it is shown unfolded.
//
// Functions of plain values, with their cases (`folded.test.ts`).

/** What is read of a message to fold it. */
export interface Said {
  id: string;
  author: string;
  body: string;
  replies?: number;
  said_by?: { instance: string; step: string; blueprint?: string } | null;
}

/** Whether `body` names somebody: `@` and a name, at the start of a word (an address `a@b.c` names nobody). */
export function namesSomebody(body: string): boolean {
  return /(^|[^\p{L}\p{N}_.@])@[\p{L}\p{N}_]/u.test(body);
}

/** What a message folds with others by: who said it and the step that had it said; nothing where it never folds. */
export function foldsBy(one: Said): string | null {
  if (!one.said_by || (one.replies ?? 0) > 0 || namesSomebody(one.body)) return null;
  return `${one.author}\n${one.said_by.instance}\n${one.said_by.step}`;
}

/** A message as it is drawn: the one before it as it is drawn, and, for the latest of what folds, what is under it. */
export interface Drawn<T extends Said> {
  message: T;
  /** What is drawn before it, which says whether it carries on from it. */
  follows?: T;
  /** On the latest of what folds: how many came before it, what they are kept as, and whether they are shown. */
  earlier?: { count: number; key: string; open: boolean };
}

/**
 * The messages of a channel as they are drawn: each run of two or more
 * that fold together is its latest, with the earlier ones under it, which
 * are drawn before it where its key is in `opened` or one of them is
 * `asked` for.
 */
export function folded<T extends Said>(list: readonly T[], opened: readonly string[], asked?: string | null): Drawn<T>[] {
  const drawn: Drawn<T>[] = [];
  let last: T | undefined;
  for (let at = 0; at < list.length; ) {
    const by = foldsBy(list[at]!);
    let end = at + 1;
    if (by !== null) while (end < list.length && foldsBy(list[end]!) === by) end += 1;
    if (end - at < 2) {
      drawn.push({ message: list[at]!, follows: last });
      last = list[at];
      at = end;
      continue;
    }
    const run = list.slice(at, end);
    const latest = run.at(-1)!;
    const key = run[0]!.id;
    const open = opened.includes(key) || (!!asked && run.slice(0, -1).some((one) => one.id === asked));
    if (open)
      for (const one of run.slice(0, -1)) {
        drawn.push({ message: one, follows: last });
        last = one;
      }
    drawn.push({ message: latest, follows: last, earlier: { count: run.length - 1, key, open } });
    last = latest;
    at = end;
  }
  return drawn;
}

/** What the press under the latest of what folds says. */
export function earlierSaid(count: number, open: boolean): string {
  const what = count === 1 ? '1 earlier update' : `${count} earlier updates`;
  return open ? `Hide ${what}` : what;
}

/** The keys kept as shown with one changed: shown, or not; only those of what folds in the channel now are kept. */
export function toggled(opened: readonly string[], key: string, open: boolean, there: readonly string[]): string[] {
  const kept = opened.filter((one) => one !== key && there.includes(one));
  return open ? [...kept, key] : kept;
}
