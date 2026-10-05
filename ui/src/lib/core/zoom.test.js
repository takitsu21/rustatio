import assert from 'node:assert/strict';
import test from 'node:test';

import {
  ZOOM_DEFAULT,
  ZOOM_MAX,
  ZOOM_MIN,
  clampZoom,
  parseStoredZoom,
  stepZoom,
  zoomPercent,
} from './zoom.js';

test('clampZoom keeps values inside the allowed range', () => {
  assert.equal(clampZoom(1.0), 1.0);
  assert.equal(clampZoom(ZOOM_MIN), ZOOM_MIN);
  assert.equal(clampZoom(ZOOM_MAX), ZOOM_MAX);
  assert.equal(clampZoom(0.1), ZOOM_MIN);
  assert.equal(clampZoom(4), ZOOM_MAX);
});

test('clampZoom falls back to the default for invalid values', () => {
  assert.equal(clampZoom(''), ZOOM_DEFAULT);
  assert.equal(clampZoom('abc'), ZOOM_DEFAULT);
  assert.equal(clampZoom(undefined), ZOOM_DEFAULT);
  assert.equal(clampZoom(NaN), ZOOM_DEFAULT);
});

test('clampZoom accepts numeric strings and rounds to two decimals', () => {
  assert.equal(clampZoom('1.25'), 1.25);
  assert.equal(clampZoom(1.23456), 1.23);
});

test('stepZoom moves by one step and saturates at the bounds', () => {
  assert.equal(stepZoom(ZOOM_DEFAULT, 'in'), 1.15);
  assert.equal(stepZoom(ZOOM_DEFAULT, 'out'), 1.05);
  assert.equal(stepZoom(ZOOM_MAX, 'in'), ZOOM_MAX);
  assert.equal(stepZoom(ZOOM_MIN, 'out'), ZOOM_MIN);
});

test('zoomPercent rounds to a whole percentage', () => {
  assert.equal(zoomPercent(1.1), 110);
  assert.equal(zoomPercent(1.15), 115);
  assert.equal(zoomPercent(ZOOM_MIN), 80);
  assert.equal(zoomPercent(ZOOM_MAX), 160);
});

test('parseStoredZoom reads persisted values and falls back cleanly', () => {
  assert.equal(parseStoredZoom('1.25'), 1.25);
  assert.equal(parseStoredZoom('0.5'), ZOOM_MIN);
  assert.equal(parseStoredZoom('9'), ZOOM_MAX);
  assert.equal(parseStoredZoom(null), ZOOM_DEFAULT);
  assert.equal(parseStoredZoom('garbage'), ZOOM_DEFAULT);
});
