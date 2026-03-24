import assert from 'node:assert/strict';
import test from 'node:test';

import { applyBaseGridFilters } from './gridFilters.js';

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
      tagFilter: '',
    }).length,
    1
  );

  assert.equal(
    applyBaseGridFilters(instances, {
      search: '',
      stateFilter: 'stopped',
      tagFilter: '',
    })[0].name,
    'Beta'
  );

  assert.equal(
    applyBaseGridFilters(instances, {
      search: '',
      stateFilter: 'all',
      tagFilter: 'movie',
    })[0].name,
    'Alpha'
  );
});
