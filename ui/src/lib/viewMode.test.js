import assert from 'node:assert/strict';
import test from 'node:test';
import { normalizeViewMode } from './viewMode.js';

test('normalizeViewMode accepts known modes', () => {
  assert.equal(normalizeViewMode('standard'), 'standard');
  assert.equal(normalizeViewMode('grid'), 'grid');
  assert.equal(normalizeViewMode('watch'), 'watch');
});

test('normalizeViewMode falls back on invalid values', () => {
  assert.equal(normalizeViewMode('unknown'), 'standard');
  assert.equal(normalizeViewMode(null), 'standard');
  assert.equal(normalizeViewMode(undefined, 'grid'), 'grid');
});
