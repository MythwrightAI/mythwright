import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { clockUnfit, dateIn, dayIn, LONGEST_ZONE, partsIn, settingOf, timeIn, whenIn, zoneKnown, zones, zonesFound, zoneSaid } from './clock.ts';

const table = JSON.parse(readFileSync(new URL('./clockcases.json', import.meta.url), 'utf8')) as { longest: number; cases: [unknown, string | null][] };

// 2026-07-02 04:07 UTC is 10:07 PM on 1 July in Denver, on summer time.
const SUMMER = Date.UTC(2026, 6, 2, 4, 7);
// 2026-01-15 05:07 UTC is 10:07 PM on 14 January in Denver, on winter time.
const WINTER = Date.UTC(2026, 0, 15, 5, 7);
const denver = { zone: 'America/Denver', hours: '12h' as const };

test('a setting is refused in the daemon\'s own sentence, case for case with the table it is held to', () => {
  assert.equal(table.longest, LONGEST_ZONE);
  assert.ok(table.cases.length > 15);
  for (const [said, refused] of table.cases) assert.equal(clockUnfit(said), refused, JSON.stringify(said));
});

test('a time is written on a clock of twelve hours by default: AM or PM, and the zone as it is called that day', () => {
  assert.equal(timeIn(SUMMER, denver), '10:07 PM MDT');
  assert.equal(timeIn(WINTER, denver), '10:07 PM MST');
  assert.equal(timeIn(Date.UTC(2026, 6, 1, 15, 5), denver), '9:05 AM MDT', 'no nought before the hour');
  assert.equal(timeIn(Date.UTC(2026, 6, 1, 6, 0), denver), '12:00 AM MDT', 'midnight is twelve');
  assert.equal(timeIn(Date.UTC(2026, 6, 1, 18, 30), denver), '12:30 PM MDT', 'and so is noon');
});

test('on a clock of twenty-four hours it is the same moment with the hour as it is', () => {
  const day = { ...denver, hours: '24h' as const };
  assert.equal(timeIn(SUMMER, day), '22:07 MDT');
  assert.equal(timeIn(WINTER, day), '22:07 MST');
  assert.equal(timeIn(Date.UTC(2026, 6, 1, 6, 0), day), '00:00 MDT');
});

test('a time is written in the reader\'s zone and no other', () => {
  assert.equal(timeIn(SUMMER, { zone: 'UTC', hours: '24h' }), '04:07 UTC');
  assert.equal(timeIn(SUMMER, { zone: 'Asia/Kolkata', hours: '12h' }).slice(0, 8), '9:37 AM ');
  assert.equal(timeIn(SUMMER, { zone: 'America/New_York', hours: '12h' }), '12:07 AM EDT');
});

test('a zone the browser does not know is written in UTC, so that a time is always written', () => {
  assert.equal(zoneKnown('Nowhere/Atall'), false);
  assert.equal(timeIn(SUMMER, { zone: 'Nowhere/Atall', hours: '24h' }), '04:07 UTC');
  assert.equal(settingOf({ zone: 'Nowhere/Atall' }), null, 'and it is no setting the page wears');
  assert.deepEqual(settingOf({ zone: 'America/Denver' }), denver, 'twelve hours where nothing is said');
});

test('today is the time, this week the weekday before it, and before that the date, each day the reader\'s', () => {
  assert.equal(whenIn(SUMMER, denver, SUMMER + 60_000), '10:07 PM MDT');
  // Five in the afternoon on 1 July in Denver, seen at SUMMER: the same day there, and yesterday in UTC.
  const earlier = Date.UTC(2026, 6, 1, 23, 0);
  assert.equal(whenIn(earlier, denver, SUMMER), '5:00 PM MDT');
  assert.equal(whenIn(earlier, { zone: 'UTC', hours: '24h' }, SUMMER), 'Wed 23:00 UTC');
  assert.equal(whenIn(SUMMER, denver, SUMMER + 26 * 3_600_000), 'Wed 10:07 PM MDT');
  assert.equal(whenIn(WINTER, denver, SUMMER), '14 Jan 10:07 PM MST');
  assert.equal(dayIn(SUMMER, 'America/Denver') + 1, dayIn(SUMMER, 'UTC'));
  assert.equal(dateIn(WINTER, 'America/Denver', SUMMER), '14 Jan');
  assert.equal(dateIn(Date.UTC(2025, 11, 1, 12), 'America/Denver', SUMMER), '1 Dec 2025');
});

test('the zones are the browser\'s, found by every word typed, and said with what each is called now', () => {
  const all = zones();
  assert.ok(all.includes('America/Denver') && all.includes('UTC'));
  assert.deepEqual(zonesFound('denver', all), ['America/Denver']);
  assert.ok(zonesFound('new york', all).includes('America/New_York'));
  assert.deepEqual(zonesFound('america nowhere', all), []);
  assert.equal(zoneSaid('America/New_York', SUMMER), 'America/New York (EDT)');
  assert.equal(partsIn(WINTER, 'America/Denver').name, 'MST');
});
