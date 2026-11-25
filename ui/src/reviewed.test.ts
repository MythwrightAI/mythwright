import assert from 'node:assert/strict';
import { test } from 'node:test';
import { doneSaid, failClosed, goesOf, headOf, lastSaid, noneListed, nothingSaid, offeredFor, partsOf, threadCalled, whereNow } from './reviewed.ts';
import type { Card, CardsPart } from './reviewed.ts';

const staging: CardsPart = { kind: 'cards', project: 'shop', column: 'staging', label: '', fail_to: 'ready', pass_to: 'done', reason: true };

function card(over: Partial<Card> = {}): Card {
  return { id: 'shop-1', title: 'Fix the cart', status: 'staging', assignee: null, labels: [], thread: [], may: true, ...over };
}

test('the parts a reading said are those of a kind the page draws', () => {
  const read = { parts: [{ kind: 'links', links: ['http://127.0.0.1:7760/'] }, { kind: 'pictures' }, staging, 'words'] };
  assert.deepEqual(
    partsOf(read).map((part) => part.kind),
    ['links', 'cards'],
  );
  assert.deepEqual(partsOf(null), []);
  assert.deepEqual(partsOf({}), []);
});

test('a part of cards says which cards and where those passed and failed go', () => {
  // The part, and what it is headed with.
  const cases: [CardsPart, { title: string; goes: string }][] = [
    [staging, { title: 'Cards in staging', goes: 'Passed are approved and go to done; failed go back to ready, with why.' }],
    [{ ...staging, pass_to: '', reason: false }, { title: 'Cards in staging', goes: 'Passed are approved and stay; failed go back to ready.' }],
    [{ ...staging, pass_to: '', pass_press: 'signed' }, { title: 'Cards in staging', goes: 'Passed are approved and have signed pressed; failed go back to ready, with why.' }],
    [{ ...staging, column: '', label: 'try', fail_to: 'needs-analysis' }, { title: 'Cards labelled try', goes: 'Passed are approved and go to done; failed go back to needs-analysis, with why.' }],
  ];
  for (const [part, head] of cases) assert.deepEqual(headOf(part), head);
  assert.equal(headOf({ ...staging, kept: true }).title, 'Cards that were in staging', 'a part that kept the cards it was told of');
  assert.equal(headOf({ ...staging, column: '', label: 'try', kept: false }).title, 'Cards labelled try');
});

test('what pressing does is said by what the part says', () => {
  // What the part says, and the sentence over it.
  const cases: [Parameters<typeof goesOf>[0], string][] = [
    [{ pass_to: 'done', fail_to: 'ready', reason: true }, 'Passed are approved and go to done; failed go back to ready, with why.'],
    [{ pass_to: 'shipped', pass_press: '', fail_to: 'needs-analysis', reason: false }, 'Passed are approved and go to shipped; failed go back to needs-analysis.'],
    [{ pass_to: '', pass_press: 'signed', fail_to: 'ready', reason: true }, 'Passed are approved and have signed pressed; failed go back to ready, with why.'],
    [{ pass_to: '', pass_press: '', fail_to: 'ready', reason: true }, 'Passed are approved and stay; failed go back to ready, with why.'],
    // A daemon from before says nothing of what is pressed.
    [{ pass_to: '', fail_to: 'doing', reason: false }, 'Passed are approved and stay; failed go back to doing.'],
  ];
  for (const [part, goes] of cases) assert.equal(goesOf(part), goes);
});

test('a card a part kept says where it is now where that is not where the part lists, and one gone says so', () => {
  const kept = { ...staging, kept: true };
  // The part, the card, and what is said where Pass and Fail would be.
  const cases: [CardsPart, Card, string | null][] = [
    [kept, card(), null],
    [kept, card({ status: 'done', may: false }), 'in done'],
    [kept, card({ status: 'ready', may: false }), 'in ready'],
    [kept, card({ gone: true, status: '', may: false }), 'no longer on the board'],
    [staging, card(), null],
    [{ ...staging, column: '', label: 'try' }, card({ status: 'review' }), null],
  ];
  for (const [part, one, said] of cases) assert.equal(whereNow(part, one), said, JSON.stringify(one));
  assert.equal(noneListed(kept), 'No cards were in staging when this was told.');
  assert.equal(noneListed(staging), 'No cards are there now.');
  assert.equal(nothingSaid(kept), 'Nothing was said on it since it came to staging.');
  assert.equal(nothingSaid({ ...staging, column: '', label: 'try' }), 'Nothing was said on it.');
});

test('a card says the last thing said on it and how many', () => {
  const said = (who: string, words: string) => ({ who, actor: `user:${who}`, words, ts: 1, topic: 'record.commented' });
  assert.equal(lastSaid(card()), null);
  assert.equal(threadCalled(card()), 'Discuss');
  const one = card({ thread: [said('Ada', 'The total is wrong.')] });
  assert.deepEqual(lastSaid(one), { who: 'Ada', words: 'The total is wrong.' });
  assert.equal(threadCalled(one), '1 comment');
  const two = card({ thread: [said('Ada', 'One.'), { ...said('', 'Which one?'), actor: 'agent:shop-worker-1' }] });
  assert.deepEqual(lastSaid(two), { who: 'agent:shop-worker-1', words: 'Which one?' }, 'by its actor where it has no name');
  assert.equal(threadCalled(two), '2 comments');
});

test('fail is closed until why is written, where the part asks for why', () => {
  // The part, what is written, and why Fail is closed.
  const cases: [CardsPart, string, string | null][] = [
    [staging, '', 'Say why it fails: that is what its agent is told when it next has it.'],
    [staging, '   ', 'Say why it fails: that is what its agent is told when it next has it.'],
    [staging, 'The total is wrong.', null],
    [{ ...staging, reason: false }, '', null],
  ];
  for (const [part, reason, closed] of cases) assert.equal(failClosed(part, reason), closed);
});

test('a card offers Pass and Fail only where the daemon says it may be, and says why not', () => {
  assert.deepEqual(offeredFor(card()), { press: true, why: null });
  const own = card({ may: false, why_not: 'scout worked on shop-1, and an agent does not pass or fail its own work' });
  assert.deepEqual(offeredFor(own), { press: false, why: own.why_not });
  assert.deepEqual(offeredFor(card({ may: false })), { press: false, why: null });
});

test('what a press came to is said in a sentence', () => {
  assert.equal(doneSaid('shop-1', { moved: 'ready', said: 'Too slow' }, 'fail'), 'shop-1 failed: moved to ready.');
  assert.equal(doneSaid('shop-2', { moved: 'done' }, 'pass'), 'shop-2 is approved, and moved to done.');
  assert.equal(doneSaid('shop-3', { pressed: 'signed' }, 'pass'), 'shop-3 is approved, and signed is pressed on it.');
  assert.equal(doneSaid('shop-4', {}, 'pass'), 'shop-4 is approved.', 'and no words are said on it for passing');
});
