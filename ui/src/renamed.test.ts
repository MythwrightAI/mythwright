import assert from 'node:assert/strict';
import { test } from 'node:test';
import { MOST, mayRename, nameOf, renaming } from './renamed.ts';

const open = { id: 'ch_1', name: 'general', members: [{ actor: 'user:mira', manages: true }, { actor: 'user:otto' }] };
const kept = { id: 'ch_2', name: 'ch_2', kept: true, title: 'Pay review', members: [{ actor: 'user:mira', manages: true }, { actor: 'user:nell' }] };
const archived = { ...open, archived: { at: 1, by: 'user:mira', name: 'Mira' } };
const who = (actor: string, scopes: string[]) => ({ actor, scopes });

test('a channel is offered renaming to whoever manages it of those who may write in it, and an archived one to nobody', () => {
  assert.equal(mayRename(open, who('user:mira', ['channels'])), true);
  assert.equal(mayRename(open, who('user:mira', ['channels:read'])), false);
  assert.equal(mayRename(open, who('user:otto', ['channels'])), false, 'a member who manages nothing');
  assert.equal(mayRename(open, who('user:owen', ['channels', 'admin'])), true, 'whoever manages the factory');
  assert.equal(mayRename(kept, who('user:mira', ['channels:write'])), true);
  assert.equal(mayRename(kept, who('user:nell', ['channels:write'])), false);
  assert.equal(mayRename(kept, who('user:otto', ['channels:write', 'admin'])), false, 'not in it, so not reading it');
  assert.equal(mayRename(kept, who('user:ada', ['*'])), true);
  assert.equal(mayRename(archived, who('user:mira', ['*'])), false);
});

test('a name is made of what is typed as the daemon makes one', () => {
  assert.equal(nameOf('The Lobby'), 'the-lobby');
  assert.equal(nameOf('#general'), 'general');
  assert.equal(nameOf('  shop --  front!! '), 'shop-front');
  assert.equal(nameOf('Café 2'), 'caf-2');
  assert.equal(nameOf(' -- '), '');
});

test('an open channel is renamed by its name, said where it is not what was typed', () => {
  assert.deepEqual(renaming(open, 'lobby'), { called: 'lobby', changes: true, says: null });
  assert.deepEqual(renaming(open, '#lobby'), { called: 'lobby', changes: true, says: null });
  assert.deepEqual(renaming(open, 'The Lobby'), { called: 'the-lobby', changes: true, says: 'It will be called #the-lobby.' });
  assert.deepEqual(renaming(open, 'General'), { called: 'general', changes: false, says: 'It will be called #general.' });
  assert.deepEqual(renaming(open, 'general'), { called: 'general', changes: false, says: null });
  assert.deepEqual(renaming(open, ' ?! '), { called: null, changes: false, says: 'A name has at least one letter or digit.' });
});

test('a private channel is renamed by its title, which is as it is typed', () => {
  assert.deepEqual(renaming(kept, ' Salary review '), { called: 'Salary review', changes: true, says: null });
  assert.deepEqual(renaming(kept, 'Pay review'), { called: 'Pay review', changes: false, says: null });
  assert.deepEqual(renaming(kept, '   '), { called: null, changes: false, says: 'It is called something.' });
  assert.equal(renaming(kept, 'x'.repeat(MOST)).called, 'x'.repeat(MOST));
  assert.deepEqual(renaming(kept, 'x'.repeat(MOST + 1)), { called: null, changes: false, says: `It is called something in ${MOST} letters at most.` });
});
