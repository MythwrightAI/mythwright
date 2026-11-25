// What a check starts is ended when the check is over, however it ends.
//
// Each check has a daemon of its own and stops it where its steps end, in
// a `finally`. That is reached where a step throws inside the `try`, and
// not otherwise: not where something throws before it or outside it (the
// walkthrough has none: a step that timed out left its daemon running),
// and not where the check itself is ended from outside, by a time limit
// or from the keyboard. A daemon left so holds its port and its data, and
// goes on for as long as the machine does.
//
// So a check says of each daemon it starts that it ends with the check
// (`endsWithThis`), which is held as the script exits, whatever made it
// exit. The check's own stop still comes first where it is reached, and
// there is then nothing left for this to do.
//
// A daemon leaves its services running when it stops, on purpose. A check
// that starts one stops the instance that owns it before its daemon, and
// says so here too (`stopsAll`), or a check cut short leaves the service.
//
// And as it exits a check looks for what it left, as the gate looks after
// the tests (`cargo xtask left`, which lists what is running here that
// nobody waits for, and ends nothing): what this check started and is
// still there once what was told to stop has had a moment to, is said,
// and the check fails for it.
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

/** What is still to be ended: each child with what is done before it is told. */
const started = new Map();
let listening = false;

function endAll() {
  for (const [child, first] of started) {
    if (child.exitCode !== null || child.signalCode !== null) continue;
    try {
      first?.();
    } catch {
      // It is told to end all the same.
    }
    child.kill('SIGTERM');
  }
  started.clear();
  looked();
}

/** The repository's own tooling, where it is built: wherever a gate has been run. */
const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const tooling = join(repo, 'target/debug/xtask');
const asked = (...args) => (existsSync(tooling) ? spawnSync(tooling, ['left', ...args], { cwd: repo, encoding: 'utf8' }) : null);

/** What was left running here before this check started anything: not its own. */
let before = '';

/**
 * Says what this check leaves running, and has the check fail for it:
 * what came from it and is still there, and what has lost its parent
 * here since it began. Where the tooling is not built nothing is said.
 */
function looked() {
  const found = asked('--after', String(process.pid), '--but', before);
  // Only what it says is left: a tooling that could not look says something else.
  if (found?.status !== 1 || !found.stdout.startsWith('left running')) return;
  console.error(`\nThis check leaves something running. A check stops what it starts before it is over, and a daemon leaves its services running, on purpose: the instance that owns one is stopped first.\n${found.stdout.trim()}`);
  process.exitCode = 1;
}

/**
 * Has `child` ended when this script is over: told to stop as the script
 * exits, if it is still running then. `first` is done before it is told,
 * and has to be done by the time it returns: nothing is waited for as a
 * script exits. Hands `child` back.
 */
export function endsWithThis(child, first) {
  started.set(child, first);
  child.once('exit', () => started.delete(child));
  if (!listening) {
    listening = true;
    before = asked('--numbers')?.stdout.trim() ?? '';
    process.on('exit', endAll);
    // Ended from outside, a script goes without saying that it exits
    // unless it is made to: with the code a shell gives for the signal.
    for (const [signal, code] of [['SIGHUP', 129], ['SIGINT', 130], ['SIGTERM', 143]]) {
      process.once(signal, () => process.exit(code));
    }
  }
  return child;
}

/**
 * What stops every instance the daemon at `url` was asked for, and with
 * each the services it owns: for `endsWithThis`, by a check that starts a
 * service. `options` are what the check runs its commands with (`cwd`,
 * `env`). Each command is given ten seconds, so a daemon that no longer
 * answers holds nothing up.
 */
export function stopsAll(binary, url, options) {
  const ask = (...args) => spawnSync(binary, [...args, '--url', url], { ...options, encoding: 'utf8', timeout: 10_000 });
  return () => {
    const listed = ask('instances', '--json');
    for (const instance of JSON.parse(listed.stdout || '{}').instances ?? []) {
      if (!instance.parent) ask('stop', instance.id);
    }
  };
}
