import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { PassThrough } from 'node:stream';
import { test } from 'node:test';
import { saysWhere } from './where.ts';

/** Something that prints and ends as a daemon a check started does. */
function daemon() {
  const child = Object.assign(new EventEmitter(), { stdout: new PassThrough() });
  return { child, says: (text: string) => child.stdout.write(text) };
}

test('where a daemon answers is read from the line it says it is running in', async () => {
  const { child, says } = daemon();
  const where = saysWhere(child);
  says('Product is running at http://127.0.0.1:41873\n  data    /somewhere\n');
  assert.equal(await where, 'http://127.0.0.1:41873');
});

test('an address that comes in pieces is read whole, once its line has ended', async () => {
  const { child, says } = daemon();
  let read = '';
  const where = saysWhere(child).then((url) => (read = url));
  says('Product is runn');
  says('ing at http://127.0.0.1:4');
  // What has come so far reads as an address, and is not one yet.
  await new Promise((turn) => setImmediate(turn));
  assert.equal(read, '');
  says('1873\n');
  assert.equal(await where, 'http://127.0.0.1:41873');
});

test('what a daemon prints afterwards is read and dropped, so that it is never held up', async () => {
  const { child, says } = daemon();
  const where = saysWhere(child);
  says('Product is running at http://127.0.0.1:41873\n');
  await where;
  // Far more than a pipe holds: each piece is taken as it is written.
  const piece = 'x'.repeat(64 * 1024);
  for (let nth = 0; nth < 64; nth += 1) says(piece);
  await new Promise((turn) => setImmediate(turn));
  assert.equal(child.stdout.readableLength, 0);
  assert.equal(child.stdout.writableLength, 0);
});

test('a daemon that ends before it says where it is fails the check, with how it ended', async () => {
  const { child, says } = daemon();
  const where = saysWhere(child);
  says('error: cannot listen on 127.0.0.1:41873: Address already in use\n');
  child.emit('exit', 1, null);
  await assert.rejects(where, /the daemon ended \(1\) before it said where it is running/);
});

test('one that ends later, having said where it is, fails nothing', async () => {
  const { child, says } = daemon();
  const where = saysWhere(child);
  says('Product is running at http://127.0.0.1:41873\n');
  assert.equal(await where, 'http://127.0.0.1:41873');
  child.emit('exit', 0, null);
  assert.equal(child.listenerCount('exit'), 0);
});
