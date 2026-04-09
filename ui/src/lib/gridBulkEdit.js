import { normalizePresetSettings } from './customPreset.js';

export const BULK_EDIT_SECTIONS = [
  'client',
  'rates',
  'initial',
  'timing',
  'randomization',
  'progressive',
  'stopConditions',
];

export function isMixed(values) {
  if (values.length <= 1) return false;
  const first = JSON.stringify(values[0]);
  return values.slice(1).some(value => JSON.stringify(value) !== first);
}

function firstValue(values, fallback = null) {
  return values.length > 0 ? values[0] : fallback;
}

function getCommonSection(instances, projector, fallback = null) {
  const values = instances.map(projector);
  return {
    mixed: isMixed(values),
    value: firstValue(values, fallback),
  };
}

function cloneBulkEditState(state) {
  return {
    ...state,
    sections: Object.fromEntries(
      Object.entries(state.sections || {}).map(([key, section]) => [
        key,
        {
          ...section,
          value: { ...(section?.value || {}) },
        },
      ])
    ),
  };
}

export function createBulkEditState(instances = []) {
  const client = getCommonSection(instances, inst => ({
    selectedClient: inst.selectedClient,
    selectedClientVersion: inst.selectedClientVersion,
    port: inst.port,
    vpnPortSync: inst.vpnPortSync ?? false,
  }));
  const rates = getCommonSection(instances, inst => ({
    uploadRate: inst.uploadRate,
    downloadRate: inst.downloadRate,
  }));
  const initial = getCommonSection(instances, inst => ({
    completionPercent: inst.completionPercent,
    initialUploaded: inst.initialUploaded,
  }));
  const timing = getCommonSection(instances, inst => ({
    updateIntervalSeconds: inst.updateIntervalSeconds,
    scrapeInterval: inst.scrapeInterval,
  }));
  const randomization = getCommonSection(instances, inst => ({
    randomizeRates: inst.randomizeRates,
    randomRangePercent: inst.randomRangePercent,
  }));
  const progressive = getCommonSection(instances, inst => ({
    progressiveRatesEnabled: inst.progressiveRatesEnabled,
    targetUploadRate: inst.targetUploadRate,
    targetDownloadRate: inst.targetDownloadRate,
    progressiveDurationHours: inst.progressiveDurationHours,
  }));
  const stopConditions = getCommonSection(instances, inst => ({
    stopAtRatioEnabled: inst.stopAtRatioEnabled,
    stopAtRatio: inst.stopAtRatio,
    randomizeRatio: inst.randomizeRatio,
    randomRatioRangePercent: inst.randomRatioRangePercent,
    effectiveStopAtRatio: inst.effectiveStopAtRatio ?? null,
    stopAtUploadedEnabled: inst.stopAtUploadedEnabled,
    stopAtUploadedGB: inst.stopAtUploadedGB,
    stopAtDownloadedEnabled: inst.stopAtDownloadedEnabled,
    stopAtDownloadedGB: inst.stopAtDownloadedGB,
    stopAtSeedTimeEnabled: inst.stopAtSeedTimeEnabled,
    stopAtSeedTimeHours: inst.stopAtSeedTimeHours,
    idleWhenNoLeechers: inst.idleWhenNoLeechers,
    idleWhenNoSeeders: inst.idleWhenNoSeeders,
    postStopAction: inst.postStopAction,
  }));

  const sections = {
    client: { apply: false, ...client },
    rates: { apply: false, ...rates },
    initial: { apply: false, ...initial },
    timing: { apply: false, ...timing },
    randomization: { apply: false, ...randomization },
    progressive: { apply: false, ...progressive },
    stopConditions: { apply: false, ...stopConditions },
  };

  return {
    selectedPresetId: '',
    selectedCount: instances.length,
    sections,
  };
}

export function applyPresetToBulkState(state, preset) {
  if (!preset?.settings) {
    return state;
  }

  const settings = normalizePresetSettings(preset.settings);
  const next = cloneBulkEditState(state);
  next.selectedPresetId = preset.id || '';

  if (
    settings.selectedClient != null ||
    settings.selectedClientVersion != null ||
    settings.port != null ||
    settings.vpnPortSync != null
  ) {
    next.sections.client.apply = true;
    next.sections.client.value = {
      ...next.sections.client.value,
      ...(settings.selectedClient != null ? { selectedClient: settings.selectedClient } : {}),
      ...(settings.selectedClientVersion != null
        ? { selectedClientVersion: settings.selectedClientVersion }
        : {}),
      ...(settings.port != null ? { port: settings.port } : {}),
      ...(settings.vpnPortSync != null ? { vpnPortSync: settings.vpnPortSync } : {}),
    };
  }

  if (settings.uploadRate != null || settings.downloadRate != null) {
    next.sections.rates.apply = true;
    next.sections.rates.value = {
      ...next.sections.rates.value,
      ...(settings.uploadRate != null ? { uploadRate: settings.uploadRate } : {}),
      ...(settings.downloadRate != null ? { downloadRate: settings.downloadRate } : {}),
    };
  }

  if (settings.completionPercent != null) {
    next.sections.initial.apply = true;
    next.sections.initial.value = {
      ...next.sections.initial.value,
      completionPercent: settings.completionPercent,
    };
  }

  if (settings.updateIntervalSeconds != null || settings.scrapeInterval != null) {
    next.sections.timing.apply = true;
    next.sections.timing.value = {
      ...next.sections.timing.value,
      ...(settings.updateIntervalSeconds != null
        ? { updateIntervalSeconds: settings.updateIntervalSeconds }
        : {}),
      ...(settings.scrapeInterval != null ? { scrapeInterval: settings.scrapeInterval } : {}),
    };
  }

  if (settings.randomizeRates != null || settings.randomRangePercent != null) {
    next.sections.randomization.apply = true;
    next.sections.randomization.value = {
      ...next.sections.randomization.value,
      ...(settings.randomizeRates != null ? { randomizeRates: settings.randomizeRates } : {}),
      ...(settings.randomRangePercent != null
        ? { randomRangePercent: settings.randomRangePercent }
        : {}),
    };
  }

  if (
    settings.progressiveRatesEnabled != null ||
    settings.targetUploadRate != null ||
    settings.targetDownloadRate != null ||
    settings.progressiveDurationHours != null
  ) {
    next.sections.progressive.apply = true;
    next.sections.progressive.value = {
      ...next.sections.progressive.value,
      ...(settings.progressiveRatesEnabled != null
        ? { progressiveRatesEnabled: settings.progressiveRatesEnabled }
        : {}),
      ...(settings.targetUploadRate != null ? { targetUploadRate: settings.targetUploadRate } : {}),
      ...(settings.targetDownloadRate != null
        ? { targetDownloadRate: settings.targetDownloadRate }
        : {}),
      ...(settings.progressiveDurationHours != null
        ? { progressiveDurationHours: settings.progressiveDurationHours }
        : {}),
    };
  }

  if (
    settings.stopAtRatioEnabled != null ||
    settings.stopAtRatio != null ||
    settings.randomizeRatio != null ||
    settings.randomRatioRangePercent != null ||
    settings.stopAtUploadedEnabled != null ||
    settings.stopAtUploadedGB != null ||
    settings.stopAtDownloadedEnabled != null ||
    settings.stopAtDownloadedGB != null ||
    settings.stopAtSeedTimeEnabled != null ||
    settings.stopAtSeedTimeHours != null ||
    settings.idleWhenNoLeechers != null ||
    settings.idleWhenNoSeeders != null ||
    settings.postStopAction != null
  ) {
    next.sections.stopConditions.apply = true;
    next.sections.stopConditions.value = {
      ...next.sections.stopConditions.value,
      ...(settings.stopAtRatioEnabled != null
        ? { stopAtRatioEnabled: settings.stopAtRatioEnabled }
        : {}),
      ...(settings.stopAtRatio != null ? { stopAtRatio: settings.stopAtRatio } : {}),
      ...(settings.randomizeRatio != null ? { randomizeRatio: settings.randomizeRatio } : {}),
      ...(settings.randomRatioRangePercent != null
        ? { randomRatioRangePercent: settings.randomRatioRangePercent }
        : {}),
      ...(settings.stopAtRatioEnabled != null ||
      settings.stopAtRatio != null ||
      settings.randomizeRatio != null ||
      settings.randomRatioRangePercent != null
        ? { effectiveStopAtRatio: null }
        : {}),
      ...(settings.stopAtUploadedEnabled != null
        ? { stopAtUploadedEnabled: settings.stopAtUploadedEnabled }
        : {}),
      ...(settings.stopAtUploadedGB != null ? { stopAtUploadedGB: settings.stopAtUploadedGB } : {}),
      ...(settings.stopAtDownloadedEnabled != null
        ? { stopAtDownloadedEnabled: settings.stopAtDownloadedEnabled }
        : {}),
      ...(settings.stopAtDownloadedGB != null
        ? { stopAtDownloadedGB: settings.stopAtDownloadedGB }
        : {}),
      ...(settings.stopAtSeedTimeEnabled != null
        ? { stopAtSeedTimeEnabled: settings.stopAtSeedTimeEnabled }
        : {}),
      ...(settings.stopAtSeedTimeHours != null
        ? { stopAtSeedTimeHours: settings.stopAtSeedTimeHours }
        : {}),
      ...(settings.idleWhenNoLeechers != null
        ? { idleWhenNoLeechers: settings.idleWhenNoLeechers }
        : {}),
      ...(settings.idleWhenNoSeeders != null
        ? { idleWhenNoSeeders: settings.idleWhenNoSeeders }
        : {}),
      ...(settings.postStopAction != null ? { postStopAction: settings.postStopAction } : {}),
    };
  }

  return next;
}

export function mergeBulkSectionsIntoInstance(instance, sections) {
  const merged = { ...instance };

  if (sections.client.apply) {
    Object.assign(merged, sections.client.value);
  }
  if (sections.rates.apply) {
    Object.assign(merged, sections.rates.value);
  }
  if (sections.initial.apply) {
    Object.assign(merged, sections.initial.value);
  }
  if (sections.timing.apply) {
    Object.assign(merged, sections.timing.value);
  }
  if (sections.randomization.apply) {
    Object.assign(merged, sections.randomization.value);
  }
  if (sections.progressive.apply) {
    Object.assign(merged, sections.progressive.value);
  }
  if (sections.stopConditions.apply) {
    const { effectiveStopAtRatio, ...stopConditionValues } = sections.stopConditions.value;
    const ratioChanged =
      instance.stopAtRatioEnabled !== stopConditionValues.stopAtRatioEnabled ||
      instance.stopAtRatio !== stopConditionValues.stopAtRatio ||
      instance.randomizeRatio !== stopConditionValues.randomizeRatio ||
      instance.randomRatioRangePercent !== stopConditionValues.randomRatioRangePercent;

    Object.assign(merged, stopConditionValues);

    if (ratioChanged) {
      merged.effectiveStopAtRatio = effectiveStopAtRatio ?? null;
    }
  }

  return merged;
}

export function buildBulkUpdateEntries(instances, sections, buildConfig) {
  return instances.map(instance => {
    const merged = mergeBulkSectionsIntoInstance(instance, sections);
    return {
      id: instance.id,
      config: buildConfig(merged),
    };
  });
}
