<script>
  import { instances, activeInstanceId, instanceActions } from '../lib/instanceStore.js';
  import { get } from 'svelte/store';
  import Button from '$lib/components/ui/button.svelte';
  import { builtInPresets } from '$lib/presets/index.js';

  let { isOpen = $bindable(false) } = $props();

  // Check if running in Tauri
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  // Subscribe to stores for reactivity
  let currentInstances = $state([]);
  let currentActiveId = $state(null);

  instances.subscribe(value => (currentInstances = value));
  activeInstanceId.subscribe(value => (currentActiveId = value));

  // Tab state
  let activeTab = $state('general');

  // Log level state (stored in localStorage)
  const LOG_LEVEL_KEY = 'rustatio-log-level';
  let logLevel = $state(localStorage.getItem(LOG_LEVEL_KEY) || 'info');

  function saveLogLevel(level) {
    logLevel = level;
    localStorage.setItem(LOG_LEVEL_KEY, level);
  }

  // Custom presets stored in localStorage
  const CUSTOM_PRESETS_KEY = 'rustatio-custom-presets';

  function loadCustomPresets() {
    try {
      const stored = localStorage.getItem(CUSTOM_PRESETS_KEY);
      return stored ? JSON.parse(stored) : [];
    } catch {
      return [];
    }
  }

  function saveCustomPresets(presets) {
    localStorage.setItem(CUSTOM_PRESETS_KEY, JSON.stringify(presets));
  }

  let customPresets = $state(loadCustomPresets());

  // Detection avoidance tips
  const detectionTips = [
    {
      title: 'Use a VPN',
      description:
        'Always use a VPN to hide your real IP. Trackers can correlate your IP with multiple torrents and detect anomalies.',
      importance: 'critical',
    },
    {
      title: 'Match Your History',
      description:
        "If you've always used qBittorrent, don't suddenly switch to uTorrent. Stick with the client you've historically used.",
      importance: 'high',
    },
    {
      title: 'Realistic Rates',
      description:
        "Don't set upload rates higher than your actual internet connection supports. A 10 Mbps connection shouldn't seed at 50 MB/s.",
      importance: 'high',
    },
    {
      title: 'Enable Randomization',
      description:
        'Real torrent transfers have variable speeds. Static rates are a red flag. Always enable rate randomization.',
      importance: 'high',
    },
    {
      title: 'Use Progressive Rates',
      description:
        'Real peers discover each other gradually. Starting at full speed is unnatural. Enable progressive rate adjustment.',
      importance: 'medium',
    },
    {
      title: 'Avoid Round Numbers',
      description:
        'Rates like exactly 100 KB/s or 50 KB/s look suspicious. The randomization feature helps avoid this, but consider setting base rates like 47 or 103 KB/s.',
      importance: 'medium',
    },
    {
      title: 'Match Completion State',
      description:
        'If faking a download, set completion percent appropriately. Reporting 0% downloaded while uploading lots is suspicious.',
      importance: 'medium',
    },
    {
      title: "Don't Overdo It",
      description:
        'Building ratio slowly over time is safer than hitting 10x ratio in a day. Set stop conditions to limit your session ratio.',
      importance: 'medium',
    },
  ];

  function close() {
    isOpen = false;
  }

  function handleBackdropClick(event) {
    if (event.target === event.currentTarget) {
      close();
    }
  }

  function applyPreset(preset) {
    const active = get(activeInstanceId);
    if (active !== null) {
      instanceActions.updateInstance(active, preset.settings);
    }
    close();
  }

  // Check if a preset matches the current instance settings
  function isPresetApplied(preset, instance) {
    if (!instance) return false;

    // Compare all settings in the preset with the instance
    for (const [key, value] of Object.entries(preset.settings)) {
      // Handle numeric comparisons with tolerance for floating point
      if (typeof value === 'number' && typeof instance[key] === 'number') {
        if (Math.abs(instance[key] - value) > 0.001) return false;
      } else if (instance[key] !== value) {
        return false;
      }
    }
    return true;
  }

  // Reactive check for applied presets - updates when instances or customPresets change
  let appliedPresetId = $derived.by(() => {
    // Access all reactive dependencies explicitly
    const instances = currentInstances;
    const activeId = currentActiveId;
    const custom = customPresets; // Must access to make reactive

    if (!instances || activeId === null) return null;

    const instance = instances.find(i => i.id === activeId);
    if (!instance) return null;

    // Check built-in presets
    for (const preset of builtInPresets) {
      if (isPresetApplied(preset, instance)) return preset.id;
    }
    // Check custom presets
    for (const preset of custom) {
      if (isPresetApplied(preset, instance)) return preset.id;
    }
    return null;
  });

  // Export current config as a custom preset
  let exportError = $state('');
  let exportSuccess = $state('');
  let showExportDialog = $state(false);
  let exportPresetName = $state('');
  let exportPresetDescription = $state('');

  function openExportDialog() {
    exportError = '';
    exportSuccess = '';
    exportPresetName = '';
    exportPresetDescription = '';

    const active = get(activeInstanceId);
    if (active === null) {
      exportError = 'No active instance. Select an instance first.';
      return;
    }

    showExportDialog = true;
  }

  async function exportPreset() {
    exportError = '';
    exportSuccess = '';

    if (!exportPresetName.trim()) {
      exportError = 'Please enter a preset name.';
      return;
    }

    const active = get(activeInstanceId);
    if (active === null) {
      exportError = 'No active instance. Select an instance first.';
      return;
    }

    const currentInstances = get(instances);
    const instance = currentInstances.find(i => i.id === active);
    if (!instance) {
      exportError = 'Instance not found.';
      return;
    }

    const presetData = {
      version: 1,
      type: 'rustatio-preset',
      name: exportPresetName.trim(),
      description:
        exportPresetDescription.trim() ||
        `Custom preset created on ${new Date().toLocaleDateString()}`,
      icon: '⭐',
      createdAt: new Date().toISOString(),
      settings: {
        selectedClient: instance.selectedClient,
        selectedClientVersion: instance.selectedClientVersion,
        uploadRate: instance.uploadRate,
        downloadRate: instance.downloadRate,
        port: instance.port,
        completionPercent: instance.completionPercent,
        randomizeRates: instance.randomizeRates,
        randomRangePercent: instance.randomRangePercent,
        updateIntervalSeconds: instance.updateIntervalSeconds,
        progressiveRatesEnabled: instance.progressiveRatesEnabled,
        targetUploadRate: instance.targetUploadRate,
        targetDownloadRate: instance.targetDownloadRate,
        progressiveDurationHours: instance.progressiveDurationHours,
        // Stop conditions
        stopAtRatioEnabled: instance.stopAtRatioEnabled,
        stopAtRatio: instance.stopAtRatio,
        stopAtUploadedEnabled: instance.stopAtUploadedEnabled,
        stopAtUploadedGB: instance.stopAtUploadedGB,
        stopAtDownloadedEnabled: instance.stopAtDownloadedEnabled,
        stopAtDownloadedGB: instance.stopAtDownloadedGB,
        stopAtSeedTimeEnabled: instance.stopAtSeedTimeEnabled,
        stopAtSeedTimeHours: instance.stopAtSeedTimeHours,
        stopWhenNoLeechers: instance.stopWhenNoLeechers,
      },
    };

    // Create a safe filename from the preset name
    const safeFilename = exportPresetName
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '');
    const defaultFilename = `rustatio-preset-${safeFilename || 'custom'}.json`;
    const jsonString = JSON.stringify(presetData, null, 2);

    if (isTauri) {
      // Use Tauri save dialog + write_file command
      try {
        const { save } = await import('@tauri-apps/plugin-dialog');
        const filePath = await save({
          defaultPath: defaultFilename,
          filters: [{ name: 'JSON', extensions: ['json'] }],
        });

        if (filePath) {
          const { invoke } = await import('@tauri-apps/api/core');
          await invoke('write_file', { path: filePath, contents: jsonString });
          exportSuccess = 'Config exported successfully';
          showExportDialog = false;
        }
      } catch (err) {
        console.error('Export failed:', err);
        exportError = `Export failed: ${err.message}`;
      }
    } else {
      // Browser: use download with suggested filename
      try {
        const blob = new Blob([jsonString], { type: 'application/json' });
        const url = URL.createObjectURL(blob);

        const a = document.createElement('a');
        a.href = url;
        a.download = defaultFilename;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);

        exportSuccess = 'Config exported successfully';
        showExportDialog = false;
      } catch (err) {
        console.error('Export failed:', err);
        exportError = `Export failed: ${err.message}`;
      }
    }
  }

  // Import preset from file
  let fileInput = $state(null);
  let importError = $state('');
  let importSuccess = $state('');

  function triggerImport() {
    fileInput?.click();
  }

  async function handleFileImport(event) {
    const file = event.target.files?.[0];
    if (!file) return;

    importError = '';
    importSuccess = '';

    try {
      const text = await file.text();
      const data = JSON.parse(text);

      // Validate preset structure
      if (data.type !== 'rustatio-preset' || !data.settings) {
        throw new Error('Invalid preset file format');
      }

      // Create custom preset object
      const newPreset = {
        id: `custom-${Date.now()}`,
        name: data.name || 'Imported Preset',
        description: data.description || 'Imported custom preset',
        icon: data.icon || '📁',
        custom: true,
        createdAt: data.createdAt || new Date().toISOString(),
        settings: data.settings,
      };

      // Add to custom presets
      customPresets = [...customPresets, newPreset];
      saveCustomPresets(customPresets);

      importSuccess = `Preset "${newPreset.name}" imported successfully`;
    } catch (err) {
      importError = `Failed to import: ${err.message}`;
    }

    // Reset file input
    if (fileInput) fileInput.value = '';
  }

  function deleteCustomPreset(presetId) {
    customPresets = customPresets.filter(p => p.id !== presetId);
    saveCustomPresets(customPresets);
  }

  function getImportanceColor(importance) {
    switch (importance) {
      case 'critical':
        return 'text-red-500 bg-red-500/10';
      case 'high':
        return 'text-amber-500 bg-amber-500/10';
      case 'medium':
        return 'text-blue-500 bg-blue-500/10';
      default:
        return 'text-muted-foreground bg-muted';
    }
  }

  function getImportanceLabel(importance) {
    switch (importance) {
      case 'critical':
        return 'Critical';
      case 'high':
        return 'Important';
      case 'medium':
        return 'Recommended';
      default:
        return 'Tip';
    }
  }
</script>

{#if isOpen}
  <div
    class="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4"
    onclick={handleBackdropClick}
    onkeydown={e => e.key === 'Escape' && close()}
    role="dialog"
    aria-modal="true"
    aria-labelledby="settings-title"
    tabindex="-1"
  >
    <div
      class="bg-card text-card-foreground rounded-xl shadow-2xl max-w-2xl w-full max-h-[85vh] flex flex-col border border-border animate-in fade-in zoom-in-95 duration-200"
    >
      <!-- Header -->
      <div class="flex items-start justify-between p-6 border-b border-border flex-shrink-0">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 bg-primary/10 rounded-lg flex items-center justify-center">
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              class="text-primary"
            >
              <circle cx="12" cy="12" r="3"></circle>
              <path
                d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
              ></path>
            </svg>
          </div>
          <div>
            <h2 id="settings-title" class="text-xl font-bold text-foreground">Settings</h2>
            <p class="text-sm text-muted-foreground">Presets and configuration</p>
          </div>
        </div>
        <button
          onclick={close}
          class="p-1 rounded hover:bg-muted transition-colors"
          aria-label="Close dialog"
        >
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>

      <!-- Tabs -->
      <div class="flex border-b border-border flex-shrink-0">
        <button
          class="flex-1 px-4 py-3 text-sm font-medium transition-colors {activeTab === 'general'
            ? 'text-primary border-b-2 border-primary bg-primary/5'
            : 'text-muted-foreground hover:text-foreground'}"
          onclick={() => (activeTab = 'general')}
        >
          General
        </button>
        <button
          class="flex-1 px-4 py-3 text-sm font-medium transition-colors {activeTab === 'presets'
            ? 'text-primary border-b-2 border-primary bg-primary/5'
            : 'text-muted-foreground hover:text-foreground'}"
          onclick={() => (activeTab = 'presets')}
        >
          Presets
        </button>
        <button
          class="flex-1 px-4 py-3 text-sm font-medium transition-colors {activeTab === 'tips'
            ? 'text-primary border-b-2 border-primary bg-primary/5'
            : 'text-muted-foreground hover:text-foreground'}"
          onclick={() => (activeTab = 'tips')}
        >
          Detection Tips
        </button>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-y-auto p-6">
        {#if activeTab === 'general'}
          <!-- General Settings Tab -->
          <div class="space-y-6">
            <p class="text-sm text-muted-foreground mb-4">
              Configure general application settings.
            </p>

            <!-- Log Level Section -->
            <div class="border border-border rounded-lg p-4">
              <h3 class="font-semibold text-foreground mb-2">Log Level</h3>
              <p class="text-sm text-muted-foreground mb-4">
                Set the verbosity of logs displayed in the console. Higher levels show more detailed
                information for debugging.
              </p>
              <div class="flex items-center gap-4">
                <label for="logLevel" class="text-sm font-medium min-w-[60px]">Level</label>
                <select
                  id="logLevel"
                  value={logLevel}
                  onchange={e => saveLogLevel(e.target.value)}
                  class="px-3 py-2 text-sm border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary/50 w-40"
                >
                  <option value="error">Error</option>
                  <option value="warn">Warning</option>
                  <option value="info">Info</option>
                  <option value="debug">Debug</option>
                  <option value="trace">Trace</option>
                </select>
              </div>
              <div class="mt-3 text-xs text-muted-foreground space-y-1">
                <p><strong>Error:</strong> Only critical errors</p>
                <p><strong>Warning:</strong> Errors and warnings</p>
                <p><strong>Info:</strong> General information (default)</p>
                <p><strong>Debug:</strong> Detailed debugging information</p>
                <p><strong>Trace:</strong> Very verbose, includes all internal operations</p>
              </div>
              <p class="mt-3 text-xs text-muted-foreground italic">
                Note: Log level changes apply to the backend. In desktop/server mode, you may need
                to restart for changes to take effect.
              </p>
            </div>
          </div>
        {:else if activeTab === 'presets'}
          <!-- Presets Tab -->
          <div class="space-y-6">
            <!-- Built-in Presets -->
            <div>
              <h3 class="text-sm font-semibold text-muted-foreground uppercase tracking-wider mb-3">
                Built-in Presets
              </h3>
              <div class="space-y-3">
                {#each builtInPresets as preset (preset.id)}
                  <div
                    class="border border-border rounded-lg p-4 hover:border-primary/50 transition-colors {preset.recommended
                      ? 'ring-1 ring-primary/30'
                      : ''}"
                  >
                    <div class="flex items-start justify-between gap-4">
                      <div class="flex-1">
                        <div class="flex items-center gap-2 mb-1">
                          <span class="text-xl">{preset.icon}</span>
                          <h3 class="font-semibold text-foreground">{preset.name}</h3>
                          {#if preset.recommended}
                            <span
                              class="text-xs px-2 py-0.5 rounded-full bg-primary/20 text-primary font-medium"
                            >
                              Recommended
                            </span>
                          {/if}
                        </div>
                        <p class="text-sm text-muted-foreground mb-3">{preset.description}</p>

                        <!-- Settings preview -->
                        <div class="flex flex-wrap gap-2 text-xs mb-3">
                          <span class="px-2 py-1 bg-muted rounded"
                            >↑ {preset.settings.uploadRate} KB/s</span
                          >
                          <span class="px-2 py-1 bg-muted rounded"
                            >↓ {preset.settings.downloadRate} KB/s</span
                          >
                          {#if preset.settings.randomizeRates}
                            <span class="px-2 py-1 bg-muted rounded"
                              >±{preset.settings.randomRangePercent}%</span
                            >
                          {/if}
                          {#if preset.settings.progressiveRatesEnabled}
                            <span class="px-2 py-1 bg-green-500/20 text-green-500 rounded"
                              >Progressive</span
                            >
                          {/if}
                          {#if preset.settings.selectedClient}
                            <span
                              class="px-2 py-1 bg-purple-500/20 text-purple-500 rounded capitalize"
                              >{preset.settings.selectedClient}</span
                            >
                          {/if}
                          <!-- Stop conditions -->
                          {#if preset.settings.stopAtRatioEnabled}
                            <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                              >Stop @ {preset.settings.stopAtRatio}x</span
                            >
                          {/if}
                          {#if preset.settings.stopAtUploadedEnabled}
                            <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                              >Stop @ {preset.settings.stopAtUploadedGB} GB ↑</span
                            >
                          {/if}
                          {#if preset.settings.stopAtDownloadedEnabled}
                            <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                              >Stop @ {preset.settings.stopAtDownloadedGB} GB ↓</span
                            >
                          {/if}
                          {#if preset.settings.stopAtSeedTimeEnabled}
                            <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                              >Stop @ {preset.settings.stopAtSeedTimeHours}h</span
                            >
                          {/if}
                          {#if preset.settings.stopWhenNoLeechers}
                            <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                              >Stop: No Leechers</span
                            >
                          {/if}
                        </div>

                        <!-- Tips -->
                        <details class="text-xs">
                          <summary
                            class="cursor-pointer text-muted-foreground hover:text-foreground transition-colors"
                          >
                            Why these settings?
                          </summary>
                          <ul class="mt-2 space-y-1 text-muted-foreground pl-4">
                            {#each preset.tips as tip, tipIndex (tipIndex)}
                              <li class="list-disc">{tip}</li>
                            {/each}
                          </ul>
                        </details>
                      </div>

                      {#if appliedPresetId === preset.id}
                        <span
                          class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-semibold rounded-lg bg-green-500/20 text-green-500"
                        >
                          <svg
                            width="14"
                            height="14"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2.5"
                          >
                            <polyline points="20 6 9 17 4 12"></polyline>
                          </svg>
                          Applied
                        </span>
                      {:else}
                        <Button size="sm" onclick={() => applyPreset(preset)}>Apply</Button>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            </div>

            <!-- Custom Presets -->
            <div>
              <h3 class="text-sm font-semibold text-muted-foreground uppercase tracking-wider mb-3">
                Custom Presets
              </h3>

              {#if customPresets.length > 0}
                <div class="space-y-3 mb-4">
                  {#each customPresets as preset (preset.id)}
                    <div
                      class="border border-border rounded-lg p-4 hover:border-primary/50 transition-colors"
                    >
                      <div class="flex items-start justify-between gap-4">
                        <div class="flex-1">
                          <div class="flex items-center gap-2 mb-1">
                            <span class="text-xl">{preset.icon}</span>
                            <h3 class="font-semibold text-foreground">{preset.name}</h3>
                            <span
                              class="text-xs px-2 py-0.5 rounded-full bg-muted text-muted-foreground"
                            >
                              Custom
                            </span>
                          </div>
                          <p class="text-sm text-muted-foreground mb-3">{preset.description}</p>

                          <!-- Settings preview -->
                          <div class="flex flex-wrap gap-2 text-xs">
                            <span class="px-2 py-1 bg-muted rounded"
                              >↑ {preset.settings.uploadRate} KB/s</span
                            >
                            <span class="px-2 py-1 bg-muted rounded"
                              >↓ {preset.settings.downloadRate} KB/s</span
                            >
                            {#if preset.settings.randomizeRates}
                              <span class="px-2 py-1 bg-muted rounded"
                                >±{preset.settings.randomRangePercent}%</span
                              >
                            {/if}
                            {#if preset.settings.progressiveRatesEnabled}
                              <span class="px-2 py-1 bg-green-500/20 text-green-500 rounded"
                                >Progressive</span
                              >
                            {/if}
                            {#if preset.settings.selectedClient}
                              <span
                                class="px-2 py-1 bg-purple-500/20 text-purple-500 rounded capitalize"
                                >{preset.settings.selectedClient}</span
                              >
                            {/if}
                            <!-- Stop conditions -->
                            {#if preset.settings.stopAtRatioEnabled}
                              <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                                >Stop @ {preset.settings.stopAtRatio}x</span
                              >
                            {/if}
                            {#if preset.settings.stopAtUploadedEnabled}
                              <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                                >Stop @ {preset.settings.stopAtUploadedGB} GB ↑</span
                              >
                            {/if}
                            {#if preset.settings.stopAtDownloadedEnabled}
                              <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                                >Stop @ {preset.settings.stopAtDownloadedGB} GB ↓</span
                              >
                            {/if}
                            {#if preset.settings.stopAtSeedTimeEnabled}
                              <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                                >Stop @ {preset.settings.stopAtSeedTimeHours}h</span
                              >
                            {/if}
                            {#if preset.settings.stopWhenNoLeechers}
                              <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                                >Stop: No Leechers</span
                              >
                            {/if}
                          </div>
                        </div>

                        <div class="flex gap-2">
                          {#if appliedPresetId === preset.id}
                            <span
                              class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-semibold rounded-lg bg-green-500/20 text-green-500"
                            >
                              <svg
                                width="14"
                                height="14"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2.5"
                              >
                                <polyline points="20 6 9 17 4 12"></polyline>
                              </svg>
                              Applied
                            </span>
                          {:else}
                            <Button size="sm" onclick={() => applyPreset(preset)}>Apply</Button>
                          {/if}
                          <button
                            onclick={() => deleteCustomPreset(preset.id)}
                            class="p-2 rounded hover:bg-red-500/10 text-muted-foreground hover:text-red-500 transition-colors"
                            aria-label="Delete preset"
                          >
                            <svg
                              width="16"
                              height="16"
                              viewBox="0 0 24 24"
                              fill="none"
                              stroke="currentColor"
                              stroke-width="2"
                            >
                              <polyline points="3 6 5 6 21 6"></polyline>
                              <path
                                d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                              ></path>
                            </svg>
                          </button>
                        </div>
                      </div>
                    </div>
                  {/each}
                </div>
              {:else}
                <p class="text-sm text-muted-foreground mb-4">
                  No custom presets yet. Export your current config or import a preset file.
                </p>
              {/if}

              <!-- Import/Export Section -->
              <div class="border border-dashed border-border rounded-lg p-4 space-y-4">
                <!-- Export current config -->
                <div>
                  <h4 class="font-medium text-foreground mb-2">Export Current Config</h4>
                  <p class="text-sm text-muted-foreground mb-3">
                    Save your current configuration as a JSON file that can be shared and imported.
                  </p>
                  <button
                    type="button"
                    onclick={openExportDialog}
                    class="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-lg font-semibold ring-offset-background transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 bg-primary text-primary-foreground shadow-lg shadow-primary/25 hover:bg-primary/90 hover:shadow-xl hover:shadow-primary/30 hover:-translate-y-0.5 active:scale-95 px-4 py-2 text-sm"
                  >
                    <svg
                      width="16"
                      height="16"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                    >
                      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                      <polyline points="7 10 12 15 17 10"></polyline>
                      <line x1="12" y1="15" x2="12" y2="3"></line>
                    </svg>
                    Export Config
                  </button>
                  {#if exportSuccess}
                    <p class="mt-2 text-sm text-green-500">{exportSuccess}</p>
                  {/if}
                </div>

                <!-- Import preset -->
                <div class="border-t border-border pt-4">
                  <h4 class="font-medium text-foreground mb-2">Import Preset File</h4>
                  <input
                    bind:this={fileInput}
                    type="file"
                    accept=".json"
                    class="hidden"
                    onchange={handleFileImport}
                  />
                  <button
                    type="button"
                    onclick={triggerImport}
                    class="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-lg font-semibold ring-offset-background transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 border-2 border-primary/20 bg-background hover:bg-primary/5 hover:border-primary/40 hover:-translate-y-0.5 active:scale-95 px-4 py-2 text-sm"
                  >
                    <svg
                      width="16"
                      height="16"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                    >
                      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                      <polyline points="17 8 12 3 7 8"></polyline>
                      <line x1="12" y1="3" x2="12" y2="15"></line>
                    </svg>
                    Import Preset
                  </button>
                  {#if importError}
                    <p class="mt-2 text-sm text-red-500">{importError}</p>
                  {/if}
                  {#if importSuccess}
                    <p class="mt-2 text-sm text-green-500">{importSuccess}</p>
                  {/if}
                </div>
              </div>
            </div>
          </div>
        {:else if activeTab === 'tips'}
          <!-- Detection Tips Tab -->
          <div class="space-y-4">
            <p class="text-sm text-muted-foreground mb-4">
              Follow these guidelines to minimize the risk of detection by private trackers.
            </p>

            {#each detectionTips as tip, index (index)}
              <div class="border border-border rounded-lg p-4">
                <div class="flex items-start gap-3">
                  <span
                    class="flex-shrink-0 text-xs font-semibold px-2 py-1 rounded {getImportanceColor(
                      tip.importance
                    )}"
                  >
                    {getImportanceLabel(tip.importance)}
                  </span>
                  <div>
                    <h3 class="font-semibold text-foreground mb-1">{tip.title}</h3>
                    <p class="text-sm text-muted-foreground">{tip.description}</p>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<!-- Export Preset Dialog -->
{#if showExportDialog}
  <div
    class="fixed inset-0 bg-black/50 z-[60] flex items-center justify-center p-4"
    onclick={e => e.target === e.currentTarget && (showExportDialog = false)}
    onkeydown={e => e.key === 'Escape' && (showExportDialog = false)}
    role="dialog"
    aria-modal="true"
    aria-labelledby="export-dialog-title"
    tabindex="-1"
  >
    <div
      class="bg-card text-card-foreground rounded-xl shadow-2xl max-w-md w-full border border-border animate-in fade-in zoom-in-95 duration-200"
    >
      <!-- Header -->
      <div class="flex items-center justify-between p-4 border-b border-border">
        <h3 id="export-dialog-title" class="text-lg font-semibold text-foreground">
          Export Preset
        </h3>
        <button
          onclick={() => (showExportDialog = false)}
          class="p-1 rounded hover:bg-muted transition-colors"
          aria-label="Close dialog"
        >
          <svg
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>

      <!-- Content -->
      <div class="p-4 space-y-4">
        <div>
          <label for="preset-name" class="block text-sm font-medium text-foreground mb-1">
            Preset Name <span class="text-red-500">*</span>
          </label>
          <input
            id="preset-name"
            type="text"
            bind:value={exportPresetName}
            placeholder="e.g., My Tracker Config"
            class="w-full px-3 py-2 text-sm border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary/50"
          />
        </div>

        <div>
          <label for="preset-description" class="block text-sm font-medium text-foreground mb-1">
            Description <span class="text-muted-foreground text-xs">(optional)</span>
          </label>
          <textarea
            id="preset-description"
            bind:value={exportPresetDescription}
            placeholder="Describe what this preset is for..."
            rows="2"
            class="w-full px-3 py-2 text-sm border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary/50 resize-none"
          ></textarea>
        </div>

        {#if exportError}
          <p class="text-sm text-red-500">{exportError}</p>
        {/if}
      </div>

      <!-- Footer -->
      <div class="flex justify-end gap-3 p-4 border-t border-border">
        <button
          type="button"
          onclick={() => (showExportDialog = false)}
          class="px-4 py-2 text-sm font-medium rounded-lg border border-border hover:bg-muted transition-colors"
        >
          Cancel
        </button>
        <button
          type="button"
          onclick={exportPreset}
          class="px-4 py-2 text-sm font-semibold rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
        >
          Export
        </button>
      </div>
    </div>
  </div>
{/if}
