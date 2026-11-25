import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { test } from 'node:test';

import { followsAny, Stream } from './events.ts';

// What the daemon passes on as it says it and does not keep, read from
// where it is written (`conductor::owed::OWN`): what the stream's `behind`
// frame names.
const owed = readFileSync(new URL('../../crates/conductor/src/owed.rs', import.meta.url), 'utf8');
const PASSED_ON: string[] = [
  ...(owed.match(/pub const OWN: \[&str; \d+\] = \[([^\]]*)\]/)?.[1] ?? '').matchAll(/"([^"]+)"|SKIPPED/g),
].map((found): string => found[1] ?? 'step.skipped');

test('the topics the daemon passes on are read from where it says them', () => {
  assert.ok(PASSED_ON.includes('run.finished') && PASSED_ON.includes('step.skipped'), PASSED_ON.join(' '));
  assert.equal(PASSED_ON.length, 11);
});

test('where the stream is behind, whatever follows a topic it names reads again, and nothing else does', () => {
  const stream = new Stream();
  const read: string[] = [];
  stream.on(['step.*', 'run.finished'], () => {}, () => read.push('run'));
  stream.on('board.**', () => {}, () => read.push('board'));
  stream.on(['run.*'], () => {});
  stream.behind(['run.finished', 'step.started']);
  assert.deepEqual(read, ['run']);
  assert.ok(followsAny(['**'], ['step.started']));
  assert.ok(!followsAny(['instance.*', 'agent.turn.*'], PASSED_ON));
});

/** Every `.ts` and `.tsx` of the client that is not a test. */
function sources(dir: URL): [string, string][] {
  const found: [string, string][] = [];
  for (const name of readdirSync(dir)) {
    const at = new URL(name, dir);
    if (statSync(at).isDirectory()) found.push(...sources(new URL(`${name}/`, dir)));
    else if (/\.tsx?$/.test(name) && !name.includes('.test.')) found.push([at.pathname, readFileSync(at, 'utf8')]);
  }
  return found;
}

/** The arguments of the call whose `(` is at `open`, each as it is written. */
function argumentsAt(text: string, open: number): string[] {
  const parts: string[] = [];
  let depth = 0;
  let start = open + 1;
  for (let at = open; at < text.length; at++) {
    const c = text[at] ?? '';
    if (c === '/' && text[at + 1] === '/') at = text.indexOf('\n', at);
    else if (c === '/' && text[at + 1] === '*') at = text.indexOf('*/', at) + 1;
    else if (c === "'" || c === '"' || c === '`') {
      const end = text.indexOf(c, at + 1);
      at = end < 0 ? text.length : end;
    } else if ('([{'.includes(c)) depth++;
    else if (')]}'.includes(c)) {
      depth--;
      if (depth === 0) {
        parts.push(text.slice(start, at));
        return parts.map((part) => part.trim()).filter(Boolean);
      }
    } else if (c === ',' && depth === 1) {
      parts.push(text.slice(start, at));
      start = at + 1;
    }
  }
  return parts;
}

/** The topics a pattern argument says, following a name to the array it is in the same file. */
function patternsOf(text: string, written: string, seen = new Set<string>()): string[] {
  const words = [...written.matchAll(/'([^']*)'|"([^"]*)"/g)].map((found): string => found[1] ?? found[2] ?? '');
  for (const [, name = ''] of written.matchAll(/(?:^|[\s,[.])([A-Z_][A-Z0-9_]*)\b/g)) {
    if (seen.has(name)) continue;
    seen.add(name);
    const declared = text.match(new RegExp(`const ${name}\\s*=\\s*(\\[[^\\]]*\\])`));
    if (declared) words.push(...patternsOf(text, declared[1] ?? '', seen));
  }
  return words;
}

test('whatever in the client follows what the daemon passes on says how it reads again', () => {
  const lacking: string[] = [];
  for (const [file, text] of sources(new URL('./', import.meta.url))) {
    if (file.endsWith('/events.ts')) continue;
    for (const [call, needs] of [['useEvents(', 4], ['stream.on(', 3]] as const) {
      let at = text.indexOf(call);
      while (at >= 0) {
        const args = argumentsAt(text, at + call.length - 1);
        const patterns = args[0] ? patternsOf(text, args[0]) : [];
        if (followsAny(patterns, PASSED_ON) && args.length < needs) {
          lacking.push(`${file.split('/src/')[1]}: ${call}${args[0]} …) follows what is passed on and says no \`again\``);
        }
        at = text.indexOf(call, at + 1);
      }
    }
  }
  assert.deepEqual(lacking, []);
});
