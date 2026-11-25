// Where a check's daemon answers is what the daemon says, and never a port
// the check found for it.
//
// Each check used to find a port first: it listened on port 0, read the
// number the system gave, let go, and told its daemon to listen there. A
// number that is let go is the system's to give again, to whoever asks
// next, and on a machine where suites and gates start daemons side by
// side somebody had asked before the check's daemon came to listen: it
// ended saying that it could not, `Address already in use`, and the
// check waited for a line that never came.
//
// So a check tells its daemon to take any port (`--listen 127.0.0.1:0`),
// which the system gives it in the moment it listens and to nobody else,
// and reads the address from the line the daemon prints once it is
// there. Nothing is picked ahead, so there is nothing to lose. A check
// that listens for itself (something put in front of a daemon) listens
// on port 0 the same way, keeps the listener and reads its address.
//
// And a daemon run from where cargo built it serves this checkout's
// client as it was last built: where that was not built from the files
// here, the check is told so there, before it reads a page of it
// (`built.ts`); where it is built again while the check runs, a check
// that fails says so as it exits.

import type { Readable } from 'node:stream';
import { builtFromHere, heldTo, servesThisClient } from './built.ts';
// Every check reads where its daemon is from here, so this is where it is told to wear a look (`LOOK`).
import './looked.ts';
// And so this is where it is told it is in the desktop app's window, with tabs, unless it is of the browser's way (`windowed.ts`).
import './windowed.ts';
// And with what every daemon a check starts is started with: settings of its own (`started.ts`).
import './started.ts';

/** What is asked of a daemon here: what it prints, and that it ended. */
export interface Started {
  stdout: Readable | null;
  once(event: 'exit', heard: (code: number | null, signal: string | null) => void): unknown;
  off(event: 'exit', heard: (code: number | null, signal: string | null) => void): unknown;
  /** The program it is, as a child that was spawned says. */
  spawnfile?: string;
}

/** The line a daemon says it is running in, whole: its address ends the line. */
const RUNNING = /is running at (\S+)\r?\n/;

/**
 * Where `daemon` says it is running, once it has: the address of the
 * client, as `http://127.0.0.1:<port>`. `daemon` is a child whose output
 * is a pipe. A daemon that ends before it has said so fails the check
 * there, with how it ended; and so does one that serves a client of this
 * checkout not built from the files here. The build it serves is noted,
 * so that a check that fails says where it was built again meanwhile.
 */
export function saysWhere(daemon: Started): Promise<string> {
  return new Promise((ready, failed) => {
    const out = daemon.stdout;
    if (!out) throw new Error("what the daemon prints is not a pipe: it is started with `stdio: [.., 'pipe', ..]`");
    let said = '';
    const ended = (code: number | null, signal: string | null) =>
      failed(new Error(`the daemon ended (${signal ?? code}) before it said where it is running`));
    const heard = (chunk: unknown) => {
      // A line may come in pieces: the address is read once its line has ended.
      said += String(chunk);
      const found = RUNNING.exec(said);
      if (!found?.[1]) return;
      out.off('data', heard);
      daemon.off('exit', ended);
      // What it prints from here on is read and dropped, so that it is
      // never held up at a pipe nobody empties.
      out.resume();
      const where = found[1];
      if (daemon.spawnfile && servesThisClient(daemon.spawnfile))
        builtFromHere().then((build) => {
          heldTo(build);
          ready(where);
        }, failed);
      else ready(where);
    };
    out.on('data', heard);
    daemon.once('exit', ended);
  });
}
