import assert from 'node:assert/strict';
import { test } from 'node:test';
import { InTurn } from './inturn.ts';

/** A view, as far as this goes: what it leaves out. */
type Hides = string[];

/** Something that saves when it is let go, as a daemon answers when it has. */
function slow(): { save: (next: Hides) => Promise<Hides>; answer: () => void; sent: Hides[] } {
  const waiting: (() => void)[] = [];
  const sent: Hides[] = [];
  return {
    sent,
    save: (next) =>
      new Promise((resolve) => {
        sent.push(next);
        waiting.push(() => resolve(next));
      }),
    answer: () => waiting.shift()?.(),
  };
}
const settled = () => new Promise((resolve) => setImmediate(resolve));

test('two changes asked for in the time one takes are each worked out from what the one before came to', async () => {
  const daemon = slow();
  const view = new InTurn<Hides>(['due']);
  // A column is left out, and another shown, before the first is saved.
  const first = view.change((now) => daemon.save([...now, 'taken']));
  const second = view.change((now) => daemon.save(now.filter((name) => name !== 'due')));
  await settled();
  // The second is not sent until the first is answered: it has nothing to be worked out from.
  assert.deepEqual(daemon.sent, [['due', 'taken']]);
  daemon.answer();
  assert.deepEqual(await first, ['due', 'taken']);
  await settled();
  // Worked out from the view as it was drawn, this was `[]`, and the first was undone.
  assert.deepEqual(daemon.sent[1], ['taken']);
  daemon.answer();
  assert.deepEqual(await second, ['taken']);
  assert.deepEqual(view.now, ['taken']);
});

test('a change that was refused leaves the thing as it was, and one that fails does not hold up the next', async () => {
  const view = new InTurn<Hides>(['due']);
  assert.equal(await view.change(async () => undefined), null);
  assert.deepEqual(view.now, ['due']);
  await assert.rejects(view.change(async () => Promise.reject(new Error('the daemon went away'))));
  assert.deepEqual(view.now, ['due']);
  assert.deepEqual(await view.change(async (now) => [...now, 'taken']), ['due', 'taken']);
});

test('a reading asked for before a change was answered and in after it is not taken', async () => {
  const daemon = slow();
  const view = new InTurn<Hides>(['due']);
  // The page reads the view again, because something was said of its table.
  const asked = view.asked();
  const change = view.change((now) => daemon.save([...now, 'taken']));
  await settled();
  daemon.answer();
  await change;
  // The reading comes in now, with the view as it was when it was taken.
  assert.equal(view.read(['due'], asked), false);
  assert.deepEqual(view.now, ['due', 'taken']);
  // Read again, it is taken; and so is one that says nothing of when it was asked.
  assert.equal(view.read(['due', 'taken', 'again'], view.asked()), true);
  assert.equal(view.read(['due']), true);
  assert.deepEqual(view.now, ['due']);
});

test('a reading that comes in while a change is still to be answered is taken, and the change is worked out from it', async () => {
  const daemon = slow();
  const view = new InTurn<Hides>(['due']);
  const first = view.change((now) => daemon.save([...now, 'taken']));
  const second = view.change((now) => daemon.save([...now, 'body']));
  await settled();
  assert.equal(view.read(['due', 'plan'], view.asked()), true);
  daemon.answer();
  await first;
  await settled();
  daemon.answer();
  assert.deepEqual(await second, ['due', 'taken', 'body']);
});
