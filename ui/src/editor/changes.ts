// The edits that turn one text into another, as small as they can
// cheaply be made.
//
// When a text is laid out again, or the file was changed by somebody
// else, the editor is handed the whole new text. Putting it in whole
// would throw the cursor to one end of it. Put in as the lines that
// changed, and within a line the part of it that changed, the cursor
// stays where it was in the text, and the whole of it is one step to undo.

export interface Change {
  from: number;
  to: number;
  insert: string;
}

/** More lines than this either side of what differs, and the middle is replaced as one piece. */
const TOO_MANY = 1_000_000;

/** The changes to `before` that make it `after`: in order, none touching another. */
export function changes(before: string, after: string): Change[] {
  if (before === after) return [];
  const a = before.split('\n');
  const b = after.split('\n');
  // Where each line of `before` starts.
  const at: number[] = [];
  let offset = 0;
  for (const line of a) {
    at.push(offset);
    offset += line.length + 1;
  }

  let head = 0;
  while (head < a.length && head < b.length && a[head] === b[head]) head += 1;
  let tail = 0;
  while (tail < a.length - head && tail < b.length - head && a[a.length - 1 - tail] === b[b.length - 1 - tail]) tail += 1;

  const out: Change[] = [];
  const hunk = (i0: number, i1: number, j0: number, j1: number) => {
    if (i1 - i0 === j1 - j0) {
      // As many lines as before: each is the line it was, changed.
      for (let k = 0; k < i1 - i0; k += 1) within(at[i0 + k]!, a[i0 + k]!, b[j0 + k]!, out);
      return;
    }
    const lines = b.slice(j0, j1).join('\n');
    if (i1 === i0) {
      // Lines put in: before the line that follows, or after the last.
      if (i0 < a.length) out.push({ from: at[i0]!, to: at[i0]!, insert: `${lines}\n` });
      else out.push({ from: before.length, to: before.length, insert: `\n${lines}` });
    } else if (j1 === j0) {
      // Lines taken out, each with the line break that went with it.
      if (i1 < a.length) out.push({ from: at[i0]!, to: at[i1]!, insert: '' });
      else out.push({ from: Math.max(at[i0]! - 1, 0), to: before.length, insert: '' });
    } else {
      out.push({ from: at[i0]!, to: at[i1 - 1]! + a[i1 - 1]!.length, insert: lines });
    }
  };

  const n = a.length - head - tail;
  const m = b.length - head - tail;
  if (n === 0 || m === 0 || n * m > TOO_MANY) {
    hunk(head, head + n, head, head + m);
    return out;
  }
  // The longest run of lines the two middles share, in order, taking for
  // the same line one that differs only in its spaces and its quotes:
  // laid out again, a line is still that line, and is changed where it
  // is, also where a value in it was put in quotes. What is between the
  // lines they share is what was put in, taken out or written anew.
  const bare = (line: string) => line.replace(/[\s"']+/g, '');
  const ka = a.slice(head, head + n).map(bare);
  const kb = b.slice(head, head + m).map(bare);
  const width = m + 1;
  const table = new Uint32Array((n + 1) * width);
  for (let i = n - 1; i >= 0; i -= 1) {
    for (let j = m - 1; j >= 0; j -= 1) {
      table[i * width + j] = ka[i] === kb[j] ? table[(i + 1) * width + j + 1]! + 1 : Math.max(table[(i + 1) * width + j]!, table[i * width + j + 1]!);
    }
  }
  let i = 0;
  let j = 0;
  let fromI = 0;
  let fromJ = 0;
  while (i < n && j < m) {
    if (ka[i] === kb[j]) {
      if (i > fromI || j > fromJ) hunk(head + fromI, head + i, head + fromJ, head + j);
      within(at[head + i]!, a[head + i]!, b[head + j]!, out);
      i += 1;
      j += 1;
      fromI = i;
      fromJ = j;
    } else if (table[(i + 1) * width + j]! >= table[i * width + j + 1]!) {
      i += 1;
    } else {
      j += 1;
    }
  }
  if (fromI < n || fromJ < m) hunk(head + fromI, head + n, head + fromJ, head + m);
  return out;
}

/**
 * One line that became another: the runs of characters that differ. A
 * line set in less and given room inside its braces is two small changes,
 * and a cursor between them stays between them.
 */
function within(start: number, was: string, now: string, out: Change[]): void {
  if (was === now) return;
  let head = 0;
  while (head < was.length && head < now.length && was[head] === now[head]) head += 1;
  let tail = 0;
  while (tail < was.length - head && tail < now.length - head && was[was.length - 1 - tail] === now[now.length - 1 - tail]) tail += 1;
  const n = was.length - head - tail;
  const m = now.length - head - tail;
  if (n === 0 || m === 0 || n * m > TOO_MANY) {
    out.push({ from: start + head, to: start + head + n, insert: now.slice(head, head + m) });
    return;
  }
  const width = m + 1;
  const table = new Uint32Array((n + 1) * width);
  for (let i = n - 1; i >= 0; i -= 1) {
    for (let j = m - 1; j >= 0; j -= 1) {
      table[i * width + j] =
        was[head + i] === now[head + j] ? table[(i + 1) * width + j + 1]! + 1 : Math.max(table[(i + 1) * width + j]!, table[i * width + j + 1]!);
    }
  }
  let i = 0;
  let j = 0;
  let fromI = 0;
  let fromJ = 0;
  const run = (toI: number, toJ: number) => {
    if (toI > fromI || toJ > fromJ) out.push({ from: start + head + fromI, to: start + head + toI, insert: now.slice(head + fromJ, head + toJ) });
  };
  while (i < n && j < m) {
    if (was[head + i] === now[head + j]) {
      run(i, j);
      i += 1;
      j += 1;
      fromI = i;
      fromJ = j;
    } else if (table[(i + 1) * width + j]! >= table[i * width + j + 1]!) {
      i += 1;
    } else {
      j += 1;
    }
  }
  run(n, m);
}

/** `text` with `list` made to it: what an editor does with them. */
export function made(text: string, list: Change[]): string {
  let out = text;
  for (let k = list.length - 1; k >= 0; k -= 1) {
    const change = list[k]!;
    out = out.slice(0, change.from) + change.insert + out.slice(change.to);
  }
  return out;
}
