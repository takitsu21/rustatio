import { writable, get } from 'svelte/store';
import { api } from '$lib/api';
import { getDefaultPreset } from '$lib/defaultPreset.js';

// Check if running in Tauri
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// Helper to convert bytes to MB (rounded to integer)
const bytesToMB = bytes => Math.round((bytes || 0) / (1024 * 1024));

// Compute the effective (randomized) stop ratio
export function computeEffectiveRatio(stopAtRatio, randomizeRatio, randomRatioRangePercent) {
  if (!randomizeRatio) {
    return null;
  }
  const base = parseFloat(stopAtRatio) || 2.0;
  const range = (parseFloat(randomRatioRangePercent) || 10) / 100;
  const variation = 1 + (Math.random() * 2 - 1) * range;
  return parseFloat((base * variation).toFixed(4));
}

// Create default instance state
function createDefaultInstance(id, defaults = {}) {
  return {
    id,

    // Torrent state
    torrent: null,
    torrentPath: '',
    stats: null,
    isRunning: false,
    isPaused: false,

    // Instance source (manual or watch_folder)
    source: defaults.source || 'manual',

    // Intervals
    updateInterval: null,

    // Cumulative stats (preserved across sessions, not editable by user)
    cumulativeUploaded: defaults.cumulativeUploaded !== undefined ? defaults.cumulativeUploaded : 0,
    cumulativeDownloaded:
      defaults.cumulativeDownloaded !== undefined ? defaults.cumulativeDownloaded : 0,

    // Form values
    selectedClient: defaults.selectedClient || 'qbittorrent',
    selectedClientVersion: defaults.selectedClientVersion || null,
    uploadRate: defaults.uploadRate !== undefined ? defaults.uploadRate : 50,
    downloadRate: defaults.downloadRate !== undefined ? defaults.downloadRate : 100,
    port: defaults.port !== undefined ? defaults.port : 6881,
    vpnPortSync: defaults.vpnPortSync !== undefined ? defaults.vpnPortSync : false,
    completionPercent: defaults.completionPercent !== undefined ? defaults.completionPercent : 0,
    initialUploaded: defaults.initialUploaded !== undefined ? defaults.initialUploaded : 0,
    initialDownloaded: defaults.initialDownloaded !== undefined ? defaults.initialDownloaded : 0,
    randomizeRates: defaults.randomizeRates !== undefined ? defaults.randomizeRates : true,
    randomRangePercent:
      defaults.randomRangePercent !== undefined ? defaults.randomRangePercent : 20,
    updateIntervalSeconds:
      defaults.updateIntervalSeconds !== undefined ? defaults.updateIntervalSeconds : 5,

    // Stop conditions
    stopAtRatioEnabled:
      defaults.stopAtRatioEnabled !== undefined ? defaults.stopAtRatioEnabled : false,
    stopAtRatio: defaults.stopAtRatio !== undefined ? defaults.stopAtRatio : 2.0,
    randomizeRatio: defaults.randomizeRatio !== undefined ? defaults.randomizeRatio : false,
    randomRatioRangePercent:
      defaults.randomRatioRangePercent !== undefined ? defaults.randomRatioRangePercent : 10,
    effectiveStopAtRatio:
      defaults.effectiveStopAtRatio !== undefined
        ? defaults.effectiveStopAtRatio
        : defaults.randomizeRatio && defaults.stopAtRatioEnabled
          ? computeEffectiveRatio(
              defaults.stopAtRatio ?? 2.0,
              true,
              defaults.randomRatioRangePercent ?? 10
            )
          : null,
    stopAtUploadedEnabled:
      defaults.stopAtUploadedEnabled !== undefined ? defaults.stopAtUploadedEnabled : false,
    stopAtUploadedGB: defaults.stopAtUploadedGB !== undefined ? defaults.stopAtUploadedGB : 10,
    stopAtDownloadedEnabled:
      defaults.stopAtDownloadedEnabled !== undefined ? defaults.stopAtDownloadedEnabled : false,
    stopAtDownloadedGB:
      defaults.stopAtDownloadedGB !== undefined ? defaults.stopAtDownloadedGB : 10,
    stopAtSeedTimeEnabled:
      defaults.stopAtSeedTimeEnabled !== undefined ? defaults.stopAtSeedTimeEnabled : false,
    stopAtSeedTimeHours:
      defaults.stopAtSeedTimeHours !== undefined ? defaults.stopAtSeedTimeHours : 24,
    idleWhenNoLeechers:
      defaults.idleWhenNoLeechers !== undefined ? defaults.idleWhenNoLeechers : false,
    idleWhenNoSeeders:
      defaults.idleWhenNoSeeders !== undefined ? defaults.idleWhenNoSeeders : false,

    // Scrape interval
    scrapeInterval: defaults.scrapeInterval !== undefined ? defaults.scrapeInterval : 60,

    // Progressive rates
    progressiveRatesEnabled:
      defaults.progressiveRatesEnabled !== undefined ? defaults.progressiveRatesEnabled : false,
    targetUploadRate: defaults.targetUploadRate !== undefined ? defaults.targetUploadRate : 100,
    targetDownloadRate:
      defaults.targetDownloadRate !== undefined ? defaults.targetDownloadRate : 200,
    progressiveDurationHours:
      defaults.progressiveDurationHours !== undefined ? defaults.progressiveDurationHours : 1,

    // Status
    statusMessage: 'Select a torrent file to begin',
    statusType: 'warning',
  };
}

async function getServerEffectiveDefaults() {
  try {
    return (await api.getDefaultConfig()) || {};
  } catch {
    return {};
  }
}

async function buildNewInstanceDefaults(defaults = {}) {
  const presetDefaults = getDefaultPreset()?.settings || {};
  const serverDefaults = !isTauri ? await getServerEffectiveDefaults() : {};

  return {
    ...serverDefaults,
    ...presetDefaults,
    ...defaults,
    vpnPortSync:
      defaults.vpnPortSync !== undefined
        ? defaults.vpnPortSync
        : presetDefaults.vpnPortSync !== undefined
          ? presetDefaults.vpnPortSync
          : serverDefaults.vpnPortSync !== undefined
            ? serverDefaults.vpnPortSync
            : false,
  };
}

// Global lock to prevent concurrent config saves from different sources
export let isConfigSaving = false;

// Prevent overlapping backend reconciliation runs (avoids duplicate inserts)
let isReconcilingBackend = false;

// Save session (localStorage for web, Tauri config for desktop)
async function saveSession(instances, activeId) {
  // Prevent concurrent saves
  if (isConfigSaving) return;

  try {
    isConfigSaving = true;

    if (isTauri) {
      // Desktop: Save to Tauri config file
      const config = get(globalConfig);
      if (!config) return;

      config.instances = instances.map(inst => ({
        torrent_path: inst.torrentPath || null,
        torrent_name: inst.torrent?.name || null,
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
        progressive_rates_enabled: inst.progressiveRatesEnabled,
        target_upload_rate: parseFloat(inst.targetUploadRate),
        target_download_rate: parseFloat(inst.targetDownloadRate),
        progressive_duration_hours: parseFloat(inst.progressiveDurationHours),
      }));

      config.active_instance_id = instances.findIndex(inst => inst.id === activeId);
      await api.updateConfig(config);
    } else {
      // Web: Save to localStorage
      const sessionData = {
        instances: instances.map(inst => ({
          torrent_path: inst.torrentPath || null,
          torrent_name: inst.torrent?.name || null,
          torrent_data: inst.torrent || null, // Save the actual torrent object for web
          selected_client: inst.selectedClient,
          selected_client_version: inst.selectedClientVersion,
          upload_rate: parseFloat(inst.uploadRate),
          download_rate: parseFloat(inst.downloadRate),
          port: parseInt(inst.port),
          vpn_port_sync: !!inst.vpnPortSync,
          completion_percent: parseFloat(inst.completionPercent),
          initial_uploaded: parseInt(inst.initialUploaded) * 1024 * 1024, // Convert MB to bytes
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
          progressive_rates_enabled: inst.progressiveRatesEnabled,
          target_upload_rate: parseFloat(inst.targetUploadRate),
          target_download_rate: parseFloat(inst.targetDownloadRate),
          progressive_duration_hours: parseFloat(inst.progressiveDurationHours),
        })),
        active_instance_id: instances.findIndex(inst => inst.id === activeId),
      };

      // Save to localStorage
      localStorage.setItem('rustatio-session', JSON.stringify(sessionData));
    }
  } catch (error) {
    console.error('Failed to save session:', error);
  } finally {
    isConfigSaving = false;
  }
}

// Load session from storage (localStorage for web, Tauri config for desktop)
function loadSessionFromStorage(config = null) {
  try {
    let sessionData;

    if (isTauri && config) {
      // Desktop: Load from Tauri config
      sessionData = config;
    } else {
      // Web: Load from localStorage
      const stored = localStorage.getItem('rustatio-session');
      if (!stored) return null;
      sessionData = JSON.parse(stored);
    }

    if (!sessionData || !sessionData.instances || sessionData.instances.length === 0) {
      return null;
    }

    return {
      instances: sessionData.instances.map(inst => ({
        torrentPath: inst.torrent_path,
        torrentName: inst.torrent_name || null,
        torrent: inst.torrent_data || null, // Restore torrent data for web
        selectedClient: inst.selected_client,
        selectedClientVersion: inst.selected_client_version,
        uploadRate: inst.upload_rate,
        downloadRate: inst.download_rate,
        port: inst.port,
        vpnPortSync: inst.vpn_port_sync || false,
        completionPercent: inst.completion_percent,
        initialUploaded: bytesToMB(inst.initial_uploaded),
        initialDownloaded: bytesToMB(inst.initial_downloaded),
        cumulativeUploaded: bytesToMB(inst.cumulative_uploaded),
        cumulativeDownloaded: bytesToMB(inst.cumulative_downloaded),
        randomizeRates: inst.randomize_rates,
        randomRangePercent: inst.random_range_percent,
        updateIntervalSeconds: inst.update_interval_seconds,
        scrapeInterval: inst.scrape_interval ?? 60,
        stopAtRatioEnabled: inst.stop_at_ratio_enabled,
        stopAtRatio: inst.stop_at_ratio,
        randomizeRatio: inst.randomize_ratio || false,
        randomRatioRangePercent: inst.random_ratio_range_percent ?? 10,
        effectiveStopAtRatio: inst.effective_stop_at_ratio ?? null,
        stopAtUploadedEnabled: inst.stop_at_uploaded_enabled,
        stopAtUploadedGB: inst.stop_at_uploaded_gb,
        stopAtDownloadedEnabled: inst.stop_at_downloaded_enabled,
        stopAtDownloadedGB: inst.stop_at_downloaded_gb,
        stopAtSeedTimeEnabled: inst.stop_at_seed_time_enabled,
        stopAtSeedTimeHours: inst.stop_at_seed_time_hours,
        idleWhenNoLeechers: inst.idle_when_no_leechers || false,
        idleWhenNoSeeders: inst.idle_when_no_seeders || false,
        progressiveRatesEnabled: inst.progressive_rates_enabled,
        targetUploadRate: inst.target_upload_rate,
        targetDownloadRate: inst.target_download_rate,
        progressiveDurationHours: inst.progressive_duration_hours,
      })),
      activeInstanceIndex: sessionData.active_instance_id,
    };
  } catch (error) {
    console.error('Failed to load session from storage:', error);
    return null;
  }
}

// Store for all instances
export const instances = writable([]);

// Store for active instance ID
export const activeInstanceId = writable(null);

// Store for global config (set from App.svelte)
export const globalConfig = writable(null);

// Writable store for active instance (manually updated to avoid orphan effect in Svelte 5)
export const activeInstance = writable(null);

// Export saveSession for use in App.svelte
export { saveSession };

// Helper function to update activeInstance store
// Only fires set() if the active instance reference actually changed,
// preventing unnecessary subscription notifications.
function updateActiveInstanceStore() {
  const $instances = get(instances);
  const $activeInstanceId = get(activeInstanceId);
  const active = $instances.find(inst => inst.id === $activeInstanceId) || $instances[0] || null;
  if (active !== get(activeInstance)) {
    activeInstance.set(active);
  }
}

function buildInstanceDefaultsFromServer(serverInst) {
  const config = serverInst.config || serverInst;
  return {
    source: serverInst.source || 'manual',
    selectedClient: config.client_type,
    selectedClientVersion: config.client_version,
    uploadRate: config.upload_rate,
    downloadRate: config.download_rate,
    port: config.port,
    vpnPortSync: config.vpn_port_sync || false,
    completionPercent: config.completion_percent,
    initialUploaded: bytesToMB(config.initial_uploaded),
    initialDownloaded: bytesToMB(config.initial_downloaded),
    cumulativeUploaded: bytesToMB(serverInst.stats.uploaded),
    cumulativeDownloaded: bytesToMB(serverInst.stats.downloaded),
    randomizeRates: config.randomize_rates,
    randomRangePercent: config.random_range_percent,
    stopAtRatioEnabled: config.stop_at_ratio !== null,
    stopAtRatio: config.stop_at_ratio || 2.0,
    randomizeRatio: config.randomize_ratio || false,
    randomRatioRangePercent: config.random_ratio_range_percent ?? 10,
    effectiveStopAtRatio: null,
    stopAtUploadedEnabled: config.stop_at_uploaded !== null,
    stopAtUploadedGB: (config.stop_at_uploaded || 0) / (1024 * 1024 * 1024),
    stopAtDownloadedEnabled: config.stop_at_downloaded !== null,
    stopAtDownloadedGB: (config.stop_at_downloaded || 0) / (1024 * 1024 * 1024),
    stopAtSeedTimeEnabled: config.stop_at_seed_time !== null,
    stopAtSeedTimeHours: (config.stop_at_seed_time || 0) / 3600,
    idleWhenNoLeechers: config.idle_when_no_leechers || false,
    idleWhenNoSeeders: config.idle_when_no_seeders || false,
    progressiveRatesEnabled: config.progressive_rates || false,
    targetUploadRate: config.target_upload_rate || 100,
    targetDownloadRate: config.target_download_rate || 200,
    progressiveDurationHours: (config.progressive_duration || 3600) / 3600,
    scrapeInterval: config.scrape_interval || 60,
  };
}

// Actions
export const instanceActions = {
  // Initialize - create first instance or restore from storage/server
  initialize: async () => {
    try {
      // Load config if in Tauri mode
      let config = null;
      if (isTauri) {
        config = await api.getConfig();
        globalConfig.set(config);
      }

      // For server mode, try to fetch existing instances from backend first.
      // Server's listInstances returns full data (config, torrent, stats).
      const isServerMode = !isTauri && typeof api.listInstances === 'function';
      if (isServerMode) {
        try {
          const serverInstances = await api.listInstances();
          if (serverInstances && serverInstances.length > 0) {
            const restoredInstances = serverInstances.map(serverInst => {
              // Create frontend instance from server state
              const instance = createDefaultInstance(
                serverInst.id,
                buildInstanceDefaultsFromServer(serverInst)
              );

              // Set torrent info (server returns summary)
              const summary = serverInst.torrent;
              instance.torrent = summary;
              instance.torrentPath = summary.name;
              instance.stats = serverInst.stats;

              // Set running state based on server state
              const state = serverInst.stats.state;
              instance.isRunning = state === 'Running';
              instance.isPaused = state === 'Paused';

              if (instance.isRunning) {
                instance.statusMessage = 'Running - restored from server';
                instance.statusType = 'running';
              } else if (instance.isPaused) {
                instance.statusMessage = 'Paused - restored from server';
                instance.statusType = 'idle';
              } else {
                instance.statusMessage = 'Ready to start faking';
                instance.statusType = 'idle';
              }

              return instance;
            });

            instances.set(restoredInstances);
            activeInstanceId.set(restoredInstances[0].id);
            updateActiveInstanceStore();

            return restoredInstances[0].id;
          }
        } catch (error) {
          console.warn(
            'Failed to fetch instances from server, falling back to localStorage:',
            error
          );
        }
      }

      // For Tauri desktop, try to restore instances from the backend.
      // The backend restores instances from desktop-state.json on startup (async spawn),
      // so we may need to retry if the frontend loads before restoration completes.
      if (isTauri) {
        try {
          let summaries = null;
          const savedSession = loadSessionFromStorage(config);
          const expectInstances =
            savedSession && savedSession.instances && savedSession.instances.length > 0;

          const maxRetries = expectInstances ? 5 : 1;
          for (let attempt = 0; attempt < maxRetries; attempt++) {
            summaries = await api.listSummaries();
            if (summaries && summaries.length > 0) break;

            if (attempt < maxRetries - 1) {
              await new Promise(r => setTimeout(r, 300));
            }
          }

          if (summaries && summaries.length > 0) {
            // Match saved config to backend instances by torrent name
            const savedInstances = savedSession?.instances ? [...savedSession.instances] : [];

            const restoredInstances = [];
            for (const summary of summaries) {
              const instanceId = String(summary.id);

              // Get full torrent data from backend
              let torrent = null;
              try {
                torrent = await api.getInstanceSummary(instanceId);
              } catch {
                // Instance may have been partially restored
              }

              // Match saved config by torrent name for user settings
              let savedConfig = null;
              if (savedInstances.length > 0) {
                const nameMatch = savedInstances.find(
                  s => s.torrentName && s.torrentName === summary.name
                );
                if (nameMatch) {
                  savedConfig = nameMatch;
                  savedInstances.splice(savedInstances.indexOf(nameMatch), 1);
                }
              }

              const defaults = savedConfig || (getDefaultPreset()?.settings ?? {});

              const instance = createDefaultInstance(instanceId, {
                ...defaults,
                cumulativeUploaded: bytesToMB(summary.uploaded),
                cumulativeDownloaded: bytesToMB(summary.downloaded),
              });

              if (torrent) {
                instance.torrent = torrent;
                instance.torrentPath = summary.name || torrent.name;
                instance.statusMessage = 'Ready to start faking';
                instance.statusType = 'idle';
              } else {
                instance.torrentPath = summary.name || '';
                instance.statusMessage = 'Torrent data unavailable';
                instance.statusType = 'warning';
              }

              instance.source = summary.source || 'manual';

              // Set running state from backend
              const state = summary.state?.toLowerCase();
              instance.isRunning =
                state === 'running' ||
                state === 'starting' ||
                state === 'idle' ||
                state === 'paused';
              instance.isPaused = state === 'paused';

              if (instance.isPaused) {
                instance.statusMessage = 'Paused';
                instance.statusType = 'idle';
                instance.statusIcon = 'pause';
              } else if (state === 'idle') {
                instance.statusMessage = 'Idling - No peers available';
                instance.statusType = 'idling';
                instance.statusIcon = 'moon';
              } else if (instance.isRunning) {
                instance.statusMessage = 'Actively faking ratio...';
                instance.statusType = 'running';
                instance.statusIcon = 'rocket';
              }

              restoredInstances.push(instance);
            }

            if (restoredInstances.length > 0) {
              instances.set(restoredInstances);

              const savedActiveIndex = savedSession?.activeInstanceIndex;
              if (
                savedActiveIndex !== null &&
                savedActiveIndex !== undefined &&
                savedActiveIndex >= 0 &&
                savedActiveIndex < restoredInstances.length
              ) {
                activeInstanceId.set(restoredInstances[savedActiveIndex].id);
              } else {
                activeInstanceId.set(restoredInstances[0].id);
              }

              updateActiveInstanceStore();
              return restoredInstances[0].id;
            }
          }
        } catch (error) {
          console.warn(
            'Failed to restore instances from Tauri backend, falling back to config:',
            error
          );
        }
      }

      // Fall back to localStorage/config restoration (web/WASM mode, or backend had no instances)
      const savedSession = loadSessionFromStorage(config);

      if (savedSession && savedSession.instances && savedSession.instances.length > 0) {
        // Restore from saved session
        const restoredInstances = [];
        for (const savedInst of savedSession.instances) {
          // Create backend instance
          const instanceId = await api.createInstance();

          // Create frontend instance with saved settings
          const instance = createDefaultInstance(instanceId, savedInst);

          // Try to restore torrent file
          if (savedInst.torrentPath) {
            if (isTauri) {
              // Desktop: Try to reload from path and register with backend
              try {
                const torrent = await api.loadInstanceTorrent(instanceId, savedInst.torrentPath);
                instance.torrent = torrent;
                instance.torrentPath = savedInst.torrentPath;
                instance.statusMessage = 'Ready to start faking';
                instance.statusType = 'idle';
              } catch {
                instance.statusMessage = 'Torrent file not found - please select again';
                instance.statusType = 'warning';
              }
            } else {
              // Web: Restore from saved torrent data
              if (savedInst.torrent) {
                instance.torrent = savedInst.torrent;
                instance.torrentPath = savedInst.torrentPath;
                instance.statusMessage = 'Ready to start faking';
                instance.statusType = 'idle';
              } else {
                instance.statusMessage = 'Please re-upload your torrent file';
                instance.statusType = 'warning';
              }
            }
          }

          restoredInstances.push(instance);
        }

        instances.set(restoredInstances);

        // Set active instance based on saved index
        if (
          savedSession.activeInstanceIndex !== null &&
          savedSession.activeInstanceIndex >= 0 &&
          savedSession.activeInstanceIndex < restoredInstances.length
        ) {
          activeInstanceId.set(restoredInstances[savedSession.activeInstanceIndex].id);
        } else {
          activeInstanceId.set(restoredInstances[0].id);
        }

        updateActiveInstanceStore();
        return restoredInstances[0].id;
      } else {
        // No saved session - create first instance with default preset if available
        const instanceId = await api.createInstance();

        const effectiveDefaults = await buildNewInstanceDefaults();

        const newInstance = createDefaultInstance(instanceId, effectiveDefaults);
        instances.set([newInstance]);
        activeInstanceId.set(instanceId);
        updateActiveInstanceStore();
        return instanceId;
      }
    } catch (error) {
      console.error('Failed to initialize:', error);
      throw error;
    }
  },

  // Add a new instance
  addInstance: async (defaults = {}) => {
    try {
      const instanceId = await api.createInstance();

      const effectiveDefaults = await buildNewInstanceDefaults(defaults);

      const newInstance = createDefaultInstance(instanceId, effectiveDefaults);

      instances.update(insts => [...insts, newInstance]);
      activeInstanceId.set(instanceId);
      updateActiveInstanceStore();

      // Save session after adding instance
      await saveSession(get(instances), instanceId);

      return instanceId;
    } catch (error) {
      console.error('Failed to create instance:', error);
      throw error;
    }
  },

  // Remove an instance
  // Set force=true to delete watch folder instances (when the file is missing)
  removeInstance: async (id, force = false) => {
    const currentInstances = get(instances);

    // Find the instance to check its source
    const instanceToRemove = currentInstances.find(inst => inst.id === id);
    if (!force && instanceToRemove && instanceToRemove.source === 'watch_folder') {
      throw new Error(
        'Cannot delete watch folder instance. Delete the torrent file from the watch folder instead.'
      );
    }

    try {
      // If this is the last instance, create a new empty one on the backend first
      let newInstanceId = null;
      let newInstance = null;
      if (currentInstances.length === 1) {
        newInstanceId = await api.createInstance();
        const effectiveDefaults = await buildNewInstanceDefaults();
        newInstance = createDefaultInstance(newInstanceId, effectiveDefaults);
      }

      // Delete the instance on the backend
      // Ignore "not found" errors - the instance may have been lost on server restart
      try {
        await api.deleteInstance(id, force);
      } catch (deleteError) {
        // Only log, don't throw - we still want to clean up the frontend
        console.warn(
          `Backend delete failed (may be expected after restart): ${deleteError.message}`
        );
      }

      let newActiveId = newInstanceId;

      // Atomically remove old instance and add new one (if last instance) in single update
      instances.update(insts => {
        const filtered = insts.filter(inst => inst.id !== id);

        // If we're removing the active instance, switch to the new one or first available
        const currentActiveId = get(activeInstanceId);
        if (currentActiveId === id) {
          newActiveId = newActiveId || filtered[0]?.id || null;
          activeInstanceId.set(newActiveId);
        }

        // Add new instance in the same update to avoid flicker
        if (newInstance) {
          return [...filtered, newInstance];
        }
        return filtered;
      });

      updateActiveInstanceStore();

      // Save session after removing instance
      await saveSession(get(instances), newActiveId || get(activeInstanceId));
    } catch (error) {
      console.error('Failed to remove instance:', error);
      throw error;
    }
  },

  // Select/switch to an instance
  selectInstance: id => {
    activeInstanceId.set(id);
    updateActiveInstanceStore();
  },

  // Update a specific instance
  updateInstance: (id, updates) => {
    // Don't update if no changes
    const currentInstances = get(instances);
    const currentInst = currentInstances.find(i => i.id === id);
    if (!currentInst) return;

    // Check if updates are actually different
    let hasChanges = false;
    for (const key in updates) {
      if (currentInst[key] !== updates[key]) {
        hasChanges = true;
        break;
      }
    }

    if (!hasChanges) return;

    instances.update(insts => {
      return insts.map(inst => {
        if (inst.id === id) {
          return { ...inst, ...updates };
        }
        return inst;
      });
    });

    // Only update active instance store if the modified instance is the active one
    if (id === get(activeInstanceId)) {
      updateActiveInstanceStore();
    }
  },

  // Update the currently active instance
  updateActiveInstance: updates => {
    const currentId = get(activeInstanceId);
    if (currentId !== null) {
      instanceActions.updateInstance(currentId, updates);
    }
  },

  // Get instance by ID
  getInstance: id => {
    const currentInstances = get(instances);
    return currentInstances.find(inst => inst.id === id);
  },

  // Get current active instance
  getActiveInstance: () => {
    return get(activeInstance);
  },

  // Merge a new instance from server (used for real-time sync with watch folder)
  // Returns true if a new instance was added, false if it already existed
  mergeServerInstance: serverInst => {
    const serverId = String(serverInst.id);
    const currentInstances = get(instances);
    const existingInstance = currentInstances.find(inst => String(inst.id) === serverId);

    if (existingInstance) {
      // Instance already exists, no need to merge
      return false;
    }

    // Create frontend instance from server state
    const instance = createDefaultInstance(serverId, buildInstanceDefaultsFromServer(serverInst));

    // Set torrent info
    instance.torrent = serverInst.torrent;
    instance.torrentPath = serverInst.torrent.name;
    instance.stats = serverInst.stats;

    // Set running state based on server state
    const state = serverInst.stats.state;
    instance.isRunning = state === 'Running';
    instance.isPaused = state === 'Paused';

    if (instance.isRunning) {
      instance.statusMessage = 'Running - added from watch folder';
      instance.statusType = 'running';
    } else if (instance.isPaused) {
      instance.statusMessage = 'Paused - added from watch folder';
      instance.statusType = 'idle';
    } else {
      instance.statusMessage = 'Ready to start - added from watch folder';
      instance.statusType = 'idle';
    }

    // Add to instances store
    instances.update(insts => [...insts, instance]);
    updateActiveInstanceStore();

    return true;
  },

  // Reconcile frontend instances with backend instances.
  // Adds missing backend instances and removes stale watch-folder instances.
  // Uses listSummaries so it works for both server and desktop backends.
  reconcileWithBackend: async (opts = {}) => {
    if (isReconcilingBackend) {
      return;
    }

    isReconcilingBackend = true;

    try {
      const pruneMissingLoaded = Boolean(opts.pruneMissingLoaded);
      const summaries = await api.listSummaries();
      if (!Array.isArray(summaries)) return;

      const backendIds = new Set(summaries.map(summary => String(summary.id)));

      const currentActive = get(activeInstanceId);
      const currentActiveKey = currentActive === null ? null : String(currentActive);
      let nextActive = currentActive;
      let removedAny = false;

      instances.update(current => {
        const filtered = current.filter(inst => {
          if (backendIds.has(String(inst.id))) return true;

          const hasLoadedTorrent = Boolean(inst.torrent) || Boolean(inst.torrentPath);
          if (inst.source === 'watch_folder') {
            removedAny = true;
            return false;
          }

          if (pruneMissingLoaded && hasLoadedTorrent) {
            removedAny = true;
            return false;
          }

          return true;
        });

        const hasActive =
          currentActiveKey !== null && filtered.some(inst => String(inst.id) === currentActiveKey);
        if (!hasActive) {
          nextActive = filtered[0]?.id ?? null;
        }

        return filtered;
      });

      const nextActiveKey = nextActive === null ? null : String(nextActive);
      if (nextActiveKey !== currentActiveKey) {
        activeInstanceId.set(nextActive);
      }

      // Ensure all backend instances exist in the standard store.
      for (const summary of summaries) {
        const exists = get(instances).some(inst => String(inst.id) === String(summary.id));
        if (!exists) {
          await instanceActions.ensureInstance(summary.id, summary);
        }
      }

      // Defensive dedupe in case overlapping UI actions inserted same id twice.
      let dedupedAny = false;
      instances.update(current => {
        const seen = new Set();
        const deduped = [];
        for (const inst of current) {
          const key = String(inst.id);
          if (seen.has(key)) {
            dedupedAny = true;
            continue;
          }
          seen.add(key);
          deduped.push(inst);
        }
        return deduped;
      });

      if (dedupedAny) {
        const currentActiveAfterDedup = get(activeInstanceId);
        const hasActive =
          currentActiveAfterDedup !== null &&
          get(instances).some(inst => String(inst.id) === String(currentActiveAfterDedup));
        if (!hasActive) {
          activeInstanceId.set(get(instances)[0]?.id ?? null);
        }
      }

      // Refresh runtime state and stats for all existing instances.
      await instanceActions.syncAllInstanceStates();

      if (removedAny || nextActiveKey !== currentActiveKey) {
        updateActiveInstanceStore();
      }
    } catch (error) {
      console.warn('Failed to reconcile instances with backend:', error);
    } finally {
      isReconcilingBackend = false;
    }
  },

  // Ensure an instance exists in the standard store (used when switching from grid to standard view)
  // If the instance doesn't exist, fetch it from the server or create a default entry.
  // Returns the instance ID if successfully ensured.
  ensureInstance: async (id, gridSummary = null) => {
    const normalizedId = String(id);
    const currentInstances = get(instances);
    const existing = currentInstances.find(inst => String(inst.id) === normalizedId);

    // Try to fetch actual config from backend and update or create the instance
    try {
      const serverInstances = await api.listInstances();
      if (serverInstances && serverInstances.length > 0) {
        const serverInst = serverInstances.find(inst => String(inst.id) === normalizedId);
        if (serverInst) {
          if (existing) {
            // Update existing instance with actual backend config
            const serverDefaults = buildInstanceDefaultsFromServer(serverInst);
            instanceActions.updateInstance(normalizedId, {
              selectedClient: serverDefaults.selectedClient,
              selectedClientVersion: serverDefaults.selectedClientVersion,
              uploadRate: serverDefaults.uploadRate,
              downloadRate: serverDefaults.downloadRate,
              port: serverDefaults.port,
              vpnPortSync: serverDefaults.vpnPortSync,
              completionPercent: serverDefaults.completionPercent,
              randomizeRates: serverDefaults.randomizeRates,
              randomRangePercent: serverDefaults.randomRangePercent,
              progressiveRatesEnabled: serverDefaults.progressiveRatesEnabled,
              targetUploadRate: serverDefaults.targetUploadRate,
              targetDownloadRate: serverDefaults.targetDownloadRate,
              progressiveDurationHours: serverDefaults.progressiveDurationHours,
              stopAtRatioEnabled: serverDefaults.stopAtRatioEnabled,
              stopAtRatio: serverDefaults.stopAtRatio,
              stopAtUploadedEnabled: serverDefaults.stopAtUploadedEnabled,
              stopAtUploadedGB: serverDefaults.stopAtUploadedGB,
              stopAtDownloadedEnabled: serverDefaults.stopAtDownloadedEnabled,
              stopAtDownloadedGB: serverDefaults.stopAtDownloadedGB,
              stopAtSeedTimeEnabled: serverDefaults.stopAtSeedTimeEnabled,
              stopAtSeedTimeHours: serverDefaults.stopAtSeedTimeHours,
              idleWhenNoLeechers: serverDefaults.idleWhenNoLeechers,
              idleWhenNoSeeders: serverDefaults.idleWhenNoSeeders,
              scrapeInterval: serverDefaults.scrapeInterval,
            });
            if (
              gridSummary?.source &&
              (gridSummary.source === 'watch_folder' || gridSummary.source === 'manual')
            ) {
              instanceActions.updateInstance(existing.id, {
                source: gridSummary.source,
              });
            }
            return String(existing.id);
          }
          instanceActions.mergeServerInstance(serverInst);
          return normalizedId;
        }
      }
    } catch {
      // listInstances may not be available or may return [] (Tauri/WASM)
    }

    if (existing) {
      if (
        gridSummary?.source &&
        (gridSummary.source === 'watch_folder' || gridSummary.source === 'manual')
      ) {
        instanceActions.updateInstance(existing.id, {
          source: gridSummary.source,
        });
      }
      return String(existing.id);
    }

    // Fallback: create a frontend instance and hydrate torrent from backend
    if (gridSummary) {
      const defaults = await buildNewInstanceDefaults();
      const source = gridSummary.source === 'watch_folder' ? 'watch_folder' : 'manual';
      const instance = createDefaultInstance(normalizedId, { ...defaults, source });
      instance.torrentPath = gridSummary.name || '';

      // Apply backend state from grid summary
      const summaryState = gridSummary.state?.toLowerCase();
      if (summaryState === 'running' || summaryState === 'starting' || summaryState === 'idle') {
        instance.isRunning = true;
        instance.isPaused = false;
      } else if (summaryState === 'paused') {
        instance.isRunning = true;
        instance.isPaused = true;
      }

      try {
        const torrent = await api.getInstanceSummary(id);
        if (torrent) {
          instance.torrent = torrent;
          instance.torrentPath = gridSummary.name || torrent.name || '';
        }
      } catch {
        instance.torrentPath = gridSummary.name || '';
      }

      if (instance.isPaused) {
        instance.statusMessage = 'Paused';
        instance.statusType = 'idle';
        instance.statusIcon = 'pause';
      } else if (summaryState === 'idle') {
        instance.statusMessage = 'Idling - No peers available';
        instance.statusType = 'idling';
        instance.statusIcon = 'moon';
      } else if (instance.isRunning) {
        instance.statusMessage = 'Actively faking ratio...';
        instance.statusType = 'running';
        instance.statusIcon = 'rocket';
      } else if (instance.torrent) {
        instance.statusMessage = 'Ready to start faking';
        instance.statusType = 'idle';
      } else {
        instance.statusMessage = gridSummary.name
          ? 'Loaded from grid view'
          : 'Select a torrent file to begin';
        instance.statusType = gridSummary.name ? 'idle' : 'warning';
      }

      instances.update(insts => [...insts, instance]);
      updateActiveInstanceStore();
      return String(instance.id);
    }

    return null;
  },

  // Add an instance to the standard store from grid import data
  addInstanceToStore: async (id, name, importDefaults = {}) => {
    const currentInstances = get(instances);
    if (currentInstances.some(inst => inst.id === id)) return;

    const defaults = await buildNewInstanceDefaults(importDefaults);
    const instance = createDefaultInstance(id, defaults);

    try {
      const torrent = await api.getInstanceSummary(id);
      if (torrent) {
        instance.torrent = torrent;
        instance.torrentPath = name || torrent.name || '';
        instance.statusMessage = 'Ready to start faking';
        instance.statusType = 'idle';
      }
    } catch {
      instance.torrentPath = name || '';
      instance.statusMessage = name ? 'Loaded from grid view' : 'Select a torrent file to begin';
      instance.statusType = name ? 'idle' : 'warning';
    }

    // If the import auto-started the instance, reflect that in the standard view
    if (importDefaults.autoStart) {
      instance.isRunning = true;
      instance.statusMessage = 'Actively faking ratio...';
      instance.statusType = 'running';
    }

    instances.update(insts => [...insts, instance]);
    updateActiveInstanceStore();
  },

  // Sync runtime state (isRunning, isPaused, stats) for a single instance from the backend.
  // Used by grid actions to propagate state changes to the standard view store.
  syncInstanceState: async (id, { isRunning, isPaused, stats } = {}) => {
    const currentInstances = get(instances);
    const existing = currentInstances.find(inst => inst.id === id);
    if (!existing) return;

    const updates = {};
    if (isRunning !== undefined) updates.isRunning = isRunning;
    if (isPaused !== undefined) updates.isPaused = isPaused;
    if (stats !== undefined) updates.stats = stats;

    if (isPaused) {
      updates.statusMessage = 'Paused';
      updates.statusType = 'idle';
      updates.statusIcon = 'pause';
    } else if (isRunning) {
      updates.statusMessage = 'Actively faking ratio...';
      updates.statusType = 'running';
      updates.statusIcon = 'rocket';
    } else if (isRunning === false) {
      updates.statusMessage = 'Ready to start faking';
      updates.statusType = 'idle';
      updates.statusIcon = null;
    }

    instanceActions.updateInstance(id, updates);
  },

  // Sync all instance states and stats from the backend summaries.
  // Called when switching from grid view back to standard view.
  syncAllInstanceStates: async () => {
    try {
      const summaries = await api.listSummaries();
      if (!summaries) return;

      const summaryMap = new Map(summaries.map(s => [String(s.id), s]));
      const currentInstances = get(instances);

      let hasAnyChanges = false;
      const updatedInstances = currentInstances.map(inst => {
        const summary = summaryMap.get(String(inst.id));
        if (!summary) return inst;

        const state = summary.state?.toLowerCase();
        const isRunning =
          state === 'running' || state === 'starting' || state === 'paused' || state === 'idle';
        const isPaused = state === 'paused';

        const updates = { isRunning, isPaused };

        if (summary.source === 'watch_folder' || summary.source === 'manual') {
          updates.source = summary.source;
        }

        if (isPaused) {
          updates.statusMessage = 'Paused';
          updates.statusType = 'idle';
          updates.statusIcon = 'pause';
        } else if (state === 'idle') {
          updates.statusMessage = 'Idling - No peers available';
          updates.statusType = 'idling';
          updates.statusIcon = 'moon';
        } else if (isRunning) {
          updates.statusMessage = 'Actively faking ratio...';
          updates.statusType = 'running';
          updates.statusIcon = 'rocket';
        } else {
          updates.statusMessage = 'Ready to start faking';
          updates.statusType = 'idle';
          updates.statusIcon = null;
        }

        // Merge available stats from summary into the existing stats object
        // Session-specific fields are preserved and refreshed by polling intervals
        updates.stats = {
          ...(inst.stats || {}),
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
        updates.completionPercent = summary.torrentCompletion;

        hasAnyChanges = true;
        return { ...inst, ...updates };
      });

      if (hasAnyChanges) {
        instances.set(updatedInstances);
        updateActiveInstanceStore();
      }
    } catch (error) {
      console.warn('Failed to sync instance states:', error);
    }
  },

  // Remove instance from frontend (used when server sends delete event)
  removeInstanceFromStore: async id => {
    const currentInstances = get(instances);

    const exists = currentInstances.some(inst => inst.id === id);
    if (!exists) {
      return false;
    }

    // If this is the last instance, create a new empty replacement first
    let newInstance = null;
    if (currentInstances.length === 1) {
      const newId = await api.createInstance();
      const defaults = await buildNewInstanceDefaults();
      newInstance = createDefaultInstance(newId, defaults);
    }

    instances.update(insts => {
      const filtered = insts.filter(inst => inst.id !== id);

      const currentActiveId = get(activeInstanceId);
      if (currentActiveId === id) {
        activeInstanceId.set(newInstance?.id || filtered[0]?.id || null);
      }

      if (newInstance) {
        return [...filtered, newInstance];
      }
      return filtered;
    });

    updateActiveInstanceStore();
    return true;
  },
};
