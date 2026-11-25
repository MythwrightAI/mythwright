import assert from 'node:assert/strict';
import { test } from 'node:test';
import { changes, made } from './changes.ts';

test('a text laid out again is changed only where it differs, so a cursor stays in its line', () => {
  const before = 'steps:\n    build:\n        do: shell\n        with: {run: make}\n';
  const after = 'steps:\n  build:\n    do: shell\n    with: { run: make }\n';
  const list = changes(before, after);
  assert.equal(made(before, list), after);
  // Three lines changed, each by what was taken from its front, and the
  // last by the spaces put inside its braces: nothing else is touched.
  assert.deepEqual(list.slice(0, 2), [
    { from: 9, to: 11, insert: '' },
    { from: 22, to: 26, insert: '' },
  ]);
  assert.ok(list.every((change) => change.to - change.from <= 4 && change.insert.length <= 1), 'no change swallows what it did not need to');
});

test('a line that is only set apart by a new blank line, and laid out, is still changed where it is', () => {
  const before = 'steps:\n  a:\n    do: noop\n  rough: {do:   noop}';
  const after = 'steps:\n  a:\n    do: noop\n\n  rough: { do: noop }\n';
  const list = changes(before, after);
  assert.equal(made(before, list), after);
  // A cursor in the middle of `noop` is before and after no change that swallows it.
  const cursor = before.lastIndexOf('noop') + 2;
  assert.ok(list.every((change) => change.to <= cursor || change.from >= cursor), JSON.stringify(list));
});

test('a line laid out with a value put in quotes, and set apart, is still changed where it is', () => {
  const before = 'steps:\n  a:\n    do: noop\n  rough: {do:   noop,when: ${ params.channel!="" }}';
  const after = 'steps:\n  a:\n    do: noop\n\n  rough: { do: noop, when: \'${ params.channel!="" }\' }\n';
  const list = changes(before, after);
  assert.equal(made(before, list), after);
  // The quotes are two small changes after the word the cursor is in.
  const cursor = before.lastIndexOf('noop') + 2;
  assert.ok(list.every((change) => change.to <= cursor || change.from >= cursor), JSON.stringify(list));
  assert.ok(list.every((change) => change.to - change.from <= 2 && change.insert.length <= 2), JSON.stringify(list));
});

test('lines put in, taken out and replaced by more or fewer are each one change', () => {
  const cases: [string, string][] = [
    ['a\nb\nc', 'a\nc'],
    ['a\nc', 'a\nb\nc'],
    ['a\nb', 'a'],
    ['a', 'a\nb'],
    ['b', 'a\nb'],
    ['a\nb\nc\n', 'a\nx\ny\nc\n'],
    ['a\nx\ny\nc\n', 'a\nb\nc\n'],
    ['', 'a\n'],
    ['a\n', ''],
    ['with: { a: 1, b: 2 }\nnext: 1\n', 'with:\n  a: 1\n  b: 2\nnext: 1\n'],
    ['same', 'same'],
  ];
  for (const [before, after] of cases) {
    const list = changes(before, after);
    assert.equal(made(before, list), after, `${JSON.stringify(before)} to ${JSON.stringify(after)}`);
    for (let k = 1; k < list.length; k += 1) assert.ok(list[k - 1]!.to <= list[k]!.from, 'in order, none touching another');
  }
  assert.deepEqual(changes('same', 'same'), []);
});

test('any text becomes any other', () => {
  // The same texts every run: a count that is carried on, not chance.
  let seed = 42;
  const next = (below: number) => {
    seed = (seed * 1103515245 + 12345) % 2147483648;
    return seed % below;
  };
  const words = ['a:', '  b: 1', '  c: { d: 2 }', '', '# said', '    e', 'f: [1, 2]'];
  const text = () => Array.from({ length: next(12) }, () => words[next(words.length)]).join('\n');
  for (let round = 0; round < 300; round += 1) {
    const before = text();
    const after = text();
    assert.equal(made(before, changes(before, after)), after, `${JSON.stringify(before)} to ${JSON.stringify(after)}`);
  }
});
