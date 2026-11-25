import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import { CARD, keyOf, LEAST_COLUMN, marginOf, marksOf, onWords, ROOM, roomy, saidOn, threadOf } from './anchored.ts';
import type { Anchor } from './anchors.ts';
import type { Comment } from './journey.ts';

const step: Anchor = { quote: 'the second step', before: 'Then run ', after: ', which', under: ['Install'] };
const gone: Anchor = { quote: 'a word nobody wrote' };

function said(seq: number, words: string, on?: Anchor): Comment {
  const line = { seq, ts: seq, topic: 'record.commented', actor: 'user:ada', who: 'Ada', said: `Ada said: ${words}`, data: { body: words } };
  return { line, said: { who: 'Ada', words, asks: false, answers: false, public: false, ...(on ? { on } : {}) } };
}

const list = [said(1, 'All of it.'), said(2, 'Which step?', step), said(3, 'Gone now.', gone), said(4, 'The second one.', { ...step })];

test('what a comment was about is said with whether its words are still there', () => {
  assert.equal(saidOn(step, true), 'on “the second step”');
  assert.equal(saidOn(gone, false), 'on “a word nobody wrote”, which is no longer in the text');
  const long = { quote: `${'word '.repeat(30)}end` };
  assert.ok(saidOn(long, true).endsWith('…”'), saidOn(long, true));
});

test('the comments on a few words are those that say so, by where they are in the history', () => {
  assert.deepEqual(onWords(list).map(({ seq }) => seq), [2, 3, 4]);
});

test('a thread is the comments its mark stands for and any said since on the same words, wherever they now are', () => {
  assert.deepEqual(threadOf(list, { anchor: step, seqs: [2] }).map(({ line }) => line.seq), [2, 4]);
  assert.deepEqual(threadOf(list, { anchor: { ...step, under: ['Elsewhere'] }, seqs: [] }).map(({ line }) => line.seq), [2, 4]);
  assert.deepEqual(threadOf(list, { anchor: { quote: 'new words' }, seqs: [] }), []);
  assert.notEqual(keyOf(step), keyOf({ ...step, before: 'Now run ' }));
});

test('comments over one stretch of text are one mark, in the order of the text', () => {
  const marks = marksOf([
    { seq: 4, from: 30, to: 40 },
    { seq: 2, from: 10, to: 20 },
    { seq: 5, from: 10, to: 20 },
    { seq: 6, from: 10, to: 25 },
  ]);
  assert.deepEqual(marks, [
    { from: 10, to: 25, seqs: [6] },
    { from: 10, to: 20, seqs: [2, 5] },
    { from: 30, to: 40, seqs: [4] },
  ]);
});

test('a mark’s comments open beside the text only where the page has room for them to the right of its column', () => {
  // A wide page: a column ending at 900 of 1500.
  assert.deepEqual(marginOf(900, 1500), { left: 900 + CARD.gap, width: CARD.most });
  // Just room enough, and narrower than the most.
  const just = 900 + CARD.gap + CARD.least + CARD.edge;
  assert.deepEqual(marginOf(900, just), { left: 924, width: CARD.least });
  // Less, and at a phone's width: the drawer.
  assert.equal(marginOf(900, just - 1), null);
  assert.equal(marginOf(358, 374), null);
});

test('a page makes room for a card beside its column where the column is then no narrower than it may be', () => {
  // 1280 wide with the side panel open, and 1440 and 1600: the words are some 910, 1070 and 1230 across.
  for (const content of [910, 1070, 1230]) assert.ok(roomy(content), `${content}`);
  assert.ok(roomy(ROOM + LEAST_COLUMN));
  assert.ok(!roomy(ROOM + LEAST_COLUMN - 1));
  // A narrow tab, and a phone.
  assert.ok(!roomy(500));
  assert.ok(!roomy(330));
  // The room made is room for a card: what is left beside the column is the card at its width.
  const column = 1070 - ROOM;
  assert.deepEqual(marginOf(column, 1070), { left: column + CARD.gap, width: 280 });
});

test('the room the style sheet makes is the room worked out here', () => {
  const sheet = readFileSync(new URL('./comments.css', import.meta.url), 'utf8');
  assert.match(sheet, new RegExp(`--room: ${ROOM}px`));
});
