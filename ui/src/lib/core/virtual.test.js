import assert from 'node:assert/strict';
import test from 'node:test';

import { getScrollTopForIndex, getWindow } from './virtual.js';

test('getWindow returns an empty window for an empty list', () => {
  const win = getWindow({ total: 0, scrollTop: 0, viewportHeight: 300, rowHeight: 28, buffer: 5 });
  assert.deepEqual(win, { startIndex: 0, endIndex: 0, offsetY: 0, totalHeight: 0 });
});

test('getWindow renders the whole list when it fits', () => {
  const win = getWindow({ total: 10, scrollTop: 0, viewportHeight: 500, rowHeight: 28, buffer: 5 });
  assert.equal(win.startIndex, 0);
  assert.equal(win.endIndex, 10);
  assert.equal(win.offsetY, 0);
  assert.equal(win.totalHeight, 280);
});

test('getWindow windows the middle of a long list with buffer', () => {
  const win = getWindow({
    total: 5000,
    scrollTop: 28000,
    viewportHeight: 560,
    rowHeight: 28,
    buffer: 10,
  });
  assert.equal(win.startIndex, 990);
  assert.equal(win.endIndex, 1031);
  assert.equal(win.offsetY, 990 * 28);
  assert.equal(win.totalHeight, 5000 * 28);
});

test('getWindow clamps at the end of the list', () => {
  const win = getWindow({
    total: 100,
    scrollTop: 100 * 28,
    viewportHeight: 280,
    rowHeight: 28,
    buffer: 10,
  });
  assert.equal(win.startIndex, 90);
  assert.equal(win.endIndex, 100);
});

test('getWindow handles negative scroll and invalid inputs safely', () => {
  const win = getWindow({
    total: 50,
    scrollTop: -100,
    viewportHeight: -5,
    rowHeight: 0,
    buffer: -3,
  });
  assert.equal(win.startIndex, 0);
  assert.equal(win.endIndex, 1);
  assert.equal(win.offsetY, 0);
  assert.equal(win.totalHeight, 50);
});

test('getScrollTopForIndex returns the current position when the row is visible', () => {
  assert.equal(
    getScrollTopForIndex({ index: 10, currentScrollTop: 200, viewportHeight: 280, rowHeight: 28 }),
    200
  );
});

test('getScrollTopForIndex scrolls up for rows above the viewport', () => {
  assert.equal(
    getScrollTopForIndex({ index: 2, currentScrollTop: 200, viewportHeight: 280, rowHeight: 28 }),
    56
  );
});

test('getScrollTopForIndex scrolls down for rows below the viewport', () => {
  assert.equal(
    getScrollTopForIndex({ index: 20, currentScrollTop: 200, viewportHeight: 280, rowHeight: 28 }),
    308
  );
});

test('getScrollTopForIndex ignores invalid indexes', () => {
  assert.equal(
    getScrollTopForIndex({ index: -1, currentScrollTop: 120, viewportHeight: 280, rowHeight: 28 }),
    120
  );
});
