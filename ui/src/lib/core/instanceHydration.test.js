import assert from 'node:assert/strict';
import test from 'node:test';

import { hydrateInstances } from './instanceHydration.js';

function serverInstance(id, extra = {}) {
  return { id, config: {}, ...extra };
}

test('hydrateInstances reads the backend once for many ids', async () => {
  let reads = 0;
  const applied = [];

  const result = await hydrateInstances(
    [1, 2, 3],
    [],
    async () => {
      reads += 1;
      return [serverInstance(1), serverInstance(2), serverInstance(3)];
    },
    async (id, summary, serverInst) => {
      applied.push([id, summary, serverInst?.id ?? null]);
      return id;
    }
  );

  assert.equal(reads, 1);
  assert.deepEqual(applied, [
    ['1', null, 1],
    ['2', null, 2],
    ['3', null, 3],
  ]);
  assert.deepEqual(result, ['1', '2', '3']);
});

test('hydrateInstances passes matching summaries and server instances through', async () => {
  const summaries = [{ id: 'a', name: 'alpha' }];
  const server = [serverInstance('a', { port: 1234 })];

  const applied = [];
  await hydrateInstances(
    ['a'],
    summaries,
    async () => server,
    async (id, summary, serverInst) => {
      applied.push({ id, summary, serverInst });
      return id;
    }
  );

  assert.equal(applied[0].id, 'a');
  assert.equal(applied[0].summary.name, 'alpha');
  assert.equal(applied[0].serverInst.port, 1234);
});

test('hydrateInstances passes null when the backend read misses an id', async () => {
  const applied = [];
  await hydrateInstances(
    ['missing'],
    [],
    async () => [serverInstance('other')],
    async (id, summary, serverInst) => {
      applied.push({ id, summary, serverInst });
      return id;
    }
  );

  assert.equal(applied[0].summary, null);
  assert.equal(applied[0].serverInst, null);
});

test('hydrateInstances treats a failed backend read as an empty list', async () => {
  const applied = [];
  const result = await hydrateInstances(
    ['a'],
    [],
    async () => {
      throw new Error('backend unavailable');
    },
    async (id, summary, serverInst) => {
      applied.push({ id, summary, serverInst });
      return id;
    }
  );

  assert.equal(applied[0].serverInst, null);
  assert.deepEqual(result, ['a']);
});

test('hydrateInstances skips the backend read for an empty id list', async () => {
  let reads = 0;
  const result = await hydrateInstances(
    [],
    [],
    async () => {
      reads += 1;
      return [];
    },
    async id => id
  );

  assert.equal(reads, 0);
  assert.deepEqual(result, []);
});

test('hydrateInstances keeps only successfully ensured ids', async () => {
  const result = await hydrateInstances(
    ['a', 'b', 'c'],
    [],
    async () => [],
    async id => (id === 'b' ? null : id)
  );

  assert.deepEqual(result, ['a', 'c']);
});
