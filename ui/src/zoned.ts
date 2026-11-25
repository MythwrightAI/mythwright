/**
 * A time of day written for a field that says its words are one with the
 * zone it is in after it (`zoned`, as `at` of a project's triggers):
 * where somebody writes a time with no zone, their own is written after
 * it, the one they read every time in (`worn` of `clock.ts`). What is not
 * a time, and what says a zone already, is left as it was written: the
 * daemon says what is wrong with it.
 */

/** A clock of twenty-four hours, as the daemon reads one: `02:00`. */
const CLOCK = /^\d{2}:\d{2}$/;

/** `written`, with `zone` after it where it is a time that says none. */
export function zoned(written: string, zone: string): string {
  const words = written.trim().split(/\s+/);
  const last = words[words.length - 1] ?? '';
  if (!zone || words.length > 2 || !CLOCK.test(last)) return written;
  return `${words.join(' ')} ${zone}`;
}
