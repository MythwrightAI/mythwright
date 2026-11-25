import assert from 'node:assert/strict';
import { test } from 'node:test';
import { type Done, doneWords } from './botdone.ts';

const record = (said: string, extra: Partial<Done> = {}): Done => ({ kind: 'record', ts: 0, said, via: 'std/lane-round', record: 'shop-17', ...extra });
const called = (id: string) => (id === 'c1' ? '#shop' : `#${id}`);

test('what the bot did to a record says the record where its history says it, with the bot left out', () => {
  assert.equal(doneWords(record('conductor moved it from ready to doing'), 'conductor', called), 'moved shop-17 from ready to doing');
  assert.equal(doneWords(record('conductor added it to draft from what shop-architect-1 wrote'), 'conductor', called), 'added shop-17 to draft from what shop-architect-1 wrote');
  assert.equal(doneWords(record('foreman moved it from doing to review, saying: it is done'), 'foreman', called), 'moved shop-17 from doing to review, saying: it is done', 'only the first it is the record');
  assert.equal(doneWords(record('conductor: created (collection notes)', { record: 'shop-notes-2' }), 'conductor', called), 'shop-notes-2: created (collection notes)', 'the usual words, after the record');
  assert.equal(doneWords(record('conductor planned the sprint'), 'conductor', called), 'planned the sprint (shop-17)', 'a sentence with no it says the record after');
  assert.equal(doneWords(record('somebody did it'), 'conductor', called), 'somebody did it (shop-17)', 'one that does not begin with the bot is said whole');
});

test('what the bot said says where and how it began', () => {
  const said: Done = { kind: 'said', ts: 0, said: 'Shop is up.\nIt has two agents.', via: 'std/fleet', channel: 'c1', message: 'm1' };
  assert.equal(doneWords(said, 'conductor', called), 'said in #shop: Shop is up.');
  assert.equal(doneWords({ ...said, said: '  ' }, 'conductor', called), 'said in #shop: something attached');
});
