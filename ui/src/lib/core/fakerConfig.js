export function getCalculatedInitialDownloaded(instance) {
  const torrentSize = instance?.torrent?.total_size || 0;
  const completionPercent = parseFloat(instance?.completionPercent ?? 0);
  return Math.floor((completionPercent / 100) * torrentSize);
}

export function buildFakerConfig(instance, clientVersions = {}, opts = {}) {
  const completionPercent = parseFloat(instance.completionPercent ?? 0);
  const initialDownloaded = opts.useCalculatedInitialDownloaded
    ? getCalculatedInitialDownloaded(instance)
    : parseInt(instance.initialDownloaded ?? 0) * 1024 * 1024;

  return {
    upload_rate: parseFloat(instance.uploadRate ?? 50),
    download_rate: parseFloat(instance.downloadRate ?? 100),
    port: parseInt(instance.port ?? 6881),
    vpn_port_sync: opts.isServerMode ? (instance.vpnPortSync ?? false) : false,
    client_type: instance.selectedClient || 'qbittorrent',
    client_version:
      instance.selectedClientVersion ||
      clientVersions[instance.selectedClient || 'qbittorrent']?.[0] ||
      '',
    initial_uploaded: parseInt(instance.initialUploaded ?? 0) * 1024 * 1024,
    initial_downloaded: initialDownloaded,
    completion_percent: completionPercent,
    num_want: 50,
    randomize_rates: instance.randomizeRates ?? true,
    random_range_percent: parseFloat(instance.randomRangePercent ?? 20),
    randomize_ratio: instance.randomizeRatio ?? false,
    random_ratio_range_percent: parseFloat(instance.randomRatioRangePercent ?? 10),
    stop_at_ratio: instance.stopAtRatioEnabled ? parseFloat(instance.stopAtRatio ?? 2.0) : null,
    effective_stop_at_ratio: instance.stopAtRatioEnabled
      ? (instance.effectiveStopAtRatio ?? null)
      : null,
    stop_at_uploaded: instance.stopAtUploadedEnabled
      ? parseFloat(instance.stopAtUploadedGB ?? 10) * 1024 * 1024 * 1024
      : null,
    stop_at_downloaded: instance.stopAtDownloadedEnabled
      ? parseFloat(instance.stopAtDownloadedGB ?? 10) * 1024 * 1024 * 1024
      : null,
    stop_at_seed_time: instance.stopAtSeedTimeEnabled
      ? parseFloat(instance.stopAtSeedTimeHours ?? 24) * 3600
      : null,
    idle_when_no_leechers: instance.idleWhenNoLeechers ?? false,
    idle_when_no_seeders: instance.idleWhenNoSeeders ?? false,
    post_stop_action: instance.postStopAction || 'idle',
    progressive_rates: instance.progressiveRatesEnabled ?? false,
    target_upload_rate: instance.progressiveRatesEnabled
      ? parseFloat(instance.targetUploadRate ?? 100)
      : null,
    target_download_rate: instance.progressiveRatesEnabled
      ? parseFloat(instance.targetDownloadRate ?? 200)
      : null,
    progressive_duration: parseFloat(instance.progressiveDurationHours ?? 1) * 3600,
    scrape_interval: parseInt(instance.scrapeInterval ?? 60),
  };
}
