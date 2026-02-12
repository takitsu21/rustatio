<script>
  import { cn } from '$lib/utils.js';
  import Button from '$lib/components/ui/button.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import Select from '$lib/components/ui/select.svelte';
  import Checkbox from '$lib/components/ui/checkbox.svelte';
  import { gridActions } from '$lib/gridStore.js';
  import { api, getRunMode } from '$lib/api.js';
  import FolderBrowser from './FolderBrowser.svelte';
  import { builtInPresets } from '$lib/presets/index.js';
  import { getDefaultPreset } from '$lib/defaultPreset.js';
  import { Upload, FolderOpen, X, FileText, ChevronDown, Settings } from '@lucide/svelte';
  import PresetIcon from '../config/PresetIcon.svelte';
  import ClientIcon from '../config/ClientIcon.svelte';
  import ClientSelect from '../config/ClientSelect.svelte';
  import VersionSelect from '../config/VersionSelect.svelte';
  import RandomizationSettings from '../config/RandomizationSettings.svelte';
  import ProgressiveRateSettings from '../config/ProgressiveRateSettings.svelte';
  import StopConditionSettings from '../config/StopConditionSettings.svelte';

  let { isOpen = $bindable(false) } = $props();

  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
  const isServer = getRunMode() === 'server';
  const isWasm = !isTauri && !isServer;

  // Import mode: 'files' or 'folder'
  let importMode = $state('files');

  // Folder source: 'local' (upload from browser) or 'server' (browse server filesystem)
  let folderSource = $state('local');

  // File upload state
  let selectedFiles = $state([]);
  let localFolderFiles = $state([]);
  let folderPath = $state('');
  let folderBrowserOpen = $state(false);

  // Config
  let mode = $state('seed');
  let customPercent = $state(50);
  let uploadRate = $state(50);
  let downloadRate = $state(0);
  let autoStart = $state(false);
  let staggerSecs = $state(5);
  let tagsInput = $state('');

  // Client selection
  let clientTypes = $state([]);
  let selectedClient = $state('');
  let selectedVersion = $state('');
  let port = $state(6881);

  // Preset selection
  let selectedPresetId = $state('');
  let presetDropdownOpen = $state(false);
  let presetDropdownRef = $state(null);

  // Advanced settings
  let randomizeRates = $state(true);
  let randomRangePercent = $state(20);
  let progressiveRatesEnabled = $state(false);
  let targetUploadRate = $state(500);
  let targetDownloadRate = $state(0);
  let progressiveDurationHours = $state(1);
  let stopAtRatioEnabled = $state(false);
  let stopAtRatio = $state(2.0);
  let stopAtUploadedEnabled = $state(false);
  let stopAtUploadedGB = $state(10);
  let stopAtDownloadedEnabled = $state(false);
  let stopAtDownloadedGB = $state(10);
  let stopAtSeedTimeEnabled = $state(false);
  let stopAtSeedTimeHours = $state(24);
  let idleWhenNoLeechers = $state(false);
  let idleWhenNoSeeders = $state(false);
  let advancedOpen = $state(false);

  // Import state
  let importing = $state(false);
  let importResult = $state(null);

  // Custom presets from localStorage
  const CUSTOM_PRESETS_KEY = 'rustatio-custom-presets';
  function loadCustomPresets() {
    try {
      const stored = localStorage.getItem(CUSTOM_PRESETS_KEY);
      return stored ? JSON.parse(stored) : [];
    } catch { return []; }
  }

  let allPresets = $derived([...builtInPresets, ...loadCustomPresets()]);

  let completionPercent = $derived(
    mode === 'seed' ? 100 : mode === 'leech' ? 0 : parseFloat(customPercent) || 50
  );

  // Folder import uses browser webkitdirectory in WASM and server local modes
  let useLocalFolderPicker = $derived(isWasm || (isServer && folderSource === 'local'));

  let clientVersions = $derived(
    clientTypes.find(c => c.id === selectedClient)?.versions || []
  );

  let advancedActiveCount = $derived(
    [
      randomizeRates,
      progressiveRatesEnabled,
      stopAtRatioEnabled,
      stopAtUploadedEnabled,
      stopAtDownloadedEnabled,
      stopAtSeedTimeEnabled,
      idleWhenNoLeechers,
      idleWhenNoSeeders,
    ].filter(Boolean).length
  );

  // Fetch client types and apply default preset on open
  $effect(() => {
    if (isOpen && clientTypes.length === 0) {
      api.getClientInfos().then(infos => {
        clientTypes = infos || [];
      }).catch(() => {});
    }
    if (isOpen) {
      const defaultPreset = getDefaultPreset();
      if (defaultPreset && !selectedPresetId) {
        applyPreset(defaultPreset);
        selectedPresetId = defaultPreset.id;
      }
    }
  });

  function applyPreset(preset) {
    const s = preset.settings || {};

    if (s.uploadRate != null) uploadRate = s.uploadRate;
    if (s.downloadRate != null) downloadRate = s.downloadRate;
    if (s.port != null) port = s.port;
    if (s.selectedClient != null) selectedClient = s.selectedClient;
    if (s.selectedClientVersion != null) selectedVersion = s.selectedClientVersion;

    if (s.completionPercent != null) {
      if (s.completionPercent === 100) mode = 'seed';
      else if (s.completionPercent === 0) mode = 'leech';
      else { mode = 'custom'; customPercent = s.completionPercent; }
    }

    if (s.randomizeRates != null) randomizeRates = s.randomizeRates;
    if (s.randomRangePercent != null) randomRangePercent = s.randomRangePercent;
    if (s.progressiveRatesEnabled != null) progressiveRatesEnabled = s.progressiveRatesEnabled;
    if (s.targetUploadRate != null) targetUploadRate = s.targetUploadRate;
    if (s.targetDownloadRate != null) targetDownloadRate = s.targetDownloadRate;
    if (s.progressiveDurationHours != null) progressiveDurationHours = s.progressiveDurationHours;
    if (s.stopAtRatioEnabled != null) stopAtRatioEnabled = s.stopAtRatioEnabled;
    if (s.stopAtRatio != null) stopAtRatio = s.stopAtRatio;
    if (s.stopAtUploadedEnabled != null) stopAtUploadedEnabled = s.stopAtUploadedEnabled;
    if (s.stopAtUploadedGB != null) stopAtUploadedGB = s.stopAtUploadedGB;
    if (s.stopAtDownloadedEnabled != null) stopAtDownloadedEnabled = s.stopAtDownloadedEnabled;
    if (s.stopAtDownloadedGB != null) stopAtDownloadedGB = s.stopAtDownloadedGB;
    if (s.stopAtSeedTimeEnabled != null) stopAtSeedTimeEnabled = s.stopAtSeedTimeEnabled;
    if (s.stopAtSeedTimeHours != null) stopAtSeedTimeHours = s.stopAtSeedTimeHours;
    if (s.idleWhenNoLeechers != null) idleWhenNoLeechers = s.idleWhenNoLeechers;
    if (s.idleWhenNoSeeders != null) idleWhenNoSeeders = s.idleWhenNoSeeders;
  }

  function handlePresetChange() {
    if (!selectedPresetId) return;
    const preset = allPresets.find(p => p.id === selectedPresetId);
    if (preset) applyPreset(preset);
  }

  function selectPreset(presetId) {
    selectedPresetId = presetId;
    presetDropdownOpen = false;
    handlePresetChange();
  }

  function handlePresetClickOutside(event) {
    if (presetDropdownRef && !presetDropdownRef.contains(event.target)) {
      presetDropdownOpen = false;
    }
  }

  $effect(() => {
    if (presetDropdownOpen) {
      document.addEventListener('click', handlePresetClickOutside);
      return () => document.removeEventListener('click', handlePresetClickOutside);
    }
  });

  async function handleFileSelect(e) {
    if (isTauri) {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: true,
        filters: [{ name: 'Torrent', extensions: ['torrent'] }],
      });
      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        selectedFiles = [...selectedFiles, ...paths.map(p => ({ name: p.split(/[/\\]/).pop(), path: p }))];
      }
    } else {
      const files = Array.from(e?.target?.files || []);
      selectedFiles = files.filter(f => f.name.endsWith('.torrent'));
    }
  }

  function handleDrop(e) {
    e.preventDefault();
    const files = Array.from(e.dataTransfer?.files || []);
    selectedFiles = [...selectedFiles, ...files.filter(f => f.name.endsWith('.torrent'))];
  }

  function handleDragOver(e) {
    e.preventDefault();
  }

  function removeFile(index) {
    selectedFiles = selectedFiles.filter((_, i) => i !== index);
  }

  function handleLocalFolderSelect(e) {
    const files = Array.from(e?.target?.files || []);
    localFolderFiles = files.filter(f => f.name.endsWith('.torrent'));
  }

  async function handleBrowseFolder() {
    if (isTauri) {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({ directory: true });
      if (selected) {
        folderPath = selected;
      }
    } else if (isServer && folderSource === 'server') {
      folderBrowserOpen = true;
    }
  }

  function handleFolderSelected(path) {
    folderPath = path;
  }

  function buildConfig() {
    const tags = tagsInput
      .split(',')
      .map(t => t.trim())
      .filter(Boolean);

    const config = {
      tags,
      autoStart,
      mode: mode === 'seed' ? 'seed' : mode === 'leech' ? 'leech' : { custom: parseFloat(customPercent) },
    };

    if (autoStart && staggerSecs > 0) {
      config.staggerStartSecs = parseInt(staggerSecs);
    }
    if (selectedClient) {
      config.clientType = selectedClient;
    }
    if (selectedVersion) {
      config.clientVersion = selectedVersion;
    }

    config.baseConfig = {
      uploadRate: parseFloat(uploadRate) || 50,
      downloadRate: parseFloat(downloadRate) || 0,
      port: parseInt(port) || 6881,
      selectedClient: selectedClient || undefined,
      selectedClientVersion: selectedVersion || undefined,
      completionPercent,
      randomizeRates,
      randomRangePercent: parseFloat(randomRangePercent),
      stopAtRatioEnabled,
      stopAtRatio: stopAtRatioEnabled ? parseFloat(stopAtRatio) : undefined,
      stopAtUploadedEnabled,
      stopAtUploadedGB: stopAtUploadedEnabled ? parseFloat(stopAtUploadedGB) : undefined,
      stopAtDownloadedEnabled,
      stopAtDownloadedGB: stopAtDownloadedEnabled ? parseFloat(stopAtDownloadedGB) : undefined,
      stopAtSeedTimeEnabled,
      stopAtSeedTimeHours: stopAtSeedTimeEnabled ? parseFloat(stopAtSeedTimeHours) : undefined,
      idleWhenNoLeechers,
      idleWhenNoSeeders,
      progressiveRatesEnabled,
      targetUploadRate: progressiveRatesEnabled ? parseFloat(targetUploadRate) : undefined,
      targetDownloadRate: progressiveRatesEnabled ? parseFloat(targetDownloadRate) : undefined,
      progressiveDurationHours: progressiveRatesEnabled ? parseFloat(progressiveDurationHours) : undefined,
    };

    return config;
  }

  async function handleImport() {
    importing = true;
    importResult = null;

    try {
      const config = buildConfig();
      let result;

      if (importMode === 'files') {
        if (selectedFiles.length === 0) {
          importResult = { error: 'No files selected' };
          return;
        }
        result = await gridActions.import(selectedFiles, config);
      } else if (useLocalFolderPicker) {
        if (localFolderFiles.length === 0) {
          importResult = { error: 'No .torrent files found in selected folder' };
          return;
        }
        result = await gridActions.import(localFolderFiles, config);
      } else {
        if (!folderPath.trim()) {
          importResult = { error: 'No folder path specified' };
          return;
        }
        result = await gridActions.importFolder(folderPath.trim(), config);
      }

      importResult = result;

      if (result.imported?.length > 0) {
        isOpen = false;
        resetForm();
      }
    } catch (error) {
      importResult = { error: error.message };
    } finally {
      importing = false;
    }
  }

  function resetForm() {
    selectedFiles = [];
    localFolderFiles = [];
    folderPath = '';
    folderSource = 'local';
    importResult = null;
    mode = 'seed';
    customPercent = 50;
    uploadRate = 50;
    downloadRate = 0;
    tagsInput = '';
    autoStart = false;
    staggerSecs = 5;
    selectedClient = '';
    selectedVersion = '';
    port = 6881;
    selectedPresetId = '';
    presetDropdownOpen = false;
    randomizeRates = true;
    randomRangePercent = 20;
    progressiveRatesEnabled = false;
    targetUploadRate = 500;
    targetDownloadRate = 0;
    progressiveDurationHours = 1;
    stopAtRatioEnabled = false;
    stopAtRatio = 2.0;
    stopAtUploadedEnabled = false;
    stopAtUploadedGB = 10;
    stopAtDownloadedEnabled = false;
    stopAtDownloadedGB = 10;
    stopAtSeedTimeEnabled = false;
    stopAtSeedTimeHours = 24;
    idleWhenNoLeechers = false;
    idleWhenNoSeeders = false;
    advancedOpen = false;
  }

  function handleBackdropClick(event) {
    if (event.target === event.currentTarget) {
      close();
    }
  }

  function close() {
    isOpen = false;
    resetForm();
  }
</script>

{#if isOpen}
  <!-- Dialog -->
  <div
    class="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4"
    onclick={handleBackdropClick}
    onkeydown={e => e.key === 'Escape' && close()}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
  >
    <div
      class="bg-card border border-border rounded-xl shadow-2xl max-w-2xl w-full max-h-[85vh] overflow-y-auto"
    >
      <!-- Header -->
      <div class="flex items-center justify-between p-4 border-b border-border">
        <h2 class="text-lg font-semibold text-foreground">Import Torrents</h2>
        <button
          onclick={close}
          class="p-1 rounded hover:bg-muted bg-transparent border-0 cursor-pointer"
          aria-label="Close"
        >
          <X size={18} class="text-muted-foreground" />
        </button>
      </div>

      <!-- Body -->
      <div class="p-4 space-y-4">
        <!-- Import Mode Tabs -->
        <div class="flex gap-1 bg-muted/50 rounded-lg p-1">
          <button
            class={cn(
              'flex-1 flex items-center justify-center gap-2 px-3 py-2 rounded-md text-sm font-medium transition-colors border-0 cursor-pointer',
              importMode === 'files'
                ? 'bg-background text-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground bg-transparent'
            )}
            onclick={() => (importMode = 'files')}
          >
            <Upload size={14} />
            Files
          </button>
          <button
            class={cn(
              'flex-1 flex items-center justify-center gap-2 px-3 py-2 rounded-md text-sm font-medium transition-colors border-0 cursor-pointer',
              importMode === 'folder'
                ? 'bg-background text-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground bg-transparent'
            )}
            onclick={() => (importMode = 'folder')}
          >
            <FolderOpen size={14} />
            Folder
          </button>
        </div>

        <!-- File Upload Area -->
        {#if importMode === 'files'}
          <div
            role="region"
            aria-label="Drop zone for torrent files"
            class="border-2 border-dashed border-border rounded-lg p-6 text-center hover:border-primary/50 transition-colors"
            ondrop={handleDrop}
            ondragover={handleDragOver}
          >
            <Upload size={24} class="mx-auto mb-2 text-muted-foreground" />
            <p class="text-sm text-muted-foreground mb-2">
              {isTauri ? 'Click to browse for .torrent files' : 'Drag & drop .torrent files here, or click to browse'}
            </p>
            {#if !isTauri}
              <input
                type="file"
                accept=".torrent"
                multiple
                onchange={handleFileSelect}
                class="hidden"
                id="grid-file-input"
              />
            {/if}
            <Button size="sm" variant="secondary" class="cursor-pointer" onclick={isTauri ? handleFileSelect : () => document.getElementById('grid-file-input')?.click()}>
              {#snippet children()}
                Browse Files
              {/snippet}
            </Button>
          </div>

          {#if selectedFiles.length > 0}
            <div class="max-h-32 overflow-y-auto space-y-1">
              {#each selectedFiles as file, i (file.name + i)}
                <div class="flex items-center justify-between px-2 py-1 bg-muted/50 rounded text-xs">
                  <span class="flex items-center gap-1.5 truncate">
                    <FileText size={12} class="text-muted-foreground flex-shrink-0" />
                    {file.name}
                  </span>
                  <button
                    onclick={() => removeFile(i)}
                    class="p-0.5 rounded hover:bg-destructive/20 bg-transparent border-0 cursor-pointer"
                  >
                    <X size={12} class="text-muted-foreground" />
                  </button>
                </div>
              {/each}
              <p class="text-xs text-muted-foreground">{selectedFiles.length} file(s) selected</p>
            </div>
          {/if}
        {:else}
          <!-- Folder source toggle (server mode only) -->
          {#if isServer}
            <div class="flex gap-1 bg-muted/50 rounded-lg p-1">
              <button
                class={cn(
                  'flex-1 px-3 py-1.5 rounded-md text-xs font-medium transition-colors border-0 cursor-pointer',
                  folderSource === 'local'
                    ? 'bg-background text-foreground shadow-sm'
                    : 'text-muted-foreground hover:text-foreground bg-transparent'
                )}
                onclick={() => (folderSource = 'local')}
              >
                Local Computer
              </button>
              <button
                class={cn(
                  'flex-1 px-3 py-1.5 rounded-md text-xs font-medium transition-colors border-0 cursor-pointer',
                  folderSource === 'server'
                    ? 'bg-background text-foreground shadow-sm'
                    : 'text-muted-foreground hover:text-foreground bg-transparent'
                )}
                onclick={() => (folderSource = 'server')}
              >
                Server Folder
              </button>
            </div>
          {/if}

          {#if useLocalFolderPicker}
            <!-- Local folder: use webkitdirectory to select from the user's machine -->
            <div
              role="region"
              aria-label="Select a folder from your computer"
              class="border-2 border-dashed border-border rounded-lg p-6 text-center hover:border-primary/50 transition-colors"
            >
              <FolderOpen size={24} class="mx-auto mb-2 text-muted-foreground" />
              <p class="text-sm text-muted-foreground mb-2">
                Select a folder from your computer
              </p>
              <input
                type="file"
                webkitdirectory
                multiple
                onchange={handleLocalFolderSelect}
                class="hidden"
                id="grid-local-folder-input"
              />
              <Button size="sm" variant="secondary" class="cursor-pointer" onclick={() => document.getElementById('grid-local-folder-input')?.click()}>
                {#snippet children()}
                  Browse Folder
                {/snippet}
              </Button>
            </div>

            {#if localFolderFiles.length > 0}
              <div class="max-h-32 overflow-y-auto space-y-1">
                {#each localFolderFiles as file, i (file.name + i)}
                  <div class="flex items-center gap-1.5 px-2 py-1 bg-muted/50 rounded text-xs">
                    <FileText size={12} class="text-muted-foreground flex-shrink-0" />
                    <span class="truncate">{file.name}</span>
                  </div>
                {/each}
                <p class="text-xs text-muted-foreground">{localFolderFiles.length} .torrent file(s) found</p>
              </div>
            {/if}
          {:else}
            <!-- Server folder or Tauri: path-based folder import -->
            <div>
              <Label>Folder Path</Label>
              <div class="flex gap-2 mt-1">
                <Input
                  bind:value={folderPath}
                  placeholder="/path/to/torrents"
                  class="flex-1"
                />
                {#if isTauri || isServer}
                  <Button size="sm" variant="secondary" onclick={handleBrowseFolder}>
                    {#snippet children()}
                      Browse
                    {/snippet}
                  </Button>
                {/if}
              </div>
              <p class="text-xs text-muted-foreground mt-1">
                All .torrent files in this {isServer ? 'server ' : ''}directory will be imported
              </p>
            </div>
          {/if}
        {/if}

        <!-- Preset Selection -->
        {#if allPresets.length > 0}
          <div>
            <Label>Preset</Label>
            <div class="relative mt-1" bind:this={presetDropdownRef}>
              <button
                type="button"
                onclick={() => (presetDropdownOpen = !presetDropdownOpen)}
                class="flex h-10 w-full items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
              >
                <span class="flex items-center gap-2">
                  {#if selectedPresetId}
                    {@const preset = allPresets.find(p => p.id === selectedPresetId)}
                    {#if preset}
                      <PresetIcon icon={preset.icon} size={16} class="text-primary" />
                      <span>{preset.name}</span>
                    {/if}
                  {:else}
                    <span class="text-muted-foreground">None (manual config)</span>
                  {/if}
                </span>
                <ChevronDown
                  size={16}
                  class="text-muted-foreground transition-transform {presetDropdownOpen ? 'rotate-180' : ''}"
                />
              </button>

              {#if presetDropdownOpen}
                <div class="absolute z-50 mt-1 w-full rounded-md border border-border bg-popover shadow-md max-h-48 overflow-y-auto">
                  <button
                    type="button"
                    onclick={() => selectPreset('')}
                    class="flex w-full items-center gap-2 px-3 py-2 text-sm hover:bg-muted transition-colors first:rounded-t-md {!selectedPresetId ? 'bg-muted' : ''}"
                  >
                    <span class="text-muted-foreground">None (manual config)</span>
                  </button>
                  {#each allPresets as preset (preset.id)}
                    <button
                      type="button"
                      onclick={() => selectPreset(preset.id)}
                      class="flex w-full items-center gap-2 px-3 py-2 text-sm hover:bg-muted transition-colors last:rounded-b-md {preset.id === selectedPresetId ? 'bg-muted' : ''}"
                    >
                      <PresetIcon icon={preset.icon} size={16} class="text-primary flex-shrink-0" />
                      <span>{preset.name}</span>
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
            {#if selectedPresetId}
              {@const preset = allPresets.find(p => p.id === selectedPresetId)}
              {#if preset?.description}
                <p class="text-xs text-muted-foreground mt-1">{preset.description}</p>
              {/if}
            {/if}
          </div>
        {/if}

        <!-- Mode Selection -->
        <div>
          <Label>Mode</Label>
          <Select bind:value={mode} class="mt-1">
            {#snippet children()}
              <option value="seed">Seed (100% complete)</option>
              <option value="leech">Leech (0% complete)</option>
              <option value="custom">Custom</option>
            {/snippet}
          </Select>
          {#if mode === 'custom'}
            <div class="mt-2">
              <Label>Completion %</Label>
              <Input type="number" bind:value={customPercent} min="0" max="100" class="mt-1" />
            </div>
          {/if}
        </div>

        <!-- Rates -->
        <div class="grid grid-cols-2 gap-3">
          <div>
            <Label>Upload Rate (KB/s)</Label>
            <Input type="number" bind:value={uploadRate} min="0" step="1" class="mt-1 text-xs" />
          </div>
          <div>
            <Label>Download Rate (KB/s)</Label>
            <Input type="number" bind:value={downloadRate} min="0" step="1" class="mt-1 text-xs" />
          </div>
        </div>

        <!-- Client Selection -->
        <div>
          <div class="flex items-center gap-2 mb-2">
            <ClientIcon clientId={selectedClient} size={16} />
            <Label>Client</Label>
          </div>
          <div class="grid grid-cols-3 gap-3">
            <ClientSelect
              clients={clientTypes}
              bind:value={selectedClient}
            />
            {#if selectedClient && clientVersions.length > 0}
              <VersionSelect
                versions={clientVersions}
                bind:value={selectedVersion}
              />
            {/if}
            <div>
              <Input
                type="number"
                bind:value={port}
                min="1024"
                max="65535"
                placeholder="Port"
                class="h-9"
              />
            </div>
          </div>
        </div>

        <!-- Tags -->
        <div>
          <Label>Tags (comma-separated)</Label>
          <Input
            bind:value={tagsInput}
            placeholder="tag1, tag2, tag3"
            class="mt-1"
          />
        </div>

        <!-- Auto Start -->
        <div class="flex items-center gap-3">
          <div class="flex items-center gap-2">
            <Checkbox bind:checked={autoStart} id="grid-auto-start" />
            <Label for="grid-auto-start" class="cursor-pointer">Auto-start after import</Label>
          </div>
          {#if autoStart}
            <div class="flex items-center gap-2">
              <Label>Stagger (sec)</Label>
              <Input type="number" bind:value={staggerSecs} min="0" max="300" class="h-8 w-20 text-xs" />
            </div>
          {/if}
        </div>

        <!-- Advanced Settings (Collapsible) -->
        <div class="border border-border rounded-lg overflow-hidden">
          <button
            class="flex items-center justify-between w-full p-3 bg-muted/30 hover:bg-muted/50 transition-colors border-0 cursor-pointer"
            onclick={() => (advancedOpen = !advancedOpen)}
          >
            <span class="flex items-center gap-2 text-sm font-medium">
              <Settings size={16} class="text-muted-foreground" />
              Advanced Settings
              {#if advancedActiveCount > 0}
                <span class="text-xs bg-primary/10 text-primary px-2 py-0.5 rounded-full">
                  {advancedActiveCount} active
                </span>
              {/if}
            </span>
            <ChevronDown
              size={16}
              class="text-muted-foreground transition-transform {advancedOpen ? 'rotate-180' : ''}"
            />
          </button>

          {#if advancedOpen}
            <div class="p-4 space-y-4 border-t border-border">
              <RandomizationSettings
                bind:enabled={randomizeRates}
                bind:rangePercent={randomRangePercent}
                uploadRate={parseFloat(uploadRate) || 50}
                downloadRate={parseFloat(downloadRate) || 0}
              />

              <ProgressiveRateSettings
                bind:enabled={progressiveRatesEnabled}
                bind:durationHours={progressiveDurationHours}
                bind:targetUploadRate
                bind:targetDownloadRate
                uploadRate={parseFloat(uploadRate) || 50}
                downloadRate={parseFloat(downloadRate) || 0}
              />

              <div>
                <span class="text-sm font-medium mb-2 block">Stop & Idle Conditions</span>
                <StopConditionSettings
                  bind:stopAtRatioEnabled
                  bind:stopAtRatio
                  bind:stopAtUploadedEnabled
                  bind:stopAtUploadedGB
                  bind:stopAtDownloadedEnabled
                  bind:stopAtDownloadedGB
                  bind:stopAtSeedTimeEnabled
                  bind:stopAtSeedTimeHours
                  bind:idleWhenNoLeechers
                  bind:idleWhenNoSeeders
                  {completionPercent}
                />
              </div>
            </div>
          {/if}
        </div>

        <!-- Import Result -->
        {#if importResult}
          <div class={cn(
            'p-3 rounded-lg text-sm',
            importResult.error
              ? 'bg-destructive/10 text-destructive'
              : 'bg-stat-upload/10 text-stat-upload'
          )}>
            {#if importResult.error}
              {importResult.error}
            {:else}
              {importResult.imported?.length || 0} torrent(s) imported successfully
              {#if importResult.errors?.length > 0}
                <div class="mt-1 text-xs text-stat-ratio">
                  {importResult.errors.length} error(s):
                  {#each importResult.errors.slice(0, 3) as err, idx (idx)}
                    <div>{err}</div>
                  {/each}
                  {#if importResult.errors.length > 3}
                    <div>...and {importResult.errors.length - 3} more</div>
                  {/if}
                </div>
              {/if}
            {/if}
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-end gap-2 p-4 border-t border-border">
        <Button onclick={close} size="sm" variant="secondary">
          {#snippet children()}
            Cancel
          {/snippet}
        </Button>
        <Button
          onclick={handleImport}
          size="sm"
          disabled={importing
            || (importMode === 'files' && selectedFiles.length === 0)
            || (importMode === 'folder' && useLocalFolderPicker && localFolderFiles.length === 0)
            || (importMode === 'folder' && !useLocalFolderPicker && !folderPath.trim())}
        >
          {#snippet children()}
            {#if importing}
              Importing...
            {:else if importMode === 'files'}
              Import {selectedFiles.length} File(s)
            {:else if useLocalFolderPicker}
              Import {localFolderFiles.length} File(s)
            {:else}
              Import from Folder
            {/if}
          {/snippet}
        </Button>
      </div>
    </div>
  </div>

  <FolderBrowser bind:isOpen={folderBrowserOpen} onSelect={handleFolderSelected} />
{/if}
