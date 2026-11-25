import assert from 'node:assert/strict';
import { test } from 'node:test';
import { addressOf, changes, fileOf, folderOf, groups, isMockups, nameOf, titleOf, variantTitle, widthCalled } from './mockups.ts';
import type { Row } from './table.ts';

const row = (name: string, data: Record<string, unknown> = {}): Row => ({ id: `shop-mockups:${name}`, number: 0, data: { path: name, ...data }, position: 0, created: 0, updated: 0 });
const kept = { name: 'mockups', store: 'mockups', at: 'mockups' };

test('a collection is mockups by how it is kept, and its folder is the one said or the one called what it is', () => {
  assert.equal(isMockups(kept), true);
  assert.equal(isMockups({ name: 'docs', store: 'documents' }), false);
  assert.equal(folderOf({ name: 'mockups', store: 'mockups' }), 'mockups');
  assert.equal(folderOf({ name: 'mockups', store: 'mockups', at: 'design/tries' }), 'design/tries');
});

test('a mockup is called what it says, or what its folder is, and its page is a file of that folder', () => {
  assert.equal(titleOf(row('pricing', { title: 'Pricing page' })), 'Pricing page');
  assert.equal(titleOf(row('pricing', { title: ' ' })), 'pricing');
  assert.equal(nameOf({ ...row('pricing'), data: {} }), 'pricing', 'what it is known by says where it is');
  assert.equal(fileOf(kept, row('pricing')), 'mockups/pricing/index.html');
  assert.equal(fileOf({ ...kept, at: 'design/tries' }, row('pricing'), 'mockup.yaml'), 'design/tries/pricing/mockup.yaml');
});

test('variants are set beside what they vary, and a variant of a variant with them', () => {
  const rows = [
    row('pricing-bold', { title: 'Bold', of: 'shop-mockups:pricing' }),
    row('checkout', { title: 'Checkout' }),
    row('pricing', { title: 'Pricing' }),
    row('pricing-bolder', { title: 'Bolder', of: 'shop-mockups:pricing-bold' }),
  ];
  const found = groups(rows).map((group) => [titleOf(group.first), group.variants.map(titleOf)]);
  assert.deepEqual(found, [
    ['Checkout', []],
    ['Pricing', ['Bold', 'Bolder']],
  ]);
});

test('a variant of something that is not there stands by itself, and two that name each other are not lost', () => {
  const orphan = groups([row('late', { title: 'Late', of: 'shop-mockups:gone' })]);
  assert.deepEqual(orphan.map((group) => titleOf(group.first)), ['Late']);
  const ring = groups([row('a', { title: 'A', of: 'shop-mockups:b' }), row('b', { title: 'B', of: 'shop-mockups:a' })]);
  const shown = ring.flatMap((group) => [group.first, ...group.variants]).map(titleOf).sort();
  assert.deepEqual(shown, ['A', 'B'], 'each is somewhere in the gallery');
});

test('a width is a phone, a tablet or as wide as there is room, and anything else is the last', () => {
  assert.equal(widthCalled('phone').across, 390);
  assert.equal(widthCalled('tablet').across, 820);
  assert.equal(widthCalled('whole').across, null);
  assert.equal(widthCalled(undefined).name, 'whole');
});

test('an event says whether there may be other mockups, and which one is to be drawn again', () => {
  const said = (topic: string, data: Record<string, unknown>) => changes({ topic, data }, 'shop', kept);
  assert.deepEqual(said('record.updated', { project: 'shop', collection: 'mockups' }), { list: true, drawn: null });
  assert.deepEqual(said('record.updated', { project: 'shop', collection: 'board' }), { list: false, drawn: null });
  assert.deepEqual(said('record.updated', { project: 'depot', collection: 'mockups' }), { list: false, drawn: null });
  assert.deepEqual(said('project.file.changed', { project: 'shop', path: 'mockups/pricing/style.css' }), { list: true, drawn: 'pricing' });
  assert.deepEqual(said('project.file.changed', { project: 'shop', path: 'mockups/pricing/art/mark.svg' }), { list: true, drawn: 'pricing' });
  assert.deepEqual(said('project.file.changed', { project: 'shop', path: 'mockups/loose.html' }), { list: false, drawn: null }, 'a file that is in no mockup');
  assert.deepEqual(said('project.file.changed', { project: 'shop', path: 'src/main.rs' }), { list: false, drawn: null });
  assert.deepEqual(said('project.rescanned', { project: 'shop' }), { list: true, drawn: null });
});

test('a variant is called after what it varies until somebody calls it something, and never what another is', () => {
  const pricing = row('pricing', { title: 'Pricing' });
  assert.equal(variantTitle(pricing, ['Pricing']), 'Pricing, another way');
  assert.equal(variantTitle(pricing, ['Pricing', 'Pricing, another way']), 'Pricing, another way 2');
});

test('a mockup has an address of its own, by what it is known by', () => {
  assert.equal(addressOf('shop-mockups:pricing'), '/mockups/shop-mockups%3Apricing');
});
