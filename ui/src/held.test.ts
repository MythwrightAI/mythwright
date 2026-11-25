import assert from 'node:assert/strict';
import { test } from 'node:test';
import { hold } from './held.ts';

/** An edge held where frames come only when the test says, with every width that was drawn. */
function held(towards: 1 | -1 = -1) {
  const drawn: number[] = [];
  let waiting: (() => void) | null = null;
  let asked = 0;
  const holding = hold({
    width: 300,
    x: 1000,
    towards,
    fit: (wanted) => Math.min(Math.max(wanted, 240), 600),
    draw: (width) => drawn.push(width),
    frame: (work) => {
      waiting = work;
      asked += 1;
      return asked;
    },
    unframe: () => {
      waiting = null;
    },
  });
  const frame = () => {
    const work = waiting;
    waiting = null;
    work?.();
  };
  return { holding, drawn, frame, asked: () => asked };
}

test('however often the pointer moves, the width is drawn once a frame, and the last place wins', () => {
  const edge = held();
  for (const x of [990, 960, 940, 900]) edge.holding.move(x);
  assert.deepEqual(edge.drawn, [], 'nothing is drawn between frames');
  assert.equal(edge.asked(), 1, 'one frame is asked for, however many moves');
  edge.frame();
  assert.deepEqual(edge.drawn, [400]);
  edge.holding.move(880);
  edge.holding.move(850);
  edge.frame();
  assert.deepEqual(edge.drawn, [400, 450]);
});

test('an edge at the left is made wider by going left, and one at the right by going right', () => {
  const left = held(-1);
  left.holding.move(950);
  left.frame();
  assert.deepEqual(left.drawn, [350]);
  const right = held(1);
  right.holding.move(950);
  right.frame();
  assert.deepEqual(right.drawn, [250]);
});

test('the least and the most it may be hold all the way', () => {
  const edge = held();
  edge.holding.move(2000);
  edge.frame();
  edge.holding.move(0);
  edge.frame();
  assert.deepEqual(edge.drawn, [240, 600]);
});

test('let go, it is as wide as it was last asked to be, drawn at once and said once', () => {
  const edge = held();
  edge.holding.move(900);
  edge.frame();
  edge.holding.move(870);
  // No frame has come since the last move: letting go draws it.
  assert.equal(edge.holding.letGo(), 430);
  assert.deepEqual(edge.drawn, [400, 430]);
  edge.frame();
  assert.deepEqual(edge.drawn, [400, 430], 'the frame that was asked for is taken back');
});

test('let go without having moved, nothing is drawn', () => {
  const edge = held();
  assert.equal(edge.holding.letGo(), 300);
  assert.deepEqual(edge.drawn, []);
});

test('called off, it is as wide as when it was taken hold of', () => {
  const edge = held();
  edge.holding.move(900);
  edge.frame();
  edge.holding.move(800);
  assert.equal(edge.holding.callOff(), 300);
  assert.deepEqual(edge.drawn, [400, 300]);
});

test('a frame with nothing new draws nothing', () => {
  const edge = held();
  edge.holding.move(900);
  edge.frame();
  edge.holding.move(900);
  edge.frame();
  assert.deepEqual(edge.drawn, [400]);
});
