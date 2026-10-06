import assert from 'node:assert/strict';
import test from 'node:test';

import { mergeSummary } from './instanceSync.js';

function baseInstance(overrides = {}) {
  return {
    id: 'inst-1',
    isRunning: false,
    isPaused: false,
    source: 'manual',
    statusMessage: 'Ready to start faking',
    statusType: 'idle',
    statusIcon: null,
    completionPercent: 100,
    stats: {
      uploaded: 100,
      downloaded: 50,
      ratio: 2,
      current_upload_rate: 10,
      current_download_rate: 5,
      seeders: 3,
      leechers: 1,
      left: 0,
      torrent_completion: 100,
      state: 'Stopped',
      is_idling: false,
      idling_reason: null,
      stop_condition_met: false,
      post_stop_action: null,
      effective_stop_at_ratio: null,
      session_id: 'keep-me',
    },
    ...overrides,
  };
}

function summary(overrides = {}) {
  return {
    id: 'inst-1',
    state: 'stopped',
    source: 'manual',
    uploaded: 100,
    downloaded: 50,
    ratio: 2,
    currentUploadRate: 10,
    currentDownloadRate: 5,
    seeders: 3,
    leechers: 1,
    left: 0,
    torrentCompletion: 100,
    ...overrides,
  };
}

test('mergeSummary keeps the same reference when nothing changed', () => {
  const instance = baseInstance();
  assert.equal(mergeSummary(instance, summary()), instance);
});

test('mergeSummary returns the same reference for a missing summary', () => {
  const instance = baseInstance();
  assert.equal(mergeSummary(instance, null), instance);
});

test('mergeSummary returns a new object when stats change', () => {
  const instance = baseInstance();
  const next = mergeSummary(instance, summary({ ratio: 3, uploaded: 200 }));

  assert.notEqual(next, instance);
  assert.equal(next.stats.ratio, 3);
  assert.equal(next.stats.uploaded, 200);
  // session-only fields survive the merge
  assert.equal(next.stats.session_id, 'keep-me');
});

test('mergeSummary maps the paused state to a paused status', () => {
  const next = mergeSummary(baseInstance({ isRunning: true }), summary({ state: 'paused' }));

  assert.equal(next.isPaused, true);
  assert.equal(next.isRunning, true);
  assert.equal(next.statusMessage, 'Paused');
  assert.equal(next.statusType, 'paused');
  assert.equal(next.statusIcon, 'pause');
});

test('mergeSummary prefers tracker issues over the state status', () => {
  const next = mergeSummary(
    baseInstance(),
    summary({ state: 'running', tracker_error: 'Tracker unavailable' })
  );

  assert.equal(next.statusType, 'warning');
  assert.equal(next.statusMessage, 'Tracker unavailable');
});

test('mergeSummary propagates the instance source', () => {
  const next = mergeSummary(baseInstance(), summary({ source: 'watch_folder' }));

  assert.equal(next.source, 'watch_folder');
});

test('mergeSummary reports stopped instances as idle', () => {
  const next = mergeSummary(
    baseInstance({
      isRunning: true,
      statusMessage: 'Actively faking ratio...',
      statusType: 'running',
      statusIcon: 'rocket',
    }),
    summary({ state: 'stopped' })
  );

  assert.equal(next.isRunning, false);
  assert.equal(next.isPaused, false);
  assert.equal(next.statusMessage, 'Ready to start faking');
  assert.equal(next.statusIcon, null);
});

test('mergeSummary maps the summary state into stats', () => {
  const next = mergeSummary(baseInstance(), summary({ state: 'running', uploaded: 200 }));

  assert.equal(next.stats.state, 'Running');
  assert.equal(next.stats.uploaded, 200);
});

test('mergeSummary treats idling instances as running', () => {
  const next = mergeSummary(
    baseInstance(),
    summary({ state: 'idle', isIdling: true, idlingReason: 'no_leechers' })
  );

  assert.equal(next.isRunning, true);
  assert.equal(next.stats.state, 'Running');
  assert.equal(next.stats.is_idling, true);
  assert.equal(next.stats.idling_reason, 'no_leechers');
});

test('mergeSummary copies stop condition fields', () => {
  const next = mergeSummary(
    baseInstance(),
    summary({
      stopConditionMet: true,
      postStopAction: 'delete_instance',
      effectiveStopAtRatio: 3.5,
    })
  );

  assert.equal(next.stats.stop_condition_met, true);
  assert.equal(next.stats.post_stop_action, 'delete_instance');
  assert.equal(next.stats.effective_stop_at_ratio, 3.5);
});
