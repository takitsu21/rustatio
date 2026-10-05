import assert from 'node:assert/strict';
import test from 'node:test';
import { normalizeViewMode } from './viewMode.js';

test('normalizeViewMode accepts known modes', () => {
  assert.equal(normalizeViewMode('standard'), 'standard');
  assert.equal(normalizeViewMode('grid'), 'grid');
  assert.equal(normalizeViewMode('watch'), 'watch');
});

test('normalizeViewMode falls back on invalid values', () => {
  assert.equal(normalizeViewMode('unknown'), 'grid');
  assert.equal(normalizeViewMode(null), 'grid');
  assert.equal(normalizeViewMode(undefined, 'standard'), 'standard');
});
