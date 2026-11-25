import assert from 'node:assert/strict';
import { test } from 'node:test';
import { howLong, wentNow, where } from './logskept.ts';

test('unsaid, what steps printed is kept for ever; said, it is let go of after so many days and what runs came to is kept', () => {
  assert.equal(howLong({ days: null }), "What runs' steps printed is kept for ever.");
  assert.equal(howLong({ days: 1 }), "What runs' steps printed, and what was said of each as it went, is let go of after a day. What each run came to is kept for ever.");
  assert.ok(howLong({ days: 30 }).includes('after 30 days'));
});

test('where it is written says the data folder where nothing was said, and where what was written before is read', () => {
  assert.equal(where({ folder: null, usual: '/data/logs', before: [] }), 'It is written in the data folder, /data/logs.');
  assert.equal(where({ folder: '/srv/logs', usual: '/data/logs', before: ['/mnt/old'] }), 'It is written in /srv/logs. What was written before is read where it is: /mnt/old.');
});

test('what went at once is said only where anything did', () => {
  assert.equal(wentNow(null), null);
  assert.equal(wentNow({ before: 0, days: 1, events: 0, files: 0, bytes: 0 }), null);
  assert.equal(wentNow({ before: 0, days: 1, events: 3, files: 1, bytes: 56 }), 'Let go of now: one file and 3 events.');
});
