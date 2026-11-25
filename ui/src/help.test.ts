import assert from 'node:assert/strict';
import { test } from 'node:test';
import { counted, isNews, letter, releasesOf, supportLink, whatToSay } from './help.ts';

test('support is an address on the web or one to write to, and nothing otherwise', () => {
  assert.equal(supportLink('https://example.invalid/help'), 'https://example.invalid/help');
  assert.equal(supportLink('  http://example.invalid/help?a=b  '), 'http://example.invalid/help?a=b');
  assert.equal(supportLink('mailto:help@example.invalid'), 'mailto:help@example.invalid');
  // Nothing written, and a daemon from before that says nothing: no line is drawn.
  for (const none of ['', '   ', null, undefined]) assert.equal(supportLink(none), null);
  // Never something a browser would run or read from the device, nor words that are no address.
  for (const not of ['javascript:alert(1)', 'file:///etc/passwd', 'data:text/html,x', 'example.invalid/help', 'https://', 'https://a b', 'ask in the channel']) assert.equal(supportLink(not), null, not);
});

test('what to say in a report is a line for the program, the system and the browser, each only where it is known', () => {
  assert.equal(
    whatToSay({ name: 'Factory', version: '0.1.0', build: 'b7', runsFrom: 'installed', os: 'Ubuntu', arch: 'x86_64', browser: 'Shell/1.0' }),
    'Factory 0.1.0 (client build b7, a copy on its own)\nSystem: Ubuntu x86_64\nBrowser: Shell/1.0',
  );
  assert.equal(whatToSay({ name: 'Factory', version: '0.1.0', runsFrom: 'checkout' }), 'Factory 0.1.0 (run from its checkout)');
  // Somebody who may not read the fleet is told no system, and says none.
  assert.equal(whatToSay({ name: 'Factory', version: '0.1.0', build: null, os: null, browser: 'Shell/1.0' }), 'Factory 0.1.0\nBrowser: Shell/1.0');
  assert.equal(whatToSay({}), '');
});

test('the releases are those a reading lists whole, in its order', () => {
  assert.deepEqual(releasesOf({ releases: [{ title: '0.2.0', at: 'release-1' }, { title: '0.1.0: preview', at: 'release-2' }], html: '<p>' }), [
    { title: '0.2.0', at: 'release-1' },
    { title: '0.1.0: preview', at: 'release-2' },
  ]);
  assert.deepEqual(releasesOf({ releases: [{ title: '', at: 'release-1' }, { title: 3 }, null, { title: '0.1.0', at: 'release-2' }] }), [{ title: '0.1.0', at: 'release-2' }]);
  for (const nothing of [null, undefined, {}, { releases: 'none' }, 'text']) assert.deepEqual(releasesOf(nothing), []);
});

test('something is new while the newest release is not the one this browser was shown', () => {
  const two = [
    { title: '0.2.0', at: 'release-1' },
    { title: '0.1.0', at: 'release-2' },
  ];
  assert.equal(isNews(two, null), true);
  assert.equal(isNews(two, '0.1.0'), true);
  assert.equal(isNews(two, '0.2.0'), false);
  // Nothing read is nothing new, whatever was kept.
  assert.equal(isNews([], null), false);
  assert.equal(isNews([], '0.1.0'), false);
});

test('a workspace with no picture is drawn by the first letter of its name', () => {
  assert.equal(letter('shop front'), 'S');
  assert.equal(letter('  annex'), 'A');
  assert.equal(letter('étage'), 'É');
  assert.equal(letter('42'), '4');
  // A letter of more than one unit is one letter.
  assert.equal(letter('😀 yard'), '😀');
  assert.equal(letter('   '), '');
});

test('how many of something is said in words', () => {
  assert.equal(counted(1, 'project', 'projects'), '1 project');
  assert.equal(counted(0, 'project', 'projects'), '0 projects');
  assert.equal(counted(3, 'project', 'projects'), '3 projects');
});
