import assert from 'node:assert/strict';
import test from 'node:test';

import {
  getIdlingReasonText,
  getIdlingStatus,
  getStatusFromStats,
  getTrackerIssue,
} from './status.js';

test('getIdlingReasonText handles snake_case and camelCase reasons', () => {
  assert.equal(getIdlingReasonText('no_leechers'), 'No leechers available');
  assert.equal(getIdlingReasonText('noLeechers'), 'No leechers available');
  assert.equal(getIdlingReasonText('no_seeders'), 'No seeders available');
  assert.equal(getIdlingReasonText('stop_condition_met'), 'Stop condition met');
});

test('getIdlingStatus avoids misleading fallback message', () => {
  assert.equal(getIdlingStatus(undefined).statusMessage, 'Idling');
  assert.equal(getIdlingStatus('unknown_reason').statusMessage, 'Idling');
});

test('getStatusFromStats uses idling reason when available', () => {
  assert.deepEqual(getStatusFromStats({ is_idling: true, idling_reason: 'noLeechers' }), {
    statusMessage: 'Idling - No leechers available',
    statusType: 'idling',
    statusIcon: 'moon',
  });
});

test('getTrackerIssue supports tracker errors from stats and summaries', () => {
  assert.deepEqual(getTrackerIssue({ tracker_error: 'Tracker unavailable' }), {
    statusMessage: 'Tracker unavailable',
    statusType: 'warning',
    statusIcon: null,
    issueLabel: 'Tracker issue',
  });

  assert.deepEqual(getTrackerIssue({ trackerError: 'Torrent not found on tracker' }), {
    statusMessage: 'Torrent not found on tracker',
    statusType: 'warning',
    statusIcon: null,
    issueLabel: 'Tracker issue',
  });
});
