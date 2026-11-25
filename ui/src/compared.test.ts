import assert from 'node:assert/strict';
import { test } from 'node:test';
import { asked, byKind, counted, counting, does, figures, inForce, KINDS, mayRule, readsOf, rolledOf, ruleAsked, ruleWritten, saidOf, stands, thing, unmeasured, unruled } from './compared.ts';

test('a way says what it starts and hands, and one that says neither is the work as it is written', () => {
  assert.equal(does({}), 'as it is written');
  assert.equal(does({ with: { role: 'first' } }), 'role=first');
  assert.equal(does({ blueprint: 'my-round', with: { agent: 'shop-small-1', tries: 2 } }), 'starts my-round, agent=shop-small-1, tries=2');
  assert.equal(does({ blueprint: 'std/lane-round', version: '3' }), 'starts std/lane-round at version 3');
  assert.equal(does({ blueprint: 'std/board', sent: true }), 'starts std/board as the text that was sent');
  assert.equal(does({ prompts: { 'std/prompts/review-seat.md': { name: 'prompts/strict.md' } } }), 'reads prompts/strict.md for std/prompts/review-seat.md');
});

test('a comparison is going, or ended with the way everything goes from then on', () => {
  assert.equal(stands({ ended: null, goes: null }), 'going');
  assert.equal(stands({ ended: 5, goes: 'small' }), 'ended; everything goes small');
  assert.equal(stands({ ended: 5, goes: null }), 'ended; everything goes as it is written');
  assert.deepEqual([inForce({ ended: null, goes: null }), inForce({ ended: 5, goes: 'small' }), inForce({ ended: 5, goes: null })], [true, true, false]);
  assert.equal(thing({ per: 'item' }), 'each item');
  assert.equal(thing({}), 'each place a step starts it');
});

test('a column has two figures for a row: how many and how long, or what was added up in all and for each thing', () => {
  const cell = (count: number, errors: number, avg: number, sum: number, per_thing?: number) => ({ count, errors, avg, sum, ...(per_thing === undefined ? {} : { per_thing }) });
  // What came of the things: how many, and how long after a thing was given its way.
  assert.deepEqual(figures({ kind: 'came' }, cell(8, 0, 750_000, 6_000_000)), ['8', '12m 30s']);
  // What was chosen and what else was measured say how many failed, where any did.
  assert.deepEqual(figures({ kind: 'chose' }, cell(14, 0, 1900, 26_600)), ['14', '1.9s']);
  assert.deepEqual(figures({ kind: 'took' }, cell(40, 2, 1200, 48_000)), ['40 (2 failed)', '1.2s']);
  // What is not a time is added up, and said for each thing the column was given where that is known.
  assert.deepEqual(figures({ kind: 'spent', unit: 'usd' }, cell(14, 0, 0.294, 4.12, 0.3433)), ['$4.12', '$0.3433']);
  assert.deepEqual(figures({ kind: 'spent', unit: 'tokens' }, cell(14, 0, 86_000, 1_204_000, 100_333.3)), ['1,204,000', '100,333']);
  assert.deepEqual(figures({ kind: 'spent', unit: 'usd' }, cell(1, 0, 0.5, 0.5)), ['$0.50', '-']);
  assert.deepEqual(Object.keys(KINDS), ['came', 'chose', 'spent', 'accepted', 'per_accepted', 'took']);
  assert.ok(Object.values(KINDS).every((kind) => kind.figures.length === 2 && !/better|best|score/i.test(kind.head + kind.figures.join(' '))));
});

test('how many of a column were accepted is said over how many it was given, and what it spent over how many were accepted', () => {
  // Three of four given were accepted; a column given nothing has no share.
  assert.deepEqual(figures({ kind: 'accepted' }, { count: 3, given: 4, rate: 0.75 }), ['3', '75%']);
  assert.deepEqual(figures({ kind: 'accepted' }, { count: 0, given: 2, rate: 0 }), ['0', '0%']);
  assert.deepEqual(figures({ kind: 'accepted' }, { count: 1 }), ['1', '-']);
  // What was spent, the work that was not accepted in it, over how many were.
  assert.deepEqual(figures({ kind: 'per_accepted', unit: 'usd' }, { sum: 1.5, accepted: 3, per_accepted: 0.5 }), ['$1.50', '$0.50']);
  assert.deepEqual(figures({ kind: 'per_accepted', unit: 'tokens' }, { sum: 1200, accepted: 0 }), ['1,200', '-']);
});

test('the rows are headed by kind in the order the daemon laid them, each kind once', () => {
  const row = (kind: 'came' | 'chose' | 'spent' | 'took', what: string) => ({ kind, what, by: [] });
  const kinds = byKind([row('came', 'moved ready>doing'), row('came', 'moved review>done'), row('chose', 'agent.turn approve'), row('spent', 'agent.cost_usd'), row('took', 'run'), row('took', 'step')]);
  assert.deepEqual(
    kinds.map((kind) => [kind.kind, kind.rows.map((one) => one.what)]),
    [
      ['came', ['moved ready>doing', 'moved review>done']],
      ['chose', ['agent.turn approve']],
      ['spent', ['agent.cost_usd']],
      ['took', ['run', 'step']],
    ],
  );
  assert.deepEqual(byKind([]), []);
});

test('what is counted of each thing is written a line each, and what is wrong with a line is said', () => {
  assert.deepEqual(counting('made=shop.made:id:tone\n\n moved = board.item.moved : id : from+to \nended=shop.ended:id'), {
    counts: [
      { name: 'made', topic: 'shop.made', thing: 'id', by: ['tone'] },
      { name: 'moved', topic: 'board.item.moved', thing: 'id', by: ['from', 'to'] },
      { name: 'ended', topic: 'shop.ended', thing: 'id' },
    ],
  });
  assert.deepEqual(counting(''), { counts: [] });
  for (const line of ['made', 'made=shop.made', '=shop.made:id', 'made=shop.made:id:how:more']) assert.match((counting(line) as { wrong: string }).wrong, /name=event:field/, line);
  assert.deepEqual(counted({ counts: [{ name: 'verdict', topic: 'review.passed', thing: 'card' }, { name: 'verdict', topic: 'review.failed', thing: 'card' }, { name: 'asked', topic: 'card.asked', thing: 'card' }] }), ['verdict', 'asked']);
  assert.deepEqual(counted({}), []);
});

test('the ways somebody writes are told to the daemon, and what is wrong with them is said', () => {
  const written = (name: string, hands = '', share = '', blueprint = '') => ({ name, hands, share, blueprint });
  assert.deepEqual(asked([written('whole', 'agent=shop-big-1'), written('parts', 'agent=shop-small-1\ntries=2', '3', 'round-in-parts')]), {
    ways: [
      { name: 'whole', share: 1, with: { agent: 'shop-big-1' } },
      { name: 'parts', share: 3, blueprint: 'round-in-parts', with: { agent: 'shop-small-1', tries: 2 } },
    ],
  });
  assert.deepEqual(asked([written('theirs')]), { ways: [{ name: 'theirs', share: 1 }] });
  // A text played side by side is sent as it was played, in place of a blueprint by its name.
  assert.deepEqual(asked([{ ...written('fork', '', '', 'other'), sent: { called: 'The fork in shop', source: 'blueprint: x' } }]), { ways: [{ name: 'fork', share: 1, source: 'blueprint: x' }] });
  assert.deepEqual(asked([{ ...written('strict'), reads: 'std/prompts/review-seat.md = prompts/strict.md\n' }]), {
    ways: [{ name: 'strict', share: 1, prompts: { 'std/prompts/review-seat.md': 'prompts/strict.md' } }],
  });
  assert.match((asked([{ ...written('a'), reads: 'prompts/strict.md' }]) as { wrong: string }).wrong, /file=other file/);
  assert.match((asked([written('')]) as { wrong: string }).wrong, /Each way is called something/);
  assert.match((asked([written('a', 'role')]) as { wrong: string }).wrong, /param=value/);
  assert.match((asked([written('a', '', '0')]) as { wrong: string }).wrong, /whole number/);
});

test('a column says which text of its way it is of, and which one the way runs now where it ran several', () => {
  const column = (text: string, version: string | null, now: boolean) => ({ way: 'mine', text, version, now, given: 2 });
  assert.equal(saidOf(column('aaaa1111', '3', false)), 'version 3 · aaaa1111');
  assert.equal(saidOf(column('bbbb2222', null, false)), 'bbbb2222');
  assert.equal(saidOf(column('cccc3333', '2', true)), 'version 2 · cccc3333, now');
  assert.equal(saidOf(column('', null, false)), '', 'nothing where the text is not known');
  // And which words it was run with, where its way reads others in place of a file that is linked.
  const reads = [{ for: 'std/prompts/review-seat.md', name: 'prompts/strict.md', words: '5a6b7c8d' }];
  assert.deepEqual(readsOf({ ...column('aaaa1111', '3', false), reads }), ['reads prompts/strict.md · 5a6b7c8d']);
  assert.deepEqual(readsOf(column('aaaa1111', '3', false)), []);
});

test('its shares are written as parts of a hundred, the rule in the daemon\'s words with what it has read, and each change the newest first', () => {
  assert.equal(rolledOf(undefined), null);
  const shared = (at: number, by: string, why: string) => ({ at, by, was: { usual: 1, slate: 1 }, now: { usual: 1, slate: 1 }, why });
  const going = rolledOf({
    parts: [
      { way: 'usual', share: 9, percent: 90 },
      { way: 'slate', share: 1, percent: 10 },
    ],
    rule: { said: '`slate` is given 10 more', since: 0, done: false, over: 20, read: [{ way: 'slate', things: 3, came: 1 }, { way: 'usual', things: 27, came: 2 }] },
    shared: [shared(1, 'rule', 'first'), shared(2, 'user:ada', 'second')],
  });
  assert.deepEqual(going, {
    parts: 'usual 90%, slate 10%',
    rule: '`slate` is given 10 more.',
    read: 'Since the shares were last set: slate 1 of 3, usual 2 of 27, of 20 each it reads before it decides.',
    changes: [
      { at: 2, by: 'user:ada', why: 'second' },
      { at: 1, by: 'its rule', why: 'first' },
    ],
  });
  const done = rolledOf({ parts: [], rule: { said: 'x', since: 0, done: true, over: 1, read: [] }, shared: [] });
  assert.equal(done?.read, 'It has taken its way as far as it goes.');
  // Said again by somebody, with no rule: their changes and no rule.
  const byHand = rolledOf({ parts: [{ way: 'a', share: 3, percent: 100 }], rule: null, shared: [shared(5, 'user:ada', 'why')] });
  assert.equal(byHand?.rule, null);
  assert.equal(byHand?.read, null);
});

test('a rule is written from its parts, and what the page can tell is wrong with it is said before anything is asked', () => {
  const ways = ['usual', 'slate'];
  const begun = unruled(ways);
  assert.deepEqual(ruleAsked(begun, ways), { rule: { grows: 'slate', against: 'usual', count: 'verdict', came: 'failed', at_least: false, over: 20, step: 10, most: 50 } });
  // A least is said only where it is written: empty, the share stays where the rule does not hold.
  assert.deepEqual(ruleAsked({ ...begun, least: ' 10 ', atLeast: true, came: 'passed' }, ways), {
    rule: { grows: 'slate', against: 'usual', count: 'verdict', came: 'passed', at_least: true, over: 20, step: 10, most: 50, least: 10 },
  });
  const wrong = (written: Partial<ReturnType<typeof unruled>>) => {
    const said = ruleAsked({ ...begun, ...written }, ways);
    return 'wrong' in said ? said.wrong : '';
  };
  assert.match(wrong({ grows: 'otter' }), /among its ways/);
  assert.match(wrong({ against: 'slate' }), /cannot be held against itself/);
  assert.match(wrong({ count: ' ' }), /the count the rule goes by/);
  assert.match(wrong({ step: 'ten' }), /whole numbers/);
  assert.match(wrong({ over: '-1' }), /whole numbers/);
  assert.match(wrong({ least: '1.5' }), /The least it goes back to is a whole number/);
  // What the daemon keeps is written again as it was, to be changed.
  const kept = { grows: 'slate', against: 'usual', count: 'verdict', came: 'failed', at_least: true, over: 3, step: 25, most: 75, least: 25 };
  assert.deepEqual(ruleAsked(ruleWritten(kept), ways), { rule: kept });
  assert.equal(ruleWritten({ ...kept, least: null }).least, '');
});

test('a rule is offered to whoever the daemon would let give one, and only for a comparison that shares its things out and is going', () => {
  const going = { asked: false, ended: null, held: ['blueprints', 'board:read'] };
  assert.equal(mayRule(['*'], going), true);
  assert.equal(mayRule(['blueprints', 'board'], going), true);
  // Holding less than whoever started it, or not applying at all: words alone.
  assert.equal(mayRule(['blueprints:run'], going), false);
  assert.equal(mayRule(['board'], { ...going, held: [] }), false);
  assert.equal(mayRule(['*'], { ...going, asked: true }), false);
  assert.equal(mayRule(['*'], { ...going, ended: 5 }), false);
});

test('a column whose agents took turns and measured no cost says the cost was not reported, and never nought', () => {
  const cell = { count: 2, avg: 1000 };
  const turned = { kind: 'chose' as const, what: 'agent.turn done', by: [cell, cell] };
  const cost = { kind: 'spent' as const, what: 'agent.cost_usd', unit: 'usd', by: [{ sum: 0.5, per_thing: 0.25 }, null] };
  const rows = [turned, cost];
  assert.equal(unmeasured(rows, cost, 1), 'not reported');
  // A column nothing of an agent's was measured in, and what is not money, are a dash.
  assert.equal(unmeasured([{ ...turned, by: [cell, null] }, cost], cost, 1), '-');
  assert.equal(unmeasured(rows, { what: 'run', unit: undefined }, 1), '-');
});
