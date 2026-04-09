import assert from 'node:assert/strict';
import test from 'node:test';

import {
  getIdlingReasonText,
  getIdlingStatus,
  getStatusFromStats,
  getTrackerIssue,
} from './status.js';
import {} from './status.js';

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

test('isNetworkConfigured defaults to true when status is absent', () => {
  const isNetworkConfigured = status => status?.configured !== false;
  assert.equal(isNetworkConfigured(null), true);
  assert.equal(isNetworkConfigured(undefined), true);
});

test('isNetworkConfigured returns false when backend reports no VPN configured', () => {
  const isNetworkConfigured = status => status?.configured !== false;
  assert.equal(isNetworkConfigured({ configured: false }), false);
});

test('shouldShowNetworkStatus hides unconfigured VPN state', () => {
  const shouldShowNetworkStatus = status => status?.configured !== false;
  assert.equal(shouldShowNetworkStatus({ configured: false }), false);
  assert.equal(shouldShowNetworkStatus({ configured: true }), true);
});

test('getVpnPortSyncEnabled requires both configuration and server enablement', () => {
  const getVpnPortSyncEnabled = status =>
    status?.configured !== false && (status?.vpn_port_sync_enabled ?? true);
  assert.equal(getVpnPortSyncEnabled({ configured: false, vpn_port_sync_enabled: true }), false);
  assert.equal(getVpnPortSyncEnabled({ configured: true, vpn_port_sync_enabled: false }), false);
  assert.equal(getVpnPortSyncEnabled({ configured: true, vpn_port_sync_enabled: true }), true);
});
