import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { HistoryLine } from './api.ts';
import { addedBy, asker, comments, decidedBy, drawn, fromWhat, happened, isQuestion, paragraphsOf, questionOf, recordOf, saidNow, speakerOf, spoken, viaOf, waitingOn, whoNow } from './journey.ts';

let seq = 0;
const line = (topic: string, who: string, data: Record<string, unknown> = {}, said = ''): HistoryLine => ({ seq: ++seq, ts: 0, topic, actor: null, who, said, data });
const asked = line('card.asked', 'a blueprint', { card: 'shop-1', asker: 'builder', question: 'Which total?' });
const answer = line('record.commented', 'Ada', { id: 'shop-1', body: 'Without tax.', by: 'user:ada' });
const answered = line('card.answered', 'a blueprint', { card: 'shop-1', asker: 'builder', body: 'Without tax.' }, 'that answered what builder asked');

test('what was asked on a card is the asker\'s, whichever blueprint said so', () => {
  assert.deepEqual(spoken(asked), { who: 'builder', words: 'Which total?', asks: true, answers: false, public: false });
  assert.deepEqual(spoken(answer), { who: 'Ada', words: 'Without tax.', asks: false, answers: false, public: false });
  assert.equal(spoken(line('board.item.moved', 'Ada', { from: 'ready', to: 'doing' })), null, 'a move is something that happened, not something said');
  assert.equal(spoken(answered), null, 'that something was the answer is said of it, and is not itself something said');
});

test('what was said is public only where whoever said it marked it so, and being the answer leaves the mark as it was', () => {
  const marked = line('record.commented', 'Ada', { id: 'shop-1', body: 'Without tax.', by: 'user:ada', public: true });
  assert.equal(spoken(marked)?.public, true);
  assert.equal(spoken(answer)?.public, false, 'said with nothing more, it is private');
  for (const other of [false, 'true', 1, null]) {
    assert.equal(spoken(line('record.commented', 'Ada', { body: 'Hm.', public: other }))?.public, false, `${String(other)} is no mark`);
  }
  assert.equal(spoken(asked)?.public, false, 'what an agent asked on a card is not something anybody marked');
  const read = drawn([asked, marked, answered]);
  assert.deepEqual([read.length, read[1]?.said?.answers, read[1]?.said?.public], [2, true, true]);
});

test('what was said that was the answer is drawn as the answer, and the line that says so is not drawn beside it', () => {
  const remark = line('record.commented', 'notes', { body: 'Still waiting.', through: 'notes' });
  const read = drawn([asked, remark, answer, answered]);
  assert.deepEqual(
    read.map((one) => [one.line.topic, one.said?.answers ?? null]),
    [
      ['card.asked', false],
      ['record.commented', false],
      ['record.commented', true],
    ],
  );
  assert.equal(drawn([asked, answer]).at(-1)?.said?.answers, false, 'until the card\'s blueprint says so, it is something said');
  // The same words said twice: the later is the answer, the earlier a remark.
  const early = line('record.commented', 'Grace', { body: 'Without tax.' });
  assert.deepEqual(
    drawn([early, asked, answer, answered]).map((one) => one.said?.answers ?? null),
    [false, false, true],
  );
  // One that says words nobody is known to have said stays, as the sentence it is.
  const stray = drawn([asked, answered]);
  assert.equal(stray.length, 2);
  assert.deepEqual([stray[1]?.said, stray[1]?.line.said], [null, 'that answered what builder asked']);
});

test('lines written before anything but a card could be spoken on are read as they were written', () => {
  const was = line('board.item.asked', 'std/lane-round', { asker: 'builder', question: 'Which total?', through: 'std/lane-round' });
  const then = line('board.item.commented', 'Ada', { body: 'Without tax.', answers: true, public: false });
  assert.deepEqual(
    drawn([was, then]).map((one) => one.said),
    [
      { who: 'builder', words: 'Which total?', asks: true, answers: false, public: false },
      { who: 'Ada', words: 'Without tax.', asks: false, answers: true, public: false },
    ],
  );
});

test('who asked what a card is waiting on is whoever asked last', () => {
  assert.equal(asker([]), null);
  assert.equal(asker([asked, answer]), 'builder');
  const again = line('card.asked', 'a blueprint', { asker: 'reviewer', question: 'And when?' });
  assert.equal(asker([asked, answer, answered, again]), 'reviewer');
});

test('the comments on a record are what was said on it, oldest first, and what happened to it is the rest', () => {
  const wrote = line('record.created', 'Ada', { id: 'shop-docs:welcome' }, 'Ada wrote it');
  const first = line('record.commented', 'Ada', { id: 'shop-docs:welcome', body: 'Does this cover Windows?' });
  const changed = line('record.updated', 'Ada', { id: 'shop-docs:welcome', changed: ['title'] }, 'Ada changed its title');
  const second = line('record.commented', 'Grace', { id: 'shop-docs:welcome', body: 'It does now.', public: true });
  const all = [wrote, first, changed, second];
  assert.deepEqual(comments(all).map((one) => [one.said.who, one.said.words, one.said.public]), [['Ada', 'Does this cover Windows?', false], ['Grace', 'It does now.', true]]);
  assert.deepEqual(comments(all).map((one) => one.line.seq), [first.seq, second.seq], 'each with the line it is, which tells it from every other');
  assert.deepEqual(happened(all), [wrote, changed]);
  // Between them they are the whole history: nothing is in both, and nothing in neither.
  assert.equal(comments(all).length + happened(all).length, all.length);
  assert.deepEqual(comments([wrote, changed]), [], 'a record nothing was said on has none');
  // On a card, what an agent asked is drawn as something said, and what somebody answered says that it answered.
  const onCard = comments([asked, answer, answered]);
  assert.deepEqual(onCard.map((one) => [one.said.asks, one.said.answers]), [[true, false], [false, true]]);
});

test("who added a record is what its history says: the factory's bot as it is called now, with whose work it was, the blueprint after, and never who applied it", () => {
  const planned = line('board.item.created', 'conductor', { id: 'shop-1', through: 'std/plan', wrote: 'agent:shop-architect-1', by: 'user:ada' });
  assert.deepEqual(addedBy([planned, answer], 'conductor'), { who: 'conductor from what shop-architect-1 wrote', via: 'std/plan' });
  assert.deepEqual(addedBy([planned], 'foreman'), { who: 'foreman from what shop-architect-1 wrote', via: 'std/plan' }, 'a bot called something else since is said by the name it has now');
  const row = line('record.created', 'conductor', { id: 'shop-agents-1', through: 'std/fleet' });
  assert.deepEqual(addedBy([row], 'conductor'), { who: 'conductor', via: 'std/fleet' });
  assert.deepEqual(addedBy([row], null), { who: 'conductor', via: 'std/fleet' }, 'before the bot is read, as the line was written');
  const byHand = line('board.item.created', 'Ada', { id: 'shop-2', by: 'user:ada' });
  assert.deepEqual(addedBy([byHand], 'conductor'), { who: 'Ada', via: null });
  const back = line('board.item.created', 'Grace', { id: 'shop-2', restored: true });
  assert.deepEqual(addedBy([back, byHand], 'conductor'), { who: 'Ada', via: null }, 'given back from the Trash, it was added before');
  assert.equal(addedBy([answer], 'conductor'), null, 'a history that says nothing of its adding says nobody');
  assert.equal(addedBy(null, 'conductor'), null, 'nor one still being read');
});

test("what a blueprint's step did is said as the factory's bot is called where it is read, with the blueprint as detail", () => {
  const moved = line('board.item.moved', 'conductor', { id: 'shop-1', through: 'std/lane-round', from: 'ready', to: 'doing' }, 'conductor moved it from ready to doing');
  assert.equal(viaOf(moved), 'std/lane-round');
  assert.equal(saidNow(moved, 'conductor'), 'conductor moved it from ready to doing');
  assert.equal(saidNow(moved, 'foreman'), 'foreman moved it from ready to doing', 'renamed, the bot is said by its new name');
  assert.equal(whoNow(moved, 'foreman'), 'foreman');
  assert.equal(saidNow(moved, null), 'conductor moved it from ready to doing', 'before the bot is read, as it was written');
  const byHand = line('board.item.moved', 'Ada', { id: 'shop-1', from: 'ready', to: 'doing', by: 'user:ada' }, 'Ada moved it from ready to doing');
  assert.deepEqual([viaOf(byHand), saidNow(byHand, 'foreman'), whoNow(byHand, 'foreman')], [null, 'Ada moved it from ready to doing', 'Ada'], "a person's is theirs");
  const empty = line('board.item.moved', 'conductor', { through: '' }, 'conductor moved it');
  assert.equal(viaOf(empty), null, 'an empty blueprint is nobody');
  const planned = line('board.item.created', 'conductor', { through: 'std/plan', wrote: 'agent:shop-architect-1' });
  assert.equal(fromWhat(planned), ' from what shop-architect-1 wrote');
  assert.equal(fromWhat(line('board.item.created', 'conductor', { through: 'std/plan', wrote: 'user:ada' })), '', 'only an agent is said to have written what a step adds from');
});

test("what a blueprint's step did as an agent decided is that agent's, with the blueprint as detail", () => {
  const data = { id: 'shop-1', through: 'std/chain-wait', decided: 'agent:shop-lead-1', from: 'review', to: 'ready' };
  const back = line('board.item.moved', 'shop-lead-1', data, 'shop-lead-1 moved it from review to ready, saying: Not here.');
  assert.deepEqual([decidedBy(back), viaOf(back), whoNow(back, 'foreman'), saidNow(back, 'foreman')], ['shop-lead-1', 'std/chain-wait', 'shop-lead-1', 'shop-lead-1 moved it from review to ready, saying: Not here.'], 'the bot called something else says nothing of it');
  assert.equal(decidedBy(line('board.item.moved', 'conductor', { through: 'std/chain-wait', decided: 'user:ada' })), null, 'only an agent decides');
});

test('whoever a line is of is who said it, who asked it, or did it: an agent by its name, the bot for a blueprint, and the bot for a record that asked', () => {
  const agents = new Set(['shop-worker-1']);
  const tables = [{ project: 'shop', name: 'contests' }, { project: 'shop', name: 'board' }];
  const asked = (asker: string, data: Record<string, unknown> = {}) =>
    ({ seq: 1, ts: 0, topic: 'card.asked', actor: 'user:u1', who: 'ada', said: '', data: { asker, question: 'which?', through: 'std/working', ...data } }) as HistoryLine;
  const of = (line: HistoryLine) => speakerOf(line, spoken(line), 'conductor', agents, tables);
  assert.deepEqual(of(asked('shop-worker-1')), { actor: 'agent:shop-worker-1', name: 'shop-worker-1' });
  assert.deepEqual(of(asked('shop-contests-3')), { actor: 'bot:conductor', name: 'conductor', record: { id: 'shop-contests-3', project: 'shop', table: 'contests' } });
  // A name nobody has now is drawn and opens nothing; a record of no table read is such a name.
  assert.deepEqual(of(asked('gone-agent')), { actor: null, name: 'gone-agent' });
  assert.deepEqual(of(asked('shop-notes-3')), { actor: null, name: 'shop-notes-3' });
  const comment = { seq: 2, ts: 0, topic: 'record.commented', actor: 'user:u1', who: 'ada', said: '', data: { body: 'hi' } } as HistoryLine;
  assert.deepEqual(of(comment), { actor: 'user:u1', name: 'ada' });
  assert.deepEqual(of({ ...comment, actor: 'key:k1' }), { actor: null, name: 'ada' });
  const moved = { seq: 3, ts: 0, topic: 'board.item.moved', actor: 'user:u1', who: 'ada', said: 'ada moved it', data: { through: 'std/lane-round' } } as HistoryLine;
  assert.deepEqual(of(moved), { actor: 'bot:conductor', name: 'conductor' });
  assert.deepEqual(of({ ...moved, data: { through: 'std/chain', decided: 'agent:shop-lead-1' } }), { actor: 'agent:shop-lead-1', name: 'shop-lead-1' });
  assert.deepEqual(of({ ...moved, data: {} }), { actor: 'user:u1', name: 'ada' });
  assert.deepEqual(recordOf('shop-12', tables), null);
  assert.deepEqual(recordOf('shop-board-12', tables), { id: 'shop-board-12', project: 'shop', table: 'board' });
});

test("a line's words are drawn by their paragraphs, each reviewer's words of a card sent back apart from the others", () => {
  const sent = 'conductor moved it from review to doing, saying: till-reviewer-1: `pay` refuses the card instead.\n\ntill-reviewer-astra-1: Same finding.';
  assert.deepEqual(paragraphsOf(sent), ['conductor moved it from review to doing, saying: till-reviewer-1: `pay` refuses the card instead.', 'till-reviewer-astra-1: Same finding.']);
  assert.deepEqual(paragraphsOf('one\ntwo'), ['one\ntwo'], 'a single break stays inside its paragraph');
  assert.deepEqual(paragraphsOf('one\n  \n\n\ntwo  \n'), ['one', 'two'], 'empty lines and lines of spaces are one break');
  assert.deepEqual(paragraphsOf('Ada moved it from draft to ready'), ['Ada moved it from draft to ready']);
  assert.deepEqual(paragraphsOf(''), ['']);
});

test('a question said on a card is drawn where what was said is, once, and what says it was asked where what happened is', () => {
  const said = line('record.commented', 'builder', { id: 'shop-1', body: '@ada @grace Which total?', through: 'std/told', decided: 'agent:builder', once: 'once asked 8' });
  const lines = [asked, said, answer, answered];
  assert.equal(isQuestion(said), true);
  assert.equal(isQuestion(answer), false, 'what a person says is no question a step asked');
  assert.equal(isQuestion(line('record.commented', 'conductor', { body: 'Note.', through: 'notes' })), false, 'nor is what a step says with no asking kept');
  assert.deepEqual(
    comments(lines).map((one) => [one.line.topic, one.said.who, one.said.asks, one.said.answers]),
    [
      ['record.commented', 'builder', true, false],
      ['record.commented', 'Ada', false, true],
    ],
  );
  assert.deepEqual(happened(lines).map((one) => one.topic), ['card.asked', 'card.answered']);
  assert.equal(asker(lines), 'builder');
  // Whom it was put to: those it names before its words, and none of what its words name.
  assert.deepEqual(questionOf(lines), { who: 'builder', whom: ['ada', 'grace'], asker: '', words: '@ada @grace Which total?' });
  const alone = line('record.commented', 'conductor', { body: 'Which entry, @ada?\n\nNobody is asked this.', through: 'std/told', once: 'once asked 9', asker: 'shop-contests-1' });
  assert.deepEqual(questionOf([...lines, alone])?.whom, []);
  assert.equal(questionOf([...lines, alone])?.asker, 'shop-contests-1');
  assert.equal(spoken(alone)?.who, 'shop-contests-1', 'a question said for a record that asks is that record\'s, which the bot speaks for');
  assert.equal(questionOf([asked, answer]), null, 'a card a program from before asked on says only that it was');
  // A card asked on before there was a question said on it keeps its asking where what was said is.
  assert.deepEqual(comments([asked]).map((one) => one.line.topic), ['card.asked']);
});

test('a card waiting on an answer says whom it waits on and that it is answered in Comments', () => {
  assert.equal(waitingOn(['adam']), 'Waiting on an answer from @adam: answer in Comments');
  assert.equal(waitingOn(['adam', 'grace']), 'Waiting on an answer from @adam and @grace: answer in Comments');
  assert.equal(waitingOn(['a', 'b', 'c']), 'Waiting on an answer from @a, @b and @c: answer in Comments');
  assert.equal(waitingOn([]), 'Waiting on an answer, asked of nobody: answer in Comments');
  assert.equal(waitingOn(null), 'Waiting on an answer: answer in Comments');
});
