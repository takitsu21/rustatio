import assert from 'node:assert/strict';
import test from 'node:test';

import { formatBytes, formatBytesCompact, formatRate, formatRateCompact } from './format.js';

test('formatBytes handles zero, boundaries, and large values', () => {
  assert.equal(formatBytes(0), '0 B');
  assert.equal(formatBytes(512), '512 B');
  assert.equal(formatBytes(1024), '1 KB');
  assert.equal(formatBytes(1536), '1.5 KB');
  assert.equal(formatBytes(1024 * 1024 * 1024 * 3.25), '3.3 GB');
});

test('formatBytesCompact drops the space between value and unit', () => {
  assert.equal(formatBytesCompact(0), '0');
  assert.equal(formatBytesCompact(1024), '1K');
  assert.equal(formatBytesCompact(1024 * 1024 * 5), '5M');
});

test('formatRate switches units at 1000 KB/s', () => {
  assert.equal(formatRate(0), '0 KB/s');
  assert.equal(formatRate(999), '999 KB/s');
  assert.equal(formatRate(1024), '1 MB/s');
  assert.equal(formatRate(1024 * 1024), '1 GB/s');
});

test('formatRateCompact compresses units for tight layouts', () => {
  assert.equal(formatRateCompact(0), '0');
  assert.equal(formatRateCompact(512), '512K');
  assert.equal(formatRateCompact(2048), '2M');
});
