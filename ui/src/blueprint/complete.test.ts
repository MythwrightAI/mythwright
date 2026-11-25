import assert from 'node:assert/strict';
import { test } from 'node:test';
import { exampleAt, explain, suggest } from './complete.ts';
import type { Suggestion } from './complete.ts';
import { caret, language, world } from './fixture.ts';
import type { ToldLanguage } from './told.ts';

/** What is offered where the `|` is. */
function offered(marked: string): Suggestion[] {
  const { text, offset } = caret(marked);
  return suggest(text, offset, language, world)?.options ?? [];
}

function labels(marked: string): string[] {
  return offered(marked).map((one) => one.label);
}

/** The text after the suggestion called `label` is taken, with `|` where the caret lands. */
function taking(marked: string, label: string): string {
  const { text, offset } = caret(marked);
  const found = suggest(text, offset, language, world)!;
  const one = found.options.find((option) => option.label === label);
  assert.ok(one, `\`${label}\` is not offered; these are: ${found.options.map((option) => option.label).join(', ')}`);
  const put = one.insert ?? one.label;
  const at = found.from + (one.caret ?? put.length);
  const after = text.slice(0, found.from) + put + text.slice(offset);
  return `${after.slice(0, at)}|${after.slice(at)}`;
}

const BUILD = ['blueprint: kv', 'params:', '  port: 7379', '  profile: { type: string, default: release }', 'state:', '  builds: 0', 'steps:', '  check:', '    do: noop', '  build:', '    do: shell', '    with:', '      run: cargo build', ''].join('\n');

test('at the top of the file, the fields a blueprint has are offered, less the ones it has', () => {
  const found = labels('blueprint: kv\nsteps:\n  a: { do: noop }\n|');
  assert.deepEqual(found.slice(0, 4), ['version', 'description', 'params', 'state']);
  assert.ok(!found.includes('blueprint') && !found.includes('steps'), 'what is there is not offered again');
  assert.ok(found.includes('on') && found.includes('outputs'));
});

test('what is typed narrows nothing here: the editor filters, and is told where the word began', () => {
  const { text, offset } = caret('blueprint: kv\nper|');
  const found = suggest(text, offset, language, world)!;
  assert.equal(found.from, 14, 'the word being written starts the replacement');
  assert.ok(found.options.some((one) => one.label === 'permissions'));
});

test('a step is offered what a step may have, starting with what it does', () => {
  const found = offered('blueprint: kv\nsteps:\n  build:\n    |');
  assert.deepEqual(found.slice(0, 6).map((one) => one.label), ['do', 'call', 'spawn', 'task', 'with', 'needs']);
  assert.equal(found.find((one) => one.label === 'task')?.detail, 'names, each a step');
  const timeout = found.find((one) => one.label === 'timeout')!;
  assert.equal(timeout.detail, 'a length of time, such as 30s or 5m');
  assert.equal(timeout.note?.says, 'Fail the step if it runs longer, so that one that hangs does not hold the run up.');
  const said = (label: string) => found.find((one) => one.label === label)?.detail;
  assert.deepEqual(['retry', 'on_fail', 'with', 'needs'].map(said), ['a number, or a mapping', 'a list, each a hook', 'arguments', 'a list of steps']);
  assert.equal(offered('blueprint: kv\n|').find((one) => one.label === 'steps')?.detail, 'names, each a step');
});

test('a step that does one thing is not offered the others', () => {
  const found = labels('blueprint: kv\nsteps:\n  build:\n    do: shell\n    |');
  assert.ok(!found.includes('do') && !found.includes('call') && !found.includes('spawn'));
  assert.equal(found[0], 'with');
});

test('the name of a step is the writer\'s own, so nothing is offered for it', () => {
  assert.deepEqual(labels('blueprint: kv\nsteps:\n  bu|'), []);
});

test('after `do`, the actions this device has are offered, each saying what it takes and gives back', () => {
  const found = offered('blueprint: kv\nsteps:\n  build:\n    do: |');
  assert.deepEqual(found.map((one) => one.label), ['noop', 'emit', 'shell', 'channel.post', 'agent.consult']);
  const shell = found.find((one) => one.label === 'shell')!;
  assert.equal(shell.detail, 'shell');
  assert.equal(shell.note?.says, 'Run a command and wait for it to finish.');
  assert.deepEqual(shell.note?.lists?.map((list) => list.title), ['Takes', 'Gives back', 'Needs']);
  assert.deepEqual(shell.note?.lists?.[0]?.rows[2], { name: 'clear_env', type: 'bool', says: 'start from an almost empty environment' });
  assert.deepEqual(shell.note?.lists?.[1]?.rows[0], { name: 'code', says: 'the exit code' });
});

test('right after the colon, what is taken brings its own space', () => {
  assert.equal(taking('steps:\n  build:\n    do:|', 'shell'), 'steps:\n  build:\n    do: shell|');
  assert.equal(taking('steps:\n  build:\n    do: sh|', 'shell'), 'steps:\n  build:\n    do: shell|');
});

test('under `with`, the arguments of the action named beside it are offered, needed ones first', () => {
  const found = offered('steps:\n  tell:\n    do: channel.post\n    with:\n      as: parrot\n      |');
  assert.deepEqual(found.map((one) => `${one.label} (${one.detail})`), ['channel (string, needed)', 'body (string, needed)']);
  assert.equal(found[0]?.note?.says, 'Which channel.');
});

test('arguments are offered inside braces too, and before the action is named there are none', () => {
  assert.deepEqual(labels('steps:\n  build: { do: shell, with: { run: make, | } }'), ['cwd', 'clear_env']);
  assert.deepEqual(labels('steps:\n  build:\n    with:\n      |'), []);
  assert.deepEqual(labels('steps:\n  build:\n    with:\n      |\n    do: shell'), ['run', 'cwd', 'clear_env'], 'the action may be named after its arguments');
});

test('an argument that is true or false is offered both', () => {
  assert.deepEqual(labels('steps:\n  build:\n    do: shell\n    with:\n      clear_env: |'), ['true', 'false']);
  assert.deepEqual(labels('steps:\n  build:\n    do: shell\n    with:\n      run: |'), [], 'what a command is, nobody can guess');
});

test('after `call`, the blueprints there are; under its `with`, what the one named is started with', () => {
  const found = offered('steps:\n  host:\n    call: |');
  assert.deepEqual(found.map((one) => one.label), ['std/slot', 'deploy']);
  assert.equal(found[0]?.detail, 'needs params');
  assert.deepEqual(found[0]?.note?.lists?.[0]?.rows[0], { name: 'version', type: 'string, needed', says: 'which version to run' });

  const params = offered('steps:\n  host:\n    call: std/slot\n    with:\n      |');
  assert.deepEqual(params.map((one) => `${one.label} (${one.detail})`), ['version (string, needed)', 'port (int)']);
});

test('`needs` is offered the other steps of the same graph, less the ones already listed', () => {
  const text = 'steps:\n  check: { do: noop }\n  fetch: { do: noop }\n  build:\n    do: shell\n    needs: [check, |]\non:\n  tick:\n    every: 1m\n    steps:\n      other: { do: noop }\n';
  const found = offered(text);
  assert.deepEqual(found.map((one) => one.label), ['fetch']);
  assert.equal(found[0]?.detail, 'do: noop');
  assert.deepEqual(labels('steps:\n  a: { do: noop }\n  b:\n    do: noop\n    needs:\n      - |'), ['a']);
});

test('inside a task, the steps of the task are the graph: a step is offered there, and `needs` and `steps.` its neighbours', () => {
  const task = 'steps:\n  fetch: { do: noop }\n  build:\n    retry: 2\n    task:\n      compile: { do: shell }\n      test:\n';
  assert.deepEqual(labels(`${task}        |`).slice(0, 4), ['do', 'call', 'spawn', 'task'], 'a step in a task is a step');
  assert.deepEqual(labels(`${task}        do: noop\n        needs: [|]\n      pack: { do: noop }`), ['compile', 'pack']);
  assert.deepEqual(labels(`${task}        do: log\n        with: { message: "\${ steps.|`), ['compile']);
  assert.ok(!labels('steps:\n  build:\n    task:\n      compile: { do: noop }\n    |').includes('do'), 'a task does one thing, which is its steps');
});

test('taking a field puts the caret where its value goes', () => {
  const step = 'steps:\n  build:\n    do: shell\n    |';
  assert.equal(taking(step, 'with'), 'steps:\n  build:\n    do: shell\n    with:\n      |');
  assert.equal(taking(step, 'on_fail'), 'steps:\n  build:\n    do: shell\n    on_fail:\n      - |');
  assert.equal(taking(step, 'needs'), 'steps:\n  build:\n    do: shell\n    needs: [|]');
  assert.equal(taking(step, 'when'), 'steps:\n  build:\n    do: shell\n    when: ${ | }');
  assert.equal(taking(step, 'retry'), 'steps:\n  build:\n    do: shell\n    retry: |');
  assert.equal(taking(step, 'timeout'), 'steps:\n  build:\n    do: shell\n    timeout: |');
  assert.equal(taking('blueprint: kv\n|', 'steps'), 'blueprint: kv\nsteps:\n  |');
  assert.equal(taking('steps:\n  build: { do: shell, | }', 'with'), 'steps:\n  build: { do: shell, with: | }', 'inside braces everything stays on the line');
});

test('what comes next is offered at once when it can be', () => {
  const found = offered('steps:\n  build:\n    |');
  const again = (label: string) => found.find((one) => one.label === label)?.again ?? false;
  assert.deepEqual(['do', 'call', 'with', 'needs', 'on_fail', 'if_fail_ask_agent'].map(again), [true, true, true, true, true, true]);
  assert.deepEqual(['timeout', 'measure', 'description', 'env'].map(again), [false, false, false, false]);
});

test('a hook on a failure is offered its fields and a decision; any other hook, no decision', () => {
  const failing = labels('steps:\n  build:\n    do: shell\n    on_fail:\n      - |');
  assert.deepEqual(failing.slice(0, 4), ['do', 'emit', 'with', 'when']);
  assert.deepEqual(failing.slice(4, 7), ['retry', 'continue', 'fail']);
  assert.ok(failing.includes('agent.consult'), 'a hook may be an action\'s bare name');

  const starting = labels('steps:\n  build:\n    do: shell\n    on_start:\n      - |');
  assert.ok(starting.includes('emit') && starting.includes('shell') && !starting.includes('retry'));
  assert.ok(labels('steps:\n  build:\n    do: shell\n    on_fail:\n      - do: |').includes('retry'));
  assert.ok(!labels('steps:\n  build:\n    do: shell\n    on_finish:\n      - do: |').includes('retry'));
});

test('a list that is waiting for its dash is offered its items with one', () => {
  assert.equal(taking('steps:\n  build:\n    do: shell\n    on_fail:\n      |', 'do'), 'steps:\n  build:\n    do: shell\n    on_fail:\n      - do: |');
  assert.equal(taking('steps:\n  a: { do: noop }\n  b:\n    do: noop\n    needs:\n      |', 'a'), 'steps:\n  a: { do: noop }\n  b:\n    do: noop\n    needs:\n      - a|');
});

test('the arguments of a hook are those of the action it names, and `emit` takes a topic', () => {
  assert.deepEqual(labels('steps:\n  build:\n    do: shell\n    on_fail:\n      - do: agent.consult\n        with: { | }'), ['prompt', 'options']);
  assert.deepEqual(labels('steps:\n  build:\n    do: shell\n    on_start:\n      - emit: { | }'), ['topic']);
});

test('a watch rule and a handler are offered their own fields, and one way of each kind', () => {
  assert.deepEqual(labels('steps:\n  build:\n    do: shell\n    watch:\n      - after: 5m\n        |'), ['do', 'emit', 'with', 'when', 'fail']);
  const handler = labels('on:\n  ready:\n    event: board.item.moved\n    |');
  assert.deepEqual(handler, ['for', 'when', 'scope', 'debounce', 'outside', 'turn', 'steps', 'do', 'call', 'with']);
  assert.deepEqual(labels('on:\n  ready:\n    event: board.item.moved\n    steps:\n      go:\n        |').slice(0, 3), ['do', 'call', 'spawn']);
});

test('a field with a few words to choose from is offered them, each saying what it means', () => {
  const scopes = offered('on:\n  ready:\n    every: 1m\n    scope: |');
  assert.deepEqual(scopes.map((one) => one.label), ['project', 'fleet', 'own', 'target']);
  assert.equal(scopes[2]?.note?.says, 'only events from this instance and its children');
  assert.deepEqual(labels('params:\n  port:\n    type: |').slice(0, 3), ['string', 'int', 'number']);
  assert.deepEqual(labels('steps:\n  a:\n    do: noop\n    allow_failure: |'), ['true', 'false']);
});

test('agents and devices are offered by name where one is asked for', () => {
  assert.deepEqual(labels('steps:\n  a:\n    do: shell\n    if_fail_ask_agent: |'), ['builder', 'reviewer']);
  assert.deepEqual(labels('steps:\n  a:\n    do: shell\n    if_fail_ask_agent: "@bu|'), ['builder', 'reviewer']);
  assert.deepEqual(labels('steps:\n  a:\n    do: shell\n    device: |'), ['local', 'builder-1']);
});

test('`permissions` is offered what the actions here need: each area, then each part of one', () => {
  const found = offered('blueprint: kv\npermissions: [|');
  assert.deepEqual(found.map((one) => one.label), ['agents', 'channels', 'shell', 'agents:write', 'channels:write']);
  assert.equal(found[2]?.note?.says, 'Everything under `shell`. Lets it use: shell.');
});

test('an expression starts with one of the names there are, or a function', () => {
  const found = offered(`${BUILD}  tell:\n    do: channel.post\n    with:\n      body: "built \${ |`);
  assert.deepEqual(found.slice(0, 4).map((one) => one.label), ['params', 'state', 'steps', 'self']);
  assert.equal(found[0]?.note?.says, 'The values this instance was started with, each by its name: `params.<name>`.');
  const slug = found.find((one) => one.label === 'slug')!;
  assert.equal(slug.insert, 'slug(');
  assert.equal(slug.note?.says, 'slug(text): lowercase words joined by dashes');
});

test('after `params.` and `state.`, what this blueprint declares', () => {
  const found = offered(`${BUILD}  tell:\n    do: channel.post\n    with:\n      body: \${ params.|`);
  assert.deepEqual(found.map((one) => `${one.label} = ${one.detail}`), ['port = 7379', 'profile = { type: string, default: release }']);
  assert.deepEqual(labels(`${BUILD}  tell:\n    do: set\n    with: { builds: \${ state.| } }`), ['builds']);
});

test('after `steps.`, the steps above this one; then what a step has; then what its action gives back', () => {
  const tell = `${BUILD}  tell:\n    do: channel.post\n    with:\n      body: `;
  assert.deepEqual(labels(`${tell}\${ steps.|\n  last: { do: noop }`), ['check', 'build'], 'not itself, and not what comes after it');
  assert.deepEqual(labels(`${tell}\${ steps.build.|`).slice(0, 5), ['ok', 'failed', 'skipped', 'status', 'out']);
  const out = offered(`${tell}\${ steps.build.out.|`);
  assert.deepEqual(out.map((one) => one.label), ['code', 'stdout']);
  assert.equal(out[0]?.note?.says, 'The exit code.');
  assert.deepEqual(labels(`${tell}"exit \${ steps.build.out.co|`), ['code', 'stdout'], 'the editor narrows it to what is typed');
});

test('in a hook, `self` is the step it belongs to', () => {
  const hook = 'steps:\n  build:\n    do: shell\n    on_finish:\n      - do: channel.post\n        with: { body: "${ self.';
  assert.deepEqual(labels(`${hook}|`).slice(0, 3), ['ok', 'failed', 'skipped']);
  assert.deepEqual(labels(`${hook}out.|`), ['code', 'stdout']);
});

test('what follows `event` and `instance` is what the daemon says is there', () => {
  assert.deepEqual(labels('on:\n  ready:\n    event: a.b\n    when: ${ event.|'), ['topic', 'data', 'source', 'ts', 'seq']);
  assert.deepEqual(labels('outputs:\n  where: ${ instance.|'), ['id', 'name', 'key', 'parent', 'project', 'device']);
  assert.deepEqual(labels('on:\n  ready:\n    event: a.b\n    when: event.data.to == "ready" && ev|').slice(0, 3), ['params', 'state', 'steps'], 'a condition without braces is one too');
});

test('outputs see the steps of the main graph', () => {
  assert.deepEqual(labels(`${BUILD}outputs:\n  code: \${ steps.|`), ['check', 'build']);
});

test('nothing is offered inside a comment, a block of text, a string in an expression or plain words', () => {
  assert.deepEqual(labels('steps: # the wo|'), []);
  assert.deepEqual(labels('steps:\n  a:\n    do: shell\n    with:\n      run: >\n        cargo bu|'), []);
  assert.deepEqual(labels('steps:\n  a:\n    when: ${ params.mode == "dr|'), []);
  assert.deepEqual(labels('steps:\n  a:\n    description: Builds the th|'), []);
  assert.deepEqual(labels('steps:\n  a:\n    timeout: |'), []);
});

test('an empty line is offered its fields when asked; a letter, a dash, a brace or a colon is asking', () => {
  const ready = (marked: string) => {
    const { text, offset } = caret(marked);
    return suggest(text, offset, language, world)?.ready;
  };
  assert.equal(ready('steps:\n  build:\n    |'), false);
  assert.equal(ready('steps:\n  build:\n    d|'), true);
  assert.equal(ready('steps:\n  build:\n    do: |'), true);
  assert.equal(ready('steps:\n  build:\n    do: shell\n    on_fail:\n      - |'), true);
  assert.equal(ready('steps:\n  build: { |'), true);
  assert.equal(ready('steps:\n  build:\n    when: ${ |'), true);
});

test('with no language to go by, nothing is offered', () => {
  assert.equal(suggest('steps:\n  a:\n    ', 17, null, world), null);
});

/** What is said about the word the `|` is on. */
function pointing(marked: string) {
  const { text, offset } = caret(marked);
  const found = explain(text, offset, language, world);
  return found && { on: text.slice(found.from, found.to), title: found.title, says: found.note.says };
}

test('pointing at a field says what it is for and what goes after it', () => {
  assert.deepEqual(pointing('steps:\n  build:\n    do: shell\n    time|out: 5m'), { on: 'timeout', title: 'timeout: a length of time, such as 30s or 5m', says: 'Fail the step if it runs longer, so that one that hangs does not hold the run up.' });
  assert.deepEqual(pointing('st|eps:\n  build: { do: shell }')?.title, 'steps: names, each a step');
  assert.equal(pointing('steps:\n  build: { do: shell, ne|eds: [] }')?.on, 'needs');
  assert.equal(pointing('steps:\n  bu|ild: { do: shell }'), null, 'a name of the writer\'s own says nothing');
});

test('pointing at an action, a blueprint or an argument says what it is', () => {
  assert.deepEqual(pointing('steps:\n  build:\n    do: sh|ell'), { on: 'shell', title: 'shell', says: 'Run a command and wait for it to finish.' });
  assert.equal(pointing('steps:\n  build:\n    on_fail:\n      - re|try\n    do: shell')?.says, 'Decides what happens to the step that failed.');
  assert.equal(pointing('steps:\n  host:\n    call: std/sl|ot')?.says, 'One version of a service.');
  assert.deepEqual(pointing('steps:\n  tell:\n    do: channel.post\n    with:\n      chan|nel: ops'), { on: 'channel', title: 'channel: string, needed', says: 'Which channel.' });
  assert.equal(pointing('on:\n  ready:\n    every: 1m\n    scope: o|wn\n    do: noop')?.says, 'Only events from this instance and its children.');
});

test('pointing at a word, an action, a name or a function shows the whole blueprint it is shown by, and at an argument none', () => {
  const { text, offset } = caret('steps:\n  build:\n    do: shell\n    ret|ry: 2');
  const found = explain(text, offset, language, world)!;
  assert.equal(found.entry, 'word/step/retry');
  assert.equal(found.note.example, exampleAt('word/step/retry', language));
  assert.ok(found.note.example?.startsWith('blueprint: ') && found.note.example.includes('retry:'), found.note.example);
  const told = language as ToldLanguage;
  assert.equal(exampleAt('word/step', language), told.shapes.find((shape) => shape.name === 'step')!.example);
  assert.equal(exampleAt('name/event', language), told.names.find((name) => name.name === 'event')!.example);
  assert.equal(exampleAt('function/slug', language), told.functions.find((one) => one.name === 'slug')!.example);
  for (const none of ['action/channel.post/channel', 'event/board.item.moved', 'word/step/retry/max', 'word/nothing', '', undefined]) {
    assert.equal(exampleAt(none, language), undefined, String(none));
  }
});

test('pointing inside an expression says what a name, a function or a member is', () => {
  const body = 'steps:\n  build:\n    do: shell\n  tell:\n    do: channel.post\n    with:\n      body: ';
  assert.equal(pointing(`${body}\${ par|ams.port }`)?.says, 'The values this instance was started with, each by its name: `params.<name>`.');
  assert.deepEqual(pointing(`${body}\${ steps.build.out.co|de }`), { on: 'code', title: 'steps.build.out.code', says: 'The exit code.' });
  assert.equal(pointing(`${body}\${ steps.build.fai|led }`)?.says, 'True when it failed.');
  assert.equal(pointing(`${body}\${ sl|ug(params.name) }`)?.says, 'slug(text): lowercase words joined by dashes');
  assert.equal(pointing(`${body}plain wo|rds`), null);
});

test('after `event:` the events a handler can hear are offered, each with what it carries', () => {
  const found = offered('blueprint: kv\non:\n  moved:\n    event: |');
  assert.deepEqual(found.map((one) => one.label), ['board.item.moved', 'board.item.removed'], 'one that is only passed on reaches no handler');
  const moved = found[0]!;
  assert.equal(moved.note?.says, 'A card was moved to another column.');
  assert.deepEqual(moved.note?.lists?.[0], {
    title: 'Carries',
    rows: [
      { name: 'id', type: 'string', says: 'the card' },
      { name: 'to', type: 'string', says: 'the column it is in now' },
      { name: 'note', type: 'string, sometimes', says: 'what was said as it was moved' },
    ],
  });
  assert.equal(taking('blueprint: kv\non:\n  moved:\n    event:|', 'board.item.moved'), 'blueprint: kv\non:\n  moved:\n    event: board.item.moved|');
});

test('under `for:` what the handler\'s event carries is offered, less what is there', () => {
  const handler = 'blueprint: kv\non:\n  moved:\n    event: board.item.moved\n';
  assert.deepEqual(labels(`${handler}    for: { | }`), ['id', 'to', 'note']);
  assert.deepEqual(labels(`${handler}    for: { id: c-1, | }`), ['to', 'note']);
  assert.deepEqual(labels(`${handler}    for:\n      |`), ['id', 'to', 'note']);
  assert.equal(taking(`${handler}    for: { | }`, 'to'), `${handler}    for: { to: | }`);
  // A handler on a timer, or on what blueprints say to each other, has nothing known to offer.
  assert.deepEqual(labels('blueprint: kv\non:\n  tick:\n    every: 5m\n    for: { | }'), []);
  assert.deepEqual(labels('blueprint: kv\non:\n  told:\n    event: shop.restocked\n    for: { | }'), []);
});

test('after `event.data.` what the event carries is offered, in the guard and in the steps', () => {
  const handler = 'blueprint: kv\non:\n  moved:\n    event: board.item.moved\n';
  assert.deepEqual(labels(`${handler}    when: \${ event.data.| }`), ['id', 'to', 'note']);
  assert.deepEqual(labels(`${handler}    steps:\n      say:\n        do: channel.post\n        with: { channel: general, body: "\${event.data.|}" }`), ['id', 'to', 'note']);
  const to = offered(`${handler}    when: \${ event.data.t| }`).find((one) => one.label === 'to')!;
  assert.equal(to.detail, 'string');
  assert.equal(to.note?.says, 'The column it is in now.');
  // What follows `event` itself is what an event has.
  assert.ok(labels(`${handler}    when: \${ event.| }`).includes('data'));
});

test('a handler on a group of topics is offered what any of them carries, and told which', () => {
  const handler = 'blueprint: kv\non:\n  any:\n    event: board.item.*\n';
  const found = offered(`${handler}    when: \${ event.data.| }`);
  assert.deepEqual(found.map((one) => one.label), ['id', 'to', 'note']);
  assert.equal(found.find((one) => one.label === 'id')?.note?.says, 'The card.');
  assert.equal(found.find((one) => one.label === 'to')?.note?.says, 'The column it is in now. Carried by board.item.moved.');
  assert.deepEqual(labels('blueprint: kv\non:\n  any:\n    event: "**"\n    for: { | }'), ['id', 'to', 'note']);
});

test('pointing at a topic, or at a field of its event, says what it is', () => {
  const handler = 'blueprint: kv\non:\n  moved:\n    event: board.item.moved\n    for: { id: c-1 }\n    when: ${ event.data.to == "done" }\n';
  const at = (word: string, within = 0) => explain(handler, handler.indexOf(word) + within, language, world);
  assert.equal(at('board.item.moved', 3)?.note.says, 'A card was moved to another column.');
  assert.equal(at('board.item.moved', 3)?.note.lists?.[0]?.title, 'Carries');
  assert.equal(at('id: c-1')?.title, 'id: string');
  assert.equal(at('id: c-1')?.note.says, 'The card.');
  assert.equal(at('to ==')?.title, 'event.data.to');
  assert.equal(at('to ==')?.note.says, 'The column it is in now.');
});
