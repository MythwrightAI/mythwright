import assert from 'node:assert/strict';
import { test } from 'node:test';
import { NOT_A_COLOUR, fromArea, fromSlider, hexOf, hsvOf, picks, stepped, typedColour } from './picking.ts';
import { STARTS } from './themes.ts';

test('a colour read from its writing is its hue, its strength and its brightness, and written again is itself', () => {
  const cases: [string, number, number, number][] = [
    ['#ff0000', 0, 1, 1],
    ['#00ff00', 120, 1, 1],
    ['#0000ff', 240, 1, 1],
    ['#ffff00', 60, 1, 1],
    ['#ff00ff', 300, 1, 1],
    ['#808080', 0, 0, 128 / 255],
    ['#000000', 0, 0, 0],
    ['#ffffff', 0, 0, 1],
  ];
  for (const [colour, hue, strength, brightness] of cases) {
    const read = hsvOf(colour);
    assert.equal(read.hue, hue, colour);
    assert.ok(Math.abs(read.strength - strength) < 1e-9, colour);
    assert.ok(Math.abs(read.brightness - brightness) < 1e-9, colour);
    assert.equal(hexOf(read), colour);
  }
});

test('every colour of the themes that ship comes back to itself through the picker', () => {
  for (const colours of Object.values(STARTS)) for (const colour of Object.values(colours)) assert.equal(hexOf(hsvOf(colour)), colour);
});

test('a grey keeps the hue the picker had, so that the slider does not jump as the area reaches its edge', () => {
  assert.equal(hsvOf('#808080', 200).hue, 200);
  assert.equal(hsvOf('#000000', 45).hue, 45);
  assert.equal(hsvOf('#ff0000', 200).hue, 0, 'a colour with a hue of its own says it');
});

test('a hue written past either end is the same hue', () => {
  assert.equal(hexOf({ hue: 360, strength: 1, brightness: 1 }), '#ff0000');
  assert.equal(hexOf({ hue: -120, strength: 1, brightness: 1 }), '#0000ff');
  assert.equal(hexOf({ hue: 0, strength: 2, brightness: -1 }), '#000000', 'strength and brightness are held to 0 and 1');
});

test('a point of the area is strength across and brightness up, held to the box', () => {
  const box = { left: 10, top: 20, width: 200, height: 100 };
  assert.deepEqual(fromArea(10, 20, box), { strength: 0, brightness: 1 });
  assert.deepEqual(fromArea(210, 120, box), { strength: 1, brightness: 0 });
  assert.deepEqual(fromArea(110, 70, box), { strength: 0.5, brightness: 0.5 });
  assert.deepEqual(fromArea(-50, 400, box), { strength: 0, brightness: 0 });
  assert.deepEqual(fromArea(5, 5, { left: 0, top: 0, width: 0, height: 0 }), { strength: 1, brightness: 0 }, 'a box with no size divides by nothing');
});

test('a point of the slider is a hue from 0 to 359', () => {
  const box = { left: 0, width: 359 };
  assert.equal(fromSlider(0, box), 0);
  assert.equal(fromSlider(180, box), 180);
  assert.equal(fromSlider(1000, box), 359);
  assert.equal(fromSlider(-4, box), 0);
});

test('the arrows move the area by a hundredth, ten with Shift, and stop at the edges', () => {
  const at = { hue: 30, strength: 0.5, brightness: 0.5 };
  assert.deepEqual(stepped(at, 'area', 'ArrowRight'), { hue: 30, strength: 0.51, brightness: 0.5 });
  assert.deepEqual(stepped(at, 'area', 'ArrowLeft'), { hue: 30, strength: 0.49, brightness: 0.5 });
  assert.deepEqual(stepped(at, 'area', 'ArrowUp'), { hue: 30, strength: 0.5, brightness: 0.51 });
  assert.deepEqual(stepped(at, 'area', 'ArrowDown', true), { hue: 30, strength: 0.5, brightness: 0.4 });
  assert.deepEqual(stepped({ ...at, strength: 0.995 }, 'area', 'ArrowRight', true), { hue: 30, strength: 1, brightness: 0.5 });
  assert.deepEqual(stepped({ ...at, brightness: 0 }, 'area', 'ArrowDown'), { hue: 30, strength: 0.5, brightness: 0 });
  assert.equal(stepped(at, 'area', 'Home'), null, 'a key that moves nothing there is left to the page');
  assert.equal(stepped(at, 'area', 'a'), null);
});

test('the arrows move the hue by a degree, ten with Shift, Home and End go to either end, and it stops there', () => {
  const at = { hue: 30, strength: 0.5, brightness: 0.5 };
  assert.equal(stepped(at, 'hue', 'ArrowRight')?.hue, 31);
  assert.equal(stepped(at, 'hue', 'ArrowUp')?.hue, 31);
  assert.equal(stepped(at, 'hue', 'ArrowLeft', true)?.hue, 20);
  assert.equal(stepped(at, 'hue', 'ArrowDown')?.hue, 29);
  assert.equal(stepped(at, 'hue', 'Home')?.hue, 0);
  assert.equal(stepped(at, 'hue', 'End')?.hue, 359);
  assert.equal(stepped({ ...at, hue: 355 }, 'hue', 'ArrowRight', true)?.hue, 359);
  assert.equal(stepped({ ...at, hue: 3 }, 'hue', 'ArrowLeft', true)?.hue, 0);
  assert.equal(stepped(at, 'hue', 'Enter'), null);
  assert.deepEqual({ ...stepped(at, 'hue', 'ArrowRight'), hue: 30 }, at, 'only the hue moves');
});

test('what is typed is taken once it reads as #rrggbb, written small, and anything else is said and not taken', () => {
  assert.deepEqual(typedColour('#2fd4c4'), { colour: '#2fd4c4' });
  assert.deepEqual(typedColour('#2FD4C4'), { colour: '#2fd4c4' });
  assert.deepEqual(typedColour('  #2fd4c4 '), { colour: '#2fd4c4' });
  for (const wrong of ['', '#', '#2fd4c', '2fd4c4', '#2fd4c4f', '#2fd4cg', 'red', '#fff', 'rgb(1, 2, 3)']) assert.deepEqual(typedColour(wrong), { wrong: NOT_A_COLOUR }, JSON.stringify(wrong));
});

test('what is offered to pick is the theme’s other seven colours by what each is, then those of the themes that ship, each once', () => {
  const colours = { ...STARTS['grid-iron']!, accent: '#2FD4C4' };
  const shipped = [
    { name: 'Grid Iron', colours: STARTS['grid-iron']! },
    { name: 'Rosé Pine', colours: STARTS['rose-pine']! },
  ];
  const offered = picks(colours, 'accent', shipped);
  assert.deepEqual(
    offered.own.map((one) => one.says),
    ['Ground', 'Text', 'Good', 'Bad', 'Note', 'Agent', 'Work'],
  );
  assert.equal(offered.own[0]!.colour, '#0c0c0c');
  // Grid Iron's own colours are the theme's but for its accent, which the theme no longer has, so only that is offered of it.
  assert.deepEqual(offered.shipped.filter((one) => one.says.startsWith('Grid Iron')), [{ colour: '#dc7b32', says: 'Grid Iron: Accent' }]);
  // Taking one changes nothing of what is offered.
  assert.deepEqual(picks({ ...colours, accent: '#dc7b32' }, 'accent', shipped), offered);
  assert.equal(offered.shipped.filter((one) => one.says.startsWith('Rosé Pine')).length, 8);
  const all = [...offered.own, ...offered.shipped].map((one) => one.colour);
  assert.equal(new Set(all).size, all.length, 'none twice');
});
