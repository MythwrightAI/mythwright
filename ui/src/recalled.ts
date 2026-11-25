// What a reader sent before, put back in the box by Up, as a terminal's
// history is.
//
// Nothing is kept for it: what they sent is what the page has read of the
// channel or the thread (`said.ts`), what arrived by an event with it, and
// of that the messages whose author is the reader. What is recalled is text
// to send again or change, and nothing of the message it came from: sending
// it says something new, and what was attached is not brought back.
//
// Functions of plain values with their cases (`recalled.test.ts`); the box
// that asks them is the channel's composer (`views/channel.tsx`).

/** What of a message the history goes by. */
export interface Said {
  author: string;
  body: string;
  thread?: string | null;
}

/**
 * What the reader sent where they are writing, oldest first: in a channel
 * what they said in it and not under a message, in a thread what they said
 * under its message. A message that said nothing (it only held what was
 * attached) is no line of it, and the same words sent twice running are one,
 * as a terminal's history keeps them.
 */
export function sentHere(said: readonly Said[] | undefined, reader: string | undefined, thread?: string): string[] {
  if (!said || !reader) return [];
  const sent: string[] = [];
  for (const one of said) {
    if (one.author !== reader) continue;
    if ((one.thread ?? undefined) !== thread) continue;
    if (one.body.trim() === '') continue;
    if (sent[sent.length - 1] === one.body) continue;
    sent.push(one.body);
  }
  return sent;
}

/** Where the reader is in what they sent, while they go back through it. */
export interface Recalling {
  /** How far back: 1 is the last thing they sent. */
  back: number;
  /** What was in the box before they began, given back past the newest. */
  draft: string;
  /** What was put in the box, by which it is known that they have not changed it since. */
  shown: string;
}

/** The box as the key comes down. */
export interface Box {
  text: string;
  /** Where the selection begins and ends. */
  from: number;
  to: number;
}

/** What the box is to hold after a key that is the history's, and where the reader then is in it. */
export interface Recalled {
  text: string;
  recalling: Recalling | null;
}

/** What else stands in the way of the history having a key. */
export interface Asked {
  /** A list of names or of commands is offered over the box: the arrows are its. */
  offered: boolean;
  /** On a phone, which has no arrows to speak of. */
  phone: boolean;
}

/**
 * What Up or Down comes to, or nothing where the key is the box's own (the
 * caret moved a line, as anywhere text is written).
 *
 * Up goes back where nothing is written before the caret (an empty box, or
 * the caret at its very start), or while the reader is going back through
 * what they sent and the caret is on the first line of what was put there;
 * it goes one further back each time, and stays at the first thing they
 * sent. Down goes forward while they are going back and the caret is on the
 * last line, and past the newest gives back what was in the box before they
 * began, empty or half written. A recalled text that was changed is theirs:
 * they are no longer going back through it (`going`), and Up begins again
 * from the newest with what they changed it to kept.
 */
export function recall(key: string, box: Box, sent: readonly string[], recalling: Recalling | null, asked: Asked): Recalled | null {
  if (key !== 'ArrowUp' && key !== 'ArrowDown') return null;
  if (asked.offered || asked.phone || box.from !== box.to) return null;
  const now = going(recalling, box.text);
  if (key === 'ArrowUp') {
    if (sent.length === 0) return null;
    const first = !box.text.slice(0, box.from).includes('\n');
    if (now ? !first : box.from !== 0) return null;
    const back = Math.min((now?.back ?? 0) + 1, sent.length);
    const text = sent[sent.length - back]!;
    return { text, recalling: { back, draft: now?.draft ?? box.text, shown: text } };
  }
  if (!now || box.text.slice(box.from).includes('\n')) return null;
  if (now.back <= 1) return { text: now.draft, recalling: null };
  const back = Math.min(now.back - 1, sent.length);
  const text = sent[sent.length - back]!;
  return { text, recalling: { ...now, back, shown: text } };
}

/** Whether the reader is still going back through what they sent: not once what was put in the box has been changed. */
export function going(recalling: Recalling | null, text: string): Recalling | null {
  return recalling && recalling.shown === text ? recalling : null;
}
