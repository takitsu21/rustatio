import assert from 'node:assert/strict';
import test from 'node:test';

import { STATUS_TONES, getStateMeta, getStatusTypeMeta, getTone } from './statusMeta.js';

test('getStateMeta resolves known states case-insensitively', () => {
  assert.equal(getStateMeta('RUNNING').label, 'Running');
  assert.equal(getStateMeta('running').tone, 'upload');
  assert.equal(getStateMeta('Paused').icon, 'pause');
});

test('getStateMeta falls back to stopped for unknown or missing states', () => {
  assert.deepEqual(getStateMeta(undefined).key, 'stopped');
  assert.equal(getStateMeta('bogus').label, 'Stopped');
  assert.equal(getStateMeta(null).tone, 'muted');
});

test('getStateMeta marks transitional states as spinning', () => {
  assert.equal(getStateMeta('starting').spin, true);
  assert.equal(getStateMeta('stopping').spin, true);
  assert.equal(getStateMeta('running').spin, undefined);
});

test('getStatusTypeMeta resolves banner types and falls back to idle', () => {
  assert.equal(getStatusTypeMeta('warning').tone, 'ratio');
  assert.equal(getStatusTypeMeta('success').label, 'Done');
  assert.equal(getStatusTypeMeta(undefined).key, 'idle');
  assert.equal(getStatusTypeMeta('unknown').key, 'idle');
});

test('getTone returns a complete tone and falls back to muted', () => {
  for (const tone of Object.values(STATUS_TONES)) {
    assert.ok(tone.text && tone.border && tone.bg && tone.dot);
  }

  assert.equal(getTone('upload').text, 'text-stat-upload');
  assert.equal(getTone('missing').text, 'text-muted-foreground');
});
