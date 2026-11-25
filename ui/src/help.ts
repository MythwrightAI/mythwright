// What the foot of the side panel says and offers beside the workspace:
// the functions of plain values, with nothing of the page in them, so
// that their tests can read them.
//
// A way to ask for support, which is an address whoever built the
// product wrote down (`support` of what the daemon says of itself) and is
// nothing where they wrote none; what somebody would say in a report of a
// problem, to be copied; and the releases, read from the daemon, with
// whether the newest is one this browser has been shown.

/** One release, as the daemon lists them: what its heading says, and where on the page of the notes it begins. */
export interface Release {
  title: string;
  at: string;
}

/**
 * Where support is asked for, as an address a browser opens: what was
 * written, where it is a page on the web or an address to write to, and
 * nothing otherwise. Nothing is drawn for nothing, and nothing else is
 * ever opened: not a script, and not a file.
 */
export function supportLink(said: string | null | undefined): string | null {
  const address = (said ?? '').trim();
  return /^(https?:\/\/|mailto:)[^\s"'<>]+$/i.test(address) ? address : null;
}

/** What is known of the program and of where it runs, for a report. Whatever is not known is left out. */
export interface Known {
  name?: string;
  version?: string;
  /** Which build of the client is being served. */
  build?: string | null;
  /** `installed` or `checkout`, as the daemon says where it runs from. */
  runsFrom?: string | null;
  /** The system of the device the daemon is on, and what it is built for. */
  os?: string | null;
  arch?: string | null;
  /** What the browser says it is. */
  browser?: string | null;
}

/**
 * What somebody would paste into a report of a problem: the program and
 * its version, the build of the client, the daemon's system and the
 * browser, a line each. It is put on the clipboard and sent nowhere.
 */
export function whatToSay(known: Known): string {
  const beside = [known.build ? `client build ${known.build}` : '', known.runsFrom === 'installed' ? 'a copy on its own' : known.runsFrom === 'checkout' ? 'run from its checkout' : ''].filter(Boolean);
  const program = [known.name, known.version].filter(Boolean).join(' ');
  const system = [known.os, known.arch].filter(Boolean).join(' ');
  return [
    program ? `${program}${beside.length ? ` (${beside.join(', ')})` : ''}` : '',
    system ? `System: ${system}` : '',
    known.browser ? `Browser: ${known.browser}` : '',
  ]
    .filter(Boolean)
    .join('\n');
}

/** The releases a reading tells of: only those that say what they are called and where they begin. */
export function releasesOf(read: unknown): Release[] {
  const listed = (read as { releases?: unknown } | null)?.releases;
  if (!Array.isArray(listed)) return [];
  return listed.flatMap((one) => (one && typeof one.title === 'string' && typeof one.at === 'string' && one.title ? [{ title: one.title, at: one.at }] : []));
}

/**
 * Whether there is something new to say: the newest release is not the
 * one this browser was last shown the list at. Nothing is new where no
 * release was read.
 */
export function isNews(releases: Release[], seen: string | null): boolean {
  const newest = releases[0]?.title;
  return newest !== undefined && newest !== seen;
}

/** The first letter of a name, as a capital: what stands for a workspace with no picture. Nothing of a name that is only spaces. */
export function letter(name: string): string {
  return ([...name.trim()][0] ?? '').toLocaleUpperCase();
}

/** How many of something, in words: `1 project`, `3 projects`. */
export function counted(count: number, one: string, many: string): string {
  return `${count} ${count === 1 ? one : many}`;
}
