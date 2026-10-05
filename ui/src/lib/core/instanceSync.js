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
  'state',
  'is_idling',
  'idling_reason',
  'stop_condition_met',
  'post_stop_action',
  'effective_stop_at_ratio',
];

const SUMMARY_STATES = {
  running: 'Running',
  starting: 'Starting',
  stopping: 'Stopping',
  paused: 'Paused',
  stopped: 'Stopped',
  idle: 'Idle',
};

// Map a summary state back to the backend FakerState casing used in stats.
function getSummaryState(summary) {
  if (summary.isIdling) return 'Running';
  return SUMMARY_STATES[summary.state?.toLowerCase()] ?? summary.state;
}

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
    const status =
      (summary.isIdling ?? instance.stats?.is_idling)
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
    state: getSummaryState(summary),
    is_idling: summary.isIdling ?? false,
    idling_reason: summary.idlingReason ?? null,
    stop_condition_met: summary.stopConditionMet ?? false,
    post_stop_action: summary.postStopAction ?? null,
    effective_stop_at_ratio: summary.effectiveStopAtRatio ?? null,
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
