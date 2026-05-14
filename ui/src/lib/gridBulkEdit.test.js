import assert from 'node:assert/strict';
import test from 'node:test';

import {
  applyPresetToBulkState,
  buildBulkUpdateEntries,
  createBulkEditState,
  mergeBulkSectionsIntoInstance,
} from './gridBulkEdit.js';

function makeInstance(overrides = {}) {
  return {
    id: overrides.id || 'a',
    selectedClient: 'qbittorrent',
    selectedClientVersion: '5.2.0',
    port: 6881,
    vpnPortSync: false,
    uploadRate: 50,
    downloadRate: 100,
    completionPercent: 100,
    initialUploaded: 0,
    updateIntervalSeconds: 5,
    scrapeInterval: 60,
    randomizeRates: true,
    randomRangePercent: 20,
    progressiveRatesEnabled: false,
    targetUploadRate: 100,
    targetDownloadRate: 200,
    progressiveDurationHours: 1,
    stopAtRatioEnabled: false,
    stopAtRatio: 2,
    randomizeRatio: false,
    randomRatioRangePercent: 10,
    stopAtUploadedEnabled: false,
    stopAtUploadedGB: 10,
    stopAtDownloadedEnabled: false,
    stopAtDownloadedGB: 10,
    stopAtSeedTimeEnabled: false,
    stopAtSeedTimeHours: 24,
    idleWhenNoLeechers: false,
    idleWhenNoSeeders: false,
    postStopAction: 'idle',
    ...overrides,
  };
}

test('createBulkEditState marks mixed sections correctly', () => {
  const state = createBulkEditState([
    makeInstance({ id: 'a', uploadRate: 50 }),
    makeInstance({ id: 'b', uploadRate: 75 }),
  ]);

  assert.equal(state.selectedCount, 2);
  assert.equal(state.sections.rates.mixed, true);
  assert.equal(state.sections.client.mixed, false);
});

test('applyPresetToBulkState enables matching sections', () => {
  const state = createBulkEditState([makeInstance()]);
  const next = applyPresetToBulkState(state, {
    id: 'balanced',
    settings: {
      uploadRate: 80,
      selectedClient: 'transmission',
      stopAtRatioEnabled: true,
      stopAtRatio: 2.5,
    },
  });

  assert.equal(next.selectedPresetId, 'balanced');
  assert.equal(next.sections.rates.apply, true);
  assert.equal(next.sections.client.apply, true);
  assert.equal(next.sections.stopConditions.apply, true);
  assert.equal(next.sections.rates.value.uploadRate, 80);
  assert.equal(next.sections.client.value.selectedClient, 'transmission');
  assert.equal(next.sections.stopConditions.value.stopAtRatio, 2.5);
});

test('applyPresetToBulkState overwrites only preset fields', () => {
  const state = createBulkEditState([
    makeInstance({
      uploadRate: 50,
      downloadRate: 100,
      idleWhenNoLeechers: false,
      idleWhenNoSeeders: true,
      postStopAction: 'stop_seeding',
    }),
  ]);
  const next = applyPresetToBulkState(state, {
    id: 'stealth-like',
    settings: {
      uploadRate: 25,
      idleWhenNoLeechers: true,
    },
  });

  assert.equal(next.sections.rates.apply, true);
  assert.equal(next.sections.stopConditions.apply, true);
  assert.equal(next.sections.rates.value.uploadRate, 25);
  assert.equal(next.sections.rates.value.downloadRate, 100);
  assert.equal(next.sections.stopConditions.value.idleWhenNoLeechers, true);
  assert.equal(next.sections.stopConditions.value.idleWhenNoSeeders, true);
  assert.equal(next.sections.stopConditions.value.postStopAction, 'stop_seeding');
});

test('applyPresetToBulkState supports legacy stopWhenNo aliases', () => {
  const state = createBulkEditState([makeInstance()]);
  const next = applyPresetToBulkState(state, {
    id: 'legacy-preset',
    settings: {
      stopWhenNoLeechers: true,
      stopWhenNoSeeders: true,
    },
  });

  assert.equal(next.sections.stopConditions.apply, true);
  assert.equal(next.sections.stopConditions.value.idleWhenNoLeechers, true);
  assert.equal(next.sections.stopConditions.value.idleWhenNoSeeders, true);
});

test('applyPresetToBulkState does not mutate the original state', () => {
  const state = createBulkEditState([makeInstance({ uploadRate: 50, downloadRate: 100 })]);
  const next = applyPresetToBulkState(state, {
    id: 'fast',
    settings: {
      uploadRate: 80,
    },
  });

  assert.equal(state.sections.rates.apply, false);
  assert.equal(state.sections.rates.value.uploadRate, 50);
  assert.equal(next.sections.rates.apply, true);
  assert.equal(next.sections.rates.value.uploadRate, 80);
});

test('mergeBulkSectionsIntoInstance only applies enabled sections', () => {
  const instance = makeInstance({ uploadRate: 50, downloadRate: 100 });
  const merged = mergeBulkSectionsIntoInstance(instance, {
    client: { apply: false, value: { selectedClient: 'transmission' } },
    rates: { apply: true, value: { uploadRate: 80, downloadRate: 120 } },
    initial: { apply: false, value: {} },
    timing: { apply: false, value: {} },
    randomization: { apply: false, value: {} },
    progressive: { apply: false, value: {} },
    stopConditions: { apply: false, value: {} },
  });

  assert.equal(merged.selectedClient, 'qbittorrent');
  assert.equal(merged.uploadRate, 80);
  assert.equal(merged.downloadRate, 120);
});

test('buildBulkUpdateEntries builds one payload per instance', () => {
  const entries = buildBulkUpdateEntries(
    [makeInstance({ id: 'a' }), makeInstance({ id: 'b' })],
    {
      client: { apply: false, value: {} },
      rates: { apply: true, value: { uploadRate: 70, downloadRate: 90 } },
      initial: { apply: false, value: {} },
      timing: { apply: false, value: {} },
      randomization: { apply: false, value: {} },
      progressive: { apply: false, value: {} },
      stopConditions: { apply: false, value: {} },
    },
    instance => ({ upload_rate: instance.uploadRate, download_rate: instance.downloadRate })
  );

  assert.deepEqual(entries, [
    { id: 'a', config: { upload_rate: 70, download_rate: 90 } },
    { id: 'b', config: { upload_rate: 70, download_rate: 90 } },
  ]);
});
