import assert from 'node:assert/strict';
import test from 'node:test';

import {
  getForwardedPort,
  getNetworkLabel,
  getPeerListenerError,
  getPeerListenerPort,
  isNetworkConfigured,
  maskIp,
} from './network.js';

test('network getters support snake_case and camelCase payloads', () => {
  assert.equal(getForwardedPort({ forwarded_port: 51413 }), 51413);
  assert.equal(getForwardedPort({ forwardedPort: 51413 }), 51413);
  assert.equal(getPeerListenerPort({ peer_listener_port: 6881 }), 6881);
  assert.equal(getPeerListenerPort({ peerListenerPort: 6881 }), 6881);
  assert.equal(getPeerListenerError({ peer_listener_error: 'bind failed' }), 'bind failed');
  assert.equal(getPeerListenerError({ peerListenerError: 'bind failed' }), 'bind failed');
});

test('isNetworkConfigured treats only explicit false as unconfigured', () => {
  assert.equal(isNetworkConfigured({ configured: false }), false);
  assert.equal(isNetworkConfigured({ configured: true }), true);
  assert.equal(isNetworkConfigured({}), true);
  assert.equal(isNetworkConfigured(null), true);
});

test('maskIp hides middle octets and truncates other formats', () => {
  assert.equal(maskIp('149.34.12.215'), '149.***.***.215');
  assert.equal(maskIp('2001:db8::1'), '2001:db8...');
  assert.equal(maskIp(''), '---');
});

test('getNetworkLabel maps VPN states to rail labels', () => {
  assert.deepEqual(getNetworkLabel({ configured: false }), {
    label: 'No VPN configured',
    tone: 'ratio',
  });
  assert.deepEqual(getNetworkLabel({ is_vpn: true, organization: 'Datacamp' }), {
    label: 'Datacamp',
    tone: 'upload',
  });
  assert.deepEqual(getNetworkLabel({ is_vpn: true }), { label: 'VPN active', tone: 'upload' });
  assert.deepEqual(getNetworkLabel({ is_vpn: false }), { label: 'No VPN', tone: 'ratio' });
  assert.equal(getNetworkLabel(null), null);
});
