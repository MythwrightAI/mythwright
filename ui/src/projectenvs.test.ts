import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { type Environment, applyClosed, asked, counted, handedAs, mayOf, nameRefused, rowsOf, secretSaid, underSaid, valueOf, written } from './projectenvs.ts';

const prod: Environment = {
  name: 'prod',
  vars: { api: { value: 'https://shop.example', agents: false }, region: { value: 'eu', agents: true }, replicas: { value: 3, agents: false } },
  secrets: {
    stripe: { set: 1, by_name: 'Ada', agents: 'STRIPE_KEY', kept: true },
    hook: { set: 2, by_name: 'Ada', agents: null, kept: false },
  },
  made: 0,
  by_name: 'Ada',
};
const stage: Environment = { name: 'stage', vars: {}, made: 0, by_name: 'Ada' };

test('the table says each environment with its variables, its secrets, what agents are handed and what runs under it', () => {
  assert.deepEqual(rowsOf([prod, stage], [{ environment: 'prod' }, { environment: 'prod' }, { environment: '' }, {}]), [
    { name: 'prod', vars: 3, secrets: 2, agents: 2, running: 2 },
    { name: 'stage', vars: 0, secrets: null, agents: 0, running: 0 },
  ]);
  assert.equal(counted(0, 'variable'), 'none');
  assert.equal(counted(1, 'variable'), '1 variable');
  assert.equal(counted(3, 'secret'), '3 secrets');
});

test('what may be done is what the daemon holds each change to', () => {
  assert.deepEqual(mayOf(['projects', 'shell', 'secrets:use']), { change: true, secrets: false });
  assert.deepEqual(mayOf(['projects:read']), { change: false, secrets: false });
  assert.deepEqual(mayOf(['projects', 'shell', 'secrets', 'environments:secrets']), { change: true, secrets: true });
  assert.deepEqual(mayOf(['*']), { change: true, secrets: true });
});

test('a secret says whether a value is kept, who set it and what an agent is handed it as, and never a value', () => {
  const ago = (at: number) => `${at}m ago`;
  assert.equal(secretSaid(prod.secrets!.stripe!, ago), 'set 1m ago by Ada');
  assert.equal(secretSaid(prod.secrets!.hook!, ago), 'Not set: no value is kept for it here');
  assert.equal(handedAs(prod.secrets!.stripe!), 'Handed to agents as STRIPE_KEY');
  assert.equal(handedAs(prod.secrets!.hook!), 'Not handed to agents');
  assert.equal(written(3), '3');
  assert.equal(written(true), 'true');
});

test('an apply asks an environment only where the project has some and nothing running already says one', () => {
  assert.equal(asked([], null), null);
  assert.deepEqual(asked(['prod', 'stage'], null), ['prod', 'stage']);
  assert.equal(asked(['prod'], 'prod'), null, 'applied over what runs under prod, it is under prod');
  assert.equal(asked(['prod'], ''), null, 'what runs under none goes on under none');
  assert.match(applyClosed(['prod'], '') ?? '', /Choose the environment it runs under/);
  assert.equal(applyClosed(['prod'], 'prod'), null);
  assert.equal(applyClosed(null, ''), null);
});

test('what runs is said under its environment, and under none where its project has some', () => {
  assert.equal(underSaid('prod', true), 'under prod');
  assert.equal(underSaid('', true), 'under no environment');
  assert.equal(underSaid(undefined, false), null);
});

test('a name is lowercase letters, digits and dashes', () => {
  assert.equal(nameRefused('staging'), null);
  assert.equal(nameRefused('prod-eu-2'), null);
  assert.match(nameRefused('') ?? '', /Give it a name/);
  assert.match(nameRefused('Staging Two') ?? '', /lowercase letters/);
});

test('a value typed is true or false, a number, or the text as it was typed', () => {
  assert.equal(valueOf('true'), true);
  assert.equal(valueOf('3'), 3);
  assert.equal(valueOf('2.5'), 2.5);
  assert.equal(valueOf('https://shop.example'), 'https://shop.example');
  assert.equal(valueOf(' eu '), ' eu ');
  assert.equal(valueOf(''), '');
});
