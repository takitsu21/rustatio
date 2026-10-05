import assert from 'node:assert/strict';
import test from 'node:test';

import {
  UNTAGGED_FILTER_VALUE,
  applyAllGridFilters,
  applyBaseGridFilters,
  buildStateFilterEntries,
  buildTagFilterEntries,
} from './gridFilters.js';

test('applyBaseGridFilters keeps search, state, and tag behavior', () => {
  const instances = [
    {
      name: 'Alpha',
      infoHash: 'abc123',
      tags: ['movie'],
      state: 'running',
    },
    {
      name: 'Beta',
      infoHash: 'def456',
      tags: ['tv'],
      state: 'stopped',
    },
  ];

  assert.equal(
    applyBaseGridFilters(instances, {
      search: 'alp',
      stateFilter: 'all',
      tagFilter: [],
    }).length,
    1
  );

  assert.equal(
    applyBaseGridFilters(instances, {
      search: '',
      stateFilter: 'stopped',
      tagFilter: [],
    })[0].name,
    'Beta'
  );

  assert.equal(
    applyBaseGridFilters(instances, {
      search: '',
      stateFilter: 'all',
      tagFilter: ['movie'],
    })[0].name,
    'Alpha'
  );
});

test('applyAllGridFilters supports multi-select tags and trackers', () => {
  const instances = [
    {
      name: 'Alpha',
      infoHash: 'abc123',
      tags: ['movie'],
      state: 'running',
      primaryTrackerHost: 'c411.org',
    },
    {
      name: 'Beta',
      infoHash: 'def456',
      tags: ['tv'],
      state: 'stopped',
      primaryTrackerHost: 'nyaa.tracker.wf',
    },
    {
      name: 'Gamma',
      infoHash: 'ghi789',
      tags: [],
      state: 'running',
      primaryTrackerHost: 'open.stealth.si',
    },
  ];

  const result = applyAllGridFilters(instances, {
    search: '',
    stateFilter: 'all',
    tagFilter: ['movie', 'tv'],
    trackerFilter: ['c411.org', 'nyaa.tracker.wf'],
  });

  assert.deepEqual(
    result.map(instance => instance.name),
    ['Alpha', 'Beta']
  );
});

test('applyBaseGridFilters matches untagged torrents when selected', () => {
  const result = applyBaseGridFilters(
    [
      { name: 'Alpha', infoHash: 'a', tags: ['movie'], state: 'running' },
      { name: 'Beta', infoHash: 'b', tags: [], state: 'stopped' },
    ],
    {
      search: '',
      stateFilter: 'all',
      tagFilter: [UNTAGGED_FILTER_VALUE],
    }
  );

  assert.deepEqual(
    result.map(instance => instance.name),
    ['Beta']
  );
});

test('buildStateFilterEntries returns ordered state counts', () => {
  assert.deepEqual(
    buildStateFilterEntries([
      { state: 'stopped' },
      { state: 'running' },
      { state: 'running' },
      { state: 'idle' },
    ]),
    [
      { value: 'running', count: 2 },
      { value: 'idle', count: 1 },
      { value: 'stopped', count: 1 },
    ]
  );
});

test('buildTagFilterEntries aggregates and searches tags', () => {
  assert.deepEqual(
    buildTagFilterEntries(
      [{ tags: ['movie', 'anime'] }, { tags: [] }, { tags: ['movie'] }, { tags: ['tv'] }],
      { tagSearch: 'mo' }
    ),
    [{ value: 'movie', label: 'movie', count: 2 }]
  );
});

test('buildTagFilterEntries includes untagged row', () => {
  assert.deepEqual(buildTagFilterEntries([{ tags: [] }, { tags: ['movie'] }, { tags: [] }]), [
    {
      value: UNTAGGED_FILTER_VALUE,
      label: 'Untagged',
      count: 2,
      muted: true,
    },
    {
      value: 'movie',
      label: 'movie',
      count: 1,
    },
  ]);
});
