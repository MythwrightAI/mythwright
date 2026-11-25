import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { BlueprintLanguage } from '../api.ts';
import { language } from './fixture.ts';
import { scanLine, start } from './scan.ts';

/** Every piece of some text, a line at a time, as `type text`. */
function pieces(text: string, known: BlueprintLanguage | null = language): string[][] {
  let state = start(known);
  return text.split('\n').map((line) => {
    const read = scanLine(line, state, known);
    state = read.state;
    return read.tokens.filter((token) => token.type !== 'space').map((token) => `${token.type} ${line.slice(token.from, token.to)}`);
  });
}

test('the words of the language, the names somebody chose and their own keys are told apart', () => {
  const read = pieces(
    ['blueprint: kv-cache', 'params:', '  port: 7379', 'steps:', '  build:', '    do: shell', '    with:', '      run: cargo build', '    timeout: 30m'].join('\n'),
  );
  assert.deepEqual(read, [
    ['field blueprint', 'mark :', 'text kv-cache'],
    ['field params', 'mark :'],
    ['name port', 'mark :', 'number 7379'],
    ['field steps', 'mark :'],
    ['name build', 'mark :'],
    ['field do', 'mark :', 'action shell'],
    ['field with', 'mark :'],
    ['key run', 'mark :', 'text cargo build'],
    ['field timeout', 'mark :', 'number 30m'],
  ]);
});

test('a word that is a field in one place is only a key in another', () => {
  const read = pieces(['state:', '  steps: 0', 'steps:', '  a:', '    do: set', '    with: { when: now, timeout: 1 }'].join('\n'));
  assert.deepEqual(read[1], ['key steps', 'mark :', 'number 0']);
  assert.deepEqual(read[5], ['field with', 'mark :', 'mark {', 'key when', 'mark :', 'text now', 'mark ,', 'key timeout', 'mark :', 'number 1', 'mark }']);
});

test('a field the language does not have is not coloured as one', () => {
  assert.deepEqual(pieces('steps:\n  a:\n    timout: 5m')[2], ['key timout', 'mark :', 'number 5m']);
});

test('an expression is read as one piece wherever it is, with what is inside it', () => {
  const [line] = pieces('outputs:\n  address: "127.0.0.1:${params.port}"').slice(1);
  assert.deepEqual(line, ['key address', 'mark :', 'string "', 'string 127.0.0.1:', 'hole ${', 'root params', 'mark .', 'member port', 'hole }', 'string "']);

  const [when] = pieces('steps:\n  a:\n    when: ${ len(steps.build.out.stdout) > 0 && params.mode != "dry" }').slice(2);
  assert.deepEqual(when, [
    'field when', 'mark :', 'hole ${',
    'function len', 'mark (', 'root steps', 'mark .', 'member build', 'mark .', 'member out', 'mark .', 'member stdout', 'mark )',
    'operator >', 'number 0', 'operator &&', 'root params', 'mark .', 'member mode', 'operator !=', 'string "dry"', 'hole }',
  ]);
});

test('a colon, a hash, a comma and braces inside an expression do not end it', () => {
  const read = pieces('steps:\n  a:\n    do: set\n    with: { n: ${ a ? {b: 1} : c }, m: ${ x # y } } # after');
  assert.deepEqual(read[3], [
    'field with', 'mark :', 'mark {', 'key n', 'mark :',
    'hole ${', 'text a', 'operator ?', 'text {', 'text b', 'operator :', 'number 1', 'text }', 'operator :', 'text c', 'hole }',
    'mark ,', 'key m', 'mark :', 'hole ${', 'text x', 'text #', 'text y', 'hole }', 'mark }', 'comment # after',
  ]);
});

test('a condition written without its braces is still an expression', () => {
  assert.deepEqual(pieces('steps:\n  a:\n    when: params.port > 1024')[2], ['field when', 'mark :', 'root params', 'mark .', 'member port', 'operator >', 'number 1024']);
});

test('only true, false and null are special words', () => {
  const read = pieces('state:\n  a: true\n  b: no\n  c: null\n  d: on\n  e: 1.5\n  f: 1h30m\n  g: v1.2');
  assert.deepEqual(
    read.slice(1).map((line) => line[2]),
    ['atom true', 'text no', 'atom null', 'text on', 'number 1.5', 'number 1h30m', 'text v1.2'],
  );
});

test('comments are comments, and a hash inside a word is not one', () => {
  const read = pieces('# top\nstate:\n  colour: a#b # the colour\n  # between');
  assert.deepEqual(read, [['comment # top'], ['field state', 'mark :'], ['key colour', 'mark :', 'text a#b', 'comment # the colour'], ['comment # between']]);
});

test('the lines of a block are text until one comes back out, and an expression in them is still one', () => {
  const read = pieces('steps:\n  a:\n    do: shell\n    with:\n      run: |\n        echo "do: this"\n\n        echo ${params.name}\n      cwd: src');
  assert.deepEqual(read[4], ['key run', 'mark :', 'mark |']);
  assert.deepEqual(read[5], ['string echo "do: this"']);
  assert.deepEqual(read[7], ['string echo ', 'hole ${', 'root params', 'mark .', 'member name', 'hole }']);
  assert.deepEqual(read[8], ['key cwd', 'mark :', 'text src']);
});

test('a list of hooks is read item by item, at its own depth or its key\'s', () => {
  const read = pieces(
    ['steps:', '  a:', '    do: shell', '    on_start:', '      - emit: { topic: build.started }', '    on_fail:', '    - do: agent.consult', '      with: { prompt: why }', '    - retry', '    timeout: 5m'].join('\n'),
  );
  assert.deepEqual(read[4], ['mark -', 'field emit', 'mark :', 'mark {', 'key topic', 'mark :', 'text build.started', 'mark }']);
  assert.deepEqual(read[6], ['mark -', 'field do', 'mark :', 'action agent.consult']);
  assert.deepEqual(read[7], ['field with', 'mark :', 'mark {', 'key prompt', 'mark :', 'text why', 'mark }']);
  assert.deepEqual(read[8], ['mark -', 'action retry']);
  assert.deepEqual(read[9], ['field timeout', 'mark :', 'number 5m'], 'the list ended at its owner\'s next key');
});

test('a whole step on one line, and a mapping that runs across lines', () => {
  const read = pieces(['steps:', '  a: { do: shell, needs: [x, y], with: { run: "make" } }', '  b: {', '    call: std/slot,', '    with: { port: 1 }', '  }', '  c:', '    do: noop'].join('\n'));
  assert.deepEqual(read[1], [
    'name a', 'mark :', 'mark {', 'field do', 'mark :', 'action shell', 'mark ,', 'field needs', 'mark :', 'mark [', 'text x', 'mark ,', 'text y', 'mark ]', 'mark ,',
    'field with', 'mark :', 'mark {', 'key run', 'mark :', 'string "', 'string make', 'string "', 'mark }', 'mark }',
  ]);
  assert.deepEqual(read[3], ['field call', 'mark :', 'blueprint std/slot', 'mark ,']);
  assert.deepEqual(read[4], ['field with', 'mark :', 'mark {', 'key port', 'mark :', 'number 1', 'mark }']);
  assert.deepEqual(read[7], ['field do', 'mark :', 'action noop'], 'reading went on after the mapping closed');
});

test('handlers hold steps of their own', () => {
  const read = pieces(['on:', '  ready:', '    event: board.item.moved', '    scope: own', '    steps:', '      round: { spawn: std/lane-round, as: "round-${event.data.id}" }'].join('\n'));
  assert.deepEqual(read[1], ['name ready', 'mark :']);
  assert.deepEqual(read[2], ['field event', 'mark :', 'text board.item.moved']);
  assert.deepEqual(read[5]?.slice(0, 6), ['name round', 'mark :', 'mark {', 'field spawn', 'mark :', 'blueprint std/lane-round']);
});

test('text is read without the language too, only less is known about it', () => {
  assert.deepEqual(pieces('steps:\n  a:\n    do: shell # go', null)[2], ['key do', 'mark :', 'text shell', 'comment # go']);
});

test('half-written text is read without complaint', () => {
  for (const text of ['steps:\n  a:\n    do', 'x: "never closed', 'x: ${ never closed', 'x: [a, {b: ', '  \t odd', '- - -', 'a: {{{{', 'a: }}}', ': :', 'k: |x']) {
    assert.doesNotThrow(() => pieces(text), text);
  }
});
