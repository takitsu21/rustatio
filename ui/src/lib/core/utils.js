import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs) {
  return twMerge(clsx(inputs));
}

/**
 * Detects the user's operating system
 * @returns {'windows' | 'macos' | 'linux' | 'unknown'}
 */
export function detectOS() {
  if (typeof window === 'undefined') return 'unknown';

  const userAgent = window.navigator.userAgent.toLowerCase();
  const platform = window.navigator.platform?.toLowerCase() || '';

  if (userAgent.indexOf('win') !== -1 || platform.indexOf('win') !== -1) {
    return 'windows';
  }

  if (userAgent.indexOf('mac') !== -1 || platform.indexOf('mac') !== -1) {
    return 'macos';
  }

  if (userAgent.indexOf('linux') !== -1 || platform.indexOf('linux') !== -1) {
    return 'linux';
  }

  return 'unknown';
}

/**
 * Gets the appropriate download file extension based on OS
 * @param {string} os - The operating system
 * @returns {string} File extension or descriptor
 */
export function getDownloadType(os) {
  switch (os) {
    case 'windows':
      return 'msi';
    case 'macos':
      return 'dmg';
    case 'linux':
      // For Linux, we'll detect the specific distro if possible
      return detectLinuxDistro();
    default:
      return 'AppImage'; // Fallback for Linux
  }
}

export function selectActiveInstanceId(restoredInstances, savedSession = null) {
  const savedActiveId = savedSession?.activeInstanceId;
  if (
    savedActiveId !== null &&
    savedActiveId !== undefined &&
    restoredInstances.some(inst => String(inst.id) === String(savedActiveId))
  ) {
    return restoredInstances.find(inst => String(inst.id) === String(savedActiveId))?.id ?? null;
  }

  if (
    savedSession?.activeInstanceIndex !== null &&
    savedSession?.activeInstanceIndex !== undefined &&
    savedSession.activeInstanceIndex >= 0 &&
    savedSession.activeInstanceIndex < restoredInstances.length
  ) {
    return restoredInstances[savedSession.activeInstanceIndex].id;
  }

  return restoredInstances[0]?.id ?? null;
}

export function getBackendInstanceStateFlags(state) {
  const normalized = String(state || '').toLowerCase();

  return {
    isRunning:
      normalized === 'running' ||
      normalized === 'starting' ||
      normalized === 'paused' ||
      normalized === 'idle',
    isPaused: normalized === 'paused',
    isIdling: normalized === 'idle',
  };
}

export function serializeSessionInstances(instances) {
  return instances.map(inst => ({
    torrent_path: inst.torrentPath || null,
    torrent_name: inst.torrent?.name || null,
    torrent_data: inst.torrent || null,
    selected_client: inst.selectedClient,
    selected_client_version: inst.selectedClientVersion,
    upload_rate: parseFloat(inst.uploadRate),
    download_rate: parseFloat(inst.downloadRate),
    port: parseInt(inst.port),
    vpn_port_sync: !!inst.vpnPortSync,
    completion_percent: parseFloat(inst.completionPercent),
    initial_uploaded: parseInt(inst.initialUploaded) * 1024 * 1024,
    initial_downloaded: parseInt(inst.initialDownloaded) * 1024 * 1024,
    cumulative_uploaded: parseInt(inst.cumulativeUploaded) * 1024 * 1024,
    cumulative_downloaded: parseInt(inst.cumulativeDownloaded) * 1024 * 1024,
    randomize_rates: inst.randomizeRates,
    random_range_percent: parseFloat(inst.randomRangePercent),
    update_interval_seconds: parseInt(inst.updateIntervalSeconds),
    scrape_interval: parseInt(inst.scrapeInterval) || 60,
    stop_at_ratio_enabled: inst.stopAtRatioEnabled,
    stop_at_ratio: parseFloat(inst.stopAtRatio),
    randomize_ratio: inst.randomizeRatio,
    random_ratio_range_percent: parseFloat(inst.randomRatioRangePercent),
    effective_stop_at_ratio: inst.effectiveStopAtRatio,
    stop_at_uploaded_enabled: inst.stopAtUploadedEnabled,
    stop_at_uploaded_gb: parseFloat(inst.stopAtUploadedGB),
    stop_at_downloaded_enabled: inst.stopAtDownloadedEnabled,
    stop_at_downloaded_gb: parseFloat(inst.stopAtDownloadedGB),
    stop_at_seed_time_enabled: inst.stopAtSeedTimeEnabled,
    stop_at_seed_time_hours: parseFloat(inst.stopAtSeedTimeHours),
    idle_when_no_leechers: inst.idleWhenNoLeechers,
    idle_when_no_seeders: inst.idleWhenNoSeeders,
    post_stop_action: inst.postStopAction,
    progressive_rates_enabled: inst.progressiveRatesEnabled,
    target_upload_rate: parseFloat(inst.targetUploadRate),
    target_download_rate: parseFloat(inst.targetDownloadRate),
    progressive_duration_hours: parseFloat(inst.progressiveDurationHours),
  }));
}

export function getActiveInstanceIndex(instances, activeId) {
  const index = instances.findIndex(inst => String(inst.id) === String(activeId));
  return index >= 0 ? index : null;
}

export function shouldRetryDesktopRestore(savedSession) {
  if (!savedSession) {
    return false;
  }

  return (
    Array.isArray(savedSession.instances) &&
    savedSession.instances.some(
      inst => Boolean(inst?.torrentPath) || Boolean(inst?.torrentName) || Boolean(inst?.torrent)
    )
  );
}

/**
 * Detects Linux distribution for appropriate package type
 * @returns {string} Package type (deb, rpm, or AppImage)
 */
function detectLinuxDistro() {
  if (typeof window === 'undefined') return 'deb';

  const userAgent = window.navigator.userAgent.toLowerCase();
  const platform = window.navigator.platform?.toLowerCase() || '';

  // Check user agent and platform for distro hints
  const fullInfo = userAgent + ' ' + platform;

  // Check for Debian/Ubuntu-based distributions
  if (
    fullInfo.includes('ubuntu') ||
    fullInfo.includes('debian') ||
    fullInfo.includes('mint') ||
    fullInfo.includes('pop') ||
    fullInfo.includes('elementary')
  ) {
    return 'deb';
  }

  // Check for Fedora/RHEL/CentOS/RPM-based distributions
  if (
    fullInfo.includes('fedora') ||
    fullInfo.includes('rhel') ||
    fullInfo.includes('centos') ||
    fullInfo.includes('red hat') ||
    fullInfo.includes('suse') ||
    fullInfo.includes('opensuse')
  ) {
    return 'rpm';
  }

  // For most Linux systems, .deb is more common than .rpm
  // Default to .deb instead of AppImage as it's more user-friendly
  return 'deb';
}
