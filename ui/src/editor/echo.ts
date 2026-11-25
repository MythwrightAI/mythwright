// Telling an editor's own words from somebody else's.
//
// The editor says what its text is now; whoever holds the text keeps that
// and, a moment later, hands it back as "the text". If more was typed in
// that moment, what comes back is already old, and putting it into the
// editor would undo the typing. So what the editor said is remembered until
// it comes back, and what comes back is only put in when it is not
// something the editor said itself: the file read again, say.

/** What the editor has said that has not come back yet, oldest first. */
export type Said = string[];

/** Notes something the editor said. */
export function say(said: Said, text: string): void {
  said.push(text);
  // Nobody is listening, or they keep something else: do not grow for ever.
  if (said.length > 64) said.splice(0, said.length - 64);
}

/**
 * Whether a text handed to the editor is one it said itself. If it is,
 * that and everything said before it has now come back.
 */
export function isEcho(said: Said, text: string): boolean {
  const at = said.lastIndexOf(text);
  if (at < 0) {
    said.length = 0;
    return false;
  }
  said.splice(0, at + 1);
  return true;
}
