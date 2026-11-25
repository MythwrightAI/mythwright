import assert from 'node:assert/strict';
import { test } from 'node:test';
import { Arriving, withArrived } from './arrived.ts';

interface Said {
  id: string;
}
const said = (...ids: string[]): Said[] => ids.map((id) => ({ id }));

/** A reading that comes in when it is let go. */
function held<T>(): { read: () => Promise<T>; comes: (got: T) => void } {
  let comes: (got: T) => void = () => {};
  const coming = new Promise<T>((resolve) => (comes = resolve));
  return { read: () => coming, comes };
}

test('what arrived while a reading was on its way comes after what was read', () => {
  assert.deepEqual(withArrived(said('a', 'b'), said('c')), said('a', 'b', 'c'));
});

test('what the reading already holds is not there twice, nor what arrived twice', () => {
  assert.deepEqual(withArrived(said('a', 'b'), said('b', 'c', 'c')), said('a', 'b', 'c'));
});

test('with nothing arrived, what was read is handed back as it is', () => {
  const read = said('a');
  assert.equal(withArrived(read, []), read);
  assert.equal(withArrived(read, said('a')), read);
});

test('what arrives for something while it is read is handed back with the reading', async () => {
  const arriving = new Arriving<Said>();
  const reading = held<Said[]>();
  const done = arriving.during('general', reading.read);
  arriving.arrive('general', { id: 'late' });
  arriving.arrive('elsewhere', { id: 'not-this' });
  reading.comes(said('early'));
  const [read, arrived] = await done;
  assert.deepEqual(withArrived(read, arrived), said('early', 'late'));
});

test('what arrives for something nobody is reading is not kept', async () => {
  const arriving = new Arriving<Said>();
  arriving.arrive('general', { id: 'before' });
  const [, arrived] = await arriving.during('general', async () => said());
  assert.deepEqual(arrived, []);
  arriving.arrive('general', { id: 'after' });
  const [, again] = await arriving.during('general', async () => said());
  assert.deepEqual(again, [], 'a reading that is over keeps nothing for the next');
});

test('of two readings side by side, the later to come in is handed what arrived during the first', async () => {
  const arriving = new Arriving<Said>();
  const [first, second] = [held<Said[]>(), held<Said[]>()];
  const one = arriving.during('general', first.read);
  const two = arriving.during('general', second.read);
  arriving.arrive('general', { id: 'm1' });
  // The second was taken before `m1` was said, and comes in last.
  first.comes(said('m1'));
  await one;
  arriving.arrive('general', { id: 'm2' });
  second.comes(said());
  const [read, arrived] = await two;
  assert.deepEqual(withArrived(read, arrived), said('m1', 'm2'));
});

test('a reading that fails keeps nothing for the next', async () => {
  const arriving = new Arriving<Said>();
  const failing = arriving.during('general', async () => {
    throw new Error('the daemon is away');
  });
  arriving.arrive('general', { id: 'lost' });
  await assert.rejects(failing);
  const [, arrived] = await arriving.during('general', async () => said());
  assert.deepEqual(arrived, []);
});
