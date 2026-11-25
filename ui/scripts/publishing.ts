// One build at a time writes into dist/ and tidies it.
//
// Two builds of one checkout may run at once (the client's check while
// an install builds, a build in a terminal beside `npm run dev`). Each
// writes its files, reads which builds are kept, writes that list and
// takes away every file no kept build lists. Side by side, one read the
// list before the other had written its own, and so took away the other's
// new files, under the page the other's entry page then loaded; or wrote
// its list over the other's, which then had its files taken away by the
// next build. So a build holds `dist.lock`, beside dist/ and not in what
// is served, from its first file written to its last file taken away, and
// a second build waits for it.
//
// The lock is a file made whole in one step (written under a name of its
// own and linked to its place, which fails where one is there already),
// saying who holds it and since when. A build that was killed holding it
// leaves it, so one held by a process that is gone, or held longer than a
// build ever takes, is taken away: by whoever first makes the one claim
// to that lock there is (`<lock>.breaking.<token>`), and only while the
// lock is still that one, so that two who find it stale do not take away
// the lock a third has just made.

import { randomBytes } from 'node:crypto';
import { link, readFile, stat, unlink, writeFile } from 'node:fs/promises';
import { setTimeout as pause } from 'node:timers/promises';

/** What the lock is called, beside dist/ (beside the folder dist/ is, where it is a link): not in what is served. */
export const LOCK = 'dist.lock';

/**
 * How long a lock is taken to be held by a build that is at work, in
 * milliseconds. Writing and tidying a build takes a second or two; one
 * held five minutes was left by a build that is gone, whose process number
 * may be another's by now.
 */
export const STALE_AFTER = 5 * 60 * 1000;

/** How often a build waiting for the lock looks again. */
export const LOOK_EVERY = 50;

/** Who holds the lock: a process, a token that is this holding's alone, and since when (ms). */
export interface Held {
  pid: number;
  token: string;
  at: number;
}

export const lockText = (held: Held): string => `${held.pid} ${held.token} ${held.at}\n`;

/** What a lock says, or nothing where it says nothing that can be read. */
export function readLock(text: string): Held | null {
  const [pid, token, at, ...more] = text.trim().split(/\s+/);
  const held = { pid: Number(pid), token: token ?? '', at: Number(at) };
  if (more.length > 0 || !Number.isInteger(held.pid) || held.pid <= 0 || !/^[0-9a-f]+$/.test(held.token) || !Number.isFinite(held.at)) {
    return null;
  }
  return held;
}

/**
 * What a build that finds the lock does: take it where there is none, wait
 * where a build at work holds it, and take it away where whoever made it
 * is gone, has held it past `STALE_AFTER`, or wrote nothing that can be
 * read (a lock is made whole, so that is no build's).
 */
export function standing(lock: string | null, now: number, alive: (pid: number) => boolean): 'free' | 'held' | 'stale' {
  if (lock === null) return 'free';
  const held = readLock(lock);
  if (!held || now - held.at >= STALE_AFTER || !alive(held.pid)) return 'stale';
  return 'held';
}

/** Whether a process of that number is there, whoever's it is. */
function alive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (err) {
    return (err as NodeJS.ErrnoException).code === 'EPERM';
  }
}

const unlessGone = (err: NodeJS.ErrnoException) => {
  if (err.code !== 'ENOENT') throw err;
};

/** Takes away a lock found stale, where nobody else has taken it away or made another since. */
async function breakStale(lock: string, seen: string): Promise<void> {
  const token = readLock(seen)?.token ?? 'unread';
  const claim = `${lock}.breaking.${token}`;
  try {
    await writeFile(claim, `${process.pid}\n`, { flag: 'wx' });
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code !== 'EEXIST') throw err;
    // A claim left by somebody who died taking the lock away would keep it
    // for ever: one as old as a stale lock is taken away too.
    const made = await stat(claim).then((s) => s.mtimeMs, () => null);
    if (made !== null && Date.now() - made >= STALE_AFTER) await unlink(claim).catch(unlessGone);
    return;
  }
  try {
    const now = await readFile(lock, 'utf8').catch(() => null);
    if (now === seen) await unlink(lock).catch(unlessGone);
  } finally {
    await unlink(claim).catch(unlessGone);
  }
}

/** Does `work` holding the lock at `lock`, waiting for whoever holds it. */
export async function whileHolding<T>(lock: string, work: () => Promise<T>): Promise<T> {
  const held: Held = { pid: process.pid, token: randomBytes(8).toString('hex'), at: Date.now() };
  const mine = `${lock}.${held.token}`;
  let said = false;
  for (;;) {
    held.at = Date.now();
    await writeFile(mine, lockText(held));
    const taken = await link(mine, lock).then(
      () => true,
      (err: NodeJS.ErrnoException) => {
        if (err.code === 'EEXIST') return false;
        throw err;
      },
    );
    await unlink(mine).catch(unlessGone);
    if (taken) break;
    const seen = await readFile(lock, 'utf8').catch(() => null);
    const stands = standing(seen, Date.now(), alive);
    if (stands === 'stale' && seen !== null) await breakStale(lock, seen);
    else if (stands === 'held') {
      if (!said) console.log(`waiting for the build that holds ${lock} (process ${readLock(seen ?? '')?.pid})`);
      said = true;
      await pause(LOOK_EVERY);
    }
  }
  try {
    return await work();
  } finally {
    // Only our own: one taken from us as stale is somebody else's by now.
    const now = await readFile(lock, 'utf8').catch(() => null);
    if (now === lockText(held)) await unlink(lock).catch(unlessGone);
  }
}
