import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { type Flag, type Listed, cameSaid, mayChangeFlags, pastItsDay, pressed, saidIn, typedValue } from './flags.ts';

const checkout: Flag = {
  id: 'shop-flags-1',
  name: 'new-checkout',
  flag: 'new-checkout',
  variable: 'FLAG_NEW_CHECKOUT',
  about: '',
  owner: '',
  until: '2026-11-20',
  kind: 'switch',
  says: { dev: { value: 'on', shown: 'on' }, prod: { value: null, shown: 'off' } },
};
const part: Flag = { ...checkout, id: 'shop-flags-2', name: 'fast-path', flag: 'fast-path', variable: 'FLAG_FAST_PATH', kind: 'percentage', until: '', says: { dev: { value: 30, shown: '30%' }, prod: { value: 0, shown: '0%' } } };

test('pressing a switch turns it the other way, and anything else asks what to say', () => {
  assert.equal(pressed(checkout, 'dev'), 'off');
  assert.equal(pressed(checkout, 'prod'), 'on');
  assert.equal(pressed(checkout, 'staging'), 'on');
  assert.equal(pressed(part, 'dev'), null);
});

test('what is typed is read as its kind, and what cannot be sent says why before anything is asked', () => {
  assert.deepEqual(typedValue('switch', ' ON '), { value: 'on' });
  assert.deepEqual(typedValue('switch', 'maybe'), { wrong: 'A switch is on or off.' });
  assert.deepEqual(typedValue('percentage', '25%'), { value: 25 });
  assert.deepEqual(typedValue('percentage', '100'), { value: 100 });
  assert.ok('wrong' in typedValue('percentage', '101'));
  assert.ok('wrong' in typedValue('percentage', '2.5'));
  assert.deepEqual(typedValue('word', 'blue'), { value: 'blue' });
  assert.deepEqual(typedValue('word', ''), { value: '' });
  assert.ok('wrong' in typedValue('word', 'Two words'));
});

test('who may change a flag is whoever writes the project', () => {
  assert.equal(mayChangeFlags(['projects']), true);
  assert.equal(mayChangeFlags(['projects:write']), true);
  assert.equal(mayChangeFlags(['projects:read', 'records:read']), false);
});

test('an answer is said in a line, a change that waits saying so in the daemon\'s words', () => {
  assert.equal(cameSaid({ came: 'set', flag: 'new-checkout', environment: 'dev', shown: 'on' }, 'new-checkout'), 'new-checkout is on in dev.');
  assert.equal(cameSaid({ came: 'same', flag: 'new-checkout', environment: 'dev' }, 'new-checkout'), 'new-checkout says that in dev already.');
  assert.equal(
    cameSaid({ came: 'asks', flag: 'new-checkout', environment: 'prod', said: 'Setting new-checkout to on in prod waits for a go-ahead' }, 'new-checkout'),
    'Setting new-checkout to on in prod waits for a go-ahead.',
  );
});

test('a flag is past its day once the day is before today, and one with no day never is', () => {
  assert.equal(pastItsDay(checkout, '2026-11-24'), true);
  assert.equal(pastItsDay(checkout, '2026-11-20'), false);
  assert.equal(pastItsDay(part, '2026-11-24'), false);
});

test('an environment\'s drawer says each flag as it is there, off where it says nothing', () => {
  const listed: Listed = { flags: [checkout, part], environments: ['dev', 'prod'], left: [] };
  assert.deepEqual(saidIn(listed, 'prod'), [
    { name: 'new-checkout', variable: 'FLAG_NEW_CHECKOUT', shown: 'off', on: false },
    { name: 'fast-path', variable: 'FLAG_FAST_PATH', shown: '0%', on: false },
  ]);
  assert.deepEqual(saidIn(listed, 'dev').map((one) => one.on), [true, true]);
  assert.deepEqual(saidIn(listed, 'staging').map((one) => one.shown), ['off', 'off']);
});
