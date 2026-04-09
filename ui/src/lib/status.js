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

export function getReadyStatus(message = 'Ready to start faking') {
  return {
    statusMessage: message,
    statusType: 'idle',
    statusIcon: null,
  };
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

export function getTrackerInvalidStatus(message = 'Torrent not found on tracker') {
  return {
    statusMessage: message,
    statusType: 'warning',
    statusIcon: null,
  };
}

export function getTrackerIssue(stats) {
  const message = stats?.tracker_error || stats?.trackerError;
  if (!message) {
    return null;
  }

  return {
    statusMessage: message,
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
