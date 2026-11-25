import assert from 'node:assert/strict';
import { test } from 'node:test';
import { changeSaid, followOf, followableOf, kindSaid, oftenSaid, placeOf, same, tabOf } from './following.ts';
import type { Follow } from './following.ts';
import { fromOf, opensItsPage } from './inbox.ts';
import type { Line } from './inbox.ts';
import type { Tab } from './tab.ts';

test('a page of a wiki, a board, a table, a library, a channel and a project can be followed from their tab, and nothing else', () => {
  const cases: [Tab, { kind: string; thing: string } | null][] = [
    [{ kind: 'topic', id: '@-topics-3' }, { kind: 'page', thing: '@-topics-3' }],
    [{ kind: 'board', project: 'shop' }, { kind: 'board', thing: 'shop/board' }],
    [{ kind: 'table', project: 'shop', name: 'stock' }, { kind: 'table', thing: 'shop/stock' }],
    [{ kind: 'library', project: 'shop', name: 'docs' }, { kind: 'library', thing: 'shop/docs' }],
    [{ kind: 'channel', id: 'ch_1' }, { kind: 'channel', thing: 'ch_1' }],
    [{ kind: 'project', id: 'shop' }, { kind: 'project', thing: 'shop' }],
    [{ kind: 'home' }, null],
    [{ kind: 'mockups', project: 'shop', name: 'mockups' }, null],
  ];
  for (const [tab, thing] of cases) {
    assert.deepEqual(followableOf(tab), thing, tab.kind);
    // And back: what is followed opens the tab it was followed from.
    if (thing) assert.deepEqual(tabOf(thing), tab, tab.kind);
  }
  assert.equal(tabOf({ kind: 'table', thing: 'shop' }), null, 'a table is named by its project and its name');
});

test('a table is one thing to the daemon whichever kind of table it is said to be', () => {
  assert.ok(same({ kind: 'table', thing: 'shop/board' }, { kind: 'board', thing: 'shop/board' }));
  assert.ok(same({ kind: 'library', thing: 'shop/docs' }, { kind: 'table', thing: 'shop/docs' }));
  assert.ok(!same({ kind: 'channel', thing: 'shop' }, { kind: 'project', thing: 'shop' }));
  const all: Follow[] = [{ kind: 'board', thing: 'shop/board', title: 'Shop board', pages: false, often: 'hourly', at: '09:00', made: 1 }];
  assert.equal(followOf(all, { kind: 'table', thing: 'shop/board' })?.often, 'hourly');
  assert.equal(followOf(all, { kind: 'project', thing: 'shop' }), null);
});

test('how a follow is said in Settings', () => {
  for (const [kind, pages, said] of [
    ['page', false, 'Page'],
    ['page', true, 'Wiki page and its pages'],
    ['board', false, 'Board'],
    ['channel', false, 'Channel'],
  ] as const) {
    assert.equal(kindSaid(kind, pages), said);
  }
  for (const [often, said] of [
    ['once', 'At once'],
    ['hourly', 'Hourly'],
    ['daily', 'Daily at 07:30'],
    ['off', 'Off'],
  ] as const) {
    assert.equal(oftenSaid(often, '07:30'), said);
  }
});

test('a change a line told of says who and what, and what that is called now where it is there', () => {
  const change = { seq: 1, record: 'shop-stock-1', label: 'Basket', noun: 'row', verb: 'added', from: 'user:acct_ada', at: 1 };
  assert.equal(changeSaid(change), 'acct_ada added a row: Basket');
  assert.equal(changeSaid({ ...change, noun: 'message', verb: 'said', label: '' }), 'acct_ada said a new message');
  assert.equal(changeSaid({ ...change, noun: 'reply', verb: 'said', label: 'Frogs' }), 'acct_ada said a new reply: Frogs');
});

test('a line of something followed is of the thing its place names, a digest of it too, and opens a page of its own', () => {
  assert.deepEqual(placeOf('table:shop/stock'), { kind: 'table', thing: 'shop/stock' });
  assert.deepEqual(placeOf('page:@-topics-3#1767229200000'), { kind: 'page', thing: '@-topics-3' });
  assert.equal(placeOf('ch_general'), null);
  const line: Line = {
    id: 'inb_1',
    owner: 'user:acct_nell',
    kind: 'following',
    about: 'table:shop/stock#1767229200000',
    place: 'table:shop/stock',
    from: 'user:acct_ada',
    why: 'digest',
    says: 'stock of Shop: 2 rows added',
    count: 1,
    created: 1,
    raised: 1,
    read: null,
    away: null,
  };
  assert.ok(opensItsPage(line));
  assert.equal(fromOf(line), 'Digest');
  assert.equal(fromOf({ ...line, why: 'follows' }), 'Following');
});
