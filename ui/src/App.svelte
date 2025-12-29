<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { listen } from '@tauri-apps/api/event';
  import { get } from 'svelte/store';

  // Import instance stores
  import {
    instances,
    activeInstance,
    instanceActions,
    globalConfig,
    saveSession,
  } from './lib/instanceStore.js';

  // Import components
  import Header from './components/Header.svelte';
  import InstanceTabs from './components/InstanceTabs.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import TorrentSelector from './components/TorrentSelector.svelte';
  import ConfigurationForm from './components/ConfigurationForm.svelte';
  import StopConditions from './components/StopConditions.svelte';
  import Controls from './components/Controls.svelte';
  import ProgressBars from './components/ProgressBars.svelte';
  import SessionStats from './components/SessionStats.svelte';
  import TotalStats from './components/TotalStats.svelte';
  import RateGraph from './components/RateGraph.svelte';
  import Logs from './components/Logs.svelte';

  // Global config (shared across instances)
  let config = $state(null);

  // Loading state to prevent UI flash during initialization
  let isInitialized = $state(false);

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

  // Theme management
  let theme = $state('system'); // system, light, dark
  let effectiveTheme = $state('light'); // The actual applied theme
  let showThemeDropdown = $state(false);

  // Logs
  let logs = $state([]);
  let showLogs = $state(false);
  let logUnlisten = null;

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
      // Load global config
      config = await invoke('get_config');
      showLogs = config.ui.show_logs || false;

      // Set global config in store for new instances
      globalConfig.set(config);

      // Initialize instance store (will restore session or create first instance with config defaults)
      await instanceActions.initialize();

      // Wait a tick for stores to update before showing UI
      await new Promise(resolve => setTimeout(resolve, 0));

      // Mark as initialized to show UI
      isInitialized = true;
    } catch (error) {
      console.error('Failed to initialize app:', error);
      devLog('error', 'Failed to load config:', error);
      // Still show UI even if there's an error
      isInitialized = true;
    }

    // Initialize theme
    initializeTheme();

    // Listen for log events from Rust backend
    try {
      logUnlisten = await listen('log-event', event => {
        const logData = event.payload;
        logs = [...logs, logData];
      });
    } catch (error) {
      console.warn('Failed to set up log listener:', error);
    }

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

      // Update port when client changes (only if not running)
      if (inst.selectedClient && !inst.isRunning && clientDefaultPorts[inst.selectedClient]) {
        instanceActions.updateInstance(inst.id, {
          port: clientDefaultPorts[inst.selectedClient],
        });
      }
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

    // Wait for stores to settle before allowing saves
    // This prevents initial subscription fires from triggering saves during app load
    await new Promise(resolve => setTimeout(resolve, 100));

    // Mark initialization as complete
    isInitializing = false;
  });

  // Config is saved via saveSession in instanceStore.js

  // Initialize theme system
  function initializeTheme() {
    const savedTheme = localStorage.getItem('rustatio-theme') || 'system';
    theme = savedTheme;
    applyTheme(savedTheme);

    if (window.matchMedia) {
      const darkModeQuery = window.matchMedia('(prefers-color-scheme: dark)');
      darkModeQuery.addEventListener('change', e => {
        if (theme === 'system') {
          effectiveTheme = e.matches ? 'dark' : 'light';
          if (effectiveTheme === 'dark') {
            document.documentElement.classList.add('dark');
            document.documentElement.style.colorScheme = 'dark';
            console.log('🔄 System theme changed to dark mode');
          } else {
            document.documentElement.classList.remove('dark');
            document.documentElement.style.colorScheme = 'light';
            console.log('🔄 System theme changed to light mode');
          }
          document.documentElement.setAttribute('data-theme', effectiveTheme);
        }
      });
    }
  }

  // Apply theme
  function applyTheme(newTheme) {
    theme = newTheme;
    localStorage.setItem('rustatio-theme', newTheme);

    if (newTheme === 'system') {
      if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
        effectiveTheme = 'dark';
      } else {
        effectiveTheme = 'light';
      }
    } else {
      effectiveTheme = newTheme;
    }

    // Apply dark class for Tailwind
    if (effectiveTheme === 'dark') {
      document.documentElement.classList.add('dark');
      document.documentElement.style.colorScheme = 'dark';
      console.log('✅ Dark mode enabled - added .dark class and color-scheme to <html>');
    } else {
      document.documentElement.classList.remove('dark');
      document.documentElement.style.colorScheme = 'light';
      console.log('☀️ Light mode enabled - removed .dark class and set color-scheme to light');
    }

    // Keep data-theme for backwards compatibility with remaining components
    document.documentElement.setAttribute('data-theme', effectiveTheme);

    // Debug: Check computed background color
    const bg = getComputedStyle(document.documentElement).getPropertyValue('--color-background');
    console.log('Current --color-background value:', bg);
  }

  // Theme selection
  function selectTheme(newTheme) {
    applyTheme(newTheme);
    showThemeDropdown = false;
  }

  function toggleThemeDropdown(event) {
    event.stopPropagation();
    showThemeDropdown = !showThemeDropdown;
  }

  // Handle click outside to close dropdown
  function handleClickOutside(event) {
    if (showThemeDropdown) {
      const themeSelector = document.querySelector('.theme-selector');
      if (themeSelector && !themeSelector.contains(event.target)) {
        showThemeDropdown = false;
      }
    }
  }

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

    // Clean up event listeners
    if (logUnlisten) {
      logUnlisten();
    }
    document.removeEventListener('click', handleClickOutside);
  });

  // Select torrent file
  async function selectTorrent() {
    if (!$activeInstance) {
      alert('No active instance');
      return;
    }

    try {
      instanceActions.updateInstance($activeInstance.id, {
        statusMessage: 'Opening file dialog...',
        statusType: 'running',
      });

      const selected = await open({
        multiple: false,
        filters: [
          {
            name: 'Torrent',
            extensions: ['torrent'],
          },
        ],
      });

      if (selected) {
        instanceActions.updateInstance($activeInstance.id, {
          statusMessage: 'Loading torrent...',
          statusType: 'running',
        });

        const torrent = await invoke('load_torrent', {
          instanceId: $activeInstance.id,
          path: selected,
        });
        devLog('log', 'Loaded torrent:', torrent);

        // Update instance with torrent info
        instanceActions.updateInstance($activeInstance.id, {
          torrent,
          torrentPath: selected,
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
      } else {
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
      }
    } catch (error) {
      instanceActions.updateInstance($activeInstance.id, {
        statusMessage: 'Failed to load torrent: ' + error,
        statusType: 'error',
      });
      alert('Failed to load torrent: ' + error);
    }
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
      // Use the initial uploaded/downloaded values from the form inputs
      // Do NOT preserve stats from previous session - each session should start fresh
      // This ensures stop conditions work correctly and don't trigger immediately
      const initialUploaded = parseInt($activeInstance.initialUploaded ?? 0) * 1024 * 1024;
      const initialDownloaded = parseInt($activeInstance.initialDownloaded ?? 0) * 1024 * 1024;

      // Reset stats and progress bars when starting a new session
      // This ensures stop conditions and progress indicators start fresh
      instanceActions.updateInstance($activeInstance.id, {
        stats: null,
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
        initial_uploaded: initialUploaded,
        initial_downloaded: initialDownloaded,
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
        progressive_rates: $activeInstance.progressiveRatesEnabled ?? false,
        target_upload_rate: $activeInstance.progressiveRatesEnabled
          ? parseFloat($activeInstance.targetUploadRate ?? 100)
          : null,
        target_download_rate: $activeInstance.progressiveRatesEnabled
          ? parseFloat($activeInstance.targetDownloadRate ?? 200)
          : null,
        progressive_duration: parseFloat($activeInstance.progressiveDurationHours ?? 1) * 3600,
      };

      await invoke('start_faker', {
        instanceId: $activeInstance.id,
        torrent: $activeInstance.torrent,
        config: fakerConfig,
      });

      // Update instance status
      instanceActions.updateInstance($activeInstance.id, {
        isRunning: true,
        isPaused: false,
        nextUpdateIn: $activeInstance.updateIntervalSeconds ?? 5,
        statusMessage: '🚀 Actively faking ratio...',
        statusType: 'running',
      });

      const intervalMs = ($activeInstance.updateIntervalSeconds ?? 5) * 1000;
      const instanceId = $activeInstance.id;

      const updateIntervalId = setInterval(async () => {
        const currentInstance = $instances.find(i => i.id === instanceId);
        if (!currentInstance || !currentInstance.isRunning) {
          devLog('log', 'Update skipped - not running');
          return;
        }

        if (currentInstance.isPaused) {
          devLog('log', 'Update skipped - paused');
          instanceActions.updateInstance(instanceId, {
            nextUpdateIn: currentInstance.updateIntervalSeconds ?? 5,
          });
          return;
        }

        try {
          await invoke('update_faker', { instanceId });
          const stats = await invoke('get_stats', { instanceId });

          instanceActions.updateInstance(instanceId, {
            stats,
            nextUpdateIn: currentInstance.updateIntervalSeconds ?? 5,
          });

          if (stats.state === 'Stopped' || stats.state === 'Completed') {
            devLog('log', 'Faker stopped automatically:', stats.state);

            const instance = $instances.find(i => i.id === instanceId);
            if (instance) {
              if (instance.updateInterval) clearInterval(instance.updateInterval);
              if (instance.countdownInterval) clearInterval(instance.countdownInterval);
              if (instance.liveStatsInterval) clearInterval(instance.liveStatsInterval);
            }

            instanceActions.updateInstance(instanceId, {
              isRunning: false,
              nextUpdateIn: 0,
              updateInterval: null,
              countdownInterval: null,
              liveStatsInterval: null,
              statusMessage: 'Stopped automatically - condition met',
              statusType: 'success',
            });
          }
        } catch (error) {
          devLog('error', 'Update error:', error);

          const instance = $instances.find(i => i.id === instanceId);
          if (instance) {
            if (instance.updateInterval) clearInterval(instance.updateInterval);
            if (instance.countdownInterval) clearInterval(instance.countdownInterval);
            if (instance.liveStatsInterval) clearInterval(instance.liveStatsInterval);
          }

          instanceActions.updateInstance(instanceId, {
            isRunning: false,
            updateInterval: null,
            countdownInterval: null,
            liveStatsInterval: null,
            statusMessage: 'Error: ' + error,
            statusType: 'error',
          });
        }
      }, intervalMs);

      // Start live stats update every 1 second
      const liveStatsIntervalId = setInterval(async () => {
        const currentInstance = $instances.find(i => i.id === instanceId);
        if (!currentInstance || !currentInstance.isRunning || currentInstance.isPaused) {
          return;
        }

        try {
          const latestStats = await invoke('update_stats_only', { instanceId });

          if (latestStats && currentInstance.isRunning) {
            instanceActions.updateInstance(instanceId, { stats: latestStats });

            if (latestStats.state === 'Stopped' || latestStats.state === 'Completed') {
              devLog('log', 'Faker stopped (detected in live stats):', latestStats.state);

              const instance = $instances.find(i => i.id === instanceId);
              if (instance) {
                if (instance.updateInterval) clearInterval(instance.updateInterval);
                if (instance.countdownInterval) clearInterval(instance.countdownInterval);
                if (instance.liveStatsInterval) clearInterval(instance.liveStatsInterval);
              }

              instanceActions.updateInstance(instanceId, {
                isRunning: false,
                nextUpdateIn: 0,
                updateInterval: null,
                countdownInterval: null,
                liveStatsInterval: null,
                statusMessage: 'Stopped automatically - condition met',
                statusType: 'success',
              });
            }
          }
        } catch (error) {
          console.debug('Live stats fetch error:', error);
        }
      }, 1000);

      // Start countdown timer
      const countdownIntervalId = setInterval(() => {
        const currentInstance = $instances.find(i => i.id === instanceId);
        if (currentInstance && !currentInstance.isPaused && currentInstance.nextUpdateIn > 0) {
          instanceActions.updateInstance(instanceId, {
            nextUpdateIn: currentInstance.nextUpdateIn - 1,
          });
        }
      }, 1000);

      // Store interval IDs in instance
      instanceActions.updateInstance($activeInstance.id, {
        updateInterval: updateIntervalId,
        liveStatsInterval: liveStatsIntervalId,
        countdownInterval: countdownIntervalId,
      });

      // Get initial stats
      const initialStats = await invoke('get_stats', { instanceId: $activeInstance.id });
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

      await invoke('stop_faker', { instanceId: $activeInstance.id });

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

      // Update instance - keep stats visible for review
      instanceActions.updateInstance($activeInstance.id, {
        isRunning: false,
        nextUpdateIn: 0,
        updateInterval: null,
        countdownInterval: null,
        liveStatsInterval: null,
        statusMessage: 'Stopped successfully - Stats available for review',
        statusType: 'success',
      });

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
      await invoke('pause_faker', { instanceId: $activeInstance.id });
      instanceActions.updateInstance($activeInstance.id, {
        isPaused: true,
        statusMessage: '⏸️ Paused',
        statusType: 'idle',
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
      await invoke('resume_faker', { instanceId: $activeInstance.id });
      instanceActions.updateInstance($activeInstance.id, {
        isPaused: false,
        statusMessage: '🚀 Actively faking ratio...',
        statusType: 'running',
      });
    } catch (error) {
      devLog('error', 'Resume error:', error);
      instanceActions.updateInstance($activeInstance.id, {
        statusMessage: 'Failed to resume: ' + error,
        statusType: 'error',
      });
    }
  }

  // Manual update
  async function manualUpdate() {
    if (!$activeInstance || !$activeInstance.isRunning) {
      return;
    }

    try {
      instanceActions.updateInstance($activeInstance.id, {
        statusMessage: 'Manually updating stats...',
        statusType: 'running',
      });

      await invoke('update_faker', { instanceId: $activeInstance.id });
      const stats = await invoke('get_stats', { instanceId: $activeInstance.id });
      instanceActions.updateInstance($activeInstance.id, {
        stats,
        nextUpdateIn: $activeInstance.updateIntervalSeconds ?? 5,
        statusMessage: '🚀 Actively faking ratio...',
        statusType: 'running',
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
          instanceActions.updateInstance(instanceId, {
            statusMessage: '🚀 Actively faking ratio...',
            statusType: 'running',
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

  function getThemeName(themeType) {
    if (themeType === 'dark') return 'Dark';
    if (themeType === 'light') return 'Light';
    return 'System';
  }
</script>

<main class="min-h-screen p-3 bg-background text-foreground">
  {#if !isInitialized}
    <div class="flex flex-col items-center justify-center min-h-[60vh] gap-6">
      <div class="w-15 h-15 border-4 border-muted border-t-primary rounded-full animate-spin"></div>
      <p class="text-xl text-muted-foreground">Loading Rustatio...</p>
    </div>
  {:else}
    <Header
      {theme}
      {showThemeDropdown}
      {getThemeName}
      {toggleThemeDropdown}
      {selectTheme}
      isRunning={$activeInstance?.isRunning || false}
      isPaused={$activeInstance?.isPaused || false}
      {startFaking}
      {stopFaking}
      {pauseFaking}
      {resumeFaking}
      {manualUpdate}
    />

    <InstanceTabs />

    <StatusBar
      statusMessage={$activeInstance?.statusMessage || 'Select a torrent file to begin'}
      statusType={$activeInstance?.statusType || 'warning'}
    />

    <div class="max-w-7xl mx-auto">
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
              instanceActions.updateInstance($activeInstance.id, updates);
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
            isRunning={$activeInstance.isRunning || false}
            onUpdate={updates => {
              instanceActions.updateInstance($activeInstance.id, updates);
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

      <!-- Controls -->
      <Controls
        isRunning={$activeInstance?.isRunning || false}
        isPaused={$activeInstance?.isPaused || false}
        nextUpdateIn={$activeInstance?.nextUpdateIn || 0}
        {startFaking}
        {stopFaking}
        {pauseFaking}
        {resumeFaking}
        {manualUpdate}
      />

      <!-- Stats -->
      {#if $activeInstance?.stats}
        <!-- Session & Total Stats -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-3 mb-3">
          <SessionStats stats={$activeInstance.stats} {formatBytes} />
          <TotalStats stats={$activeInstance.stats} {formatBytes} />
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
            config.ui.show_logs = updates.showLogs;
            try {
              await invoke('update_config', { config });
            } catch (error) {
              console.error('Failed to save logs config:', error);
            }
          }
        }}
      />
    </div>
  {/if}
</main>
