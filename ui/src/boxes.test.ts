import assert from 'node:assert/strict';
import { test } from 'node:test';
import { boxedOf, boxOf, infoBoxOf, lists, MOST_NOTE_LINES, noteOf, seriesOf, STARTERS } from './boxes.ts';
import type { Series } from './boxes.ts';
import { read } from './prose.ts';

const FROG = ['title: Frog', 'picture: attachment:ratt_1a2b', 'caption: A common frog', 'Order: Anura', 'Species: 7,600+', 'Range: every continent but [Antarctica](topic:@-topics-7)'].join('\n');

test('an info box is its title, its picture and a fact a line, links kept in the words', () => {
  assert.deepEqual(infoBoxOf(FROG), {
    box: {
      is: 'infobox',
      title: 'Frog',
      picture: { to: 'attachment:ratt_1a2b', caption: 'A common frog' },
      rows: [
        { key: 'Order', value: 'Anura' },
        { key: 'Species', value: '7,600+' },
        { key: 'Range', value: 'every continent but [Antarctica](topic:@-topics-7)' },
      ],
    },
  });
});

test('an info box needs no picture, and a value may hold a colon and an address', () => {
  assert.deepEqual(infoBoxOf('title: Toad\n\nSite: https://example.org/toads\nTime: 10:30'), {
    box: {
      is: 'infobox',
      title: 'Toad',
      picture: null,
      rows: [
        { key: 'Site', value: 'https://example.org/toads' },
        { key: 'Time', value: '10:30' },
      ],
    },
  });
});

test('an info box that cannot be read says what is wrong, in one line', () => {
  assert.deepEqual(infoBoxOf('Order: Anura'), { wrong: 'An info box needs a line title: <what it is about>.' });
  assert.deepEqual(infoBoxOf('title: Frog\nOrder Anura'), { wrong: 'Line 2 is not a fact written as Key: value.' });
  assert.deepEqual(infoBoxOf('title: Frog\nOrder:'), { wrong: 'Line 2 says nothing after “Order:”.' });
  assert.deepEqual(infoBoxOf('title: Frog\ntitle: Toad'), { wrong: 'Line 2 gives a second title: an info box has one.' });
  assert.deepEqual(infoBoxOf('title: Frog\ncaption: Green'), { wrong: 'A caption is of a picture: add a line picture: attachment:<id>.' });
  assert.deepEqual(infoBoxOf('title: Frog\npicture: a frog'), { wrong: 'Line 2: a picture is one address, attachment:<id>.' });
  // A fact written with a capital is a fact, and not the box's own setting.
  assert.deepEqual(infoBoxOf('title: Dune\nTitle: Dune Messiah'), { box: { is: 'infobox', title: 'Dune', picture: null, rows: [{ key: 'Title', value: 'Dune Messiah' }] } });
});

test('a series is its title and its pages, grouped where it says, at the side unless placed at the end', () => {
  const text = ['title: Amphibians', '[Frogs](topic:@-topics-5)', '## Others', '- [Toads](topic:@-topics-6) (dry ones)', '[Newts](topic:@-topics-8#12)'].join('\n');
  assert.deepEqual(seriesOf(text), {
    box: {
      is: 'series',
      title: 'Amphibians',
      place: 'side',
      groups: [
        { name: null, pages: [{ line: '[Frogs](topic:@-topics-5)', topic: '@-topics-5' }] },
        {
          name: 'Others',
          pages: [
            { line: '[Toads](topic:@-topics-6) (dry ones)', topic: '@-topics-6' },
            { line: '[Newts](topic:@-topics-8#12)', topic: '@-topics-8' },
          ],
        },
      ],
    },
  });
  const end = seriesOf('place: end\ntitle: Amphibians\n[Frogs](topic:@-topics-5)');
  assert.equal('box' in end && end.box.is === 'series' && end.box.place, 'end');
});

test('a series that cannot be read says what is wrong, in one line', () => {
  assert.deepEqual(seriesOf('[Frogs](topic:@-topics-5)'), { wrong: 'A series needs a line title: <what its pages have in common>.' });
  assert.deepEqual(seriesOf('title: Amphibians\n'), { wrong: 'A series lists its pages, a link a line: none is written yet.' });
  assert.deepEqual(seriesOf('title: Amphibians\nFrogs'), { wrong: 'Line 2 is not a link to a page: write it [Its title](topic:<id>), typed as [[.' });
  assert.deepEqual(seriesOf('title: A\n[Site](https://example.org)'), { wrong: 'Line 2 is not a link to a page: write it [Its title](topic:<id>), typed as [[.' });
  assert.deepEqual(seriesOf('title: A\n![a picture](topic:@-topics-5)'), { wrong: 'Line 2 is not a link to a page: write it [Its title](topic:<id>), typed as [[.' });
  assert.deepEqual(seriesOf('title: A\nplace: top\n[F](topic:@-topics-5)'), { wrong: 'Line 2: a series is placed at the side or at the end (place: side, place: end).' });
});

test('a series says whether it lists the page it is drawn on', () => {
  const found = seriesOf('title: A\n[F](topic:@-topics-5)');
  const series = 'box' in found ? (found.box as Series) : null;
  assert.ok(series);
  assert.equal(lists(series, '@-topics-5'), true);
  assert.equal(lists(series, '@-topics-6'), false);
  assert.equal(lists(series, null), false);
});

test('a note is a line or two, and one longer says so', () => {
  assert.deepEqual(noteOf('\nThis page is about frogs. For stories, see [Frogs in culture](topic:@-topics-9).\n'), {
    box: { is: 'note', lines: ['This page is about frogs. For stories, see [Frogs in culture](topic:@-topics-9).'] },
  });
  assert.deepEqual(noteOf('  \n'), { wrong: 'A note says where else to look: it says nothing yet.' });
  assert.deepEqual(noteOf('a\nb\nc\nd'), { wrong: `A note at the top is a line or two; this one has ${MOST_NOTE_LINES + 1}.` });
});

test("an article draws its first note at its top, its boxes at its side and a series placed at the end after it, and leaves them out of its text", () => {
  const body = [
    '```note',
    'About frogs. See [Frogs in culture](topic:@-topics-9).',
    '```',
    '',
    'Frogs are amphibians.',
    '',
    '```infobox',
    FROG,
    '```',
    '',
    '```series',
    'title: Amphibians',
    '[Frogs](topic:@-topics-5)',
    '```',
    '',
    '```series',
    'title: More',
    'place: end',
    '[Toads](topic:@-topics-6)',
    '```',
    '',
    '```note',
    'A second note stays where it is.',
    '```',
    '',
    '```infobox',
    'Order: Anura',
    '```',
  ].join('\n');
  const blocks = read(body);
  const boxed = boxedOf(blocks);
  assert.deepEqual(boxed.head?.lines, ['About frogs. See [Frogs in culture](topic:@-topics-9).']);
  assert.deepEqual(
    boxed.side.map((box) => box.title),
    ['Frog', 'Amphibians'],
  );
  assert.deepEqual(
    boxed.end.map((box) => box.title),
    ['More'],
  );
  // Drawn apart: the note, the box, both series; the second note and the box with no title stay in the text.
  assert.deepEqual([...boxed.apart], [0, 2, 3, 4]);
  assert.deepEqual(
    blocks.filter((_block, at) => !boxed.apart.has(at)).map((block) => (block.is === 'code' ? block.lang : block.is)),
    ['text', 'note', 'infobox'],
  );
});

test('an article with no boxes has nothing apart from its text', () => {
  const boxed = boxedOf(read('Frogs.\n\n```js\nlet a = 1;\n```\n'));
  assert.deepEqual(boxed, { head: null, side: [], end: [], apart: new Set() });
});

test("what the editor's menu puts in reads as a box, but for a series, which waits for its pages", () => {
  assert.ok('box' in boxOf('infobox', STARTERS.infobox));
  assert.ok('box' in boxOf('note', STARTERS.note));
  assert.deepEqual(boxOf('series', STARTERS.series), { wrong: 'A series lists its pages, a link a line: none is written yet.' });
});
