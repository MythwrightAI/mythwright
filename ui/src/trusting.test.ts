import assert from 'node:assert/strict';
import { test } from 'node:test';
import { trusting } from './trusting.ts';

const shop = (trusted?: boolean) => ({ name: 'Shop', ...(trusted === undefined ? {} : { trusted }) });

test('trusting a project is offered to whoever manages the factory, and to nobody else', () => {
  const cases: [string[], boolean][] = [
    [['*'], true],
    [['admin'], true],
    [['projects', 'channels', 'board', 'agents', 'shell'], false],
    [['projects:write', 'blueprints'], false],
    [[], false],
  ];
  for (const [scopes, offered] of cases) {
    assert.equal(trusting(shop(false), scopes) !== null, offered, scopes.join(' '));
    assert.equal(trusting(shop(true), scopes) !== null, offered, scopes.join(' '));
  }
});

test('a project that is not trusted is offered trusting, and one that is, taking it back', () => {
  const trust = trusting(shop(false), ['admin'])!;
  assert.equal(trust.to, true);
  assert.equal(trust.line, 'Trust this project…');
  assert.equal(trust.asks, 'Trust Shop?');
  assert.equal(trust.button, 'Trust it');
  assert.match(trust.says, /used in place of the one that ships/);
  assert.match(trust.says, /Anyone who can write to its folder/);
  const back = trusting(shop(true), ['admin'])!;
  assert.equal(back.to, false);
  assert.equal(back.line, 'Stop trusting…');
  assert.equal(back.asks, 'Stop trusting Shop?');
  assert.equal(back.button, 'Stop trusting');
  assert.match(back.says, /blueprints that ship will be used again/);
});

test('nothing is offered where the daemon says nothing of whether a project is trusted', () => {
  assert.equal(trusting(shop(), ['*']), null);
});
