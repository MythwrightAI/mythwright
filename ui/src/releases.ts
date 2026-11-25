// The releases as the page has them, and whether the newest is one this
// browser has been shown.
//
// The notes of each release came with the program: the daemon hands out
// which there are and the notes as a page, and nothing is fetched from
// anywhere else. They are read once it is known who is signed in. What
// was last shown is one person's own: kept in this browser under who is
// signed in, as the workspace they look at is, and asked of nobody; a
// browser that keeps nothing goes on with what it was shown since the
// page was opened. What all of that comes to is `help.ts`.

import { computed, effect, signal } from '@preact/signals';
import { call } from './api.ts';
import { isNews, releasesOf } from './help.ts';
import type { Release } from './help.ts';
import { me, openTab } from './store.ts';

const SAVED = 'client.releases.seen';
const kept = (actor: string) => `${SAVED}:${actor}`;

/** The releases there are, the newest first. Empty until they are read. */
export const releases = signal<Release[]>([]);
/** The notes as a page, as the daemon made it: nothing until it is read. */
export const notes = signal<string | null>(null);
/** The newest release this browser was shown the list at. */
const seen = signal<string | null>(null);

/** Reads them, as whoever is signed in. A daemon from before has none, and then nothing is new. */
export async function readReleases(): Promise<void> {
  try {
    const read = await call<{ releases: Release[]; html: string }>('GET', '/releases');
    releases.value = releasesOf(read);
    notes.value = typeof read.html === 'string' ? read.html : null;
  } catch {
    releases.value = [];
    notes.value = null;
  }
}

const actor = computed(() => me.value?.actor ?? null);
// Read once it is known who is signed in, with what this browser kept for them; let go of when nobody is.
effect(() => {
  const who = actor.value;
  seen.value = null;
  if (!who) {
    releases.value = [];
    notes.value = null;
    return;
  }
  try {
    seen.value = localStorage.getItem(kept(who));
  } catch {
    /* nothing is kept here: shown once for each time the page is opened */
  }
  void readReleases();
});

/** Whether the newest release is one this browser has not been shown the list at. */
export const news = computed(() => isNews(releases.value, seen.value));

/** The list of releases was shown: what is newest is new no longer. */
export function sawReleases(): void {
  const newest = releases.peek()[0]?.title;
  if (newest === undefined || newest === seen.peek()) return;
  seen.value = newest;
  const who = actor.peek();
  if (!who) return;
  try {
    localStorage.setItem(kept(who), newest);
  } catch {
    /* not remembered */
  }
}

/** Which part of the notes somebody asked for, and which asking it was: the page brings each into view once. */
export const shownRelease = signal<{ at: string; asked: number } | null>(null);

/** Opens the notes at one release, or with nothing said at their head. */
export function showRelease(at = ''): void {
  shownRelease.value = { at, asked: (shownRelease.peek()?.asked ?? 0) + 1 };
  openTab({ kind: 'releases' });
}
