export function getIdlingReasonText(reason) {
  if (reason === 'stop_condition_met' || reason === 'stopConditionMet') {
    return 'Stop condition met';
  }

  if (reason === 'no_leechers' || reason === 'noLeechers') {
    return 'No leechers available';
  }

  if (reason === 'no_seeders' || reason === 'noSeeders') {
    return 'No seeders available';
  }

  return null;
}

export function getRunningStatus(message = 'Actively faking ratio...') {
  return {
    statusMessage: message,
    statusType: 'running',
    statusIcon: 'rocket',
  };
}

export function getPausedStatus(message = 'Paused') {
  return {
    statusMessage: message,
    statusType: 'paused',
    statusIcon: 'pause',
  };
}

export function formatRetrySeconds(retryAtMs, nowMs = Date.now()) {
  if (retryAtMs == null) {
    return null;
  }

  const remainingMs = Math.max(0, retryAtMs - nowMs);
  return Math.ceil(remainingMs / 1000);
}

export function getTrackerIssue(stats) {
  const message = stats?.tracker_error || stats?.trackerError;
  if (!message) {
    return null;
  }

  const retryAtMs = stats?.tracker_retry_at_ms ?? stats?.trackerRetryAtMs;
  const retrySecs = formatRetrySeconds(retryAtMs);
  const statusMessage =
    message === 'Tracker unavailable' && retrySecs != null
      ? retrySecs > 0
        ? `Tracker unavailable, retrying in ${retrySecs}s`
        : 'Tracker unavailable, retrying now'
      : message;

  return {
    statusMessage,
    statusType: 'warning',
    statusIcon: null,
    issueLabel: 'Tracker issue',
  };
}

export function getIdlingStatus(reason) {
  const text = getIdlingReasonText(reason);

  return {
    statusMessage: text ? `Idling - ${text}` : 'Idling',
    statusType: 'idling',
    statusIcon: 'moon',
  };
}

export function getStatusFromStats(stats) {
  const trackerIssue = getTrackerIssue(stats);
  if (trackerIssue) {
    return trackerIssue;
  }

  if (stats?.is_idling) {
    return getIdlingStatus(stats.idling_reason);
  }

  return getRunningStatus();
}

/**
 * Canonical presentation state for an instance object (frontend store shape).
 */
export function getInstanceState(instance) {
  if (!instance) return 'stopped';
  if (instance.isRunning) {
    if (instance.isPaused) return 'paused';
    if (instance.stats?.is_idling) return 'idle';
    return 'running';
  }
  return 'stopped';
}
