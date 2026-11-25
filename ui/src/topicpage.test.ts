import assert from 'node:assert/strict';
import { test } from 'node:test';
import { firstLine, isTopicId, lineOf, mayAnswer, named, partsOf, statusChoices, statusOf, statusSaid, titleKey, titled, topicOfEvent, whereSaid, wholeOf } from './topicpage.ts';
import type { Status, TopicLine } from './topicpage.ts';

const handed = (id: string, data: Record<string, unknown>, more: Record<string, unknown> = {}) => ({ id, number: Number(id.split('-').pop()), data, created: 10, updated: 20, ...more });
const line = (id: string, title: string, more: Partial<TopicLine> = {}): TopicLine => ({ ...lineOf(handed(id, { title })), ...more });

test('a topic is listed by its title, its status, its place and how many replies it has, never its body', () => {
  const read = lineOf(handed('@-topics-3', { title: 'Why the build is slow', status: 'answered', body: '# Goal\nfast', summary: '## Now\nIt is the linker.', workspace: 'home', project: null }, { replies: 4 }));
  assert.deepEqual(read, { id: '@-topics-3', number: 3, title: 'Why the build is slow', status: 'answered', summary: 'Now', workspace: 'home', project: null, replies: 4, created: 10, updated: 20, parent: null, position: 3 });
  assert.ok(!('body' in read));
  // A page of a wiki says the page it is under, and stands where its position says.
  const page = lineOf(handed('@-topics-5', { title: 'Rolling back', parent: '@-topics-3' }, { position: 2.5 }));
  assert.equal(page.parent, '@-topics-3');
  assert.equal(page.position, 2.5);
  assert.equal(lineOf(handed('@-topics-6', { title: 'A wiki', parent: '' })).parent, null, 'a parent that says nothing is none');
});

test('a topic that says nothing of its status, or something the page does not know, has none', () => {
  assert.equal(statusOf(undefined), null);
  assert.equal(statusOf(null), null);
  assert.equal(statusOf(''), null);
  assert.equal(statusOf('pondering'), null);
  assert.equal(statusOf('open'), 'open');
  assert.equal(statusOf('standing'), 'standing');
  assert.equal(lineOf(handed('@-topics-4', { title: 'A page' })).status, null);
});

test('a reply is marked as the answer only in a topic tracked as a question', () => {
  assert.equal(mayAnswer(null), false);
  assert.equal(mayAnswer('open'), true);
  assert.equal(mayAnswer('answered'), true);
  assert.equal(mayAnswer('standing'), true);
});

test('the status is chosen among the four or none, the one it has marked, and answered only by marking a reply', () => {
  const choices = statusChoices('open');
  assert.deepEqual(
    choices.map((one) => [one.label, one.value, one.chosen]),
    [
      ['Open', 'open', true],
      ['Answered', 'answered', false],
      ['Closed', 'closed', false],
      ['Standing', 'standing', false],
      ['Not tracked', null, false],
    ],
  );
  assert.match(choices[1]!.closed ?? '', /Mark as the answer/);
  assert.equal(choices.filter((one) => one.closed).length, 1, 'only Answered waits on a reply');
  const none = statusChoices(null);
  assert.deepEqual(none.filter((one) => one.chosen).map((one) => one.label), ['Not tracked']);
  // One that is answered may be chosen again, which changes nothing.
  assert.equal(statusChoices('answered')[1]!.closed, undefined);
});

test('a topic whole has its body, its summary whole, what answered it and who follows it', () => {
  const whole = wholeOf(handed('@-topics-1', { title: 'T', body: 'the post', summary: 'known', answer: '17', followers: ['user:ada', 3] }, { created_by: 'user:ada' }));
  assert.equal(whole.body, 'the post');
  assert.equal(whole.summaryWhole, 'known');
  assert.equal(whole.answer, 17);
  assert.deepEqual(whole.followers, ['user:ada']);
  assert.equal(whole.createdBy, 'user:ada');
  assert.equal(wholeOf(handed('@-topics-1', { title: 'T', answer: null })).answer, null);
  assert.equal(wholeOf(handed('@-topics-1', { title: 'T' })).answer, null);
});

test('the first line of a document is what it says first, without the marks of a heading, an item or a quote', () => {
  assert.equal(firstLine('\n\n## What is known\nmore'), 'What is known');
  assert.equal(firstLine('- one item'), 'one item');
  assert.equal(firstLine('> quoted'), 'quoted');
  assert.equal(firstLine('1. first'), 'first');
  assert.equal(firstLine('   \n'), null);
});

test('a title is told from another by its words, whatever its capitals and spaces, as the daemon tells them', () => {
  assert.equal(titleKey('  Why  the Build\tis SLOW '), 'why the build is slow');
});

test('a topic is named by its id or by its title, and by nothing where two are called alike', () => {
  const lines = [line('@-topics-1', 'Payments'), line('@-topics-2', 'Builds'), line('@-topics-3', 'builds ')];
  assert.equal(isTopicId('@-topics-12'), true);
  assert.equal(isTopicId('Payments'), false);
  assert.equal(named(lines, '@-topics-2')?.title, 'Builds');
  assert.equal(named(lines, 'PAYMENTS')?.id, '@-topics-1');
  assert.equal(named(lines, 'builds'), null);
  assert.equal(named(lines, 'nothing'), null);
});

test('an open topic, or one with no status, says nothing of it beside its title, and any other says it', () => {
  assert.equal(statusSaid('open'), null);
  assert.equal(statusSaid(null), null);
  assert.equal(statusSaid('answered'), 'answered');
  assert.equal(statusSaid('standing'), 'standing');
});

test("a project's topic says its project's name beside it, and a workspace's own says nothing", () => {
  const projects = [{ id: 'shop', name: 'Shop' }];
  assert.equal(whereSaid({ project: 'shop' }, projects), 'Shop');
  assert.equal(whereSaid({ project: 'gone' }, projects), 'gone');
  assert.equal(whereSaid({ project: null }, projects), null);
});

test('what is said of a topic and of its table is followed, and what is said of any other record is not', () => {
  assert.deepEqual(topicOfEvent({ topic: 'topic.revised', data: { record: '@-topics-2', title: 'T' } }), { topic: '@-topics-2' });
  assert.deepEqual(topicOfEvent({ topic: 'record.created', data: { id: '@-topics-4', project: '@', collection: 'topics' } }), { topic: '@-topics-4' });
  assert.deepEqual(topicOfEvent({ topic: 'record.commented', data: { id: '@-topics-4', body: 'yes' } }), { topic: '@-topics-4' });
  assert.equal(topicOfEvent({ topic: 'record.updated', data: { id: 'shop-4', project: 'shop', collection: 'board' } }), null);
  assert.equal(topicOfEvent({ topic: 'record.created', data: { id: '@-categories-1', project: '@', collection: 'categories' } }), null);
  assert.equal(topicOfEvent({ topic: 'channel.message', data: {} }), null);
});

test('a title is its words with the spaces between folded, and is refused where it is nothing or too long, as the daemon refuses it', () => {
  assert.deepEqual(titled('  Payment   options '), { title: 'Payment options', refused: null });
  assert.equal(titled('   ').refused, 'A topic is called something: write its title.');
  assert.equal(titled('x'.repeat(201)).refused, "A topic's title is 200 characters at most.");
  assert.equal(titled('é'.repeat(200)).refused, null);
});

test('a topic whose post says something is an article, its talk on a tab beside it, and one that says nothing is a thread', () => {
  const plain = { body: '', summaryWhole: '', parent: null };
  const may = { mayChange: true, chosen: null, writing: false, summing: false };
  assert.deepEqual(partsOf(plain, may), { article: false, showing: null, summary: false, post: false, addSummary: true, writeArticle: true, writePost: false });
  assert.deepEqual(partsOf({ ...plain, body: '# Goal\nfast' }, may), { article: true, showing: 'article', summary: false, post: false, addSummary: true, writeArticle: false, writePost: false });
  assert.equal(partsOf({ ...plain, body: '  \n ' }, may).article, false, 'a post of nothing but spaces says nothing');
  // A wiki's page is an article, written or not: it opens on Article, ready to write.
  assert.equal(partsOf({ ...plain, parent: '@-topics-1' }, may).showing, 'article');
  // Talk where it was chosen or asked for; a thread has no tabs, whatever was chosen.
  assert.equal(partsOf({ ...plain, body: 'x' }, { ...may, chosen: 'talk' }).showing, 'talk');
  assert.equal(partsOf(plain, { ...may, chosen: 'talk' }).showing, null);
});

test('write an article and add summary draw their part before anything is written, and only for whoever may change it', () => {
  const plain = { body: '', summaryWhole: '', parent: null };
  const may = { mayChange: true, chosen: null, writing: true, summing: true };
  assert.deepEqual(partsOf(plain, may), { article: true, showing: 'article', summary: true, post: false, addSummary: false, writeArticle: false, writePost: false });
  const reads = { ...may, mayChange: false };
  assert.deepEqual(partsOf(plain, reads), { article: false, showing: null, summary: false, post: false, addSummary: false, writeArticle: false, writePost: false });
  assert.deepEqual(partsOf({ ...plain, summaryWhole: 'It is the linker.' }, reads), { article: false, showing: null, summary: true, post: false, addSummary: false, writeArticle: false, writePost: false });
});

test('a tracked question is a thread whatever its post says, its post drawn over its replies; standing or untracked, one that says something is an article', () => {
  const said = { body: 'A clean build takes ten minutes.', summaryWhole: '', parent: null };
  const may = { mayChange: true, chosen: null, writing: false, summing: false };
  const thread = { article: false, showing: null, summary: false, post: true, addSummary: true, writeArticle: false, writePost: false };
  const article = { article: true, showing: 'article', summary: false, post: false, addSummary: true, writeArticle: false, writePost: false };
  const cases: [Status | null, object][] = [
    ['open', thread],
    ['answered', thread],
    ['closed', thread],
    ['standing', article],
    [null, article],
  ];
  for (const [status, parts] of cases) assert.deepEqual(partsOf({ ...said, status }, may), parts, String(status));
  // Talk chosen, or asked for by an address, is no tab on a question.
  assert.equal(partsOf({ ...said, status: 'open' }, { ...may, chosen: 'talk' }).showing, null);
  // A question's page under a page is a thread too.
  assert.equal(partsOf({ ...said, parent: '@-topics-1', status: 'answered' }, may).article, false);
  // A question with no post offers one, which is drawn in the thread as it is written; never an article, and never to anybody else.
  const bare = { ...said, body: '', status: 'open' as const };
  assert.deepEqual(partsOf(bare, may), { ...thread, post: false, writePost: true });
  assert.deepEqual(partsOf(bare, { ...may, writing: true }), thread);
  assert.deepEqual(partsOf(bare, { ...may, mayChange: false, writing: true }), { ...thread, post: false, addSummary: false });
  // Not tracked any more, what was being written is an article.
  assert.equal(partsOf({ ...bare, status: null }, { ...may, writing: true }).showing, 'article');
});

test('History is a third tab of an article, and shows in a thread’s place where it is asked for', () => {
  const plain = { body: '', summaryWhole: '', parent: null };
  const how = { mayChange: true, chosen: 'history' as const, writing: false, summing: false };
  assert.equal(partsOf({ ...plain, body: 'x' }, how).showing, 'history');
  assert.equal(partsOf(plain, how).showing, 'history', 'a thread shows its history in its place');
  assert.equal(partsOf(plain, how).article, false, 'and is a thread still');
  assert.equal(partsOf(plain, { ...how, mayChange: false }).showing, 'history', 'whoever reads it may read its history');
  assert.equal(partsOf({ ...plain, status: 'open', body: 'x' }, how).showing, 'history', 'a question tracked too');
});
