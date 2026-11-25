import assert from 'node:assert/strict';
import { test } from 'node:test';
import { CANNOT_OPEN } from './pagelinks.ts';
import { cut, plainOf, previewOf, repliesSaid, wantsWhole, whereOf } from './pagepreview.ts';
import type { Reply } from './replies.ts';
import type { TopicLine, TopicWhole } from './topicpage.ts';

const line = (id: string, title: string, parent: string | null, more: Partial<TopicLine> = {}): TopicLine => ({
  id,
  number: 0,
  title,
  status: null,
  summary: null,
  workspace: null,
  project: null,
  parent,
  position: 0,
  replies: 0,
  created: 0,
  updated: 0,
  ...more,
});
const whole = (of: TopicLine, body: string, summary = ''): TopicWhole => ({ ...of, body, summaryWhole: summary, answer: null, followers: [], createdBy: null });
const reply = (seq: number, who: string, body: string): Reply => ({ seq, parent: null, by: null, who, body, at: 0 });

const wiki = line('@-topics-1', 'Amphibians', null);
const frogs = line('@-topics-2', 'Frogs', wiki.id, { summary: 'Small **jumping** animals, see [Toads](topic:@-topics-4).', replies: 3 });
const tree = line('@-topics-3', 'Tree frogs', frogs.id);
const lines = [wiki, frogs, tree, line('@-topics-4', 'Toads', wiki.id)];
const everyone = () => true;

test('a page says its title, its wiki and the page above it, the first line of its summary as plain text, and its replies, from its line alone', () => {
  assert.deepEqual(previewOf(lines, frogs, null, null), {
    title: 'Frogs',
    wiki: 'Amphibians',
    under: null,
    says: 'Small jumping animals, see Toads.',
    replies: '3 replies',
    reading: false,
    reply: null,
  });
  assert.equal(wantsWhole(frogs, null), false);
  assert.deepEqual(whereOf(lines, tree.id), { wiki: 'Amphibians', under: 'Frogs' });
  assert.deepEqual(whereOf(lines, wiki.id), { wiki: null, under: null });
});

test('a page with no summary is read whole, and says the first lines of its article', () => {
  assert.equal(wantsWhole(tree, null), true);
  assert.equal(previewOf(lines, tree, null, null).reading, true);
  const read = previewOf(lines, tree, null, { topic: whole(tree, '# Tree frogs\n\n![A frog](x.png)\n\nThey climb.[^1]\n\n- green\n- small\n\n```mermaid\ngraph TD\n```\n\n[^1]: A book.'), replies: [reply(7, 'Ada', 'Yes.')] });
  assert.equal(read.says, 'They climb. green small');
  assert.equal(read.replies, '1 reply');
  assert.equal(read.reading, false);
});

test('a page read whole says the start of its summary over its article', () => {
  assert.equal(previewOf(lines, frogs, null, { topic: whole(frogs, 'The article.', 'The summary.\n\nIts second line.'), replies: [] }).says, 'The summary. Its second line.');
});

test('a link to a reply says who said it and its first lines, and that it is gone where the page has it no longer', () => {
  assert.equal(wantsWhole(frogs, 7), true);
  assert.equal(previewOf(lines, frogs, 7, null).reply, null);
  const read = { topic: whole(frogs, 'x'), replies: [reply(7, 'Ada', '> quoted\n\nI saw *three*.'), reply(8, '', 'Hm.')] };
  assert.deepEqual(previewOf(lines, frogs, 7, read).reply, { who: 'Ada', says: 'quoted I saw three.' });
  assert.deepEqual(previewOf(lines, frogs, 8, read).reply, { who: 'Somebody', says: 'Hm.' });
  assert.equal(previewOf(lines, frogs, 9, read).reply, 'gone');
});

test('nothing of a page the reader may not read is said: not above a page, nor as the words of a link to it', () => {
  // The wiki is not among what they may read.
  const hidden = [frogs, tree];
  assert.deepEqual(whereOf(hidden, tree.id), { wiki: 'Frogs', under: null });
  assert.deepEqual(whereOf(hidden, frogs.id), { wiki: null, under: null });
  const says = previewOf(hidden, frogs, null, { topic: whole(frogs, 'Read [the secret study](topic:@-topics-99) and [Tree frogs](topic:@-topics-3).'), replies: [] }).says;
  assert.equal(says, `Read ${CANNOT_OPEN} and Tree frogs.`);
});

test('what a page says is cut at a word to a few lines, and says nothing where nothing is written', () => {
  assert.equal(cut('one two three four', 12), 'one two…');
  assert.equal(cut('short', 12), 'short');
  assert.equal(plainOf('## Only a heading\n\n---', everyone), null);
  assert.equal(plainOf('', everyone), null);
  const long = plainOf('word '.repeat(200), everyone)!;
  assert.ok(long.length <= 241 && long.endsWith('…'));
  assert.equal(repliesSaid(0), 'No replies');
});
