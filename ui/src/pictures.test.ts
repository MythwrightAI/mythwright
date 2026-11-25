import assert from 'node:assert/strict';
import { test } from 'node:test';
import { attachmentOf, captionOf, drawnAs, elsewhereSaid, figureOf, mediaOf, missingSaid, placeOf, takenIn, withPictures, written } from './pictures.ts';
import { read, spans } from './prose.ts';

test('a line that is nothing but a picture is a figure, with its caption, where it leads and where it stands', () => {
  assert.deepEqual(figureOf('![The build, step by step](attachment:ratt_1a2b)'), { caption: 'The build, step by step', to: 'attachment:ratt_1a2b', place: null });
  assert.deepEqual(figureOf('  ![Shop](attachment:ratt_9 "side")  '), { caption: 'Shop', to: 'attachment:ratt_9', place: 'side' });
  assert.deepEqual(figureOf('![](attachment:ratt_9 "across")'), { caption: '', to: 'attachment:ratt_9', place: 'across' });
  assert.deepEqual(figureOf('![a \\[b\\] c](attachment:ratt_9)'), { caption: 'a [b] c', to: 'attachment:ratt_9', place: null });
  assert.equal(figureOf('see ![Shop](attachment:ratt_9) here'), null);
  assert.equal(figureOf('[Shop](https://example.com)'), null);
  assert.equal(figureOf('![Shop]'), null);
});

test('where a picture stands is side or across, whatever its capitals, and anything else says nothing', () => {
  assert.equal(placeOf('side'), 'side');
  assert.equal(placeOf(' Across '), 'across');
  assert.equal(placeOf('a title'), null);
  assert.equal(placeOf(undefined), null);
});

test('a picture written as it is kept reads back as the same picture', () => {
  for (const picture of [
    { caption: 'Shop', to: 'attachment:ratt_1', place: null },
    { caption: 'a [b] c \\ d', to: 'attachment:ratt_1', place: 'side' as const },
    { caption: '', to: 'attachment:ratt_1', place: 'across' as const },
  ]) {
    assert.deepEqual(figureOf(written(picture)), picture);
  }
  assert.equal(written({ caption: 'Shop', to: 'attachment:ratt_1', place: 'side' }), '![Shop](attachment:ratt_1 "side")');
});

test('a picture leads to an attachment by its id, and anything else leads elsewhere', () => {
  assert.equal(attachmentOf('attachment:ratt_1a2b'), 'ratt_1a2b');
  assert.equal(attachmentOf('attachment:../x'), null);
  assert.equal(attachmentOf('https://example.com/a.png'), null);
  assert.equal(attachmentOf('ratt_1a2b'), null);
});

test('a picture is drawn by what is attached: a picture or an SVG drawn, a page as a view, another file to be saved', () => {
  const attached = [
    { id: 'ratt_p', name: 'shot.png', media: 'image/png' },
    { id: 'ratt_s', name: 'flow.svg', media: 'image/svg+xml' },
    { id: 'ratt_v', name: 'chart.html', media: 'text/html; charset=utf-8' },
    { id: 'ratt_f', name: 'notes.pdf', media: 'application/pdf' },
  ];
  const at = (to: string) => drawnAs({ caption: '', to, place: null }, attached);
  assert.deepEqual(at('attachment:ratt_p'), { as: 'picture', id: 'ratt_p' });
  assert.deepEqual(at('attachment:ratt_s'), { as: 'picture', id: 'ratt_s' });
  assert.deepEqual(at('attachment:ratt_v'), { as: 'view', id: 'ratt_v' });
  assert.deepEqual(at('attachment:ratt_f'), { as: 'file', id: 'ratt_f', name: 'notes.pdf' });
  // Not listed (gone, or another record's), or nothing known: tried as a picture, which says so where it cannot be read.
  assert.deepEqual(at('attachment:ratt_gone'), { as: 'picture', id: 'ratt_gone' });
  assert.deepEqual(drawnAs({ caption: '', to: 'attachment:ratt_v', place: null }, null), { as: 'picture', id: 'ratt_v' });
  // Somewhere else on the web is never fetched, and is a link.
  assert.deepEqual(at('https://example.com/a.png'), { as: 'link', to: 'https://example.com/a.png' });
  // Anything else is nothing to press: the marks as they were written.
  assert.deepEqual(at('javascript:alert(1)'), { as: 'written', text: '![](javascript:alert(1))' });
  assert.deepEqual(at('pictures/dot.png'), { as: 'written', text: '![](pictures/dot.png)' });
});

test('a picture that cannot be read says so in a line, by its caption where it has one', () => {
  assert.equal(missingSaid({ caption: 'Shop', to: 'attachment:ratt_1', place: null }), '“Shop” is not here: it was removed, or it is not yours to see.');
  assert.equal(missingSaid({ caption: ' ', to: 'attachment:ratt_1', place: null }), 'A picture is not here: it was removed, or it is not yours to see.');
});

test('a picture somewhere else is a link that says its caption and that it is not fetched', () => {
  const said = elsewhereSaid({ caption: 'Logo', to: 'https://example.com/a.png', place: null });
  assert.equal(said.words, 'Logo');
  assert.match(said.title, /not fetched/);
  assert.equal(elsewhereSaid({ caption: '', to: 'https://example.com/a.png', place: null }).words, 'https://example.com/a.png');
});

test('pictures, SVGs and pages are put in the text where they are pasted, and other files are not', () => {
  for (const media of ['image/png', 'image/jpeg', 'image/gif', 'image/webp', 'image/svg+xml', 'text/html', 'text/html; charset=utf-8']) assert.ok(takenIn(media), media);
  for (const media of ['application/pdf', 'text/plain', 'application/octet-stream', '']) assert.ok(!takenIn(media), media);
});

test('a page begins captioned by its name, and a picture pasted by nothing', () => {
  assert.equal(captionOf('Build times.html', 'text/html'), 'Build times');
  assert.equal(captionOf('pasted.png', 'image/png'), '');
});

test('what is written is read with its pictures: a line of one is a figure, and one among words stays in its line', () => {
  assert.deepEqual(read('Before.\n\n![Shop](attachment:ratt_1 "side")\n\nAfter.'), [
    { is: 'text', spans: [{ is: 'text', text: 'Before.' }] },
    { is: 'figure', caption: 'Shop', to: 'attachment:ratt_1', place: 'side' },
    { is: 'text', spans: [{ is: 'text', text: 'After.' }] },
  ]);
  assert.deepEqual(read('Before.\n![Shop](attachment:ratt_1)\nAfter.'), [
    { is: 'text', spans: [{ is: 'text', text: 'Before.' }] },
    { is: 'figure', caption: 'Shop', to: 'attachment:ratt_1', place: null },
    { is: 'text', spans: [{ is: 'text', text: 'After.' }] },
  ]);
  assert.deepEqual(spans('see ![my_shop](attachment:ratt_1) and **more**'), [
    { is: 'text', text: 'see ' },
    { is: 'image', caption: 'my_shop', to: 'attachment:ratt_1', place: null },
    { is: 'text', text: ' and ' },
    { is: 'bold', inside: [{ is: 'text', text: 'more' }] },
  ]);
  // A picture somewhere else is read as one too, and drawn as a link; a link is still a link.
  assert.deepEqual(spans('a ![logo](https://example.com/a.png) b [site](https://example.com)'), [
    { is: 'text', text: 'a ' },
    { is: 'image', caption: 'logo', to: 'https://example.com/a.png', place: null },
    { is: 'text', text: ' b ' },
    { is: 'link', to: 'https://example.com', inside: [{ is: 'text', text: 'site' }] },
  ]);
});

test('a picture inside a block of code or under an item of a list is not a figure', () => {
  assert.deepEqual(read('```\n![Shop](attachment:ratt_1)\n```'), [{ is: 'code', text: '![Shop](attachment:ratt_1)' }]);
  const listed = read('- one\n  ![Shop](attachment:ratt_1)');
  assert.equal(listed.length, 1);
  assert.equal(listed[0]!.is, 'list');
});

test('text with no picture in it is read as it always was', () => {
  for (const text of ['Plain words.', '[x](https://example.com) and ! [y]', '!not a picture', 'a ![ b', '# Head\n\n- one\n- two']) {
    assert.ok(read(text).every((block) => block.is !== 'figure'), text);
  }
});

test('pictures put in a reply each stand on a line of their own, wherever the caret was, and the caret goes after them', () => {
  const one = { caption: '', to: 'attachment:ratt_1', place: null };
  const two = { caption: 'Flow', to: 'attachment:ratt_2', place: null };
  assert.deepEqual(withPictures('', 0, [one]), { text: '![](attachment:ratt_1)\n', caret: 23 });
  assert.deepEqual(withPictures('See this', 8, [one]), { text: 'See this\n![](attachment:ratt_1)\n', caret: 32 });
  assert.deepEqual(withPictures('See this:\nand that', 9, [one, two]), { text: 'See this:\n![](attachment:ratt_1)\n![Flow](attachment:ratt_2)\nand that', caret: 60 });
  assert.deepEqual(withPictures('Before\n', 7, [one]).text, 'Before\n![](attachment:ratt_1)\n');
  // Each is read back as a figure.
  assert.deepEqual(read(withPictures('See this', 8, [one, two]).text).map((block) => block.is), ['text', 'figure', 'figure']);
});

test('an SVG or a page that says nothing of what it is, is known by its name', () => {
  assert.equal(mediaOf({ name: 'flow.svg', type: '' }), 'image/svg+xml');
  assert.equal(mediaOf({ name: 'chart.HTML', type: '' }), 'text/html');
  assert.equal(mediaOf({ name: 'shot.png', type: 'image/png' }), 'image/png');
  assert.equal(mediaOf({ name: 'notes', type: '' }), '');
});
