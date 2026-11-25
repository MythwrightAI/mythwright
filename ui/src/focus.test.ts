import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { test } from 'node:test';

test('nothing is marked as having the keyboard in the accent of the theme', () => {
  const here = new URL('.', import.meta.url);
  const found: string[] = [];
  for (const file of readdirSync(here).filter((name) => name.endsWith('.css'))) {
    // A rule is what stands before a `{` and what is in it; one whose selector says focus says no accent.
    for (const [, selector = '', body = ''] of readFileSync(new URL(file, here), 'utf8').matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
      const focused = selector.split(',').filter((one) => /:focus(?!-within)/.test(one));
      if (focused.length > 0 && /--accent/.test(body)) found.push(`${file}: ${focused.join(',').trim()}`);
    }
  }
  assert.deepEqual(found, []);
});
