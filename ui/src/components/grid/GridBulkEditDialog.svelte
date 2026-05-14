<script>
  import { api } from '$lib/api.js';
  import BaseModal from '../common/BaseModal.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Checkbox from '$lib/components/ui/checkbox.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import Select from '$lib/components/ui/select.svelte';
  import RandomizationSettings from '../config/RandomizationSettings.svelte';
  import ProgressiveRateSettings from '../config/ProgressiveRateSettings.svelte';
  import StopConditionSettings from '../config/StopConditionSettings.svelte';
  import GridClientConfigFields from './GridClientConfigFields.svelte';
  import { builtInPresets } from '$lib/presets/index.js';
  import { normalizePreset, normalizePresets } from '$lib/customPreset.js';
  import {
    applyPresetToBulkState,
    buildBulkUpdateEntries,
    createBulkEditState,
    mergeBulkSectionsIntoInstance,
  } from '$lib/gridBulkEdit.js';
  import { buildFakerConfig } from '$lib/fakerConfig.js';
  import { ArrowUpDown, Clock, LoaderCircle, Settings, Target, Timer, X } from '@lucide/svelte';

  let {
    isOpen = $bindable(false),
    selectedIds = [],
    selectedSummaries = [],
    fallbackInstances = [],
    clients = [],
    clientVersions = {},
    currentForwardedPort = null,
    vpnPortSyncVisible = false,
    vpnPortSyncEnabled = true,
    networkStatusConfigured = true,
    networkStatusError = null,
    isServerMode = false,
    onApply = async () => {},
  } = $props();

  let loading = $state(false);
  let saving = $state(false);
  let loadError = $state('');
  let saveError = $state('');
  let customPresets = $state([]);
  let hydratedInstances = $state([]);
  let sections = $state(createBulkEditState().sections);
  let selectedPresetId = $state('');
  let clientValue = $state('qbittorrent');
  let versionValue = $state('');
  let clientPort = $state(6881);
  let clientVpnPortSync = $state(false);
  let uploadRate = $state(50);
  let downloadRate = $state(100);
  let completionPercent = $state(100);
  let initialUploaded = $state(0);
  let updateIntervalSeconds = $state(5);
  let scrapeInterval = $state(60);
  let randomizeRates = $state(true);
  let randomRangePercent = $state(20);
  let progressiveRatesEnabled = $state(false);
  let targetUploadRate = $state(100);
  let targetDownloadRate = $state(200);
  let progressiveDurationHours = $state(1);
  let stopAtRatioEnabled = $state(false);
  let stopAtRatio = $state(2.0);
  let randomizeRatio = $state(false);
  let randomRatioRangePercent = $state(10);
  let effectiveStopAtRatio = $state(null);
  let stopAtUploadedEnabled = $state(false);
  let stopAtUploadedGB = $state(10);
  let stopAtDownloadedEnabled = $state(false);
  let stopAtDownloadedGB = $state(10);
  let stopAtSeedTimeEnabled = $state(false);
  let stopAtSeedTimeHours = $state(24);
  let idleWhenNoLeechers = $state(false);
  let idleWhenNoSeeders = $state(false);
  let postStopAction = $state('idle');

  let selectedCount = $derived(selectedIds.length);
  let allPresets = $derived([...builtInPresets, ...normalizePresets(customPresets)]);
  let networkStatusUnavailable = $derived(networkStatusError === 'unavailable');
  let vpnPortSyncBlocked = $derived(
    !networkStatusConfigured || !vpnPortSyncEnabled || networkStatusUnavailable
  );
  let canSubmit = $derived(
    !loading &&
      !saving &&
      hydratedInstances.length > 0 &&
      Object.values(sections).some(section => section.apply)
  );

  function applyState(state) {
    sections = state.sections;
    selectedPresetId = state.selectedPresetId;
  }

  function setSectionApply(sectionKey, apply) {
    sections = {
      ...sections,
      [sectionKey]: {
        ...sections[sectionKey],
        apply,
      },
    };
  }

  function updateSection(sectionKey, updates) {
    sections = {
      ...sections,
      [sectionKey]: {
        ...sections[sectionKey],
        value: {
          ...sections[sectionKey].value,
          ...updates,
        },
      },
    };
  }

  $effect(() => {
    const client = sections.client?.value || {};
    const rates = sections.rates?.value || {};
    const initial = sections.initial?.value || {};
    const timing = sections.timing?.value || {};
    const randomization = sections.randomization?.value || {};
    const progressive = sections.progressive?.value || {};
    const stopConditions = sections.stopConditions?.value || {};

    clientValue = client.selectedClient || 'qbittorrent';
    versionValue = client.selectedClientVersion || '';
    clientPort = client.port ?? 6881;
    clientVpnPortSync = client.vpnPortSync ?? false;
    uploadRate = rates.uploadRate ?? 50;
    downloadRate = rates.downloadRate ?? 100;
    completionPercent = initial.completionPercent ?? 100;
    initialUploaded = initial.initialUploaded ?? 0;
    updateIntervalSeconds = timing.updateIntervalSeconds ?? 5;
    scrapeInterval = timing.scrapeInterval ?? 60;
    randomizeRates = randomization.randomizeRates ?? true;
    randomRangePercent = randomization.randomRangePercent ?? 20;
    progressiveRatesEnabled = progressive.progressiveRatesEnabled ?? false;
    targetUploadRate = progressive.targetUploadRate ?? 100;
    targetDownloadRate = progressive.targetDownloadRate ?? 200;
    progressiveDurationHours = progressive.progressiveDurationHours ?? 1;
    stopAtRatioEnabled = stopConditions.stopAtRatioEnabled ?? false;
    stopAtRatio = stopConditions.stopAtRatio ?? 2.0;
    randomizeRatio = stopConditions.randomizeRatio ?? false;
    randomRatioRangePercent = stopConditions.randomRatioRangePercent ?? 10;
    effectiveStopAtRatio = stopConditions.effectiveStopAtRatio ?? null;
    stopAtUploadedEnabled = stopConditions.stopAtUploadedEnabled ?? false;
    stopAtUploadedGB = stopConditions.stopAtUploadedGB ?? 10;
    stopAtDownloadedEnabled = stopConditions.stopAtDownloadedEnabled ?? false;
    stopAtDownloadedGB = stopConditions.stopAtDownloadedGB ?? 10;
    stopAtSeedTimeEnabled = stopConditions.stopAtSeedTimeEnabled ?? false;
    stopAtSeedTimeHours = stopConditions.stopAtSeedTimeHours ?? 24;
    idleWhenNoLeechers = stopConditions.idleWhenNoLeechers ?? false;
    idleWhenNoSeeders = stopConditions.idleWhenNoSeeders ?? false;
    postStopAction = stopConditions.postStopAction || 'idle';
  });

  function autoEnable(sectionKey, updates) {
    const nextUpdates =
      sectionKey === 'stopConditions' &&
      ('stopAtRatioEnabled' in updates ||
        'stopAtRatio' in updates ||
        'randomizeRatio' in updates ||
        'randomRatioRangePercent' in updates)
        ? { ...updates, effectiveStopAtRatio: null }
        : updates;

    setSectionApply(sectionKey, true);
    updateSection(sectionKey, nextUpdates);
  }

  function close() {
    isOpen = false;
    loadError = '';
    saveError = '';
  }

  function mapServerInstance(serverInst) {
    const config = serverInst.config || serverInst;
    return {
      id: String(serverInst.id),
      torrent: serverInst.torrent || null,
      selectedClient: config.client_type,
      selectedClientVersion: config.client_version,
      uploadRate: config.upload_rate,
      downloadRate: config.download_rate,
      port: config.port,
      vpnPortSync: config.vpn_port_sync || false,
      completionPercent: config.completion_percent,
      initialUploaded: Math.round((config.initial_uploaded || 0) / (1024 * 1024)),
      initialDownloaded: Math.round((config.initial_downloaded || 0) / (1024 * 1024)),
      effectiveStopAtRatio: config.effective_stop_at_ratio ?? null,
      randomizeRates: config.randomize_rates,
      randomRangePercent: config.random_range_percent,
      updateIntervalSeconds: config.update_interval_seconds ?? 5,
      scrapeInterval: config.scrape_interval ?? 60,
      stopAtRatioEnabled: config.stop_at_ratio !== null,
      stopAtRatio: config.stop_at_ratio || 2.0,
      randomizeRatio: config.randomize_ratio || false,
      randomRatioRangePercent: config.random_ratio_range_percent ?? 10,
      stopAtUploadedEnabled: config.stop_at_uploaded !== null,
      stopAtUploadedGB: (config.stop_at_uploaded || 0) / (1024 * 1024 * 1024),
      stopAtDownloadedEnabled: config.stop_at_downloaded !== null,
      stopAtDownloadedGB: (config.stop_at_downloaded || 0) / (1024 * 1024 * 1024),
      stopAtSeedTimeEnabled: config.stop_at_seed_time !== null,
      stopAtSeedTimeHours: (config.stop_at_seed_time || 0) / 3600,
      idleWhenNoLeechers: config.idle_when_no_leechers || false,
      idleWhenNoSeeders: config.idle_when_no_seeders || false,
      postStopAction: config.post_stop_action || 'idle',
      progressiveRatesEnabled: config.progressive_rates || false,
      targetUploadRate: config.target_upload_rate || 100,
      targetDownloadRate: config.target_download_rate || 200,
      progressiveDurationHours: (config.progressive_duration || 3600) / 3600,
    };
  }

  function mapSummaryInstance(summary = {}) {
    return {
      id: String(summary.id),
      torrent: null,
      selectedClient: 'qbittorrent',
      selectedClientVersion: clientVersions.qbittorrent?.[0] || '5.2.0',
      uploadRate: 50,
      downloadRate: 100,
      port: 6881,
      vpnPortSync: false,
      completionPercent: summary.torrentCompletion ?? 100,
      initialUploaded: Math.round((summary.uploaded || 0) / (1024 * 1024)),
      initialDownloaded: Math.round((summary.downloaded || 0) / (1024 * 1024)),
      effectiveStopAtRatio: null,
      randomizeRates: true,
      randomRangePercent: 20,
      updateIntervalSeconds: 5,
      scrapeInterval: 60,
      stopAtRatioEnabled: false,
      stopAtRatio: 2.0,
      randomizeRatio: false,
      randomRatioRangePercent: 10,
      stopAtUploadedEnabled: false,
      stopAtUploadedGB: 10,
      stopAtDownloadedEnabled: false,
      stopAtDownloadedGB: 10,
      stopAtSeedTimeEnabled: false,
      stopAtSeedTimeHours: 24,
      idleWhenNoLeechers: false,
      idleWhenNoSeeders: false,
      postStopAction: 'idle',
      progressiveRatesEnabled: false,
      targetUploadRate: 100,
      targetDownloadRate: 200,
      progressiveDurationHours: 1,
    };
  }

  function mapFallbackInstance(instance) {
    return {
      id: String(instance.id),
      torrent: instance.torrent || null,
      selectedClient: instance.selectedClient,
      selectedClientVersion: instance.selectedClientVersion,
      uploadRate: instance.uploadRate,
      downloadRate: instance.downloadRate,
      port: instance.port,
      vpnPortSync: instance.vpnPortSync ?? false,
      completionPercent: instance.completionPercent,
      initialUploaded: instance.initialUploaded,
      initialDownloaded: instance.initialDownloaded,
      effectiveStopAtRatio: instance.effectiveStopAtRatio ?? null,
      randomizeRates: instance.randomizeRates,
      randomRangePercent: instance.randomRangePercent,
      updateIntervalSeconds: instance.updateIntervalSeconds,
      scrapeInterval: instance.scrapeInterval,
      stopAtRatioEnabled: instance.stopAtRatioEnabled,
      stopAtRatio: instance.stopAtRatio,
      randomizeRatio: instance.randomizeRatio,
      randomRatioRangePercent: instance.randomRatioRangePercent,
      stopAtUploadedEnabled: instance.stopAtUploadedEnabled,
      stopAtUploadedGB: instance.stopAtUploadedGB,
      stopAtDownloadedEnabled: instance.stopAtDownloadedEnabled,
      stopAtDownloadedGB: instance.stopAtDownloadedGB,
      stopAtSeedTimeEnabled: instance.stopAtSeedTimeEnabled,
      stopAtSeedTimeHours: instance.stopAtSeedTimeHours,
      idleWhenNoLeechers: instance.idleWhenNoLeechers,
      idleWhenNoSeeders: instance.idleWhenNoSeeders,
      postStopAction: instance.postStopAction,
      progressiveRatesEnabled: instance.progressiveRatesEnabled,
      targetUploadRate: instance.targetUploadRate,
      targetDownloadRate: instance.targetDownloadRate,
      progressiveDurationHours: instance.progressiveDurationHours,
    };
  }

  function resolveHydratedInstance(id, byId) {
    const serverInst = byId.get(String(id));
    if (serverInst) {
      return mapServerInstance(serverInst);
    }

    const fallbackInst = fallbackInstances.find(item => String(item.id) === String(id));
    if (fallbackInst) {
      return mapFallbackInstance(fallbackInst);
    }

    return mapSummaryInstance(selectedSummaries.find(item => String(item.id) === String(id)));
  }

  async function hydrate() {
    if (!isOpen || selectedIds.length === 0) {
      hydratedInstances = [];
      applyState(createBulkEditState());
      return;
    }

    loading = true;
    loadError = '';
    saveError = '';

    try {
      customPresets = normalizePresets((await api.listCustomPresets()) || []);
      const serverInstances = (await api.listInstances()) || [];
      const byId = new Map(serverInstances.map(inst => [String(inst.id), inst]));
      hydratedInstances = selectedIds.map(id => resolveHydratedInstance(id, byId));
      applyState(createBulkEditState(hydratedInstances));
    } catch (error) {
      hydratedInstances = selectedIds.map(id => resolveHydratedInstance(id, new Map()));
      applyState(createBulkEditState(hydratedInstances));
      if (hydratedInstances.length === 0) {
        loadError = error.message || 'Failed to load selected instances.';
      }
    } finally {
      loading = false;
    }
  }

  function handlePresetChange(event) {
    const presetId = event.target.value;
    selectedPresetId = presetId;
    if (!presetId) {
      return;
    }

    const preset = normalizePreset(allPresets.find(item => item.id === presetId));
    if (!preset) {
      return;
    }

    applyState(
      applyPresetToBulkState(
        {
          selectedPresetId,
          selectedCount,
          sections,
        },
        preset
      )
    );
  }

  async function handleApply() {
    if (!canSubmit) {
      return;
    }

    saving = true;
    saveError = '';

    try {
      const mergedInstances = hydratedInstances.map(instance =>
        mergeBulkSectionsIntoInstance(instance, sections)
      );
      const entries = buildBulkUpdateEntries(hydratedInstances, sections, instance =>
        buildFakerConfig(instance, clientVersions, {
          isServerMode,
          useCalculatedInitialDownloaded: true,
        })
      );
      await onApply(entries, mergedInstances);
      close();
    } catch (error) {
      saveError = error.message || 'Failed to apply changes.';
    } finally {
      saving = false;
    }
  }

  $effect(() => {
    if (isOpen) {
      hydrate();
    }
  });
</script>

{#if isOpen}
  <BaseModal
    bind:open={isOpen}
    onClose={close}
    titleId="grid-bulk-edit-title"
    maxWidthClass="max-w-4xl"
    panelClass="max-h-[90vh] overflow-y-auto"
  >
    <div class="flex items-center justify-between border-b border-border p-4">
      <div>
        <h2 id="grid-bulk-edit-title" class="text-lg font-semibold text-foreground">
          Edit Selected
        </h2>
        <p class="text-xs text-muted-foreground">
          Apply checked sections to {selectedCount} selected instance{selectedCount !== 1
            ? 's'
            : ''}.
        </p>
      </div>
      <button
        onclick={close}
        class="rounded p-1 hover:bg-muted bg-transparent border-0 cursor-pointer"
        aria-label="Close"
      >
        <X size={18} class="text-muted-foreground" />
      </button>
    </div>

    <div class="space-y-4 p-4">
      {#if loadError}
        <div
          class="rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
        >
          {loadError}
        </div>
      {/if}

      {#if saveError}
        <div
          class="rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
        >
          {saveError}
        </div>
      {/if}

      {#if loading}
        <div
          class="flex items-center gap-2 rounded-lg border border-border bg-muted/30 p-4 text-sm text-muted-foreground"
        >
          <LoaderCircle size={16} class="animate-spin" />
          Loading selected instance configs...
        </div>
      {:else if hydratedInstances.length > 0}
        <div class="rounded-lg border border-border bg-muted/20 p-4 space-y-2">
          <Label for="bulk-preset" class="text-sm font-medium">Preset</Label>
          <Select id="bulk-preset" value={selectedPresetId} onchange={handlePresetChange}>
            <option value="">No preset</option>
            {#each allPresets as preset (preset.id)}
              <option value={preset.id}>{preset.name}</option>
            {/each}
          </Select>
          <p class="text-xs text-muted-foreground">
            Presets enable and fill the sections they define. Tags stay in the separate tag action.
          </p>
        </div>

        <section class="relative z-10 rounded-lg border border-border">
          <div
            class="flex items-center justify-between gap-3 rounded-t-lg border-b border-border bg-muted/30 px-4 py-3"
          >
            <div class="flex items-center gap-2">
              <Checkbox
                checked={sections.client.apply}
                aria-label="Override client settings"
                onchange={checked => setSectionApply('client', checked)}
              />
              <div>
                <div class="flex items-center gap-2 text-sm font-medium">
                  <span>Override</span>
                  <Settings size={16} class="text-muted-foreground" />
                  Client
                </div>
                <div class="text-xs text-muted-foreground">
                  {sections.client.mixed
                    ? 'Mixed current values across selection'
                    : 'Same current value across selection'}
                </div>
              </div>
            </div>
            <span class="text-[11px] text-muted-foreground"
              >{sections.client.apply ? 'Will apply' : 'Skipped'}</span
            >
          </div>
          <div class="p-4">
            <GridClientConfigFields
              {clients}
              versions={clientVersions[clientValue] || []}
              bind:selectedClient={clientValue}
              bind:selectedVersion={versionValue}
              bind:port={clientPort}
              onClientChange={() =>
                autoEnable('client', {
                  selectedClient: clientValue,
                  selectedClientVersion: clientVersions[clientValue]?.[0] || versionValue,
                })}
              onVersionChange={() => autoEnable('client', { selectedClientVersion: versionValue })}
              portDisabled={vpnPortSyncVisible &&
                clientVpnPortSync &&
                networkStatusConfigured &&
                vpnPortSyncEnabled}
              onPortInput={() =>
                autoEnable('client', { port: parseInt(clientPort || '6881', 10) || 6881 })}
            >
              {#snippet portHeaderExtra()}
                {#if vpnPortSyncVisible}
                  <div class="flex items-center gap-1.5 text-[11px] text-muted-foreground">
                    <Checkbox
                      checked={clientVpnPortSync}
                      disabled={vpnPortSyncBlocked && !clientVpnPortSync}
                      onchange={checked => {
                        clientVpnPortSync = checked;
                        const updates = { vpnPortSync: checked };
                        if (checked && currentForwardedPort) {
                          clientPort = currentForwardedPort;
                          updates.port = currentForwardedPort;
                        }
                        autoEnable('client', updates);
                      }}
                    />
                    VPN sync
                  </div>
                {/if}
              {/snippet}

              {#snippet portFooter()}
                {#if vpnPortSyncVisible && !networkStatusConfigured}
                  <p class="mt-1 text-[11px] text-amber-400">No VPN configured.</p>
                {:else if vpnPortSyncVisible && !vpnPortSyncEnabled}
                  <p class="mt-1 text-[11px] text-amber-400">VPN sync is disabled on the server.</p>
                {:else if vpnPortSyncVisible && networkStatusUnavailable}
                  <p class="mt-1 text-[11px] text-amber-400">Gluetun status is unavailable.</p>
                {/if}
              {/snippet}
            </GridClientConfigFields>
          </div>
        </section>

        <section class="rounded-lg border border-border overflow-hidden">
          <div
            class="flex items-center justify-between gap-3 border-b border-border bg-muted/30 px-4 py-3"
          >
            <div class="flex items-center gap-2">
              <Checkbox
                checked={sections.rates.apply}
                aria-label="Override transfer rates"
                onchange={checked => setSectionApply('rates', checked)}
              />
              <div>
                <div class="flex items-center gap-2 text-sm font-medium">
                  <span>Override</span>
                  <ArrowUpDown size={16} class="text-muted-foreground" />
                  Transfer Rates
                </div>
                <div class="text-xs text-muted-foreground">
                  {sections.rates.mixed
                    ? 'Mixed current values across selection'
                    : 'Same current value across selection'}
                </div>
              </div>
            </div>
            <span class="text-[11px] text-muted-foreground"
              >{sections.rates.apply ? 'Will apply' : 'Skipped'}</span
            >
          </div>
          <div class="p-4">
            <GridClientConfigFields
              mode="rates"
              bind:uploadRate
              bind:downloadRate
              showUnit={true}
              onUploadInput={() =>
                autoEnable('rates', { uploadRate: parseFloat(uploadRate || '0') })}
              onDownloadInput={() =>
                autoEnable('rates', { downloadRate: parseFloat(downloadRate || '0') })}
            />
          </div>
        </section>

        <section class="rounded-lg border border-border overflow-hidden">
          <div
            class="flex items-center justify-between gap-3 border-b border-border bg-muted/30 px-4 py-3"
          >
            <div class="flex items-center gap-2">
              <Checkbox
                checked={sections.initial.apply}
                aria-label="Override initial state"
                onchange={checked => setSectionApply('initial', checked)}
              />
              <div>
                <div class="flex items-center gap-2 text-sm font-medium">
                  <span>Override</span>
                  <Clock size={16} class="text-muted-foreground" />
                  Initial State
                </div>
                <div class="text-xs text-muted-foreground">
                  {sections.initial.mixed
                    ? 'Mixed current values across selection'
                    : 'Same current value across selection'}
                </div>
              </div>
            </div>
            <span class="text-[11px] text-muted-foreground"
              >{sections.initial.apply ? 'Will apply' : 'Skipped'}</span
            >
          </div>
          <div class="grid gap-3 p-4 md:grid-cols-2">
            <div>
              <Label class="mb-1.5 block text-xs text-muted-foreground">Completion</Label>
              <div class="flex items-center gap-2">
                <Input
                  type="number"
                  bind:value={completionPercent}
                  min="0"
                  max="100"
                  oninput={() =>
                    autoEnable('initial', {
                      completionPercent: parseFloat(completionPercent || '0'),
                    })}
                />
                <span class="text-sm text-muted-foreground">%</span>
              </div>
            </div>
            <div>
              <Label class="mb-1.5 block text-xs text-muted-foreground">Already Uploaded</Label>
              <div class="flex items-center gap-2">
                <Input
                  type="number"
                  bind:value={initialUploaded}
                  min="0"
                  step="1"
                  oninput={() =>
                    autoEnable('initial', {
                      initialUploaded: Math.round(parseFloat(initialUploaded || '0')),
                    })}
                />
                <span class="text-sm text-muted-foreground">MB</span>
              </div>
            </div>
          </div>
        </section>

        <section class="rounded-lg border border-border overflow-hidden">
          <div
            class="flex items-center justify-between gap-3 border-b border-border bg-muted/30 px-4 py-3"
          >
            <div class="flex items-center gap-2">
              <Checkbox
                checked={sections.timing.apply}
                aria-label="Override timing"
                onchange={checked => setSectionApply('timing', checked)}
              />
              <div>
                <div class="flex items-center gap-2 text-sm font-medium">
                  <span>Override</span>
                  <Timer size={16} class="text-muted-foreground" />
                  Timing
                </div>
                <div class="text-xs text-muted-foreground">
                  {sections.timing.mixed
                    ? 'Mixed current values across selection'
                    : 'Same current value across selection'}
                </div>
              </div>
            </div>
            <span class="text-[11px] text-muted-foreground"
              >{sections.timing.apply ? 'Will apply' : 'Skipped'}</span
            >
          </div>
          <div class="p-4">
            <GridClientConfigFields
              mode="timing"
              bind:updateIntervalSeconds
              bind:scrapeInterval
              showUnit={true}
              onRefreshInput={() =>
                autoEnable('timing', {
                  updateIntervalSeconds: parseInt(updateIntervalSeconds || '5', 10) || 5,
                })}
              onScrapeInput={() =>
                autoEnable('timing', {
                  scrapeInterval: parseInt(scrapeInterval || '60', 10) || 60,
                })}
            />
          </div>
        </section>

        <section class="rounded-lg border border-border overflow-hidden">
          <div
            class="flex items-center justify-between gap-3 border-b border-border bg-muted/30 px-4 py-3"
          >
            <div class="flex items-center gap-2">
              <Checkbox
                checked={sections.randomization.apply}
                aria-label="Override randomization"
                onchange={checked => setSectionApply('randomization', checked)}
              />
              <div>
                <div class="flex items-center gap-2 text-sm font-medium">
                  <span>Override</span>
                  <span>Randomization</span>
                </div>
                <div class="text-xs text-muted-foreground">
                  {sections.randomization.mixed
                    ? 'Mixed current values across selection'
                    : 'Same current value across selection'}
                </div>
              </div>
            </div>
            <span class="text-[11px] text-muted-foreground"
              >{sections.randomization.apply ? 'Will apply' : 'Skipped'}</span
            >
          </div>
          <div class="p-4">
            <RandomizationSettings
              bind:enabled={randomizeRates}
              bind:rangePercent={randomRangePercent}
              {uploadRate}
              {downloadRate}
              onchange={updates => autoEnable('randomization', updates)}
            />
          </div>
        </section>

        <section class="rounded-lg border border-border overflow-hidden">
          <div
            class="flex items-center justify-between gap-3 border-b border-border bg-muted/30 px-4 py-3"
          >
            <div class="flex items-center gap-2">
              <Checkbox
                checked={sections.progressive.apply}
                aria-label="Override progressive rates"
                onchange={checked => setSectionApply('progressive', checked)}
              />
              <div>
                <div class="flex items-center gap-2 text-sm font-medium">
                  <span>Override</span>
                  <span>Progressive Rates</span>
                </div>
                <div class="text-xs text-muted-foreground">
                  {sections.progressive.mixed
                    ? 'Mixed current values across selection'
                    : 'Same current value across selection'}
                </div>
              </div>
            </div>
            <span class="text-[11px] text-muted-foreground"
              >{sections.progressive.apply ? 'Will apply' : 'Skipped'}</span
            >
          </div>
          <div class="p-4">
            <ProgressiveRateSettings
              bind:enabled={progressiveRatesEnabled}
              bind:durationHours={progressiveDurationHours}
              bind:targetUploadRate
              bind:targetDownloadRate
              {uploadRate}
              {downloadRate}
              onchange={updates => autoEnable('progressive', updates)}
            />
          </div>
        </section>

        <section class="rounded-lg border border-border overflow-hidden">
          <div
            class="flex items-center justify-between gap-3 border-b border-border bg-muted/30 px-4 py-3"
          >
            <div class="flex items-center gap-2">
              <Checkbox
                checked={sections.stopConditions.apply}
                aria-label="Override stop conditions"
                onchange={checked => setSectionApply('stopConditions', checked)}
              />
              <div>
                <div class="flex items-center gap-2 text-sm font-medium">
                  <span>Override</span>
                  <Target size={16} class="text-muted-foreground" />
                  Stop Conditions
                </div>
                <div class="text-xs text-muted-foreground">
                  {sections.stopConditions.mixed
                    ? 'Mixed current values across selection'
                    : 'Same current value across selection'}
                </div>
              </div>
            </div>
            <span class="text-[11px] text-muted-foreground"
              >{sections.stopConditions.apply ? 'Will apply' : 'Skipped'}</span
            >
          </div>
          <div class="p-4">
            <StopConditionSettings
              bind:stopAtRatioEnabled
              bind:stopAtRatio
              bind:randomizeRatio
              bind:randomRatioRangePercent
              {effectiveStopAtRatio}
              bind:stopAtUploadedEnabled
              bind:stopAtUploadedGB
              bind:stopAtDownloadedEnabled
              bind:stopAtDownloadedGB
              bind:stopAtSeedTimeEnabled
              bind:stopAtSeedTimeHours
              bind:idleWhenNoLeechers
              bind:idleWhenNoSeeders
              bind:postStopAction
              {completionPercent}
              onchange={updates => autoEnable('stopConditions', updates)}
            />
          </div>
        </section>
      {/if}
    </div>

    <div class="flex items-center justify-between gap-3 border-t border-border p-4">
      <p class="text-xs text-muted-foreground">
        Only checked sections will be merged into each selected instance.
      </p>
      <div class="flex items-center gap-2">
        <Button onclick={close} size="sm" variant="secondary">
          {#snippet children()}Cancel{/snippet}
        </Button>
        <Button onclick={handleApply} size="sm" disabled={!canSubmit}>
          {#snippet children()}
            {#if saving}
              Applying...
            {:else}
              Apply Changes
            {/if}
          {/snippet}
        </Button>
      </div>
    </div>
  </BaseModal>
{/if}
