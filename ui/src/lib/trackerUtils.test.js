import assert from 'node:assert/strict';
import test from 'node:test';

import {
  buildTrackerFilterEntries,
  extractTrackerHost,
  getPrimaryTrackerHost,
  normalizeTrackerHost,
} from './trackerUtils.js';

test('extractTrackerHost normalizes announce urls and host-like values', () => {
  assert.equal(
    extractTrackerHost('https://Tracker.Example.com:443/announce?passkey=123'),
    'tracker.example.com'
  );
  assert.equal(extractTrackerHost('udp://open.stealth.si:80/announce'), 'open.stealth.si');
  assert.equal(extractTrackerHost('tracker.torrent.eu.org/announce'), 'tracker.torrent.eu.org');
  assert.equal(extractTrackerHost(''), '');
});

test('getPrimaryTrackerHost prefers summary field and falls back safely', () => {
  assert.equal(getPrimaryTrackerHost({ primaryTrackerHost: 'A.B.C' }), 'a.b.c');
  assert.equal(getPrimaryTrackerHost({ primary_tracker_host: 'X.Y.Z' }), 'x.y.z');
  assert.equal(
    getPrimaryTrackerHost({ torrent: { announce: 'https://c411.org/announce' } }),
    'c411.org'
  );
});

test('buildTrackerFilterEntries aggregates counts and applies tracker search', () => {
  const entries = buildTrackerFilterEntries(
    [
      { primaryTrackerHost: 'c411.org' },
      { primaryTrackerHost: 'c411.org' },
      { primaryTrackerHost: 'nyaa.tracker.wf' },
      { primaryTrackerHost: '' },
    ],
    { trackerSearch: 'c4' }
  );

  assert.deepEqual(entries, [
    {
      value: 'c411.org',
      label: 'c411.org',
      count: 2,
      initial: 'C',
      iconUrl: 'https://c411.org/favicon.ico',
    },
  ]);
  assert.equal(normalizeTrackerHost('Test.Host.'), 'test.host');
});

test('multi-select tracker filtering can match more than one host', () => {
  const selected = new Set(['c411.org', 'nyaa.tracker.wf']);
  const values = [
    { primaryTrackerHost: 'c411.org' },
    { primaryTrackerHost: 'open.stealth.si' },
    { primaryTrackerHost: 'nyaa.tracker.wf' },
  ]
    .filter(instance => selected.has(getPrimaryTrackerHost(instance)))
    .map(instance => getPrimaryTrackerHost(instance));

  assert.deepEqual(values, ['c411.org', 'nyaa.tracker.wf']);
});
