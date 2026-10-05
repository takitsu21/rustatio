import { getGridLiveRate } from '../grid/gridMetrics.js';

function normalizeState(state) {
  return String(state ?? 'stopped').toLowerCase();
}

function isActiveState(state) {
  const value = normalizeState(state);
  return value === 'running' || value === 'starting' || value === 'idle';
}

/**
 * Aggregate server instance summaries (grid polling shape).
 */
export function aggregateSummaries(list = []) {
  let uploaded = 0;
  let downloaded = 0;
  let uploadRate = 0;
  let downloadRate = 0;
  let running = 0;
  let paused = 0;

  for (const item of list) {
    if (!item) continue;

    if (isActiveState(item.state)) {
      running += 1;
    } else if (normalizeState(item.state) === 'paused') {
      paused += 1;
    }

    uploaded += item.uploaded || 0;
    downloaded += item.downloaded || 0;
    uploadRate += getGridLiveRate(item.state, item.currentUploadRate);
    downloadRate += getGridLiveRate(item.state, item.currentDownloadRate);
  }

  const total = list.filter(Boolean).length;
  return {
    total,
    running,
    paused,
    stopped: Math.max(0, total - running - paused),
    uploaded,
    downloaded,
    uploadRate,
    downloadRate,
    ratio: downloaded > 0 ? uploaded / downloaded : 0,
  };
}

/**
 * Aggregate frontend instances (standard view store shape).
 */
export function aggregateInstances(list = []) {
  let uploaded = 0;
  let downloaded = 0;
  let uploadRate = 0;
  let downloadRate = 0;
  let running = 0;
  let paused = 0;
  let total = 0;

  for (const item of list) {
    if (!item) continue;
    total += 1;

    if (item.isRunning && item.isPaused) {
      paused += 1;
    } else if (item.isRunning) {
      running += 1;
    }

    const stats = item.stats;
    if (stats) {
      uploaded += stats.uploaded || 0;
      downloaded += stats.downloaded || 0;
      if (item.isRunning && !item.isPaused) {
        uploadRate += stats.current_upload_rate || 0;
        downloadRate += stats.current_download_rate || 0;
      }
    }
  }

  return {
    total,
    running,
    paused,
    stopped: Math.max(0, total - running - paused),
    uploaded,
    downloaded,
    uploadRate,
    downloadRate,
    ratio: downloaded > 0 ? uploaded / downloaded : 0,
  };
}
