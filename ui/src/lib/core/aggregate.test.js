import assert from 'node:assert/strict';
import test from 'node:test';

import { aggregateInstances, aggregateSummaries } from './aggregate.js';

test('aggregateSummaries counts states and live rates only while active', () => {
  const totals = aggregateSummaries([
    {
      state: 'running',
      uploaded: 100,
      downloaded: 40,
      currentUploadRate: 10,
      currentDownloadRate: 4,
    },
    { state: 'idle', uploaded: 50, downloaded: 20, currentUploadRate: 5, currentDownloadRate: 2 },
    {
      state: 'paused',
      uploaded: 10,
      downloaded: 5,
      currentUploadRate: 99,
      currentDownloadRate: 99,
    },
    { state: 'stopped', currentUploadRate: 99, currentDownloadRate: 99 },
  ]);

  assert.equal(totals.total, 4);
  assert.equal(totals.running, 2);
  assert.equal(totals.paused, 1);
  assert.equal(totals.stopped, 1);
  assert.equal(totals.uploaded, 160);
  assert.equal(totals.downloaded, 65);
  assert.equal(totals.uploadRate, 15);
  assert.equal(totals.downloadRate, 6);
  assert.equal(totals.ratio, 160 / 65);
});

test('aggregateSummaries ignores empty entries and guards zero downloads', () => {
  const totals = aggregateSummaries([null, undefined]);

  assert.equal(totals.total, 0);
  assert.equal(totals.ratio, 0);
  assert.equal(totals.running, 0);
});

test('aggregateInstances reads nested stats and paused state', () => {
  const totals = aggregateInstances([
    {
      isRunning: true,
      isPaused: false,
      stats: { uploaded: 200, downloaded: 80, current_upload_rate: 20, current_download_rate: 8 },
    },
    {
      isRunning: true,
      isPaused: true,
      stats: { uploaded: 10, downloaded: 5, current_upload_rate: 50, current_download_rate: 50 },
    },
    { isRunning: false, stats: { uploaded: 1, downloaded: 1 } },
  ]);

  assert.equal(totals.total, 3);
  assert.equal(totals.running, 1);
  assert.equal(totals.paused, 1);
  assert.equal(totals.stopped, 1);
  assert.equal(totals.uploaded, 211);
  assert.equal(totals.downloaded, 86);
  assert.equal(totals.uploadRate, 20);
  assert.equal(totals.downloadRate, 8);
});

test('aggregateInstances tolerates a missing instance', () => {
  assert.deepEqual(aggregateInstances([{}]), {
    total: 1,
    running: 0,
    paused: 0,
    stopped: 1,
    uploaded: 0,
    downloaded: 0,
    uploadRate: 0,
    downloadRate: 0,
    ratio: 0,
  });
});
