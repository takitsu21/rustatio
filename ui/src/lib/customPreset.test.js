import assert from 'node:assert/strict';
import test from 'node:test';

import { buildCustomPreset, buildPresetExportData } from './customPreset.js';

function makeInstance() {
  return {
    selectedClient: 'qbittorrent',
    selectedClientVersion: '5.1.4',
    uploadRate: 50,
    downloadRate: 100,
    port: 6881,
    vpnPortSync: true,
    completionPercent: 100,
    randomizeRates: true,
    randomRangePercent: 20,
    updateIntervalSeconds: 5,
    scrapeInterval: 60,
    progressiveRatesEnabled: true,
    targetUploadRate: 80,
    targetDownloadRate: 120,
    progressiveDurationHours: 3,
    stopAtRatioEnabled: true,
    stopAtRatio: 2.5,
    randomizeRatio: false,
    randomRatioRangePercent: 10,
    stopAtUploadedEnabled: false,
    stopAtUploadedGB: 10,
    stopAtDownloadedEnabled: false,
    stopAtDownloadedGB: 10,
    stopAtSeedTimeEnabled: true,
    stopAtSeedTimeHours: 24,
    idleWhenNoLeechers: true,
    idleWhenNoSeeders: false,
    postStopAction: 'pause',
  };
}

test('buildCustomPreset creates a storable custom preset', () => {
  const now = new Date('2026-03-27T12:00:00.000Z');
  const preset = buildCustomPreset(makeInstance(), {
    id: 'custom-fixed',
    name: '  My preset  ',
    description: '  Great preset  ',
    now,
  });

  assert.equal(preset.id, 'custom-fixed');
  assert.equal(preset.name, 'My preset');
  assert.equal(preset.description, 'Great preset');
  assert.equal(preset.custom, true);
  assert.equal(preset.created_at, '2026-03-27T12:00:00.000Z');
  assert.equal(preset.settings.selectedClient, 'qbittorrent');
  assert.equal(preset.settings.postStopAction, 'pause');
});

test('buildPresetExportData wraps preset data for file export', () => {
  const now = new Date('2026-03-27T12:00:00.000Z');
  const data = buildPresetExportData(makeInstance(), { name: 'Exported', now });

  assert.equal(data.type, 'rustatio-preset');
  assert.equal(data.name, 'Exported');
  assert.equal(data.createdAt, '2026-03-27T12:00:00.000Z');
  assert.equal(data.settings.uploadRate, 50);
});

test('buildCustomPreset rejects missing preset names', () => {
  assert.throws(
    () => buildCustomPreset(makeInstance(), { name: '   ' }),
    /Please enter a preset name/
  );
});
