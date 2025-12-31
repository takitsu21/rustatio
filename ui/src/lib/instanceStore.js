import { writable, get } from 'svelte/store';
import { api } from '$lib/api';

// Check if running in Tauri
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

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

    // Intervals
    updateInterval: null,
    liveStatsInterval: null,
    countdownInterval: null,

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
    nextUpdateIn: 0,
  };
}

// Global lock to prevent concurrent config saves from different sources
export let isConfigSaving = false;

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
        selected_client: inst.selectedClient,
        selected_client_version: inst.selectedClientVersion,
        upload_rate: parseFloat(inst.uploadRate),
        download_rate: parseFloat(inst.downloadRate),
        port: parseInt(inst.port),
        completion_percent: parseFloat(inst.completionPercent),
        initial_uploaded: parseInt(inst.initialUploaded) * 1024 * 1024,
        initial_downloaded: parseInt(inst.initialDownloaded) * 1024 * 1024,
        cumulative_uploaded: parseInt(inst.cumulativeUploaded) * 1024 * 1024,
        cumulative_downloaded: parseInt(inst.cumulativeDownloaded) * 1024 * 1024,
        randomize_rates: inst.randomizeRates,
        random_range_percent: parseFloat(inst.randomRangePercent),
        update_interval_seconds: parseInt(inst.updateIntervalSeconds),
        stop_at_ratio_enabled: inst.stopAtRatioEnabled,
        stop_at_ratio: parseFloat(inst.stopAtRatio),
        stop_at_uploaded_enabled: inst.stopAtUploadedEnabled,
        stop_at_uploaded_gb: parseFloat(inst.stopAtUploadedGB),
        stop_at_downloaded_enabled: inst.stopAtDownloadedEnabled,
        stop_at_downloaded_gb: parseFloat(inst.stopAtDownloadedGB),
        stop_at_seed_time_enabled: inst.stopAtSeedTimeEnabled,
        stop_at_seed_time_hours: parseFloat(inst.stopAtSeedTimeHours),
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
          torrent_data: inst.torrent || null, // Save the actual torrent object for web
          selected_client: inst.selectedClient,
          selected_client_version: inst.selectedClientVersion,
          upload_rate: parseFloat(inst.uploadRate),
          download_rate: parseFloat(inst.downloadRate),
          port: parseInt(inst.port),
          completion_percent: parseFloat(inst.completionPercent),
          initial_uploaded: parseInt(inst.initialUploaded) * 1024 * 1024, // Convert MB to bytes
          initial_downloaded: parseInt(inst.initialDownloaded) * 1024 * 1024,
          cumulative_uploaded: parseInt(inst.cumulativeUploaded) * 1024 * 1024,
          cumulative_downloaded: parseInt(inst.cumulativeDownloaded) * 1024 * 1024,
          randomize_rates: inst.randomizeRates,
          random_range_percent: parseFloat(inst.randomRangePercent),
          update_interval_seconds: parseInt(inst.updateIntervalSeconds),
          stop_at_ratio_enabled: inst.stopAtRatioEnabled,
          stop_at_ratio: parseFloat(inst.stopAtRatio),
          stop_at_uploaded_enabled: inst.stopAtUploadedEnabled,
          stop_at_uploaded_gb: parseFloat(inst.stopAtUploadedGB),
          stop_at_downloaded_enabled: inst.stopAtDownloadedEnabled,
          stop_at_downloaded_gb: parseFloat(inst.stopAtDownloadedGB),
          stop_at_seed_time_enabled: inst.stopAtSeedTimeEnabled,
          stop_at_seed_time_hours: parseFloat(inst.stopAtSeedTimeHours),
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
        torrent: inst.torrent_data || null, // Restore torrent data for web
        selectedClient: inst.selected_client,
        selectedClientVersion: inst.selected_client_version,
        uploadRate: inst.upload_rate,
        downloadRate: inst.download_rate,
        port: inst.port,
        completionPercent: inst.completion_percent,
        initialUploaded: inst.initial_uploaded / (1024 * 1024), // Convert bytes to MB
        initialDownloaded: inst.initial_downloaded / (1024 * 1024),
        cumulativeUploaded: (inst.cumulative_uploaded || 0) / (1024 * 1024),
        cumulativeDownloaded: (inst.cumulative_downloaded || 0) / (1024 * 1024),
        randomizeRates: inst.randomize_rates,
        randomRangePercent: inst.random_range_percent,
        updateIntervalSeconds: inst.update_interval_seconds,
        stopAtRatioEnabled: inst.stop_at_ratio_enabled,
        stopAtRatio: inst.stop_at_ratio,
        stopAtUploadedEnabled: inst.stop_at_uploaded_enabled,
        stopAtUploadedGB: inst.stop_at_uploaded_gb,
        stopAtDownloadedEnabled: inst.stop_at_downloaded_enabled,
        stopAtDownloadedGB: inst.stop_at_downloaded_gb,
        stopAtSeedTimeEnabled: inst.stop_at_seed_time_enabled,
        stopAtSeedTimeHours: inst.stop_at_seed_time_hours,
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
function updateActiveInstanceStore() {
  const $instances = get(instances);
  const $activeInstanceId = get(activeInstanceId);
  const active = $instances.find(inst => inst.id === $activeInstanceId) || $instances[0] || null;
  activeInstance.set(active);
}

// Actions
export const instanceActions = {
  // Initialize - create first instance or restore from storage
  initialize: async () => {
    try {
      // Load config if in Tauri mode
      let config = null;
      if (isTauri) {
        config = await api.getConfig();
        globalConfig.set(config);
      }

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
              // Desktop: Try to reload from path
              try {
                const torrent = await api.loadTorrent(savedInst.torrentPath);
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
        // No saved session - create first instance with defaults
        const instanceId = await api.createInstance();

        const newInstance = createDefaultInstance(instanceId, {});
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
      const newInstance = createDefaultInstance(instanceId, defaults);

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
  removeInstance: async id => {
    const currentInstances = get(instances);

    // Don't remove if it's the last instance
    if (currentInstances.length <= 1) {
      console.warn('Cannot remove the last instance');
      return;
    }

    try {
      // Delete the instance on the backend
      await api.deleteInstance(id);

      let newActiveId = null;

      // Remove from frontend
      instances.update(insts => {
        const filtered = insts.filter(inst => inst.id !== id);

        // If we're removing the active instance, switch to the first one
        const currentActiveId = get(activeInstanceId);
        if (currentActiveId === id) {
          newActiveId = filtered[0]?.id || null;
          activeInstanceId.set(newActiveId);
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
    updateActiveInstanceStore();
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
};
