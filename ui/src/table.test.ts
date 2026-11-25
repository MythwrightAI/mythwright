import assert from 'node:assert/strict';
import { test } from 'node:test';
import { FLAG_COLUMN_MOST, RENAMING, asks, changed, choosable, columns, dayOf, extras, filled, flagColumn, goes, goesBy, heading, idsOf, inUse, keptIn, knownBy, layouts, leavingOut, leftOut, leftOutAtFirst, nameable, named, namedAll, namedBy, nameFor, naming, nextSort, nothingSaid, offeredBy, renames, sorted, sentBack, shownIn, titleField, waitsFor, weeksOf } from './table.ts';
import type { Change, Field, Named, Row } from './table.ts';

const field = (name: string, kind: string, of: string[] = []): Field => ({ name, kind, of, required: false });
// A board's fields, in the order a map gives them: by name.
const card = [field('assignee', 'actor'), field('body', 'text'), field('labels', 'list'), field('status', 'choice', ['backlog', 'doing', 'done']), field('title', 'text')];
const row = (number: number, data: Record<string, unknown>): Row => ({ id: `r-${number}`, number, data, position: number, created: 0, updated: 0 });

test('a table begins with what a record is known by, then what is glanced at, and long text last', () => {
  assert.deepEqual(
    columns(card).map((one) => one.name),
    ['title', 'status', 'assignee', 'labels', 'body'],
  );
});

test('a record with no title is known by its first text', () => {
  assert.equal(titleField([field('count', 'number'), field('name', 'text')])?.name, 'name');
});

test('a table can be laid out every way, offered in one order whatever its views are laid out as', () => {
  const offered = (views: { kind: string }[]) => layouts(card, views).map((one) => one.kind);
  assert.deepEqual(offered([]), ['table', 'board', 'list', 'gallery', 'calendar']);
  // The view being changed is one of these: laid out another way, it must find the choices where they were.
  assert.deepEqual(offered([{ kind: 'board' }, { kind: 'gallery' }]), offered([]));
  assert.deepEqual(offered([{ kind: 'list' }]), offered([{ kind: 'table' }]));
});

test('a table with no choice among its fields cannot be a board', () => {
  assert.deepEqual(
    layouts([field('title', 'text'), field('count', 'number')]).map((one) => one.kind),
    ['table', 'list', 'gallery', 'calendar'],
  );
});

test('a column is headed by its field, as a person would write it', () => {
  assert.equal(heading(field('due_date', 'text')), 'Due date');
});

test('rows are in the order they were added until a column is pressed', () => {
  const rows = [row(2, { title: 'b' }), row(1, { title: 'c' }), row(3, { title: 'a' })];
  assert.deepEqual(sorted(rows, card, null).map((one) => one.number), [1, 2, 3]);
  assert.deepEqual(sorted(rows, card, { field: 'title', down: false }).map((one) => one.number), [3, 2, 1]);
  assert.deepEqual(sorted(rows, card, { field: 'title', down: true }).map((one) => one.number), [1, 2, 3]);
});

test('a choice sorts in the order of what can be chosen, and the empty ones are last either way', () => {
  const rows = [row(1, { status: 'done' }), row(2, {}), row(3, { status: 'backlog' }), row(4, { status: 'doing' })];
  assert.deepEqual(sorted(rows, card, { field: 'status', down: false }).map((one) => one.number), [3, 4, 1, 2]);
  assert.deepEqual(sorted(rows, card, { field: 'status', down: true }).map((one) => one.number), [1, 4, 3, 2]);
});

test('pressing a heading sorts up, then down, then not at all', () => {
  const up = nextSort(null, 'title');
  assert.deepEqual(up, { field: 'title', down: false });
  assert.deepEqual(nextSort(up, 'title'), { field: 'title', down: true });
  assert.equal(nextSort({ field: 'title', down: true }, 'title'), null);
  assert.deepEqual(nextSort(up, 'status'), { field: 'status', down: false });
});

test('a new view is called what it is, with a number where that is taken', () => {
  assert.equal(nameFor('calendar', ['board']), 'Calendar');
  assert.equal(nameFor('calendar', ['board', 'calendar']), 'Calendar 2');
  assert.equal(nameFor('table', ['Table', 'Table 2']), 'Table 3');
});

test('a board goes by a choice, and a calendar by a day its records are given before the day they were made', () => {
  const dated = [...card, field('due', 'date')];
  assert.equal(goesBy('board', dated), 'status');
  assert.equal(goesBy('calendar', dated), 'due');
  assert.equal(goesBy('calendar', dated, 'updated'), 'updated');
  assert.equal(goesBy('calendar', card), 'created', 'a table with no day of its own');
  assert.equal(goesBy('table', dated), null);
});

test('a record is on the day its date says, and on none when it has none', () => {
  assert.equal(dayOf(row(1, { due: '2026-11-30' }), 'due'), '2026-11-30');
  assert.equal(dayOf(row(1, {}), 'due'), null);
  assert.equal(dayOf({ ...row(1, {}), created: new Date(2026, 10, 9, 15).getTime() }, 'created'), '2026-11-09');
});

test('a month is whole weeks from Monday, filled from the months either side', () => {
  const weeks = weeksOf(2026, 10);
  assert.equal(weeks.length, 6, 'November 2026 begins on a Sunday and ends on a Monday');
  assert.deepEqual(weeks[0], ['2026-10-26', '2026-10-27', '2026-10-28', '2026-10-29', '2026-10-30', '2026-10-31', '2026-11-01']);
  assert.equal(weeks.at(-1)![0], '2026-11-30');
  assert.equal(weeksOf(2027, 1).length, 4, 'February 2027 is four whole weeks');
});

test('a table says where it is kept when that is not the installation itself', () => {
  assert.equal(keptIn({ name: 'board', store: 'local' }), null);
  assert.deepEqual(keptIn({ name: 'board', store: 'files' }, 'card'), {
    words: 'kept as files in board/',
    more: "Each card is a file in the project's board folder",
  });
  assert.deepEqual(keptIn({ name: 'board', store: 'service', at: 'http://127.0.0.1:7801' }, 'card'), {
    words: 'kept in a database at 127.0.0.1:7801',
    more: 'The cards are kept by a store service: a database that a blueprint made and hosts, at http://127.0.0.1:7801',
  });
  // One whose address the daemon did not say is still said to be in a service.
  assert.equal(keptIn({ name: 'notes', store: 'service', at: null })?.words, 'kept in a database at an address that is not known');
});

// A card's sprint: sprints are known by their title, releases by the name each must have.
const sprint = field('sprint', 'record', ['sprints']);
const others: Named = {
  sprints: { fields: [field('title', 'text')], rows: [row(2, { title: 'December' }), row(1, { title: 'November' }), row(3, { title: ' ' })] },
  releases: { fields: [field('notes', 'text'), { ...field('name', 'text'), required: true }], rows: [row(1, { name: 'Spring', notes: 'Long.' })] },
};

test('a record that another names is known by its title, else by the first text it must have, else by its id', () => {
  assert.equal(knownBy(others.sprints!.rows[0]!, others.sprints!.fields), 'December');
  assert.equal(knownBy(others.releases!.rows[0]!, others.releases!.fields), 'Spring');
  assert.equal(knownBy(others.sprints!.rows[2]!, others.sprints!.fields), 'r-3', 'a title of nothing but spaces is no name');
  assert.equal(knownBy(row(4, { notes: 'Long.' }), [field('notes', 'text')]), 'r-4', 'text it need not have is not what it is known by');
});

test('a record whose table says which fields it is called by is known by what they hold, in their order, before its title', () => {
  // An attempt of a card's A/B test: its title is what the test finds it by, and nobody else's word for it.
  const attempts = [{ ...field('title', 'text'), required: true }, { ...field('way', 'text'), calls: 2 }, { ...field('card', 'text'), calls: 1 }];
  assert.equal(knownBy(row(1, { title: 'shop-contests-3-quick', card: 'shop-7', way: 'quick' }), attempts), 'shop-7 · quick');
  assert.equal(knownBy(row(2, { title: 'shop-contests-3-slow', card: 'shop-7' }), attempts), 'shop-7', 'what says nothing is left out');
  assert.equal(knownBy(row(3, { title: 'shop-contests-3-slow' }), attempts), 'shop-contests-3-slow', 'and where they all say nothing, its title');
  // A test, called by the card it is of, whatever names it.
  assert.equal(knownBy(row(4, { card: 'shop-7', contenders: ['quick', 'slow'] }), [{ ...field('card', 'record', ['board']), calls: 1 }]), 'shop-7');
});

test('a field that names a record is read as what that record is known by', () => {
  assert.deepEqual(named(sprint, 'r-1', others), { id: 'r-1', words: 'November', gone: false });
  assert.equal(named(sprint, null, others), null);
  assert.equal(named(sprint, '', others), null);
});

test('a field that names a record no longer there says so, and not before what it could name has been read', () => {
  assert.deepEqual(named(sprint, 'r-9', others), { id: 'r-9', words: 'r-9', gone: true });
  assert.deepEqual(named(sprint, 'r-9', {}), { id: 'r-9', words: 'r-9', gone: false });
});

test('a field that names a record is chosen from the records of the collection it names, in the order they were made', () => {
  assert.deepEqual(nameable(sprint, 'r-1', others), [
    { value: 'r-1', name: 'November' },
    { value: 'r-2', name: 'December' },
    { value: 'r-3', name: 'r-3' },
  ]);
  assert.deepEqual(nameable(field('release', 'record', ['releases']), null, others), [{ value: 'r-1', name: 'Spring' }]);
  assert.deepEqual(nameable(field('epic', 'record', ['epics']), null, others), [], 'a collection that has not been read offers nothing');
});

test('a record that is gone stays among what its field can be, so that leaving it alone does not change it', () => {
  assert.deepEqual(nameable(sprint, 'r-9', others)[0], { value: 'r-9', name: 'r-9, which is no longer there' });
  assert.equal(nameable(sprint, 'r-9', others).length, 4);
});

test('a record is not offered itself to name', () => {
  const after = field('after', 'record', ['sprints']);
  assert.deepEqual(
    nameable(after, null, others, 'r-2').map((one) => one.value),
    ['r-1', 'r-3'],
  );
});

test('what a record names comes with who has it, before its labels', () => {
  assert.deepEqual(
    columns([...card, sprint]).map((one) => one.name),
    ['title', 'status', 'assignee', 'sprint', 'labels', 'body'],
  );
});

test('what records name is read again when one of them is added, changed or taken away, and not for any other', () => {
  const names = ['sprints', 'releases'];
  for (const topic of ['record.created', 'record.updated', 'record.removed']) {
    assert.ok(RENAMING.includes(topic), `${topic} is heard`);
    assert.equal(renames({ topic, data: { project: 'shop', collection: 'sprints', id: 'shop-sprints-1' } }, 'shop', names), true, topic);
    assert.equal(renames({ topic, data: { project: 'shop', collection: 'notes' } }, 'shop', names), false, `${topic} of something no card names`);
    assert.equal(renames({ topic, data: { project: 'farm', collection: 'sprints' } }, 'shop', names), false, `${topic} in another project`);
  }
  assert.equal(renames({ topic: 'collection.ensured', data: { project: 'shop', name: 'sprints' } }, 'shop', names), true);
  assert.equal(renames({ topic: 'project.rescanned', data: { project: 'shop' } }, 'shop', names), true);
  assert.equal(renames({ topic: 'project.file.changed', data: { project: 'shop', path: 'sprints/2.md' } }, 'shop', names), true);
  assert.equal(renames({ topic: 'project.file.changed', data: { project: 'shop', path: 'src/main.rs' } }, 'shop', names), false);
  assert.equal(renames({ topic: 'record.created' }, 'shop', names), false, 'one that says nothing of itself');
});


test('a card\'s form has a place for what else its board keeps of it, and not for what is asked on it or what is kept on it for the blueprints that look after it', () => {
  const board = [...card, field('due', 'date'), sprint, field('points', 'number'), field('blocked', 'flag'), field('question', 'text'), field('waiting', 'text'), field('rounds', 'number'), field('asker', 'text'), field('taken', 'text'), field('again', 'list')];
  assert.deepEqual(
    extras(board).map((one) => one.name),
    ['sprint', 'points', 'blocked'],
    'in the order a table shows them, after a card\'s own, which the form already has',
  );
  assert.deepEqual(extras(card), [], 'a board that keeps nothing more of a card has nothing more in its form');
});

test('a form has no place for what its table says is kept on a record for whatever looks after it, whatever it is called', () => {
  const own = (name: string, kind: string): Field => ({ ...field(name, kind), quiet: true, hidden: true });
  const board = [...card, sprint, own('moved', 'list'), own('ledger', 'text'), { ...field('part_of', 'text'), quiet: true }, { ...field('archived', 'flag'), hidden: true }];
  assert.deepEqual(
    extras(board).map((one) => one.name),
    ['sprint', 'archived', 'part_of'],
    'what is only quiet, or only left out of a view at first, is still somebody\'s to fill in',
  );
  assert.deepEqual(
    filled(board).map((one) => one.name),
    [...card.map((one) => one.name), 'sprint', 'part_of', 'archived'],
    'and the same of any table\'s record read whole',
  );
});

test('a card that waits its turn says what for, and one that does not says nothing', () => {
  assert.equal(waitsFor({ waiting: 'one card is worked at a time here' }), 'one card is worked at a time here');
  assert.equal(waitsFor({ waiting: '' }), '');
  assert.equal(waitsFor({ question: 'Which total?' }), '', 'what is asked on it is another thing');
  assert.equal(waitsFor(undefined), '');
});

test('a card says how often it was sent back from review, and nothing of it when it never was', () => {
  assert.equal(sentBack({ rounds: 2 }), 2);
  assert.equal(sentBack({ rounds: 0 }), 0);
  assert.equal(sentBack({ rounds: null, question: 'Which total?' }), 0);
  assert.equal(sentBack(undefined), 0);
});

test('a card is waiting when something is asked on it, which it says itself', () => {
  assert.equal(asks({ question: 'Which total?' }), 'Which total?');
  assert.equal(asks({ question: '' }), '');
  assert.equal(asks({ question: null, sprint: 'shop-sprints-1' }), '');
  assert.equal(asks(undefined), '', 'a card whose board keeps nothing more of it');
});

test('a row is opened into the rows of the tables whose records name it, and into nothing where none does', () => {
  const tables = [
    { name: 'years', fields: [field('title', 'text')] },
    { name: 'quarters', fields: [field('title', 'text'), field('year', 'record', ['years'])] },
    { name: 'months', fields: [field('title', 'text'), field('quarter', 'record', ['quarters'])] },
    // One that names two, and one that names itself.
    { name: 'goals', fields: [field('title', 'text'), field('for', 'record', ['years']), field('reviewed', 'record', ['quarters'])] },
    { name: 'tasks', fields: [field('title', 'text'), field('part', 'record', ['tasks'])] },
  ];
  assert.deepEqual(namedBy(tables, 'years'), [
    { table: 'goals', field: 'for' },
    { table: 'quarters', field: 'year' },
  ]);
  assert.deepEqual(namedBy(tables, 'quarters'), [
    { table: 'goals', field: 'reviewed' },
    { table: 'months', field: 'quarter' },
  ]);
  assert.deepEqual(namedBy(tables, 'months'), []);
  // A table that names itself is not opened into itself: its rows are all there already.
  assert.deepEqual(namedBy(tables, 'tasks'), []);

  const quarter = (number: number, year: unknown): Row => ({ id: `q${number}`, number, data: { title: `Q${number}`, year }, position: number, created: 0, updated: 0 });
  const quarters = [quarter(3, 'life-years:2026'), quarter(1, 'life-years:2026'), quarter(2, 'life-years:2025'), quarter(4, null)];
  assert.deepEqual(naming(quarters, 'year', 'life-years:2026').map((one) => one.id), ['q1', 'q3']);
  assert.deepEqual(naming(quarters, 'year', 'life-years:2024'), []);
});

test('a field that names several records says each, is chosen from by names no two share, and opens rows the same', () => {
  const years: Row[] = [2025, 2026, 2027].map((year, at) => ({ id: `life-years:${year}`, number: at + 1, data: { title: at === 2 ? '2026' : String(year) }, position: at + 1, created: 0, updated: 0 }));
  const others: Named = { years: { fields: [field('title', 'text')], rows: years } };
  const spans = field('spans', 'records', ['years']);
  assert.deepEqual(idsOf(['life-years:2025', '', 'life-years:2026']), ['life-years:2025', 'life-years:2026']);
  assert.deepEqual(idsOf('life-years:2025'), ['life-years:2025']);
  assert.deepEqual(idsOf(null), []);
  // Each as one is said, with one that is gone said to be.
  assert.deepEqual(namedAll(spans, ['life-years:2025', 'life-years:1999'], others), [
    { id: 'life-years:2025', words: '2025', gone: false },
    { id: 'life-years:1999', words: 'life-years:1999', gone: true },
  ]);
  // Two that are called the same are told apart by their ids; one that is gone is kept to choose from.
  assert.deepEqual(choosable(spans, ['life-years:1999'], others), [
    { id: 'life-years:1999', name: 'life-years:1999, which is no longer there' },
    { id: 'life-years:2025', name: '2025' },
    { id: 'life-years:2026', name: '2026 (life-years:2026)' },
    { id: 'life-years:2027', name: '2026 (life-years:2027)' },
  ]);
  // A table that names several is opened into from each of them.
  const tables = [
    { name: 'years', fields: [field('title', 'text')] },
    { name: 'goals', fields: [field('title', 'text'), spans] },
  ];
  assert.deepEqual(namedBy(tables, 'years'), [{ table: 'goals', field: 'spans' }]);
  const goal = (number: number, of: unknown): Row => ({ id: `g${number}`, number, data: { spans: of }, position: number, created: 0, updated: 0 });
  const goals = [goal(1, ['life-years:2025', 'life-years:2026']), goal(2, ['life-years:2026']), goal(3, null), goal(4, 'life-years:2026')];
  assert.deepEqual(naming(goals, 'spans', 'life-years:2026').map((one) => one.id), ['g1', 'g2', 'g4']);
  assert.deepEqual(naming(goals, 'spans', 'life-years:2025').map((one) => one.id), ['g1']);
});

test('a view leaves out what it says, and one that says nothing what its table says to begin with', () => {
  const kept = [field('title', 'text'), field('status', 'choice'), { ...field('taken', 'text'), hidden: true }, { ...field('again', 'list'), hidden: true }, field('due', 'date')];
  const names = (view: Parameters<typeof shownIn>[0]) => shownIn(view, kept).map((one) => one.name);
  // The blueprint's own view, saying nothing: what the table's fields say is left out is.
  assert.deepEqual(leftOut({ name: 'table', kind: 'table', by: null, saved: false }, kept), ['taken', 'again']);
  assert.deepEqual(names({ name: 'table', kind: 'table', by: null }), ['title', 'status', 'due']);
  // One that says which leaves out those, and shows what its table would have left out.
  assert.deepEqual(names({ name: 'table', kind: 'table', by: null, saved: true, hides: ['due', 'taken'] }), ['title', 'status', 'again']);
  assert.deepEqual(names({ name: 'table', kind: 'table', by: null, hides: [] }), ['title', 'status', 'taken', 'again', 'due']);
});

test('a view somebody saved before a view could say what it leaves out leaves out what its table says to begin with, until somebody turns one on', () => {
  const kept = [field('title', 'text'), { ...field('taken', 'text'), hidden: true }];
  assert.deepEqual(leftOut({ name: 'Table', kind: 'table', by: null, saved: true }, kept), ['taken']);
  assert.deepEqual(leftOut({ name: 'Table', kind: 'table', by: null, saved: true, hides: null }, kept), ['taken']);
  // Turned on there, the view says what it leaves out, which is then nothing.
  assert.deepEqual(leftOut({ name: 'Table', kind: 'table', by: null, saved: true, hides: leavingOut({ name: 'Table', kind: 'table', by: null, saved: true }, kept, 'taken', true) }, kept), []);
});

test('what a record is known by is never left out, and a name that is no field leaves nothing out', () => {
  const kept = [{ ...field('title', 'text'), hidden: true }, field('status', 'choice')];
  assert.deepEqual(leftOutAtFirst(kept), []);
  assert.deepEqual(leftOut({ name: 'table', kind: 'table', by: null, hides: ['title', 'weather', 'status'] }, kept), ['status']);
});

test('a new view leaves out what its table says to begin with, whatever another view shows', () => {
  const kept = [field('title', 'text'), { ...field('taken', 'text'), hidden: true }, field('due', 'date')];
  assert.deepEqual(leftOutAtFirst(kept), ['taken']);
});

test('a column turned on or off is what the view then leaves out, said whole', () => {
  const kept = [field('title', 'text'), { ...field('taken', 'text'), hidden: true }, field('due', 'date')];
  const view = { name: 'table', kind: 'table', by: null };
  assert.deepEqual(leavingOut(view, kept, 'taken', true), []);
  assert.deepEqual(leavingOut(view, kept, 'due', false), ['taken', 'due']);
  assert.deepEqual(leavingOut({ ...view, hides: ['due'] }, kept, 'due', false), ['due']);
});

test('a change to a view is worked out from the view as it is when it is made, so two made one after the other both hold', () => {
  const kept = [field('title', 'text'), { ...field('taken', 'text'), hidden: true }, field('due', 'date'), field('status', 'choice')];
  const board = { name: 'board', kind: 'board', by: 'status' };
  const table = { name: 'Table', kind: 'table', by: null, saved: true, hides: ['due'] };
  const show =
    (name: string, shown: boolean): Change =>
    (view, fields) => ({ name: view.name, kind: view.kind, hides: leavingOut(view, fields, name, shown) });
  // Taken is left out; then Due is shown, in the view as that left it.
  const first = changed(kept, [board, table], 'Table', show('taken', false));
  assert.deepEqual(first.hides, ['due', 'taken']);
  const second = changed(kept, [board, { ...table, ...first }], 'Table', show('due', true));
  assert.deepEqual(second.hides, ['taken']);
  // Worked out from the view as the page had drawn it, the second took the first away.
  assert.deepEqual(changed(kept, [board, table], 'Table', show('due', true)).hides, []);
});

test('what a change does not say of a view is carried over, and the view looked at is the first where none is called that', () => {
  const kept = [field('title', 'text'), { ...field('taken', 'text'), hidden: true }, field('due', 'date'), field('status', 'choice')];
  const board = { name: 'board', kind: 'board', by: 'status', group: 'status' };
  const table = { name: 'Table', kind: 'table', by: null, saved: true, hides: ['due'] };
  // Called something else: what it leaves out goes with it.
  assert.deepEqual(changed(kept, [board, table], 'Table', (view) => ({ name: 'Cards', kind: view.kind, was: view.name })), { name: 'Cards', kind: 'table', was: 'Table', hides: ['due'], by: null, group: null, done: null, only: null, columns: null, freezes: null, figures: null, widths: null });
  // No view of that name, or none named: the first.
  for (const looking of ['Gone', null]) assert.equal(changed(kept, [board, table], looking, (view) => ({ name: view.name, kind: view.kind })).name, 'board');
  // A new one says what it leaves out itself, whatever the one looked at shows; and is told what there is, to be called something else.
  const added = changed(kept, [board, table], 'Table', (_, fields, views) => ({ name: nameFor('table', views.map((one) => one.name)), kind: 'table', hides: leftOutAtFirst(fields) }));
  assert.deepEqual([added.name, added.hides], ['Table 2', ['taken']]);
});

test('what a view says of its columns goes with it where the layout has a use for it: their order to a table, a list and a gallery, what it is frozen up to, its figures and its widths to a table alone', () => {
  const kept = [field('title', 'text'), field('points', 'number'), field('due', 'date'), field('status', 'choice')];
  const table = { name: 'Table', kind: 'table', by: null, saved: true, columns: ['due', 'gone', 'points'], freezes: 'due', figures: { points: 'sum', gone: 'filled' }, widths: { title: 320, gone: 90 }, group: 'status' };
  // Called something else it is still a table: all of it is carried, less what names a column that is gone.
  const renamed = changed(kept, [table], 'Table', (view) => ({ name: 'Rows', kind: view.kind, was: view.name }));
  assert.deepEqual([renamed.columns, renamed.freezes, renamed.figures, renamed.widths, renamed.group], [['due', 'points'], 'due', { points: 'sum' }, { title: 320 }, 'status']);
  // A list and a gallery say a record's fields side by side in that order, and have no column to freeze or foot to say a figure at.
  for (const kind of ['list', 'gallery']) {
    const laid = changed(kept, [table], 'Table', (view) => ({ name: view.name, kind }));
    assert.deepEqual([laid.columns, laid.freezes, laid.figures, laid.widths, laid.group], [['due', 'points'], null, null, null, null]);
  }
  // A board has no columns of fields at all, and is still gathered.
  const board = changed(kept, [table], 'Table', (view) => ({ name: view.name, kind: 'board' }));
  assert.deepEqual([board.columns, board.freezes, board.figures, board.widths, board.group], [null, null, null, null, 'status']);
  // A view frozen up to a column that is gone is frozen no more.
  assert.equal(goes('table', kept, { ...table, freezes: 'gone' }).freezes, null);
  // A view kept before any of this could be said says none of it.
  assert.deepEqual(goes('table', kept, { name: 'Old', kind: 'table', by: null }), { by: null, group: null, done: null, only: null, columns: null, freezes: null, figures: null, widths: null });
});

test('a text offers the words its table says, and nothing else offers any', () => {
  assert.deepEqual(offeredBy({ ...field('does', 'text'), suggests: ['plan', 'work', 'review'] }), ['plan', 'work', 'review']);
  assert.deepEqual(offeredBy(field('notes', 'text')), []);
  // What a choice can be is its own (`of`), and is all it can be.
  assert.deepEqual(offeredBy({ ...field('picks', 'choice'), suggests: ['first'] }), []);
});

test('choosing nothing of a choice is called by the word nothing stands for, where its table says one', () => {
  const priority: Field = { ...field('priority', 'choice', ['urgent', 'high', 'normal', 'low']), unsaid: ['normal'] };
  assert.equal(nothingSaid(priority), 'nothing (normal)');
  assert.equal(nothingSaid(field('status', 'choice', ['backlog', 'done'])), 'nothing');
  assert.equal(nothingSaid({ kind: 'list', unsaid: ['board'] }), 'nothing');
});

test("a form's flags stand in one column a little past the longest of their names, and none where there is no flag", () => {
  const flag = (name: string, label_chosen?: string) => ({ name, kind: 'flag', label_chosen });
  assert.equal(flagColumn([flag('holds'), flag('again'), { name: 'title', kind: 'text' }]), '6ch');
  assert.equal(flagColumn([flag('merge'), flag('in_flight')]), '10ch');
  // What somebody called a column is what is drawn, and so what is measured.
  assert.equal(flagColumn([flag('x', 'Waits for a person')]), '19ch');
  // A name longer than the column wraps in it.
  assert.equal(flagColumn([flag('x', 'a'.repeat(60))]), `${FLAG_COLUMN_MOST}ch`);
  assert.equal(flagColumn([{ name: 'title', kind: 'text' }]), null);
  assert.equal(flagColumn([]), null);
});

test("a card's labels offer those already on its board's cards, after any its table declared", () => {
  const rows = [row(1, { labels: ['urgent', 'bug'] }), row(2, { labels: [] }), row(3, { labels: ['Bug', 'before-preview', 'urgent'] }), row(4, { title: 'no labels at all' })];
  assert.deepEqual(inUse(card, 'labels', rows), ['before-preview', 'Bug', 'bug', 'urgent']);
  // What the table declared comes first, in its order, and is not offered twice.
  const declared = card.map((one) => (one.name === 'labels' ? field('labels', 'list', ['urgent', 'soon']) : one));
  assert.deepEqual(inUse(declared, 'labels', rows), ['urgent', 'soon', 'before-preview', 'Bug', 'bug']);
  // Blank words and what is not a list are not labels.
  assert.deepEqual(inUse(card, 'labels', [row(1, { labels: ['  ', ' shop '] }), row(2, { labels: 'shop' })]), ['shop']);
  assert.deepEqual(inUse(card, 'labels', []), []);
});
