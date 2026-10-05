<script>
  import { instances, activeInstanceId, instanceActions } from '$lib/core/instanceStore.js';
  import { get } from 'svelte/store';
  import { api } from '$lib/api.js';
  import Button from '$lib/components/ui/button.svelte';
  import BaseModal from '../common/BaseModal.svelte';
  import { builtInPresets } from '$lib/presets/index.js';
  import {
    getDefaultPreset,
    getDefaultPresetId,
    setDefaultPreset,
    clearDefaultPreset,
    refreshDefaultPreset,
  } from '$lib/presets/defaultPreset.js';
  import {
    buildCustomPreset,
    buildPresetExportData,
    normalizePreset,
    normalizePresets,
    normalizePresetSettings,
  } from '$lib/presets/customPreset.js';
  import {
    THEMES,
    THEME_CATEGORIES,
    getTheme,
    selectTheme,
  } from '$lib/themes/themeStore.svelte.js';
  import { ZOOM_MAX, ZOOM_MIN, zoomPercent } from '$lib/core/zoom.js';
  import { getZoom, resetZoom, zoomIn, zoomOut } from '$lib/core/zoomStore.svelte.js';
  import {
    Settings,
    X,
    Check,
    Trash2,
    Download,
    Upload,
    Save,
    Monitor,
    ZoomIn,
    ZoomOut,
  } from '@lucide/svelte';
  import PresetIcon from '../config/PresetIcon.svelte';

  let { isOpen = $bindable(false) } = $props();

  // Check if running in Tauri
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  // Interface zoom (global app preference)
  const isMac =
    typeof navigator !== 'undefined' && /mac/i.test(navigator.platform || navigator.userAgent);
  const modifierLabel = isMac ? '⌘' : 'Ctrl';
  let zoomValue = $derived(getZoom());
  let zoomPct = $derived(zoomPercent(zoomValue));

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
    api.setLogLevel(level);
  }

  // Window Close Behavior state
  const CLOSE_BEHAVIOR_KEY = 'rustatio-close-behavior';
  function getCloseBehavior() {
    return localStorage.getItem(CLOSE_BEHAVIOR_KEY) || 'prompt';
  }

  let closeBehavior = $state(getCloseBehavior());

  function saveCloseBehavior(behavior) {
    closeBehavior = behavior;
    localStorage.setItem(CLOSE_BEHAVIOR_KEY, behavior);
  }

  $effect(() => {
    if (isOpen) {
      closeBehavior = getCloseBehavior();
    }
  });

  let customPresets = $state([]);

  // Default preset state
  let defaultPresetId = $state(getDefaultPresetId());
  let defaultPresetName = $state(getDefaultPreset()?.name || 'Rustatio defaults');

  async function loadPresetState() {
    try {
      customPresets = normalizePresets((await api.listCustomPresets()) || []);
      const preset = await refreshDefaultPreset();
      defaultPresetId = preset?.id || null;
      defaultPresetName = preset?.name || 'Rustatio defaults';
    } catch (e) {
      console.warn('Failed to load preset state:', e);
    }
  }

  async function setAsDefault(preset) {
    await setDefaultPreset(preset);
    defaultPresetId = preset.id;
    defaultPresetName = preset.name || 'Unnamed preset';
  }

  async function clearDefault() {
    await clearDefaultPreset();
    defaultPresetId = null;
    defaultPresetName = 'Rustatio defaults';
  }

  $effect(() => {
    if (isOpen) {
      loadPresetState();
    }
  });

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

  function applyPreset(preset) {
    const active = get(activeInstanceId);
    if (active !== null) {
      instanceActions.updateInstance(active, normalizePresetSettings(preset.settings));
    }
    close();
  }

  // Check if a preset matches the current instance settings
  function isPresetApplied(preset, instance) {
    if (!instance) return false;

    // Compare all settings in the preset with the instance
    for (const [key, value] of Object.entries(normalizePresetSettings(preset.settings))) {
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
  let showSaveDialog = $state(false);
  let exportPresetName = $state('');
  let exportPresetDescription = $state('');

  function getActiveInstanceOrThrow() {
    const active = get(activeInstanceId);
    if (active === null) {
      throw new Error('No active instance. Select an instance first.');
    }

    const currentItems = get(instances);
    const instance = currentItems.find(i => i.id === active);
    if (!instance) {
      throw new Error('Instance not found.');
    }

    return instance;
  }

  function resetPresetDialog() {
    exportError = '';
    exportSuccess = '';
    exportPresetName = '';
    exportPresetDescription = '';
  }

  function openSaveDialog() {
    resetPresetDialog();

    try {
      getActiveInstanceOrThrow();
      showSaveDialog = true;
    } catch (err) {
      exportError = err.message;
    }
  }

  function openExportDialog() {
    resetPresetDialog();

    try {
      getActiveInstanceOrThrow();
      showExportDialog = true;
    } catch (err) {
      exportError = err.message;
    }
  }

  async function savePreset(setDefault = false) {
    exportError = '';
    exportSuccess = '';

    try {
      const instance = getActiveInstanceOrThrow();
      const preset = buildCustomPreset(instance, {
        name: exportPresetName,
        description: exportPresetDescription,
      });

      await api.upsertCustomPreset(preset);
      customPresets = [...customPresets.filter(p => p.id !== preset.id), preset];

      if (setDefault) {
        await setAsDefault(preset);
      }

      exportSuccess = setDefault
        ? `Preset "${preset.name}" saved and set as default`
        : `Preset "${preset.name}" saved successfully`;
      showSaveDialog = false;
    } catch (err) {
      exportError = err.message;
    }
  }

  async function exportPreset() {
    exportError = '';
    exportSuccess = '';
    let presetData;

    try {
      const instance = getActiveInstanceOrThrow();
      presetData = buildPresetExportData(instance, {
        name: exportPresetName,
        description: exportPresetDescription,
      });
    } catch (err) {
      exportError = err.message;
      return;
    }

    // Create a safe filename from the preset name
    const safeFilename = presetData.name
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
      const newPreset = normalizePreset({
        id: `custom-${Date.now()}`,
        name: data.name || 'Imported Preset',
        description: data.description || 'Imported custom preset',
        icon: data.icon || 'folder',
        custom: true,
        created_at: data.created_at || data.createdAt || new Date().toISOString(),
        settings: data.settings,
      });

      // Add to custom presets
      await api.upsertCustomPreset(newPreset);
      customPresets = [...customPresets.filter(p => p.id !== newPreset.id), newPreset];

      importSuccess = `Preset "${newPreset.name}" imported successfully`;
    } catch (err) {
      importError = `Failed to import: ${err.message}`;
    }

    // Reset file input
    if (fileInput) fileInput.value = '';
  }

  async function deleteCustomPreset(presetId) {
    await api.deleteCustomPreset(presetId);
    customPresets = customPresets.filter(p => p.id !== presetId);
    if (defaultPresetId === presetId) {
      await clearDefault();
    }
  }

  function getImportanceColor(importance) {
    switch (importance) {
      case 'critical':
        return 'text-stat-leecher bg-stat-leecher/10';
      case 'high':
        return 'text-stat-ratio bg-stat-ratio/10';
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
  <BaseModal
    bind:open={isOpen}
    onClose={close}
    titleId="settings-title"
    maxWidthClass="max-w-2xl"
    panelClass="max-h-[85vh] flex flex-col"
  >
    <!-- Header -->
    <div class="flex flex-shrink-0 items-center justify-between border-b border-border px-3 py-2">
      <div class="flex items-center gap-2">
        <span class="flex h-6 w-6 items-center justify-center border border-border bg-muted">
          <Settings size={12} class="text-muted-foreground" />
        </span>
        <div>
          <h2
            id="settings-title"
            class="text-xs font-semibold uppercase tracking-wider text-foreground"
          >
            Settings
          </h2>
          <p class="text-[0.625rem] text-muted-foreground">Presets and configuration</p>
        </div>
      </div>
      <button
        onclick={close}
        class="flex h-6 w-6 cursor-pointer items-center justify-center text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
        aria-label="Close dialog"
      >
        <X size={14} />
      </button>
    </div>

    <!-- Tabs -->
    <div class="flex flex-shrink-0 border-b border-border">
      <button
        class="flex-1 cursor-pointer px-3 py-1.5 text-[0.6875rem] font-medium transition-colors {activeTab ===
        'general'
          ? 'border-b-2 border-primary bg-primary/5 text-primary'
          : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => (activeTab = 'general')}
      >
        General
      </button>
      <button
        class="flex-1 cursor-pointer px-3 py-1.5 text-[0.6875rem] font-medium transition-colors {activeTab ===
        'presets'
          ? 'border-b-2 border-primary bg-primary/5 text-primary'
          : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => (activeTab = 'presets')}
      >
        Presets
      </button>
      <button
        class="flex-1 cursor-pointer px-3 py-1.5 text-[0.6875rem] font-medium transition-colors {activeTab ===
        'tips'
          ? 'border-b-2 border-primary bg-primary/5 text-primary'
          : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => (activeTab = 'tips')}
      >
        Detection tips
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-3">
      {#if activeTab === 'general'}
        <!-- General Settings Tab -->
        <div class="space-y-3">
          <p class="text-[0.6875rem] text-muted-foreground">
            Configure general application settings.
          </p>

          <!-- Log Level Section -->
          <div class="border border-border p-3">
            <h3
              class="mb-1.5 text-[0.625rem] font-semibold uppercase tracking-[0.14em] text-muted-foreground"
            >
              Log level
            </h3>
            <p class="mb-2.5 text-[0.6875rem] text-muted-foreground">
              Set the verbosity of logs displayed in the console. Higher levels show more detailed
              information for debugging.
            </p>
            <div class="flex items-center gap-4">
              <label for="logLevel" class="min-w-[3.75rem] text-[0.6875rem] text-muted-foreground"
                >Level</label
              >
              <select
                id="logLevel"
                value={logLevel}
                onchange={e => saveLogLevel(e.target.value)}
                class="h-8 w-40 border border-input bg-background px-2.5 text-xs text-foreground focus:border-ring focus:outline-none"
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
              Note: Log level changes apply to the backend.
            </p>
          </div>

          <!-- Window Behavior Section (Tauri only) -->
          {#if isTauri}
            <div class="border border-border p-3">
              <h3
                class="mb-1.5 text-[0.625rem] font-semibold uppercase tracking-[0.14em] text-muted-foreground"
              >
                Window Behavior
              </h3>
              <p class="mb-2.5 text-[0.6875rem] text-muted-foreground">
                Choose the action that occurs when you click the window close button.
              </p>
              <div class="flex items-center gap-4">
                <label
                  for="closeBehavior"
                  class="min-w-[3.75rem] text-[0.6875rem] text-muted-foreground">On Close</label
                >
                <select
                  id="closeBehavior"
                  value={closeBehavior}
                  onchange={e => saveCloseBehavior(e.target.value)}
                  class="h-8 border border-input bg-background px-2.5 text-xs text-foreground focus:border-ring focus:outline-none w-56"
                >
                  <option value="prompt">Ask every time</option>
                  <option value="tray">Minimize to tray</option>
                  <option value="quit">Quit application</option>
                </select>
              </div>
              <div class="mt-3 text-xs text-muted-foreground space-y-1">
                <p><strong>Ask every time:</strong> Show a prompt to choose action</p>
                <p><strong>Minimize to tray:</strong> Keep app running in background</p>
                <p><strong>Quit application:</strong> Completely close the application</p>
              </div>
            </div>
          {/if}

          <!-- Theme Section -->
          <div class="border border-border p-3">
            <h3
              class="mb-1.5 text-[0.625rem] font-semibold uppercase tracking-[0.14em] text-muted-foreground"
            >
              Theme
            </h3>
            <p class="mb-2.5 text-[0.6875rem] text-muted-foreground">
              Choose your preferred color theme.
            </p>
            <div class="flex items-center gap-4">
              <label
                for="themeSelect"
                class="min-w-[3.75rem] text-[0.6875rem] text-muted-foreground">Theme</label
              >
              <select
                id="themeSelect"
                value={getTheme()}
                onchange={e => selectTheme(e.target.value)}
                class="h-8 border border-input bg-background px-2.5 text-xs text-foreground focus:border-ring focus:outline-none w-56"
              >
                {#each Object.entries(THEME_CATEGORIES) as [categoryId, category] (categoryId)}
                  <optgroup label={category.name}>
                    {#each category.themes as themeId (themeId)}
                      {@const themeOption = THEMES[themeId]}
                      <option value={themeOption.id}>{themeOption.name}</option>
                    {/each}
                  </optgroup>
                {/each}
              </select>
            </div>
            <p class="mt-3 text-xs text-muted-foreground">
              {THEMES[getTheme()]?.description || ''}
            </p>
          </div>

          <!-- Interface Section -->
          <div class="border border-border p-3">
            <h3
              class="mb-1.5 text-[0.625rem] font-semibold uppercase tracking-[0.14em] text-muted-foreground"
            >
              Interface
            </h3>
            <p class="mb-2.5 text-[0.6875rem] text-muted-foreground">
              Scale the whole interface up or down.
            </p>
            <div class="flex items-center gap-4">
              <span
                class="flex min-w-[3.75rem] items-center gap-1.5 text-[0.6875rem] text-muted-foreground"
              >
                <Monitor size={12} />
                Zoom
              </span>
              <div class="flex items-center gap-1">
                <button
                  type="button"
                  class="flex h-6 w-6 cursor-pointer items-center justify-center border border-border text-muted-foreground transition-colors hover:bg-muted hover:text-foreground disabled:pointer-events-none disabled:opacity-40"
                  onclick={zoomOut}
                  disabled={zoomValue <= ZOOM_MIN}
                  title="Zoom out"
                  aria-label="Zoom out"
                >
                  <ZoomOut size={12} />
                </button>
                <span
                  class="w-10 text-center text-xs font-semibold tabular-nums text-foreground"
                  title="Interface zoom"
                >
                  {zoomPct}%
                </span>
                <button
                  type="button"
                  class="flex h-6 w-6 cursor-pointer items-center justify-center border border-border text-muted-foreground transition-colors hover:bg-muted hover:text-foreground disabled:pointer-events-none disabled:opacity-40"
                  onclick={zoomIn}
                  disabled={zoomValue >= ZOOM_MAX}
                  title="Zoom in"
                  aria-label="Zoom in"
                >
                  <ZoomIn size={12} />
                </button>
                <button
                  type="button"
                  class="ml-1 h-6 cursor-pointer border border-border px-2 text-[0.625rem] text-muted-foreground transition-colors hover:bg-muted hover:text-foreground disabled:pointer-events-none disabled:opacity-40"
                  onclick={resetZoom}
                  disabled={zoomPct === 100}
                  title="Reset zoom to 100%"
                >
                  Reset
                </button>
              </div>
            </div>
            <p class="mt-2 text-[0.625rem] leading-4 text-muted-foreground">
              Shortcuts: {modifierLabel} + / {modifierLabel} − · {modifierLabel} + wheel ·
              {modifierLabel} 0 resets.
            </p>
          </div>
        </div>
      {:else if activeTab === 'presets'}
        <!-- Presets Tab -->
        <div class="space-y-6">
          <!-- Info box about Apply vs Default -->
          <div class="border border-border bg-background p-3">
            <p class="text-[0.6875rem] text-muted-foreground">
              <span class="font-semibold text-foreground">Apply</span> applies a preset to the
              current instance only.
              <span class="font-semibold text-foreground">Set Default</span> makes new instances/torrents
              use this preset's settings automatically.
            </p>
            <p class="text-xs text-muted-foreground mt-2">
              Default used for watch-folder imports:
              <span class="text-foreground font-medium">{defaultPresetName}</span>
            </p>
          </div>

          <!-- Built-in Presets -->
          <div>
            <h3
              class="mb-2 text-[0.625rem] font-semibold uppercase tracking-[0.14em] text-muted-foreground"
            >
              Built-in Presets
            </h3>
            <div class="space-y-3">
              {#each builtInPresets as preset (preset.id)}
                <div
                  class="border border-border p-3 transition-colors hover:border-primary/40 {preset.recommended
                    ? 'ring-1 ring-primary/30'
                    : ''}"
                >
                  <!-- Header row with title and action button -->
                  <div class="flex items-start justify-between gap-3 mb-2">
                    <div class="flex items-center gap-2 flex-wrap flex-1 min-w-0">
                      <PresetIcon icon={preset.icon} size={20} class="flex-shrink-0 text-primary" />
                      <h3 class="font-semibold text-foreground">{preset.name}</h3>
                      {#if preset.recommended}
                        <span
                          class="border border-primary/40 bg-primary/10 px-1.5 py-px text-[0.5625rem] font-semibold uppercase tracking-wider text-primary"
                        >
                          Recommended
                        </span>
                      {/if}
                    </div>
                    <!-- Action buttons in header -->
                    <div class="flex items-center gap-1 flex-shrink-0">
                      {#if appliedPresetId === preset.id}
                        <span
                          class="inline-flex items-center gap-1 border border-stat-upload/40 bg-stat-upload/10 px-1.5 py-0.5 text-[0.625rem] font-semibold text-stat-upload"
                        >
                          <Check size={14} strokeWidth={2.5} />
                          Applied
                        </span>
                      {:else}
                        <Button size="sm" onclick={() => applyPreset(preset)}>Apply</Button>
                      {/if}
                      {#if defaultPresetId === preset.id}
                        <button
                          onclick={() => clearDefault()}
                          class="ml-1 cursor-pointer border border-primary/40 bg-primary/10 px-1.5 py-0.5 text-[0.625rem] font-medium text-primary transition-colors hover:bg-primary/20"
                          title="Click to clear default"
                        >
                          ★ Default
                        </button>
                      {:else}
                        <button
                          onclick={() => setAsDefault(preset)}
                          class="ml-1 cursor-pointer border border-border px-1.5 py-0.5 text-[0.625rem] font-medium transition-colors hover:bg-muted"
                          title="Set as default for new instances"
                        >
                          Set Default
                        </button>
                      {/if}
                    </div>
                  </div>

                  <p class="mb-2 text-[0.6875rem] text-muted-foreground">{preset.description}</p>

                  <!-- Settings preview -->
                  <div class="flex flex-wrap gap-2 text-xs mb-2">
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
                      <span class="px-2 py-1 bg-stat-upload/20 text-stat-upload rounded"
                        >Progressive</span
                      >
                    {/if}
                    {#if preset.settings.selectedClient}
                      <span class="px-2 py-1 bg-purple-500/20 text-purple-500 rounded capitalize"
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
                    {#if preset.settings.idleWhenNoLeechers}
                      <span class="px-2 py-1 bg-purple-500/20 text-purple-500 rounded"
                        >Idle: No Leechers</span
                      >
                    {/if}
                    {#if preset.settings.idleWhenNoSeeders}
                      <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                        >Idle: No Seeders</span
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
              {/each}
            </div>
          </div>

          <!-- Custom Presets -->
          <div>
            <h3
              class="mb-2 text-[0.625rem] font-semibold uppercase tracking-[0.14em] text-muted-foreground"
            >
              Custom Presets
            </h3>

            {#if customPresets.length > 0}
              <div class="space-y-3 mb-4">
                {#each customPresets as preset (preset.id)}
                  <div class="border border-border p-3 transition-colors hover:border-primary/40">
                    <!-- Header row with title and action buttons -->
                    <div class="flex items-start justify-between gap-3 mb-2">
                      <div class="flex items-center gap-2 flex-wrap flex-1 min-w-0">
                        <PresetIcon
                          icon={preset.icon}
                          size={20}
                          class="flex-shrink-0 text-primary"
                        />
                        <h3 class="font-semibold text-foreground">{preset.name}</h3>
                        <span
                          class="border border-border bg-muted px-1.5 py-px text-[0.5625rem] uppercase tracking-wider text-muted-foreground"
                        >
                          Custom
                        </span>
                      </div>
                      <!-- Action buttons in header -->
                      <div class="flex items-center gap-1 flex-shrink-0">
                        {#if appliedPresetId === preset.id}
                          <span
                            class="inline-flex items-center gap-1 border border-stat-upload/40 bg-stat-upload/10 px-1.5 py-0.5 text-[0.625rem] font-semibold text-stat-upload"
                          >
                            <Check size={14} strokeWidth={2.5} />
                            Applied
                          </span>
                        {:else}
                          <Button size="sm" onclick={() => applyPreset(preset)}>Apply</Button>
                        {/if}
                        {#if defaultPresetId === preset.id}
                          <button
                            onclick={() => clearDefault()}
                            class="cursor-pointer border border-primary/40 bg-primary/10 px-1.5 py-0.5 text-[0.625rem] font-medium text-primary transition-colors hover:bg-primary/20"
                            title="Click to clear default"
                          >
                            ★ Default
                          </button>
                        {:else}
                          <button
                            onclick={() => setAsDefault(preset)}
                            class="cursor-pointer border border-border px-1.5 py-0.5 text-[0.625rem] font-medium transition-colors hover:bg-muted"
                            title="Set as default for new instances"
                          >
                            Set Default
                          </button>
                        {/if}
                        <button
                          onclick={() => deleteCustomPreset(preset.id)}
                          class="p-2 rounded hover:bg-stat-leecher/10 text-muted-foreground hover:text-stat-leecher transition-colors"
                          aria-label="Delete preset"
                        >
                          <Trash2 size={16} />
                        </button>
                      </div>
                    </div>

                    <p class="mb-2 text-[0.6875rem] text-muted-foreground">{preset.description}</p>

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
                        <span class="px-2 py-1 bg-stat-upload/20 text-stat-upload rounded"
                          >Progressive</span
                        >
                      {/if}
                      {#if preset.settings.selectedClient}
                        <span class="px-2 py-1 bg-purple-500/20 text-purple-500 rounded capitalize"
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
                      {#if preset.settings.idleWhenNoLeechers}
                        <span class="px-2 py-1 bg-purple-500/20 text-purple-500 rounded"
                          >Idle: No Leechers</span
                        >
                      {/if}
                      {#if preset.settings.idleWhenNoSeeders}
                        <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                          >Idle: No Seeders</span
                        >
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            {:else}
              <p class="mb-2.5 text-[0.6875rem] text-muted-foreground">
                No custom presets yet. Save the current instance as a preset or import a preset
                file.
              </p>
            {/if}

            <!-- Import/Export Section -->
            <div class="space-y-3 border border-dashed border-border p-3">
              <!-- Save current config -->
              <div>
                <h4 class="font-medium text-foreground mb-2">Save Current as Preset</h4>
                <p class="mb-2 text-[0.6875rem] text-muted-foreground">
                  Save the active instance configuration directly into your custom presets.
                </p>
                <button
                  type="button"
                  onclick={openSaveDialog}
                  class="cursor-pointer bg-primary px-2.5 py-1 text-[0.6875rem] font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                >
                  <Save size={16} />
                  Save Preset
                </button>
                {#if exportSuccess}
                  <p class="text-[0.6875rem] text-stat-upload">{exportSuccess}</p>
                {/if}
              </div>

              <!-- Export current config -->
              <div class="border-t border-border pt-4">
                <h4 class="font-medium text-foreground mb-2">Export Current Config</h4>
                <p class="mb-2 text-[0.6875rem] text-muted-foreground">
                  Save your current configuration as a JSON file that can be shared and imported.
                </p>
                <button
                  type="button"
                  onclick={openExportDialog}
                  class="cursor-pointer bg-primary px-2.5 py-1 text-[0.6875rem] font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                >
                  <Download size={16} />
                  Export Config
                </button>
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
                  class="cursor-pointer border border-border px-2.5 py-1 text-[0.6875rem] font-medium transition-colors hover:bg-muted"
                >
                  <Upload size={16} />
                  Import Preset
                </button>
                {#if importError}
                  <p class="text-[0.6875rem] text-stat-leecher">{importError}</p>
                {/if}
                {#if importSuccess}
                  <p class="text-[0.6875rem] text-stat-upload">{importSuccess}</p>
                {/if}
              </div>
            </div>
          </div>
        </div>
      {:else if activeTab === 'tips'}
        <!-- Detection Tips Tab -->
        <div class="space-y-4">
          <p class="mb-2.5 text-[0.6875rem] text-muted-foreground">
            Follow these guidelines to minimize the risk of detection by private trackers.
          </p>

          {#each detectionTips as tip, index (index)}
            <div class="border border-border p-3">
              <div class="flex flex-col gap-2">
                <div class="flex items-center justify-between gap-3">
                  <h3 class="font-semibold text-foreground">{tip.title}</h3>
                  <span
                    class="flex-shrink-0 text-xs font-semibold px-2 py-1 rounded {getImportanceColor(
                      tip.importance
                    )}"
                  >
                    {getImportanceLabel(tip.importance)}
                  </span>
                </div>
                <p class="text-[0.6875rem] text-muted-foreground">{tip.description}</p>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </BaseModal>
{/if}

<!-- Save Preset Dialog -->
{#if showSaveDialog}
  <BaseModal
    bind:open={showSaveDialog}
    onClose={() => (showSaveDialog = false)}
    titleId="save-dialog-title"
    maxWidthClass="max-w-md"
    panelClass="max-h-[85vh] flex flex-col"
  >
    <div class="flex items-center justify-between p-4 border-b border-border">
      <h3
        id="save-dialog-title"
        class="text-xs font-semibold uppercase tracking-wider text-foreground"
      >
        Save preset
      </h3>
      <button
        onclick={() => (showSaveDialog = false)}
        class="p-1 flex h-6 w-6 items-center justify-center text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
        aria-label="Close dialog"
      >
        <X size={18} />
      </button>
    </div>

    <div class="p-4 space-y-4">
      <div>
        <label
          for="save-preset-name"
          class="mb-1 block text-[0.625rem] uppercase tracking-wider text-muted-foreground"
        >
          Preset Name <span class="text-stat-leecher">*</span>
        </label>
        <input
          id="save-preset-name"
          type="text"
          bind:value={exportPresetName}
          placeholder="e.g., My Tracker Config"
          class="w-full h-8 border border-input bg-background px-2.5 text-xs text-foreground focus:border-ring focus:outline-none"
        />
      </div>

      <div>
        <label
          for="save-preset-description"
          class="mb-1 block text-[0.625rem] uppercase tracking-wider text-muted-foreground"
        >
          Description <span class="text-muted-foreground text-xs">(optional)</span>
        </label>
        <textarea
          id="save-preset-description"
          bind:value={exportPresetDescription}
          placeholder="Describe what this preset is for..."
          rows="2"
          class="w-full h-8 border border-input bg-background px-2.5 text-xs text-foreground focus:border-ring focus:outline-none resize-none"
        ></textarea>
      </div>

      {#if exportError}
        <p class="text-sm text-stat-leecher">{exportError}</p>
      {/if}
    </div>

    <div class="flex justify-end gap-3 p-4 border-t border-border">
      <button
        type="button"
        onclick={() => (showSaveDialog = false)}
        class="cursor-pointer border border-border px-2.5 py-1 text-[0.6875rem] font-medium transition-colors hover:bg-muted"
      >
        Cancel
      </button>
      <button
        type="button"
        onclick={() => savePreset(false)}
        class="cursor-pointer border border-border px-2.5 py-1 text-[0.6875rem] font-medium transition-colors hover:bg-muted"
      >
        Save
      </button>
      <button
        type="button"
        onclick={() => savePreset(true)}
        class="cursor-pointer bg-primary px-2.5 py-1 text-[0.6875rem] font-semibold text-primary-foreground transition-colors hover:bg-primary/90"
      >
        Save and Set Default
      </button>
    </div>
  </BaseModal>
{/if}

<!-- Export Preset Dialog -->
{#if showExportDialog}
  <BaseModal
    bind:open={showExportDialog}
    onClose={() => (showExportDialog = false)}
    titleId="export-dialog-title"
    maxWidthClass="max-w-md"
    panelClass="max-h-[85vh] flex flex-col"
  >
    <!-- Header -->
    <div class="flex items-center justify-between p-4 border-b border-border">
      <h3
        id="export-dialog-title"
        class="text-xs font-semibold uppercase tracking-wider text-foreground"
      >
        Export preset
      </h3>
      <button
        onclick={() => (showExportDialog = false)}
        class="p-1 flex h-6 w-6 items-center justify-center text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
        aria-label="Close dialog"
      >
        <X size={18} />
      </button>
    </div>

    <!-- Content -->
    <div class="p-4 space-y-4">
      <div>
        <label
          for="preset-name"
          class="mb-1 block text-[0.625rem] uppercase tracking-wider text-muted-foreground"
        >
          Preset Name <span class="text-stat-leecher">*</span>
        </label>
        <input
          id="preset-name"
          type="text"
          bind:value={exportPresetName}
          placeholder="e.g., My Tracker Config"
          class="w-full h-8 border border-input bg-background px-2.5 text-xs text-foreground focus:border-ring focus:outline-none"
        />
      </div>

      <div>
        <label
          for="preset-description"
          class="mb-1 block text-[0.625rem] uppercase tracking-wider text-muted-foreground"
        >
          Description <span class="text-muted-foreground text-xs">(optional)</span>
        </label>
        <textarea
          id="preset-description"
          bind:value={exportPresetDescription}
          placeholder="Describe what this preset is for..."
          rows="2"
          class="w-full h-8 border border-input bg-background px-2.5 text-xs text-foreground focus:border-ring focus:outline-none resize-none"
        ></textarea>
      </div>

      {#if exportError}
        <p class="text-sm text-stat-leecher">{exportError}</p>
      {/if}
    </div>

    <!-- Footer -->
    <div class="flex justify-end gap-3 p-4 border-t border-border">
      <button
        type="button"
        onclick={() => (showExportDialog = false)}
        class="cursor-pointer border border-border px-2.5 py-1 text-[0.6875rem] font-medium transition-colors hover:bg-muted"
      >
        Cancel
      </button>
      <button
        type="button"
        onclick={exportPreset}
        class="cursor-pointer bg-primary px-2.5 py-1 text-[0.6875rem] font-semibold text-primary-foreground transition-colors hover:bg-primary/90"
      >
        Export
      </button>
    </div>
  </BaseModal>
{/if}
