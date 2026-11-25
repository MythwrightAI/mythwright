import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Diagnostic } from './api.ts';
import { withSaid } from './warned.ts';

const at = (code: string, line: number, message = code): Diagnostic => ({
  severity: 'warning',
  code,
  message,
  span: { start: { line, col: 3 }, end: { line, col: 9 } },
});

test('what an apply said that the check does not is listed after what the check says', () => {
  const checked = [at('unwoken', 4)];
  const said = [at('unwoken', 4), at('unheard', 7, 'this is applied for no project')];
  assert.deepEqual(
    withSaid(checked, said).map((one) => one.code),
    ['unwoken', 'unheard'],
  );
});

test('what both say is said once, and the same thing of two handlers twice', () => {
  const checked = [at('unheard', 3), at('unheard', 9)];
  assert.deepEqual(withSaid(checked, [at('unheard', 9), at('unheard', 3)]), checked);
  assert.deepEqual(
    withSaid([], [at('unheard', 3), at('unheard', 9)]).map((one) => one.span.start.line),
    [3, 9],
  );
});

test('an apply that said nothing leaves what the check says as it is', () => {
  const checked = [at('unwoken', 4)];
  assert.deepEqual(withSaid(checked, []), checked);
});
