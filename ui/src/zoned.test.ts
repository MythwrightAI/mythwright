import assert from 'node:assert/strict';
import { test } from 'node:test';
import { zoned } from './zoned.ts';

test('a time written with no zone is given the reader\'s own, and anything else is left as it was written', () => {
  const cases: [string, string][] = [
    ['02:00', '02:00 Europe/Zagreb'],
    ['  02:00 ', '02:00 Europe/Zagreb'],
    ['mon,thu 09:30', 'mon,thu 09:30 Europe/Zagreb'],
    ['02:00 UTC', '02:00 UTC'],
    ['mon 02:00 America/New_York', 'mon 02:00 America/New_York'],
    ['2:00', '2:00'],
    ['noon', 'noon'],
    ['', ''],
    ['every day 02:00', 'every day 02:00'],
  ];
  for (const [written, meant] of cases) assert.equal(zoned(written, 'Europe/Zagreb'), meant, written);
  assert.equal(zoned('02:00', ''), '02:00');
});
