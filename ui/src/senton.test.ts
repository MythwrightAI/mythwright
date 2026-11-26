import assert from 'node:assert/strict';
import { test } from 'node:test';
import { sentOn } from './senton.ts';

test('the browser is sent on only to a page of a library or a mockup on the same daemon', () => {
  const cases: [string, string | null][] = [
    ['?to=/library/shop/docs/', '/library/shop/docs/'],
    ['?to=/library/shop/docs/guides/install.md', '/library/shop/docs/guides/install.md'],
    ['?to=/library/shop/docs/%3Fsize%3D2', '/library/shop/docs/?size=2'],
    ['?to=/mockups/shop-mockups%3Apricing', '/mockups/shop-mockups:pricing'],
    ['', null],
    ['?to=', null],
    ['?project=shop', null],
    // Anywhere else, here or elsewhere: nowhere.
    ['?to=/api/me', null],
    ['?to=/settings', null],
    ['?to=https://example.com/library/', null],
    ['?to=//example.com/library/', null],
    ['?to=/library/..%5C..%5Cx', null],
    ['?to=library/shop/docs/', null],
  ];
  for (const [search, wanted] of cases) assert.equal(sentOn(search), wanted, search);
});
