<script>
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { ChevronDown, Check } from '@lucide/svelte';
  import {
    initWasm,
    api,
    listenToLogs,
    listenToInstanceEvents,
    getRunMode,
    checkAuthStatus,
    verifyAuthToken,
    getAuthToken,
  } from './lib/api.js';

  // Import instance stores
  import {
    instances,
    activeInstance,
    activeInstanceId,
    instanceActions,
    saveSession,
  } from './lib/instanceStore.js';

  // Import components
  import Header from './components/Header.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import TorrentSelector from './components/TorrentSelector.svelte';
  import ConfigurationForm from './components/ConfigurationForm.svelte';
  import StopConditions from './components/StopConditions.svelte';
  import ProgressBars from './components/ProgressBars.svelte';
  import SessionStats from './components/SessionStats.svelte';
  import TotalStats from './components/TotalStats.svelte';
  import RateGraph from './components/RateGraph.svelte';
  import Logs from './components/Logs.svelte';
  import ProxySettings from './components/ProxySettings.svelte';
  import UpdateChecker from './components/UpdateChecker.svelte';
  import ThemeIcon from './components/ThemeIcon.svelte';
  import DownloadButton from './components/DownloadButton.svelte';
  import AuthPage from './components/AuthPage.svelte';

  // Import theme store
  import {
    THEMES,
    THEME_CATEGORIES,
    getTheme,
    getShowThemeDropdown,
    toggleThemeDropdown,
    selectTheme,
    initializeTheme,
    handleClickOutside,
    getThemeName,
  } from './lib/themeStore.svelte.js';

  // Check if running in Tauri
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  // Loading state to prevent UI flash during initialization
  let isInitialized = $state(false);

  // Authentication state
  let showAuthDialog = $state(false);
  let _isAuthenticated = $state(false);

  // Flag to prevent store subscriptions from firing during initialization
  let isInitializing = true;

  // Available client versions
  const clientVersions = {
    utorrent: ['3.5.5', '3.5.4', '3.5.3', '3.4.9', '3.4.8', '2.2.1'],
    qbittorrent: ['5.1.4', '5.1.3', '5.0.2', '4.6.7', '4.5.5', '4.4.5'],
    transmission: ['4.0.5', '4.0.4', '4.0.3', '3.00', '2.94', '2.93'],
    deluge: ['2.1.1', '2.0.5', '2.0.3', '1.3.15'],
  };

  // Default ports for each client
  const clientDefaultPorts = {
    utorrent: 6881,
    qbittorrent: 6881,
    transmission: 51413,
    deluge: 6881,
  };

  // Logs
  let logs = $state([]);
  let showLogs = $state(false);

  // Sidebar state
  let sidebarOpen = $state(false);
  let sidebarCollapsed = $state(false);

  // Development logging helper - only logs in development mode
  function devLog(level, ...args) {
    if (import.meta.env.DEV) {
      console[level](...args);
    }
  }

  // Available clients
  const clients = [
    { id: 'utorrent', name: 'µTorrent' },
    { id: 'qbittorrent', name: 'qBittorrent' },
    { id: 'transmission', name: 'Transmission' },
    { id: 'deluge', name: 'Deluge' },
  ];

  // Store cleanup functions
  let unsubActiveInstance = null;
  let unsubSessionSave = null;
  let instanceEventsCleanup = null;

  // Debounce timer for syncing config to server
  let configSyncTimeout = null;

  // Beforeunload handler reference for cleanup
  let beforeUnloadHandler = null;

  // Track previous client to detect changes
  let previousClient = null;

  // Global error handler
  if (typeof window !== 'undefined') {
    window.addEventListener('error', event => {
      console.error('Global error caught:', event.error);
      console.error('Error message:', event.message);
      console.error('Error stack:', event.error?.stack);
    });

    window.addEventListener('unhandledrejection', event => {
      console.error('Unhandled promise rejection:', event.reason);
    });
  }

  // Load configuration on mount
  onMount(async () => {
    try {
      // Initialize WASM/detect server mode
      await initWasm();

      // Check if authentication is required (server mode only)
      const { authEnabled } = await checkAuthStatus();

      if (authEnabled) {
        // Check if we have a valid token stored
        const storedToken = getAuthToken();
        if (storedToken) {
          const { valid } = await verifyAuthToken();
          if (valid) {
            _isAuthenticated = true;
          } else {
            // Token is invalid, show auth dialog
            showAuthDialog = true;
            isInitialized = true;
            return; // Don't continue initialization until authenticated
          }
        } else {
          // No token stored, show auth dialog
          showAuthDialog = true;
          isInitialized = true;
          return; // Don't continue initialization until authenticated
        }
      } else {
        // Auth not enabled, consider authenticated
        _isAuthenticated = true;
      }

      // Continue with normal initialization
      await continueInitialization();
    } catch (error) {
      console.error('Failed to initialize app:', error);
      devLog('error', 'Failed to initialize:', error);
      // Still show UI even if there's an error
      isInitialized = true;
    }

    // Initialize theme
    initializeTheme();

    // Close dropdown when clicking outside
    document.addEventListener('click', handleClickOutside);

    // Set up reactive subscriptions using store.subscribe instead of $effect
    // This avoids the orphan effect error in Svelte 5
    unsubActiveInstance = activeInstance.subscribe(inst => {
      if (!inst || isInitializing) return;

      // Update client version when client changes
      if (inst.selectedClient && clientVersions[inst.selectedClient]) {
        if (
          !inst.selectedClientVersion ||
          !clientVersions[inst.selectedClient].includes(inst.selectedClientVersion)
        ) {
          instanceActions.updateInstance(inst.id, {
            selectedClientVersion: clientVersions[inst.selectedClient][0],
          });
        }
      }

      // Update port when client changes (only if not running and client actually changed)
      if (
        inst.selectedClient &&
        !inst.isRunning &&
        clientDefaultPorts[inst.selectedClient] &&
        previousClient !== null &&
        previousClient !== inst.selectedClient
      ) {
        instanceActions.updateInstance(inst.id, {
          port: clientDefaultPorts[inst.selectedClient],
        });
      }

      // Track current client for next comparison
      previousClient = inst.selectedClient;
    });

    // Config save is handled by saveSession below, so we don't need a separate subscription

    // Auto-save session when instances change (throttled to prevent infinite loops)
    // Don't save session if any instance is currently running to avoid saves during faking
    let saveSessionTimeout = null;
    let hasCompletedFirstSave = false;

    unsubSessionSave = instances.subscribe(insts => {
      if (isInitializing) return;

      // Skip if any instance is running (faking)
      const hasRunningInstance = insts.some(inst => inst.isRunning);
      if (hasRunningInstance) return;

      const activeInst = get(activeInstance);
      if (insts.length > 0 && activeInst) {
        // Throttle session saves to prevent infinite loops
        clearTimeout(saveSessionTimeout);
        saveSessionTimeout = setTimeout(() => {
          // Skip the first save after initialization - this is just the initial load
          if (!hasCompletedFirstSave) {
            hasCompletedFirstSave = true;
            return;
          }

          saveSession(insts, activeInst.id);
        }, 500);
      }
    });

    // Set up beforeunload warning for WASM mode only
    // In WASM mode, refreshing the page stops all running torrents
    // Server mode persists state, so no warning needed there
    const runMode = getRunMode();
    if (runMode === 'wasm') {
      beforeUnloadHandler = event => {
        const currentInstances = get(instances);
        const hasRunning = currentInstances.some(inst => inst.isRunning);

        if (hasRunning) {
          // Standard way to show beforeunload dialog
          event.preventDefault();
          // Chrome requires returnValue to be set
          event.returnValue = '';
          return '';
        }
      };
      window.addEventListener('beforeunload', beforeUnloadHandler);
    }

    // Wait for stores to settle before allowing saves
    // This prevents initial subscription fires from triggering saves during app load
    await new Promise(resolve => setTimeout(resolve, 100));

    // Mark initialization as complete
    isInitializing = false;
  });

  // Continue initialization after authentication (called after auth is verified)
  async function continueInitialization() {
    // Load config from localStorage
    const storedShowLogs = localStorage.getItem('rustatio-show-logs');
    showLogs = storedShowLogs ? JSON.parse(storedShowLogs) : false;

    // Log level priority for filtering
    const LOG_LEVELS = { error: 0, warn: 1, info: 2, debug: 3, trace: 4 };

    // Set up log listener (works for both Tauri and web)
    await listenToLogs(logEvent => {
      // Filter logs based on configured log level
      const configuredLevel = localStorage.getItem('rustatio-log-level') || 'info';
      const eventPriority = LOG_LEVELS[logEvent.level] ?? 2;
      const configuredPriority = LOG_LEVELS[configuredLevel] ?? 2;

      // Only show logs at or below the configured level
      if (eventPriority > configuredPriority) {
        return;
      }

      logs = [...logs, logEvent];

      // Limit logs to prevent memory issues (keep last 500)
      if (logs.length > 500) {
        logs = logs.slice(-500);
      }
    });

    // Initialize instance store (will restore session from localStorage)
    await instanceActions.initialize();

    // Start polling for any instances that were restored in a running state (server mode)
    // This ensures UI updates after page refresh when instances are still running on server
    const restoredInstances = get(instances);
    for (const inst of restoredInstances) {
      if (inst.isRunning && !inst.isPaused) {
        devLog('log', `Starting polling for restored running instance ${inst.id}`);
        startPollingForInstance(inst.id, inst.updateIntervalSeconds ?? 5);
      }
    }

    // Set up instance events subscription for real-time sync (server mode only)
    // This allows watch folder instances to appear without page refresh
    if (getRunMode() === 'server') {
      instanceEventsCleanup = listenToInstanceEvents(async event => {
        devLog('log', 'Received instance event:', event);

        if (event.type === 'created') {
          // Fetch full instance info from server
          try {
            const serverInstances = await api.listInstances();
            const newInstance = serverInstances.find(inst => inst.id === event.id);

            if (newInstance) {
              const wasAdded = instanceActions.mergeServerInstance(newInstance);
              if (wasAdded && newInstance.stats.state === 'Running') {
                // Start polling for the new running instance
                startPollingForInstance(event.id, newInstance.config.update_interval || 5);
              }
            }
          } catch (error) {
            console.error('Failed to fetch new instance:', error);
          }
        } else if (event.type === 'deleted') {
          // Remove instance from frontend store
          instanceActions.removeInstanceFromStore(event.id);
        }
      });
    }

    // Wait a tick for stores to update before showing UI
    await new Promise(resolve => setTimeout(resolve, 0));

    // Mark as initialized to show UI
    isInitialized = true;
  }

  // Handle successful authentication
  async function handleAuthenticated() {
    _isAuthenticated = true;
    showAuthDialog = false;

    // Continue initialization after successful auth
    await continueInitialization();

    // Initialize theme (needs to happen after isInitialized is set)
    initializeTheme();
  }

  // Config is saved via saveSession in instanceStore.js

  // Cleanup on unmount
  onDestroy(() => {
    // Clean up intervals for active instance
    if ($activeInstance) {
      if ($activeInstance.updateInterval) {
        clearInterval($activeInstance.updateInterval);
      }
      if ($activeInstance.countdownInterval) {
        clearInterval($activeInstance.countdownInterval);
      }
      if ($activeInstance.liveStatsInterval) {
        clearInterval($activeInstance.liveStatsInterval);
      }
    }

    // Clean up store subscriptions
    if (unsubActiveInstance) {
      unsubActiveInstance();
    }
    if (unsubSessionSave) {
      unsubSessionSave();
    }

    // Clean up instance events subscription
    if (instanceEventsCleanup) {
      instanceEventsCleanup();
    }

    // Clean up event listeners
    document.removeEventListener('click', handleClickOutside);

    // Clean up beforeunload handler
    if (beforeUnloadHandler) {
      window.removeEventListener('beforeunload', beforeUnloadHandler);
    }

    // Clean up config sync timeout
    if (configSyncTimeout) {
      clearTimeout(configSyncTimeout);
    }
  });

  // Select torrent file (called from TorrentSelector with File object)
  async function selectTorrent(file) {
    if (!$activeInstance) {
      alert('No active instance');
      return;
    }

    if (!file) {
      // User cancelled - only update status if no torrent is loaded
      if (!$activeInstance.torrent) {
        instanceActions.updateInstance($activeInstance.id, {
          statusMessage: 'Select a torrent file to begin',
          statusType: 'warning',
        });
      } else {
        // Keep existing status (torrent still loaded)
        instanceActions.updateInstance($activeInstance.id, {
          statusMessage: 'Ready to start faking',
          statusType: 'idle',
        });
      }
      return;
    }

    try {
      instanceActions.updateInstance($activeInstance.id, {
        statusMessage: 'Loading torrent...',
        statusType: 'running',
      });

      // In server mode, use loadInstanceTorrent to create a server-side instance
      // This allows the instance to persist across page refreshes
      const runMode = getRunMode();
      let torrent;
      if (runMode === 'server' && api.loadInstanceTorrent) {
        torrent = await api.loadInstanceTorrent($activeInstance.id, file);
      } else {
        torrent = await api.loadTorrent(file);
      }
      devLog('log', 'Loaded torrent:', torrent);

      // For desktop (Tauri): save the full file path
      // For web: save the torrent name (we'll serialize the torrent object itself)
      const torrentPath = isTauri ? file : file.name;

      // Update instance with torrent info
      instanceActions.updateInstance($activeInstance.id, {
        torrent,
        torrentPath,
        statusMessage: 'Torrent loaded successfully',
        statusType: 'success',
      });

      const instanceId = $activeInstance.id;
      setTimeout(() => {
        // Only update status if the instance is not running
        const instance = $instances.find(i => i.id === instanceId);
        if (instance && !instance.isRunning) {
          instanceActions.updateInstance(instanceId, {
            statusMessage: 'Ready to start faking',
            statusType: 'idle',
          });
        }
      }, 2000);
    } catch (error) {
      instanceActions.updateInstance($activeInstance.id, {
        statusMessage: 'Failed to load torrent: ' + error,
        statusType: 'error',
      });
      alert('Failed to load torrent: ' + error);
    }
  }

  // =============================================================================
  // Polling Helper Functions
  // =============================================================================

  // Get instance by ID from the store
  function getInstance(instanceId) {
    return $instances.find(i => i.id === instanceId);
  }

  // Clear all polling intervals for an instance
  function clearInstanceIntervals(instanceId) {
    const instance = getInstance(instanceId);
    if (instance) {
      if (instance.updateInterval) clearInterval(instance.updateInterval);
      if (instance.countdownInterval) clearInterval(instance.countdownInterval);
      if (instance.liveStatsInterval) clearInterval(instance.liveStatsInterval);
    }
  }

  // Handle auto-stop when a stop condition is met
  async function handleAutoStop(instanceId, stats) {
    try {
      await api.stopFaker(instanceId);
    } catch (error) {
      console.warn('Failed to stop faker on backend:', error);
    }

    clearInstanceIntervals(instanceId);

    // Save cumulative stats (convert bytes to MB)
    const cumulativeUploaded = Math.round(stats.uploaded / (1024 * 1024));
    const cumulativeDownloaded = Math.round(stats.downloaded / (1024 * 1024));

    instanceActions.updateInstance(instanceId, {
      isRunning: false,
      nextUpdateIn: 0,
      updateInterval: null,
      countdownInterval: null,
      liveStatsInterval: null,
      statusMessage: 'Stopped automatically - condition met',
      statusType: 'success',
      cumulativeUploaded,
      cumulativeDownloaded,
    });
  }

  // Handle polling error
  function handlePollingError(instanceId, error) {
    devLog('error', 'Polling error:', error);
    clearInstanceIntervals(instanceId);

    instanceActions.updateInstance(instanceId, {
      isRunning: false,
      updateInterval: null,
      countdownInterval: null,
      liveStatsInterval: null,
      statusMessage: 'Error: ' + error,
      statusType: 'error',
    });
  }

  // Check if stats indicate the faker should auto-stop
  function shouldAutoStop(stats) {
    return stats.state === 'Stopped' || stats.state === 'Completed';
  }

  // =============================================================================
  // Polling Intervals
  // =============================================================================

  // Create tracker announce interval (sends updates to tracker periodically)
  function createTrackerAnnounceInterval(instanceId, intervalMs) {
    return setInterval(async () => {
      const instance = getInstance(instanceId);

      if (!instance || !instance.isRunning) {
        devLog('log', 'Update skipped - not running');
        return;
      }

      if (instance.isPaused) {
        devLog('log', 'Update skipped - paused');
        instanceActions.updateInstance(instanceId, {
          nextUpdateIn: instance.updateIntervalSeconds ?? 5,
        });
        return;
      }

      try {
        await api.updateFaker(instanceId);
        const stats = await api.getStats(instanceId);

        instanceActions.updateInstance(instanceId, {
          stats,
          nextUpdateIn: instance.updateIntervalSeconds ?? 5,
        });

        if (shouldAutoStop(stats)) {
          await handleAutoStop(instanceId, stats);
        }
      } catch (error) {
        handlePollingError(instanceId, error);
      }
    }, intervalMs);
  }

  // Create live stats interval (updates UI every second without tracker announce)
  function createLiveStatsInterval(instanceId) {
    return setInterval(async () => {
      const instance = getInstance(instanceId);

      if (!instance || !instance.isRunning || instance.isPaused) {
        return;
      }

      try {
        const stats = await api.updateStatsOnly(instanceId);

        if (stats && instance.isRunning) {
          instanceActions.updateInstance(instanceId, { stats });

          if (shouldAutoStop(stats)) {
            await handleAutoStop(instanceId, stats);
          }
        }
      } catch (error) {
        console.debug('Live stats fetch error:', error);
      }
    }, 1000);
  }

  // Create countdown timer interval (decrements nextUpdateIn every second)
  function createCountdownInterval(instanceId) {
    return setInterval(() => {
      const instance = getInstance(instanceId);

      if (instance && !instance.isPaused && instance.nextUpdateIn > 0) {
        instanceActions.updateInstance(instanceId, {
          nextUpdateIn: instance.nextUpdateIn - 1,
        });
      }
    }, 1000);
  }

  // =============================================================================
  // Main Polling Entry Point
  // =============================================================================

  // Start all polling intervals for an instance
  // Used by startFaking() and after page refresh to restore running instances
  function startPollingForInstance(instanceId, intervalSeconds = 5) {
    const intervalMs = intervalSeconds * 1000;

    const updateIntervalId = createTrackerAnnounceInterval(instanceId, intervalMs);
    const liveStatsIntervalId = createLiveStatsInterval(instanceId);
    const countdownIntervalId = createCountdownInterval(instanceId);

    // Store interval IDs in instance for cleanup
    instanceActions.updateInstance(instanceId, {
      updateInterval: updateIntervalId,
      liveStatsInterval: liveStatsIntervalId,
      countdownInterval: countdownIntervalId,
      nextUpdateIn: intervalSeconds,
    });

    return { updateIntervalId, liveStatsIntervalId, countdownIntervalId };
  }

  // Start faking
  async function startFaking() {
    if (!$activeInstance) {
      alert('No active instance');
      return;
    }

    if (!$activeInstance.torrent) {
      instanceActions.updateInstance($activeInstance.id, {
        statusMessage: 'Please select a torrent file first',
        statusType: 'error',
      });
      alert('Please select a torrent file first');
      return;
    }

    try {
      // Calculate downloaded from completion percentage and torrent size
      const torrentSize = $activeInstance.torrent?.total_size || 0;
      const completionPercent = parseFloat($activeInstance.completionPercent ?? 0);
      const calculatedDownloaded = Math.floor((completionPercent / 100) * torrentSize);

      // For display purposes (placeholder stats), use cumulative values if available
      // This ensures the UI shows the correct totals immediately while starting
      const hasCumulativeStats =
        $activeInstance.cumulativeUploaded > 0 || $activeInstance.cumulativeDownloaded > 0;
      const displayUploaded = hasCumulativeStats
        ? parseInt($activeInstance.cumulativeUploaded ?? 0) * 1024 * 1024
        : parseInt($activeInstance.initialUploaded ?? 0) * 1024 * 1024;
      const displayDownloaded = hasCumulativeStats
        ? parseInt($activeInstance.cumulativeDownloaded ?? 0) * 1024 * 1024
        : calculatedDownloaded;

      // Preserve cumulative stats display while starting
      // Create initial stats object to show cumulative values immediately
      const calculatedLeft = torrentSize - calculatedDownloaded;

      // Calculate initial progress values to avoid jumps
      // Use uploaded/downloaded if downloaded > 0, otherwise use uploaded/torrent_size
      const initialRatio =
        displayDownloaded > 0
          ? displayUploaded / displayDownloaded
          : torrentSize > 0
            ? displayUploaded / torrentSize
            : 0;

      // Ratio progress is based on session ratio (starts at 0), not cumulative ratio
      // So initial ratio progress should always be 0 when starting a new session

      const placeholderStats = hasCumulativeStats
        ? {
            // Cumulative (from previous sessions)
            uploaded: displayUploaded,
            downloaded: displayDownloaded,
            ratio: initialRatio,

            // Torrent state
            left: calculatedLeft,
            seeders: 0,
            leechers: 0,
            state: 'Starting',

            // Session (starts fresh)
            session_uploaded: 0,
            session_downloaded: 0,
            session_ratio: 0.0,
            elapsed_time: { secs: 0, nanos: 0 },

            // Rates
            current_upload_rate: 0,
            current_download_rate: 0,
            average_upload_rate: 0,
            average_download_rate: 0,

            // Progress (session-based)
            upload_progress: 0,
            download_progress: 0,
            ratio_progress: 0,
            seed_time_progress: 0,

            // ETA
            eta_ratio: null,
            eta_uploaded: null,
            eta_seed_time: null,

            // History
            upload_rate_history: [],
            download_rate_history: [],
            ratio_history: [],
          }
        : null;

      instanceActions.updateInstance($activeInstance.id, {
        stats: placeholderStats,
        statusMessage: 'Starting ratio faker...',
        statusType: 'running',
      });

      const fakerConfig = {
        upload_rate: parseFloat($activeInstance.uploadRate ?? 50),
        download_rate: parseFloat($activeInstance.downloadRate ?? 100),
        port: parseInt($activeInstance.port ?? 6881),
        client_type: $activeInstance.selectedClient || 'qbittorrent',
        client_version:
          $activeInstance.selectedClientVersion ||
          clientVersions[$activeInstance.selectedClient || 'qbittorrent'][0],
        // Always send the original user-specified values
        // The server will handle using cumulative stats internally for the RatioFaker
        initial_uploaded: parseInt($activeInstance.initialUploaded ?? 0) * 1024 * 1024,
        initial_downloaded: calculatedDownloaded,
        completion_percent: parseFloat($activeInstance.completionPercent ?? 0),
        num_want: 50,
        randomize_rates: $activeInstance.randomizeRates ?? true,
        random_range_percent: parseFloat($activeInstance.randomRangePercent ?? 20),
        stop_at_ratio: $activeInstance.stopAtRatioEnabled
          ? parseFloat($activeInstance.stopAtRatio ?? 2.0)
          : null,
        stop_at_uploaded: $activeInstance.stopAtUploadedEnabled
          ? parseFloat($activeInstance.stopAtUploadedGB ?? 10) * 1024 * 1024 * 1024
          : null,
        stop_at_downloaded: $activeInstance.stopAtDownloadedEnabled
          ? parseFloat($activeInstance.stopAtDownloadedGB ?? 10) * 1024 * 1024 * 1024
          : null,
        stop_at_seed_time: $activeInstance.stopAtSeedTimeEnabled
          ? parseFloat($activeInstance.stopAtSeedTimeHours ?? 24) * 3600
          : null,
        stop_when_no_leechers: $activeInstance.stopWhenNoLeechers ?? false,
        progressive_rates: $activeInstance.progressiveRatesEnabled ?? false,
        target_upload_rate: $activeInstance.progressiveRatesEnabled
          ? parseFloat($activeInstance.targetUploadRate ?? 100)
          : null,
        target_download_rate: $activeInstance.progressiveRatesEnabled
          ? parseFloat($activeInstance.targetDownloadRate ?? 200)
          : null,
        progressive_duration: parseFloat($activeInstance.progressiveDurationHours ?? 1) * 3600,
      };

      await api.startFaker($activeInstance.id, $activeInstance.torrent, fakerConfig);

      // Update instance status
      instanceActions.updateInstance($activeInstance.id, {
        isRunning: true,
        isPaused: false,
        statusMessage: 'Actively faking ratio...',
        statusType: 'running',
        statusIcon: 'rocket',
      });

      // Start polling intervals for UI updates
      const intervalSeconds = $activeInstance.updateIntervalSeconds ?? 5;
      startPollingForInstance($activeInstance.id, intervalSeconds);

      // Get initial stats
      const initialStats = await api.getStats($activeInstance.id);
      instanceActions.updateInstance($activeInstance.id, { stats: initialStats });
    } catch (error) {
      instanceActions.updateInstance($activeInstance.id, {
        statusMessage: 'Failed to start: ' + error,
        statusType: 'error',
      });
      alert('Failed to start faker: ' + error);
    }
  }

  // Stop faking
  async function stopFaking() {
    if (!$activeInstance) {
      alert('No active instance');
      return;
    }

    try {
      instanceActions.updateInstance($activeInstance.id, {
        statusMessage: 'Stopping faker...',
        statusType: 'running',
      });

      // Get final stats from backend before stopping to save cumulative totals
      let finalStats = null;
      try {
        finalStats = await api.getStats($activeInstance.id);
      } catch (error) {
        console.warn('Failed to get final stats before stopping:', error);
        finalStats = $activeInstance.stats; // Fallback to current stats
      }

      await api.stopFaker($activeInstance.id);

      // Clear intervals
      if ($activeInstance.updateInterval) {
        clearInterval($activeInstance.updateInterval);
      }
      if ($activeInstance.countdownInterval) {
        clearInterval($activeInstance.countdownInterval);
      }
      if ($activeInstance.liveStatsInterval) {
        clearInterval($activeInstance.liveStatsInterval);
      }

      // Save cumulative stats for next session
      const updates = {
        isRunning: false,
        nextUpdateIn: 0,
        updateInterval: null,
        countdownInterval: null,
        liveStatsInterval: null,
        statusMessage: 'Stopped successfully - Stats available for review',
        statusType: 'success',
      };

      // Update cumulative stats with final totals (convert bytes to MB)
      if (finalStats) {
        updates.cumulativeUploaded = Math.round(finalStats.uploaded / (1024 * 1024));
        updates.cumulativeDownloaded = Math.round(finalStats.downloaded / (1024 * 1024));
      }

      // Update instance - keep stats visible for review
      instanceActions.updateInstance($activeInstance.id, updates);

      const instanceId = $activeInstance.id;
      setTimeout(() => {
        // Only update status if the instance is not running
        const instance = $instances.find(i => i.id === instanceId);
        if (instance && !instance.isRunning) {
          instanceActions.updateInstance(instanceId, {
            statusMessage: 'Ready to start a new session',
            statusType: 'idle',
          });
        }
      }, 2000);
    } catch (error) {
      instanceActions.updateInstance($activeInstance.id, {
        statusMessage: 'Failed to stop: ' + error,
        statusType: 'error',
      });
      alert('Failed to stop faker: ' + error);
    }
  }

  // Pause faking
  async function pauseFaking() {
    if (!$activeInstance) {
      alert('No active instance');
      return;
    }

    try {
      await api.pauseFaker($activeInstance.id);
      instanceActions.updateInstance($activeInstance.id, {
        isPaused: true,
        statusMessage: 'Paused',
        statusType: 'idle',
        statusIcon: 'pause',
      });
    } catch (error) {
      devLog('error', 'Pause error:', error);
      instanceActions.updateInstance($activeInstance.id, {
        statusMessage: 'Failed to pause: ' + error,
        statusType: 'error',
      });
    }
  }

  // Resume faking
  async function resumeFaking() {
    if (!$activeInstance) {
      alert('No active instance');
      return;
    }

    try {
      await api.resumeFaker($activeInstance.id);
      instanceActions.updateInstance($activeInstance.id, {
        isPaused: false,
        statusMessage: 'Actively faking ratio...',
        statusType: 'running',
        statusIcon: 'rocket',
      });
    } catch (error) {
      devLog('error', 'Resume error:', error);
      instanceActions.updateInstance($activeInstance.id, {
        statusMessage: 'Failed to resume: ' + error,
        statusType: 'error',
      });
    }
  }

  // Start all instances with torrents loaded
  async function startAllInstances() {
    const currentInstances = get(instances);
    const instancesToStart = currentInstances.filter(inst => inst.torrent && !inst.isRunning);

    if (instancesToStart.length === 0) {
      return;
    }

    const previousActiveId = get(activeInstanceId);

    for (const instance of instancesToStart) {
      // Temporarily select this instance and start it
      instanceActions.selectInstance(instance.id);

      try {
        await startFaking();
      } catch (error) {
        console.error(`Failed to start instance ${instance.id}:`, error);
      }
    }

    // Restore previous active instance if it still exists
    const updatedInstances = get(instances);
    if (previousActiveId && updatedInstances.find(i => i.id === previousActiveId)) {
      instanceActions.selectInstance(previousActiveId);
    }
  }

  // Stop all running instances
  async function stopAllInstances() {
    const currentInstances = get(instances);
    const instancesToStop = currentInstances.filter(inst => inst.isRunning);

    if (instancesToStop.length === 0) {
      return;
    }

    const previousActiveId = get(activeInstanceId);

    for (const instance of instancesToStop) {
      // Temporarily select this instance and stop it
      instanceActions.selectInstance(instance.id);

      try {
        await stopFaking();
      } catch (error) {
        console.error(`Failed to stop instance ${instance.id}:`, error);
      }
    }

    // Restore previous active instance if it still exists
    const updatedInstances = get(instances);
    if (previousActiveId && updatedInstances.find(i => i.id === previousActiveId)) {
      instanceActions.selectInstance(previousActiveId);
    }
  }

  // Manual update
  async function manualUpdate() {
    if (!$activeInstance || !$activeInstance.isRunning) {
      return;
    }

    try {
      const isPausedBeforeUpdate = $activeInstance.isPaused;

      instanceActions.updateInstance($activeInstance.id, {
        statusMessage: 'Manually updating stats...',
        statusType: 'running',
      });

      await api.updateFaker($activeInstance.id);
      const stats = await api.getStats($activeInstance.id);

      // Restore the correct status message based on paused state
      const statusMessage = isPausedBeforeUpdate ? 'Paused' : 'Actively faking ratio...';
      const statusType = isPausedBeforeUpdate ? 'idle' : 'running';
      const statusIcon = isPausedBeforeUpdate ? 'pause' : 'rocket';

      instanceActions.updateInstance($activeInstance.id, {
        stats,
        nextUpdateIn: $activeInstance.updateIntervalSeconds ?? 5,
        statusMessage,
        statusType,
        statusIcon,
      });
    } catch (error) {
      devLog('error', 'Manual update error:', error);
      instanceActions.updateInstance($activeInstance.id, {
        statusMessage: 'Update failed: ' + error,
        statusType: 'error',
      });

      const instanceId = $activeInstance.id;
      setTimeout(() => {
        // Only update status if the instance is still running
        const instance = $instances.find(i => i.id === instanceId);
        if (instance && instance.isRunning) {
          const statusMessage = instance.isPaused ? 'Paused' : 'Actively faking ratio...';
          const statusType = instance.isPaused ? 'idle' : 'running';
          const statusIcon = instance.isPaused ? 'pause' : 'rocket';
          instanceActions.updateInstance(instanceId, {
            statusMessage,
            statusType,
            statusIcon,
          });
        }
      }, 2000);
    }
  }

  // Format bytes
  function formatBytes(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  // Format duration
  function formatDuration(seconds) {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = Math.floor(seconds % 60);
    return `${h}h ${m}m ${s}s`;
  }

  // Sync instance config to server (debounced)
  // This ensures form changes persist across page refreshes in server mode
  function syncConfigToServer(instanceId) {
    // Only sync in server mode
    if (getRunMode() !== 'server') return;

    const instance = $instances.find(i => i.id === instanceId);
    if (!instance) return;

    // Only sync if instance has a torrent loaded (exists on server) and is not running
    if (!instance.torrent || instance.isRunning) return;

    // Clear existing timeout
    if (configSyncTimeout) {
      clearTimeout(configSyncTimeout);
    }

    // Debounce: wait 500ms after last change before syncing
    configSyncTimeout = setTimeout(async () => {
      try {
        // Build FakerConfig from instance state
        const torrentSize = instance.torrent?.total_size || 0;
        const completionPercent = parseFloat(instance.completionPercent ?? 0);
        const calculatedDownloaded = Math.floor((completionPercent / 100) * torrentSize);

        const config = {
          upload_rate: parseFloat(instance.uploadRate ?? 50),
          download_rate: parseFloat(instance.downloadRate ?? 100),
          port: parseInt(instance.port ?? 6881),
          client_type: instance.selectedClient || 'qbittorrent',
          client_version:
            instance.selectedClientVersion ||
            clientVersions[instance.selectedClient || 'qbittorrent'][0],
          initial_uploaded: parseInt(instance.initialUploaded ?? 0) * 1024 * 1024,
          initial_downloaded: calculatedDownloaded,
          completion_percent: completionPercent,
          num_want: 50,
          randomize_rates: instance.randomizeRates ?? true,
          random_range_percent: parseFloat(instance.randomRangePercent ?? 20),
          stop_at_ratio: instance.stopAtRatioEnabled
            ? parseFloat(instance.stopAtRatio ?? 2.0)
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
          stop_when_no_leechers: instance.stopWhenNoLeechers ?? false,
          progressive_rates: instance.progressiveRatesEnabled ?? false,
          target_upload_rate: instance.progressiveRatesEnabled
            ? parseFloat(instance.targetUploadRate ?? 100)
            : null,
          target_download_rate: instance.progressiveRatesEnabled
            ? parseFloat(instance.targetDownloadRate ?? 200)
            : null,
          progressive_duration: parseFloat(instance.progressiveDurationHours ?? 1) * 3600,
        };

        await api.updateInstanceConfig(instanceId, config);
        devLog('log', `Synced config for instance ${instanceId} to server`);
      } catch (error) {
        console.warn('Failed to sync config to server:', error);
      }
    }, 500);
  }
</script>

{#if !isInitialized}
  <div class="flex flex-col items-center justify-center min-h-screen gap-6 bg-background">
    <div class="w-15 h-15 border-4 border-muted border-t-primary rounded-full animate-spin"></div>
    <p class="text-xl text-muted-foreground">Loading Rustatio...</p>
  </div>
{:else if showAuthDialog}
  <!-- Authentication Page (shown when auth is required but not authenticated) -->
  <AuthPage onAuthenticated={handleAuthenticated} />
{:else}
  <div class="flex h-screen bg-background text-foreground">
    <!-- Sidebar -->
    <Sidebar
      bind:isOpen={sidebarOpen}
      bind:isCollapsed={sidebarCollapsed}
      onStartAll={startAllInstances}
      onStopAll={stopAllInstances}
    />

    <!-- Main Content -->
    <div class="flex-1 flex flex-col overflow-hidden">
      <!-- Theme Toggle (Absolute Top-Right) -->
      <div class="fixed top-4 right-4 z-30 flex items-center gap-3">
        {#if !isTauri}
          <div class="hidden sm:block">
            <DownloadButton />
          </div>
        {/if}
        <div class="relative theme-selector">
          <button
            onclick={toggleThemeDropdown}
            class="bg-secondary text-secondary-foreground border-2 border-border rounded-lg p-2 flex items-center gap-2 cursor-pointer transition-all hover:bg-primary hover:border-primary hover:text-primary-foreground active:scale-[0.98] shadow-lg"
            title="Theme: {getThemeName(getTheme())}"
            aria-label="Toggle theme menu"
          >
            <ThemeIcon theme={getTheme()} />
            <ChevronDown
              size={14}
              class="transition-transform {getShowThemeDropdown() ? 'rotate-180' : ''}"
            />
          </button>
          {#if getShowThemeDropdown()}
            <div
              class="absolute top-[calc(100%+0.5rem)] right-0 bg-card text-card-foreground border border-border/50 rounded-xl shadow-2xl p-1.5 min-w-[200px] max-h-[400px] overflow-y-auto z-50 backdrop-blur-xl animate-in fade-in slide-in-from-top-2 duration-200"
            >
              {#each Object.entries(THEME_CATEGORIES) as [categoryId, category] (categoryId)}
                <!-- Category Header -->
                <div
                  class="px-3 py-1.5 text-xs font-semibold text-muted-foreground uppercase tracking-wider {categoryId !==
                  'default'
                    ? 'mt-2 border-t border-border pt-2'
                    : ''}"
                >
                  {category.name}
                </div>

                {#each category.themes as themeId (themeId)}
                  {@const themeOption = THEMES[themeId]}
                  <button
                    class="w-full flex items-center gap-3 px-3 py-2 border-none cursor-pointer rounded-lg transition-all {getTheme() ===
                    themeOption.id
                      ? 'bg-primary text-primary-foreground shadow-sm'
                      : 'bg-transparent text-card-foreground hover:bg-secondary/80'}"
                    onclick={() => selectTheme(themeOption.id)}
                  >
                    <ThemeIcon theme={themeOption.id} />
                    <div class="flex-1 text-left">
                      <span class="text-sm font-medium">{themeOption.name}</span>
                      {#if themeOption.description}
                        <span class="block text-xs opacity-70">{themeOption.description}</span>
                      {/if}
                    </div>
                    {#if getTheme() === themeOption.id}
                      <Check size={16} strokeWidth={2.5} />
                    {/if}
                  </button>
                {/each}
              {/each}
            </div>
          {/if}
        </div>
      </div>

      <!-- Header -->
      <Header onToggleSidebar={() => (sidebarOpen = !sidebarOpen)} />

      <!-- Full-width border separator -->
      <div class="border-b-2 border-primary/20"></div>

      <!-- Status Bar -->
      <StatusBar
        statusMessage={$activeInstance?.statusMessage || 'Select a torrent file to begin'}
        statusType={$activeInstance?.statusType || 'warning'}
        statusIcon={$activeInstance?.statusIcon || null}
        isRunning={$activeInstance?.isRunning || false}
        isPaused={$activeInstance?.isPaused || false}
        {startFaking}
        {stopFaking}
        {pauseFaking}
        {resumeFaking}
        {manualUpdate}
      />

      <!-- Scrollable Content Area -->
      <div class="flex-1 overflow-y-auto p-3">
        <div class="max-w-7xl mx-auto">
          <!-- CORS Proxy Settings -->
          <ProxySettings />
          <!-- Torrent Selection & Configuration -->
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3 mb-3">
            <TorrentSelector torrent={$activeInstance?.torrent} {selectTorrent} {formatBytes} />

            {#if $activeInstance}
              <ConfigurationForm
                {clients}
                {clientVersions}
                selectedClient={$activeInstance.selectedClient}
                selectedClientVersion={$activeInstance.selectedClientVersion}
                port={$activeInstance.port}
                uploadRate={$activeInstance.uploadRate}
                downloadRate={$activeInstance.downloadRate}
                completionPercent={$activeInstance.completionPercent}
                initialUploaded={$activeInstance.initialUploaded}
                updateIntervalSeconds={$activeInstance.updateIntervalSeconds}
                randomizeRates={$activeInstance.randomizeRates}
                randomRangePercent={$activeInstance.randomRangePercent}
                progressiveRatesEnabled={$activeInstance.progressiveRatesEnabled}
                targetUploadRate={$activeInstance.targetUploadRate}
                targetDownloadRate={$activeInstance.targetDownloadRate}
                progressiveDurationHours={$activeInstance.progressiveDurationHours}
                isRunning={$activeInstance.isRunning || false}
                onUpdate={updates => {
                  // Reset cumulative stats if user changes initial values
                  if (
                    updates.initialUploaded !== undefined ||
                    updates.completionPercent !== undefined
                  ) {
                    updates.cumulativeUploaded = 0;
                    updates.cumulativeDownloaded = 0;
                  }
                  instanceActions.updateInstance($activeInstance.id, updates);
                  // Sync config to server (debounced) so it persists across page refreshes
                  syncConfigToServer($activeInstance.id);
                }}
              />
            {/if}
          </div>

          <!-- Stop Conditions & Progress Bars -->
          {#if $activeInstance}
            {@const hasActiveStopCondition =
              $activeInstance.stopAtRatioEnabled ||
              $activeInstance.stopAtUploadedEnabled ||
              $activeInstance.stopAtDownloadedEnabled ||
              $activeInstance.stopAtSeedTimeEnabled}
            {@const showProgressBars = hasActiveStopCondition && $activeInstance?.stats}

            <div class="grid grid-cols-1 {showProgressBars ? 'md:grid-cols-2' : ''} gap-3 mb-3">
              <StopConditions
                stopAtRatioEnabled={$activeInstance.stopAtRatioEnabled}
                stopAtRatio={$activeInstance.stopAtRatio}
                stopAtUploadedEnabled={$activeInstance.stopAtUploadedEnabled}
                stopAtUploadedGB={$activeInstance.stopAtUploadedGB}
                stopAtDownloadedEnabled={$activeInstance.stopAtDownloadedEnabled}
                stopAtDownloadedGB={$activeInstance.stopAtDownloadedGB}
                stopAtSeedTimeEnabled={$activeInstance.stopAtSeedTimeEnabled}
                stopAtSeedTimeHours={$activeInstance.stopAtSeedTimeHours}
                stopWhenNoLeechers={$activeInstance.stopWhenNoLeechers}
                isRunning={$activeInstance.isRunning || false}
                onUpdate={updates => {
                  instanceActions.updateInstance($activeInstance.id, updates);
                  // Sync config to server (debounced) so it persists across page refreshes
                  syncConfigToServer($activeInstance.id);
                }}
              />

              {#if showProgressBars}
                <ProgressBars
                  stats={$activeInstance.stats}
                  stopAtRatioEnabled={$activeInstance.stopAtRatioEnabled}
                  stopAtRatio={$activeInstance.stopAtRatio}
                  stopAtUploadedEnabled={$activeInstance.stopAtUploadedEnabled}
                  stopAtUploadedGB={$activeInstance.stopAtUploadedGB}
                  stopAtDownloadedEnabled={$activeInstance.stopAtDownloadedEnabled}
                  stopAtDownloadedGB={$activeInstance.stopAtDownloadedGB}
                  stopAtSeedTimeEnabled={$activeInstance.stopAtSeedTimeEnabled}
                  stopAtSeedTimeHours={$activeInstance.stopAtSeedTimeHours}
                  {formatBytes}
                  {formatDuration}
                />
              {/if}
            </div>
          {/if}

          <!-- Stats -->
          {#if $activeInstance?.stats}
            <!-- Session & Total Stats -->
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3 mb-3">
              <SessionStats stats={$activeInstance.stats} {formatBytes} {formatDuration} />
              <TotalStats
                stats={$activeInstance.stats}
                torrent={$activeInstance.torrent}
                {formatBytes}
              />
            </div>

            <!-- Performance & Peer Analytics (merged) -->
            <div class="mb-3">
              <RateGraph stats={$activeInstance.stats} {formatDuration} />
            </div>
          {/if}

          <!-- Logs Section -->
          <Logs
            bind:logs
            bind:showLogs
            onUpdate={async updates => {
              if (updates.showLogs !== undefined) {
                showLogs = updates.showLogs;
                localStorage.setItem('rustatio-show-logs', JSON.stringify(updates.showLogs));
              }
            }}
          />
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- Update Checker (only shown in Tauri) -->
<UpdateChecker />
