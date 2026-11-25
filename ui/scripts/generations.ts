// Which builds of the client stay on disk.
//
// People have the client open while it is rebuilt, and a page fetches some
// of its parts (the editor, the Reference) only when they are first
// used. So a build must not take away the files of the builds before it: a
// page still on one of those would ask for a part and be told it does not
// exist. A page is left open for days, and the client is rebuilt and
// installed several times a day, so it is not the build before that is
// kept but every build a page may still be on: those replaced within a
// week, and twenty of them at most, so that the folder cannot grow without
// end however often it is built (a build is some 3.6 MB, most of it parts
// every build has its own of, so twenty is some 75 MB at the worst).
//
// The same rule is the installed copy's (`xtask/src/generations.rs`, which
// a test of the tooling holds to these two figures).

/** One build: its id, when it was made (ms since the epoch), and the files it is made of, relative to `dist/`. */
export interface Build {
  id: string;
  /** Absent in a list a build from before this wrote: such a build is taken to have been made when it is first read. */
  at?: number;
  files: string[];
}

/** A day, in milliseconds. */
export const DAY = 24 * 60 * 60 * 1000;

/**
 * How long a build is kept once another has taken its place, in days. A
 * page left open over a weekend and the days either side of it is still
 * on a build that is there.
 */
export const KEPT_FOR_DAYS = 7;
export const KEPT_FOR = KEPT_FOR_DAYS * DAY;

/**
 * How many builds before the current one are kept at most, however young.
 * A week of installs at a few a day is under twenty; a development build
 * on every save is more, and its pages reload by themselves.
 */
export const KEPT_MOST = 20;

/**
 * The builds to keep once `made` exists, newest first: `made`, then each
 * build before it whose place was taken less than `KEPT_FOR` before `made`
 * was made, `KEPT_MOST` of them at most. A build's place is taken when the
 * one after it is made, so the build just before `made` is always kept.
 */
export function generations(before: Build[], made: Build & { at: number }): Build[] {
  // Building the same thing again is not a new generation, and must not
  // push the previous one out.
  const older = before
    .filter((build) => build.id !== made.id)
    .map((build) => ({ ...build, at: build.at ?? made.at }))
    // Stable, so builds that say the same time stay in the order listed.
    .sort((a, b) => b.at - a.at);
  const kept: Build[] = [made];
  for (const build of older) {
    const replaced = kept.at(-1)?.at ?? made.at;
    if (kept.length > KEPT_MOST || made.at - replaced >= KEPT_FOR) break;
    kept.push(build);
  }
  return kept;
}

/** The files among `present` that no kept build is made of. */
export function leftovers(kept: Build[], present: string[]): string[] {
  const wanted = new Set(kept.flatMap((build) => build.files));
  return present.filter((file) => !wanted.has(file));
}
