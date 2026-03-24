export function buildCustomPreset(instance, { name, description = '', id, now = new Date() } = {}) {
  if (!instance) {
    throw new Error('Instance not found.');
  }

  const trimmedName = name?.trim();
  if (!trimmedName) {
    throw new Error('Please enter a preset name.');
  }

  const createdAt =
    typeof now?.toISOString === 'function' ? now.toISOString() : new Date().toISOString();

  return {
    id: id || `custom-${Date.now()}`,
    name: trimmedName,
    description:
      description.trim() || `Custom preset created on ${new Date(createdAt).toLocaleDateString()}`,
    icon: 'star',
    custom: true,
    created_at: createdAt,
    settings: {
      selectedClient: instance.selectedClient,
      selectedClientVersion: instance.selectedClientVersion,
      uploadRate: instance.uploadRate,
      downloadRate: instance.downloadRate,
      port: instance.port,
      vpnPortSync: instance.vpnPortSync ?? false,
      completionPercent: instance.completionPercent,
      randomizeRates: instance.randomizeRates,
      randomRangePercent: instance.randomRangePercent,
      updateIntervalSeconds: instance.updateIntervalSeconds,
      scrapeInterval: instance.scrapeInterval,
      progressiveRatesEnabled: instance.progressiveRatesEnabled,
      targetUploadRate: instance.targetUploadRate,
      targetDownloadRate: instance.targetDownloadRate,
      progressiveDurationHours: instance.progressiveDurationHours,
      stopAtRatioEnabled: instance.stopAtRatioEnabled,
      stopAtRatio: instance.stopAtRatio,
      randomizeRatio: instance.randomizeRatio,
      randomRatioRangePercent: instance.randomRatioRangePercent,
      stopAtUploadedEnabled: instance.stopAtUploadedEnabled,
      stopAtUploadedGB: instance.stopAtUploadedGB,
      stopAtDownloadedEnabled: instance.stopAtDownloadedEnabled,
      stopAtDownloadedGB: instance.stopAtDownloadedGB,
      stopAtSeedTimeEnabled: instance.stopAtSeedTimeEnabled,
      stopAtSeedTimeHours: instance.stopAtSeedTimeHours,
      idleWhenNoLeechers: instance.idleWhenNoLeechers,
      idleWhenNoSeeders: instance.idleWhenNoSeeders,
      postStopAction: instance.postStopAction,
    },
  };
}

export function buildPresetExportData(instance, options) {
  const preset = buildCustomPreset(instance, options);

  return {
    version: 1,
    type: 'rustatio-preset',
    name: preset.name,
    description: preset.description,
    icon: preset.icon,
    createdAt: preset.created_at,
    settings: preset.settings,
  };
}
