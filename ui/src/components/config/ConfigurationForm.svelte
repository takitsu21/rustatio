<script>
  import Card from '$lib/components/ui/card.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import {
    Settings,
    ArrowUpDown,
    Clock,
    Upload,
    Download,
  } from '@lucide/svelte';
  import ClientIcon from './ClientIcon.svelte';
  import ClientSelect from './ClientSelect.svelte';
  import VersionSelect from './VersionSelect.svelte';
  import RandomizationSettings from './RandomizationSettings.svelte';
  import ProgressiveRateSettings from './ProgressiveRateSettings.svelte';

  let {
    clients,
    clientVersions,
    selectedClient,
    selectedClientVersion,
    port,
    uploadRate,
    downloadRate,
    completionPercent,
    initialUploaded,
    updateIntervalSeconds,
    randomizeRates,
    randomRangePercent,
    progressiveRatesEnabled,
    targetUploadRate,
    targetDownloadRate,
    progressiveDurationHours,
    isRunning,
    onUpdate,
  } = $props();

  // Local state for form values (defaults match createDefaultInstance)
  let localSelectedClient = $state('qbittorrent');
  let localSelectedClientVersion = $state(null);
  let localPort = $state(6881);
  let localUploadRate = $state(50);
  let localDownloadRate = $state(100);
  let localCompletionPercent = $state(0);
  let localInitialUploaded = $state(0);
  let localUpdateIntervalSeconds = $state(5);
  let localRandomizeRates = $state(true);
  let localRandomRangePercent = $state(20);
  let localProgressiveRatesEnabled = $state(false);
  let localTargetUploadRate = $state(100);
  let localTargetDownloadRate = $state(200);
  let localProgressiveDurationHours = $state(1);

  // Track if we're currently editing to prevent external updates from interfering
  let isEditing = $state(false);

  // Update local state when props change (only when not actively editing)
  $effect(() => {
    if (!isEditing) {
      localSelectedClient = selectedClient;
      localSelectedClientVersion = selectedClientVersion;
      localPort = port;
      localUploadRate = uploadRate;
      localDownloadRate = downloadRate;
      localCompletionPercent = completionPercent;
      localInitialUploaded = initialUploaded;
      localUpdateIntervalSeconds = updateIntervalSeconds;
      localRandomizeRates = randomizeRates;
      localRandomRangePercent = randomRangePercent;
      localProgressiveRatesEnabled = progressiveRatesEnabled;
      localTargetUploadRate = targetUploadRate;
      localTargetDownloadRate = targetDownloadRate;
      localProgressiveDurationHours = progressiveDurationHours;
    }
  });

  // Helper to call onUpdate
  function updateValue(key, value) {
    if (onUpdate) {
      onUpdate({ [key]: value });
    }
  }

  // Validation constants
  const PORT_MIN = 1024;
  const PORT_MAX = 65535;
  const COMPLETION_MIN = 0;
  const COMPLETION_MAX = 100;

  // Validate and sanitize port value
  function validatePort(value) {
    const parsed = parseInt(value, 10);
    if (isNaN(parsed) || parsed < PORT_MIN) {
      return PORT_MIN;
    }
    if (parsed > PORT_MAX) {
      return PORT_MAX;
    }
    return parsed;
  }

  // Validate and sanitize completion percent value
  function validateCompletionPercent(value) {
    const parsed = parseFloat(value);
    if (isNaN(parsed) || parsed < COMPLETION_MIN) {
      return COMPLETION_MIN;
    }
    if (parsed > COMPLETION_MAX) {
      return COMPLETION_MAX;
    }
    return parsed;
  }

  // Handle port input - only update if it's a valid number
  function handlePortInput() {
    const parsed = parseInt(localPort, 10);
    if (!isNaN(parsed)) {
      updateValue('port', parsed);
    }
  }

  // Handle port blur - validate and fix invalid values
  function handlePortBlur() {
    const validPort = validatePort(localPort);
    if (validPort !== localPort) {
      localPort = validPort;
      updateValue('port', validPort);
    }
    isEditing = false;
  }

  // Handle completion percent input
  function handleCompletionPercentInput() {
    const parsed = parseFloat(localCompletionPercent);
    if (!isNaN(parsed)) {
      updateValue('completionPercent', parsed);
    }
  }

  // Handle completion percent blur - validate and fix invalid values
  function handleCompletionPercentBlur() {
    const validPercent = validateCompletionPercent(localCompletionPercent);
    if (validPercent !== localCompletionPercent) {
      localCompletionPercent = validPercent;
      updateValue('completionPercent', validPercent);
    }
    isEditing = false;
  }

  // Focus/blur handlers to track editing state
  function handleFocus() {
    isEditing = true;
  }

  function handleBlur() {
    isEditing = false;
  }
</script>

<Card class="p-3">
  <h2 class="mb-4 text-primary text-lg font-semibold flex items-center gap-2">
    <Settings size={20} /> Configuration
  </h2>

  <!-- Client Settings -->
  <div class="mb-4">
    <div class="flex items-center gap-2 mb-3">
      <ClientIcon clientId={localSelectedClient} size={18} />
      <span class="text-sm font-medium">Client</span>
    </div>
    <div class="bg-muted/50 rounded-lg border border-border p-3">
      <div class="grid grid-cols-3 gap-3">
        <div>
          <Label for="client" class="text-xs text-muted-foreground mb-1.5 block">Type</Label>
          <ClientSelect
            {clients}
            bind:value={localSelectedClient}
            disabled={isRunning}
            onchange={() => updateValue('selectedClient', localSelectedClient)}
          />
        </div>
        <div>
          <Label for="clientVersion" class="text-xs text-muted-foreground mb-1.5 block"
            >Version</Label
          >
          <VersionSelect
            versions={clientVersions[localSelectedClient] || []}
            bind:value={localSelectedClientVersion}
            disabled={isRunning}
            onchange={() => updateValue('selectedClientVersion', localSelectedClientVersion)}
          />
        </div>
        <div>
          <Label for="port" class="text-xs text-muted-foreground mb-1.5 block">Port</Label>
          <Input
            id="port"
            type="number"
            bind:value={localPort}
            disabled={isRunning}
            min="1024"
            max="65535"
            class="h-9"
            onfocus={handleFocus}
            onblur={handlePortBlur}
            oninput={handlePortInput}
          />
        </div>
      </div>
    </div>
  </div>

  <!-- Transfer Rates -->
  <div class="mb-4">
    <div class="flex items-center gap-2 mb-3">
      <ArrowUpDown size={16} class="text-muted-foreground" />
      <span class="text-sm font-medium">Transfer Rates</span>
    </div>
    <div class="bg-muted/50 rounded-lg border border-border overflow-hidden">
      <div class="grid grid-cols-2">
        <div class="p-3 border-r border-border">
          <div class="flex items-center gap-2 mb-2">
            <Upload size={14} class="text-stat-upload" />
            <span class="text-xs text-muted-foreground">Upload</span>
          </div>
          <div class="flex items-center gap-2">
            <Input
              id="upload"
              type="number"
              bind:value={localUploadRate}
              disabled={isRunning}
              min="0"
              step="0.1"
              class="flex-1 h-9 text-center font-medium"
              onfocus={handleFocus}
              onblur={handleBlur}
              oninput={() => updateValue('uploadRate', localUploadRate)}
            />
            <span class="text-sm text-muted-foreground">KB/s</span>
          </div>
        </div>
        <div class="p-3">
          <div class="flex items-center gap-2 mb-2">
            <Download size={14} class="text-stat-download" />
            <span class="text-xs text-muted-foreground">Download</span>
          </div>
          <div class="flex items-center gap-2">
            <Input
              id="download"
              type="number"
              bind:value={localDownloadRate}
              disabled={isRunning}
              min="0"
              step="0.1"
              class="flex-1 h-9 text-center font-medium"
              onfocus={handleFocus}
              onblur={handleBlur}
              oninput={() => updateValue('downloadRate', localDownloadRate)}
            />
            <span class="text-sm text-muted-foreground">KB/s</span>
          </div>
          {#if localDownloadRate > 0 && localCompletionPercent >= 100}
            <p class="text-[10px] text-orange-500 mt-1">No effect at 100% completion</p>
          {/if}
        </div>
      </div>
    </div>
  </div>

  <!-- Initial State -->
  <div class="mb-4">
    <div class="flex items-center gap-2 mb-3">
      <Clock size={16} class="text-muted-foreground" />
      <span class="text-sm font-medium">Initial State</span>
    </div>
    <div class="bg-muted/50 rounded-lg border border-border p-3">
      <div class="grid grid-cols-3 gap-3">
        <div>
          <Label for="completion" class="text-xs text-muted-foreground mb-1.5 block"
            >Completion</Label
          >
          <div class="flex items-center gap-2">
            <Input
              id="completion"
              type="number"
              bind:value={localCompletionPercent}
              disabled={isRunning}
              min="0"
              max="100"
              class="flex-1 h-9 text-center"
              onfocus={handleFocus}
              onblur={handleCompletionPercentBlur}
              oninput={handleCompletionPercentInput}
            />
            <span class="text-sm text-muted-foreground">%</span>
          </div>
        </div>
        <div>
          <Label for="initialUp" class="text-xs text-muted-foreground mb-1.5 block"
            >Already Uploaded</Label
          >
          <div class="flex items-center gap-2">
            <Input
              id="initialUp"
              type="number"
              bind:value={localInitialUploaded}
              disabled={isRunning}
              min="0"
              step="1"
              class="flex-1 h-9 text-center"
              onfocus={handleFocus}
              onblur={handleBlur}
              oninput={() => updateValue('initialUploaded', Math.round(localInitialUploaded || 0))}
            />
            <span class="text-sm text-muted-foreground">MB</span>
          </div>
        </div>
        <div>
          <Label for="updateInterval" class="text-xs text-muted-foreground mb-1.5 block"
            >Update Interval</Label
          >
          <div class="flex items-center gap-2">
            <Input
              id="updateInterval"
              type="number"
              bind:value={localUpdateIntervalSeconds}
              disabled={isRunning}
              min="1"
              max="300"
              step="1"
              class="flex-1 h-9 text-center"
              onfocus={handleFocus}
              onblur={handleBlur}
              oninput={() => updateValue('updateIntervalSeconds', localUpdateIntervalSeconds)}
            />
            <span class="text-sm text-muted-foreground">sec</span>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- Randomization -->
  <div class="mb-3">
    <RandomizationSettings
      bind:enabled={localRandomizeRates}
      bind:rangePercent={localRandomRangePercent}
      uploadRate={localUploadRate}
      downloadRate={localDownloadRate}
      disabled={isRunning}
      onchange={updates => {
        for (const [key, value] of Object.entries(updates)) updateValue(key, value);
      }}
    />
  </div>

  <!-- Progressive Rates -->
  <div class="mb-0">
    <ProgressiveRateSettings
      bind:enabled={localProgressiveRatesEnabled}
      bind:durationHours={localProgressiveDurationHours}
      bind:targetUploadRate={localTargetUploadRate}
      bind:targetDownloadRate={localTargetDownloadRate}
      uploadRate={localUploadRate}
      downloadRate={localDownloadRate}
      disabled={isRunning}
      onchange={updates => {
        for (const [key, value] of Object.entries(updates)) updateValue(key, value);
      }}
    />
  </div>
</Card>
