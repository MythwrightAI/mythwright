import assert from 'node:assert/strict';
import { test } from 'node:test';
import { onceIn, opensWith } from './opened.ts';

test('a link’s secret is read from after its `#once=`, and nothing else is taken for one', () => {
  assert.equal(onceIn('#once=once_abc123'), 'once_abc123');
  assert.equal(onceIn(''), null);
  assert.equal(onceIn('#newest'), null);
  assert.equal(onceIn('#once='), null);
  assert.equal(onceIn('#once=once_abc"<script>'), null);
});

test('what opens the client signed in is the factory’s own command, by its profile where it has one', () => {
  assert.equal(opensWith('mw'), 'mw open');
  assert.equal(opensWith('mw', 'dev1'), 'mw --profile dev1 open');
});
