import assert from 'node:assert/strict';
import { test } from 'node:test';
import { isAbout, MOST_BYTES, refusedBefore, shownAs, sizeSaid, tabCalled } from './recordfiles.ts';

test('what a browser draws or plays without running anything is opened in place, words are read, and the rest is saved', () => {
  const cases: [string, string][] = [
    ['image/png', 'picture'],
    ['image/jpeg', 'picture'],
    ['image/webp', 'picture'],
    ['image/svg+xml', 'saved'],
    ['video/mp4', 'video'],
    ['video/webm', 'video'],
    ['video/quicktime', 'saved'],
    ['text/plain', 'text'],
    ['text/plain; charset=utf-8', 'text'],
    ['text/csv', 'text'],
    ['text/markdown', 'text'],
    ['application/json', 'text'],
    ['text/html', 'saved'],
    ['application/pdf', 'saved'],
    ['application/octet-stream', 'saved'],
  ];
  for (const [media, shown] of cases) assert.equal(shownAs(media), shown, media);
});

test('a size is said in bytes, then kilobytes rounded up, then megabytes', () => {
  assert.equal(sizeSaid(1), '1 byte');
  assert.equal(sizeSaid(13), '13 bytes');
  assert.equal(sizeSaid(1024), '1 KB');
  assert.equal(sizeSaid(1025), '2 KB');
  assert.equal(sizeSaid(3 * 1024 * 1024), '3.0 MB');
});

test('the tab says how many are attached only once that is known and there are any', () => {
  assert.equal(tabCalled(null), 'Attachments');
  assert.equal(tabCalled(0), 'Attachments');
  assert.equal(tabCalled(3), 'Attachments (3)');
});

test('a file with nothing in it, or larger than the daemon takes, is refused before it is sent', () => {
  assert.equal(refusedBefore({ name: 'a.png', size: 0 }), 'There is nothing in a.png to attach.');
  assert.equal(refusedBefore({ name: 'a.png', size: MOST_BYTES }), null);
  assert.match(refusedBefore({ name: 'a.mp4', size: MOST_BYTES + 1 }) ?? '', /too large/);
});

test('what is added or taken away is followed for its own record and no other', () => {
  const event = (topic: string, id: string) => ({ topic, data: { id } });
  assert.equal(isAbout(event('record.attached', 'shop-1'), 'shop-1'), true);
  assert.equal(isAbout(event('record.detached', 'shop-1'), 'shop-1'), true);
  assert.equal(isAbout(event('record.attached', 'shop-2'), 'shop-1'), false);
  assert.equal(isAbout(event('record.commented', 'shop-1'), 'shop-1'), false);
});
