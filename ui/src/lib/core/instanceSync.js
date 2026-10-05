import { getIdlingStatus, getStatusFromStats, getTrackerIssue } from './status.js';

const STAT_KEYS = [
  'uploaded',
  'downloaded',
  'ratio',
  'current_upload_rate',
  'current_download_rate',
  'seeders',
  'leechers',
  'left',
  'torrent_completion',
];

function sameStats(left, right) {
  const a = left || {};
  const b = right || {};
  return STAT_KEYS.every(key => a[key] === b[key]);
}

/**
 * Merge a backend summary into a frontend instance.
 * Returns the original reference when nothing changed, so poll updates
 * don't invalidate rows that stayed the same.
 */
export function mergeSummary(instance, summary) {
  if (!summary) return instance;

  const state = summary.state?.toLowerCase();
  const isRunning =
    state === 'running' || state === 'starting' || state === 'paused' || state === 'idle';
  const isPaused = state === 'paused';

  const next = { ...instance, isRunning, isPaused };

  const trackerIssue = getTrackerIssue(summary);
  if (trackerIssue) {
    next.statusMessage = trackerIssue.statusMessage;
    next.statusType = trackerIssue.statusType;
    next.statusIcon = trackerIssue.statusIcon;
  } else if (isPaused) {
    next.statusMessage = 'Paused';
    next.statusType = 'paused';
    next.statusIcon = 'pause';
  } else if (state === 'idle') {
    const status = instance.stats?.is_idling
      ? getStatusFromStats(instance.stats)
      : getIdlingStatus();
    next.statusMessage = status.statusMessage;
    next.statusType = status.statusType;
    next.statusIcon = status.statusIcon;
  } else if (isRunning) {
    next.statusMessage = 'Actively faking ratio...';
    next.statusType = 'running';
    next.statusIcon = 'rocket';
  } else {
    next.statusMessage = 'Ready to start faking';
    next.statusType = 'idle';
    next.statusIcon = null;
  }

  if (summary.source === 'watch_folder' || summary.source === 'manual') {
    next.source = summary.source;
  }

  next.stats = {
    ...(instance.stats || {}),
    uploaded: summary.uploaded,
    downloaded: summary.downloaded,
    ratio: summary.ratio,
    current_upload_rate: summary.currentUploadRate,
    current_download_rate: summary.currentDownloadRate,
    seeders: summary.seeders,
    leechers: summary.leechers,
    left: summary.left,
    torrent_completion: summary.torrentCompletion,
  };
  next.completionPercent = summary.torrentCompletion;

  const unchanged =
    instance.isRunning === next.isRunning &&
    instance.isPaused === next.isPaused &&
    instance.statusMessage === next.statusMessage &&
    instance.statusType === next.statusType &&
    (instance.statusIcon ?? null) === (next.statusIcon ?? null) &&
    instance.source === next.source &&
    instance.completionPercent === next.completionPercent &&
    sameStats(instance.stats, next.stats);

  return unchanged ? instance : next;
}
