import assert from 'node:assert/strict';
import { test } from 'node:test';
import { filling, goes, KINDS, kindCalled, labelled, mayShape, option, placing, refusedLabel, saying, shapes, whose } from './added.ts';
import type { Field, Row } from './table.ts';
import { heading } from './table.ts';

const field = (name: string, kind = 'text', more: Partial<Field> = {}): Field => ({ name, kind, of: [], required: false, ...more });
const row = (data: Record<string, unknown>): Row => ({ id: 'r', number: 1, data, position: 1, created: 0, updated: 0 });

test('a column is headed by what somebody called it, and by its name as it reads where nobody did', () => {
  assert.equal(heading(field('part_of')), 'Part of');
  assert.equal(heading(field('in-flight')), 'In flight');
  assert.equal(heading(field('points', 'number', { label_chosen: 'Story Points' })), 'Story Points');
  // A daemon says nothing, or null, where nobody called it anything.
  assert.equal(heading(field('points', 'number', { label_chosen: null })), 'Points');
});

test('what a column somebody adds may hold is the kinds with no authority in them, and each is called something', () => {
  assert.deepEqual(
    KINDS.map((one) => one.kind),
    ['text', 'number', 'choice', 'list', 'date', 'flag', 'record', 'records'],
  );
  assert.equal(kindCalled('choice'), 'Select');
  assert.equal(kindCalled('flag'), 'Checkbox');
  // One that is its table's to declare is still called something where it is shown.
  assert.equal(kindCalled('actor'), 'Person');
  assert.equal(kindCalled('sprocket'), 'sprocket');
});

test('adding, retyping and taking away take the scope of their own beside what saving a view takes', () => {
  const stock = { name: 'stock' };
  assert.equal(mayShape(['*'], stock), true);
  assert.equal(mayShape(['records', 'columns'], stock), true);
  // An agent holds the whole of records, and not this.
  assert.equal(mayShape(['projects', 'board', 'records'], stock), false);
  assert.equal(mayShape(['columns', 'records:read'], stock), false);
  // A board's take changing cards, and a table that asks more, that too.
  assert.equal(mayShape(['records', 'columns'], { name: 'board' }), false);
  assert.equal(mayShape(['board', 'columns'], { name: 'board' }), true);
  assert.equal(mayShape(['records', 'columns'], { name: 'agents', takes: 'agents:write' }), false);
  assert.equal(mayShape(['records', 'columns', 'agents'], { name: 'agents', takes: 'agents:write' }), true);
  // A library's records and mockups are people's files: no column is written into them.
  assert.equal(mayShape(['*'], { name: 'docs', store: 'documents' }), false);
  assert.equal(mayShape(['*'], { name: 'mockups', store: 'mockups' }), false);
  assert.equal(mayShape(['*'], { name: 'notes', store: 'files' }), true);
});

test('the lines of a menu that change the columns are there for whoever may, of a column that can be', () => {
  const of = (more: Partial<Parameters<typeof shapes>[0]>) => shapes({ may: true, shapes: true, added: true, kind: 'text', known: false, ...more });
  assert.deepEqual(of({}), ['type', 'insert-left', 'insert-right', 'duplicate', 'delete']);
  assert.deepEqual(of({ kind: 'choice' }), ['type', 'options', 'insert-left', 'insert-right', 'duplicate', 'delete']);
  // A declared column: nothing retypes it or takes it away, and it is copied and inserted beside.
  assert.deepEqual(of({ added: false }), ['insert-left', 'insert-right', 'duplicate']);
  assert.deepEqual(of({ added: false, kind: 'choice' }), ['options', 'insert-left', 'insert-right', 'duplicate']);
  // What a record is known by has nothing put before it; a kind nobody may add is not copied.
  assert.deepEqual(of({ added: false, known: true }), ['insert-right', 'duplicate']);
  assert.deepEqual(of({ added: false, kind: 'actor' }), ['insert-left', 'insert-right']);
  // Somebody who may save a view and does not hold the scope: the order of a choice, and nothing else.
  assert.deepEqual(of({ shapes: false }), []);
  assert.deepEqual(of({ shapes: false, kind: 'choice' }), ['options']);
  assert.deepEqual(of({ shapes: false, may: false, kind: 'choice' }), []);
});

test('a word a choice offers is written as the daemon keeps it, and a label is one line', () => {
  assert.equal(option('In a Hurry'), 'in-a-hurry');
  assert.equal(option('  fine!  '), 'fine');
  assert.equal(option('2026/Q4'), '2026-q4');
  assert.equal(option('—'), '');
  assert.equal(labelled('  Due date '), 'Due date');
  assert.equal(labelled(''), null);
  assert.equal(labelled('two\nlines'), null);
  assert.equal(labelled('x'.repeat(64)), 'x'.repeat(64));
  assert.equal(labelled('x'.repeat(65)), null);
  const fields = [field('title'), field('points', 'number', { label_chosen: 'Score' }), field('due_date', 'date')];
  assert.equal(refusedLabel(fields, 'Notes'), null);
  assert.equal(refusedLabel(fields, ' '), 'A column is called something.');
  assert.equal(refusedLabel(fields, 'score'), 'There is a column called Score already.');
  assert.equal(refusedLabel(fields, 'Due date'), 'There is a column called Due date already.');
  // Calling a column what it is called already is no clash with itself; its name alone is free where it is headed otherwise.
  assert.equal(refusedLabel(fields, 'Score', 'points'), null);
  assert.equal(refusedLabel(fields, 'Points'), null);
});

test('a column that is inserted stands beside the one it was asked beside, and one added after the last', () => {
  const table = { fields: [field('title'), field('stage', 'choice'), field('points', 'number'), field('notes')] };
  // As they stand with nothing said: what a record is known by, then by kind.
  const view = { name: 'rows', kind: 'table', columns: null };
  assert.deepEqual(placing(table, view, 'points', -1), { view: 'rows', kind: 'table', order: ['title', 'stage', 'points', 'notes'], before: 'points' });
  assert.deepEqual(placing(table, view, 'points', 1).before, 'notes');
  assert.deepEqual(placing(table, view, 'notes', 1).before, null);
  assert.deepEqual(placing(table, view, null, 1).before, null);
  assert.deepEqual(placing(table, view, 'gone', -1).before, null);
  // In the order the view says.
  const ordered = { name: 'rows', kind: 'table', columns: ['notes', 'points'] };
  assert.deepEqual(placing(table, ordered, 'notes', 1), { view: 'rows', kind: 'table', order: ['title', 'notes', 'points', 'stage'], before: 'points' });
  // In groups, every column is named band by band: the others stay where they are.
  const grouped = { fields: table.fields, groups: [{ name: 'How', fields: ['points', 'stage'] }] };
  assert.deepEqual(placing(grouped, view, 'points', 1), { view: 'rows', kind: 'table', order: ['title', 'notes', 'points', 'stage'], before: 'stage' });
});

test('what goes with a column is said before it is taken away', () => {
  const rows = [row({ notes: 'a' }), row({ notes: '' }), row({ notes: null }), row({}), row({ notes: [] }), row({ notes: 0 }), row({ notes: false })];
  assert.equal(saying(rows, 'notes'), 3);
  const notes = field('notes', 'text', { label_chosen: 'Remarks', added: true });
  assert.equal(goes(notes, rows), 'Remarks is taken away for everybody who opens this table. What 3 rows say in it goes with it. It cannot be put back.');
  assert.match(goes(notes, [row({ notes: 'a' })]), /What 1 row says in it goes with it\./);
  assert.match(goes(notes, []), /No row says anything in it\./);
});

test('what filling a column takes is said from what its table and the field say, with whether whoever looks may', () => {
  const plain = filling(field('notes'), { name: 'stock' }, ['records']);
  assert.deepEqual(plain, { needs: [{ words: 'records:write, to change a row of this table', held: true }], may: true, notes: [] });
  assert.equal(filling(field('notes'), { name: 'stock' }, ['records:read']).may, false);
  assert.equal(filling(field('title'), { name: 'board' }, ['records']).needs[0]!.words, 'board:write, to change a card of this table');
  // What the table asks, and what it is kept private under.
  const agents = { name: 'agents', takes: 'agents:write', reads: 'private:fleet', steers: 'keys' };
  const wall = field('wall', 'flag', { takes_of: { false: 'shell' }, required: true, quiet: true });
  const asked = filling(wall, agents, ['records', 'agents']);
  assert.deepEqual(
    asked.needs.map((one) => [one.words.split(',')[0], one.held]),
    [
      ['records:write', true],
      ['private:fleet', false],
      ['agents:write', true],
      ['shell', false],
    ],
  );
  assert.equal(asked.may, false);
  // What only one value takes does not stop the column being filled with another.
  assert.equal(filling(wall, { name: 'agents', takes: 'agents:write' }, ['records', 'agents']).may, true);
  assert.equal(filling(field('permission', 'choice', { takes: 'shell' }), { name: 'agents' }, ['records']).may, false);
  // Several scopes are each asked, of any value and of one.
  const withs = filling(field('with', 'list', { takes: ['shell', 'secrets:use'] }), { name: 'segments' }, ['records', 'shell']);
  assert.deepEqual(
    withs.needs.map((one) => [one.words, one.held]),
    [
      ['records:write, to change a row of this table', true],
      ['shell, which this column asks of any value', true],
      ['secrets:use, which this column asks of any value', false],
    ],
  );
  assert.equal(withs.may, false);
  assert.equal(filling(field('with', 'list', { takes: ['shell', 'secrets:use'] }), { name: 'segments' }, ['records', 'shell', 'secrets']).may, true);
  const loud = filling(field('loud', 'flag', { takes_of: { true: ['shell', 'secrets:use'] } }), { name: 'keys' }, ['records']);
  assert.deepEqual(loud.needs.slice(1).map((one) => one.words), ['shell, to give it the value true', 'secrets:use, to give it the value true']);
  assert.equal(loud.may, true);
  assert.deepEqual(asked.notes, ['Every row has to say something here.', 'A row here steers the rows of keys: it is written by whoever could write each of those.', 'A change of it alone is no line of a row’s history.']);
  const holds = filling(field('holds', 'list', { takes_each: true, unsaid: ['board', 'records'] }), { name: 'agents' }, ['*']);
  assert.deepEqual(holds.notes, ['Each scope written here takes holding it; a row that leaves it empty gives board, records.']);
  // A choice says the word nothing chosen stands for (a card's priority).
  const priority = filling(field('priority', 'choice', { of: ['urgent', 'high', 'normal', 'low'], unsaid: ['normal'] }), { name: 'board' }, ['*']);
  assert.deepEqual(priority.notes, ['A row that leaves it empty stands for normal.']);
});

test('whose a column is, and what blueprints call it, is said of every column', () => {
  const added = whose(field('story_points', 'number', { added: true, label_chosen: 'Story Points' }));
  assert.deepEqual(added, { words: 'A column somebody added to this table', more: 'Blueprints and files call it story_points.' });
  const declared = whose(field('status', 'choice'), 'std/board');
  assert.equal(declared.words, 'This column is read by std/board, which declares it');
  assert.equal(declared.more, 'What it holds, and whether it is there, are changed where the table is declared. Blueprints and files call it status.');
  assert.equal(whose(field('status', 'choice')).words, 'This column is read by what declares the table');
});
