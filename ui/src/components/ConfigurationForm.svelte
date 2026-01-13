<script>
  import Card from '$lib/components/ui/card.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Select from '$lib/components/ui/select.svelte';
  import Checkbox from '$lib/components/ui/checkbox.svelte';

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

  // Local state for form values
  let localSelectedClient = $state();
  let localSelectedClientVersion = $state();
  let localPort = $state();
  let localUploadRate = $state();
  let localDownloadRate = $state();
  let localCompletionPercent = $state();
  let localInitialUploaded = $state();
  let localUpdateIntervalSeconds = $state();
  let localRandomizeRates = $state();
  let localRandomRangePercent = $state();
  let localProgressiveRatesEnabled = $state();
  let localTargetUploadRate = $state();
  let localTargetDownloadRate = $state();
  let localProgressiveDurationHours = $state();

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
  <h2 class="mb-3 text-primary text-lg font-semibold">⚙️ Configuration</h2>

  <!-- Client Settings -->
  <div class="mb-3">
    <h3
      class="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3 pb-2 border-b border-border"
    >
      Client
    </h3>
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
      <div class="flex flex-col gap-2">
        <Label for="client">Type</Label>
        <Select
          id="client"
          bind:value={localSelectedClient}
          disabled={isRunning}
          onchange={() => updateValue('selectedClient', localSelectedClient)}
        >
          {#each clients as client (client.id)}
            <option value={client.id}>{client.name}</option>
          {/each}
        </Select>
      </div>

      <div class="flex flex-col gap-2">
        <Label for="clientVersion">Version</Label>
        <Select
          id="clientVersion"
          bind:value={localSelectedClientVersion}
          disabled={isRunning}
          onchange={() => updateValue('selectedClientVersion', localSelectedClientVersion)}
        >
          {#each clientVersions[localSelectedClient] || [] as version (version)}
            <option value={version}>{version}</option>
          {/each}
        </Select>
      </div>

      <div class="flex flex-col gap-2">
        <Label for="port">Port</Label>
        <Input
          id="port"
          type="number"
          bind:value={localPort}
          disabled={isRunning}
          min="1024"
          max="65535"
          onfocus={handleFocus}
          onblur={handlePortBlur}
          oninput={handlePortInput}
        />
        <span class="text-xs text-muted-foreground">Range: 1024-65535</span>
      </div>
    </div>
  </div>

  <!-- Transfer Rates -->
  <div class="mb-3">
    <h3
      class="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3 pb-2 border-b border-border"
    >
      Transfer Rates
    </h3>
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <div class="flex flex-col gap-2">
        <Label for="upload">↑ Upload (KB/s)</Label>
        <Input
          id="upload"
          type="number"
          bind:value={localUploadRate}
          disabled={isRunning}
          min="0"
          step="0.1"
          onfocus={handleFocus}
          onblur={handleBlur}
          oninput={() => updateValue('uploadRate', localUploadRate)}
        />
      </div>

      <div class="flex flex-col gap-2">
        <Label for="download">↓ Download (KB/s)</Label>
        <Input
          id="download"
          type="number"
          bind:value={localDownloadRate}
          disabled={isRunning}
          min="0"
          step="0.1"
          onfocus={handleFocus}
          onblur={handleBlur}
          oninput={() => updateValue('downloadRate', localDownloadRate)}
        />
      </div>
    </div>
  </div>

  <!-- Initial State -->
  <div class="mb-3">
    <h3
      class="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3 pb-2 border-b border-border"
    >
      Initial State
    </h3>
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
      <div class="flex flex-col gap-2">
        <Label for="completion">Completion %</Label>
        <Input
          id="completion"
          type="number"
          bind:value={localCompletionPercent}
          disabled={isRunning}
          min="0"
          max="100"
          onfocus={handleFocus}
          onblur={handleCompletionPercentBlur}
          oninput={handleCompletionPercentInput}
        />
        <span class="text-xs text-muted-foreground">Range: 0-100</span>
      </div>

      <div class="flex flex-col gap-2">
        <Label for="initialUp">Uploaded (MB)</Label>
        <Input
          id="initialUp"
          type="number"
          bind:value={localInitialUploaded}
          disabled={isRunning}
          min="0"
          onfocus={handleFocus}
          onblur={handleBlur}
          oninput={() => updateValue('initialUploaded', localInitialUploaded)}
        />
      </div>

      <div class="flex flex-col gap-2">
        <Label for="updateInterval">Interval (sec)</Label>
        <Input
          id="updateInterval"
          type="number"
          bind:value={localUpdateIntervalSeconds}
          disabled={isRunning}
          min="1"
          max="300"
          step="1"
          onfocus={handleFocus}
          onblur={handleBlur}
          oninput={() => updateValue('updateIntervalSeconds', localUpdateIntervalSeconds)}
        />
      </div>
    </div>
  </div>

  <!-- Randomization -->
  <div class="mb-3">
    <div class="flex items-center gap-3 mb-3">
      <Checkbox
        id="randomize"
        checked={localRandomizeRates}
        disabled={isRunning}
        onchange={checked => {
          localRandomizeRates = checked;
          updateValue('randomizeRates', checked);
        }}
      />
      <Label for="randomize" class="cursor-pointer font-medium"
        >Randomize rates for realistic behavior</Label
      >
    </div>

    {#if localRandomizeRates}
      <div class="bg-muted p-5 rounded-lg border border-border">
        <div class="flex justify-between items-center mb-3">
          <Label for="randomRange">Random Range</Label>
          <span
            class="text-lg font-bold text-primary px-3 py-1 bg-background rounded-md border border-primary"
            >±{localRandomRangePercent}%</span
          >
        </div>
        <input
          id="randomRange"
          type="range"
          bind:value={localRandomRangePercent}
          disabled={isRunning}
          min="1"
          max="50"
          step="1"
          class="w-full h-2 rounded bg-primary appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-6 [&::-webkit-slider-thumb]:h-6 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-white [&::-webkit-slider-thumb]:border-4 [&::-webkit-slider-thumb]:border-primary [&::-webkit-slider-thumb]:shadow-lg [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:transition-transform [&::-webkit-slider-thumb]:hover:scale-110 [&::-moz-range-thumb]:w-6 [&::-moz-range-thumb]:h-6 [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:bg-white [&::-moz-range-thumb]:border-4 [&::-moz-range-thumb]:border-primary [&::-moz-range-thumb]:shadow-lg [&::-moz-range-thumb]:cursor-pointer [&::-moz-range-thumb]:transition-transform [&::-moz-range-thumb]:hover:scale-110 [&::-moz-range-track]:bg-transparent disabled:opacity-50 disabled:cursor-not-allowed"
          onfocus={handleFocus}
          onblur={handleBlur}
          oninput={() => updateValue('randomRangePercent', localRandomRangePercent)}
        />
        <div class="flex justify-between mt-2">
          <span class="text-xs text-muted-foreground">±1%</span>
          <span class="text-xs text-muted-foreground">±50%</span>
        </div>
      </div>
    {/if}
  </div>

  <!-- Progressive Rates -->
  <div class="mb-0">
    <h3
      class="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3 pb-2 border-b border-border"
    >
      Progressive Rates
    </h3>
    <div class="flex items-center gap-3 p-2 bg-muted rounded-md mb-3">
      <Checkbox
        id="progressive-enabled"
        checked={localProgressiveRatesEnabled}
        disabled={isRunning}
        onchange={checked => {
          localProgressiveRatesEnabled = checked;
          updateValue('progressiveRatesEnabled', checked);
        }}
      />
      <Label
        for="progressive-enabled"
        class="cursor-pointer text-sm text-muted-foreground font-medium"
        >Enable Progressive Adjustment</Label
      >
    </div>

    {#if localProgressiveRatesEnabled}
      <div class="flex flex-col gap-2">
        <div class="flex items-center gap-2 p-2 bg-muted rounded-md">
          <Label for="targetUpload" class="min-w-[70px] text-xs text-muted-foreground font-semibold"
            >↑ Target</Label
          >
          <Input
            id="targetUpload"
            type="number"
            bind:value={localTargetUploadRate}
            disabled={isRunning}
            min="0"
            step="0.1"
            class="flex-1 max-w-[100px] h-9"
            onfocus={handleFocus}
            onblur={handleBlur}
            oninput={() => updateValue('targetUploadRate', localTargetUploadRate)}
          />
          <span class="text-xs text-muted-foreground font-semibold min-w-[40px]">KB/s</span>
        </div>

        <div class="flex items-center gap-2 p-2 bg-muted rounded-md">
          <Label
            for="targetDownload"
            class="min-w-[70px] text-xs text-muted-foreground font-semibold">↓ Target</Label
          >
          <Input
            id="targetDownload"
            type="number"
            bind:value={localTargetDownloadRate}
            disabled={isRunning}
            min="0"
            step="0.1"
            class="flex-1 max-w-[100px] h-9"
            onfocus={handleFocus}
            onblur={handleBlur}
            oninput={() => updateValue('targetDownloadRate', localTargetDownloadRate)}
          />
          <span class="text-xs text-muted-foreground font-semibold min-w-[40px]">KB/s</span>
        </div>

        <div class="flex items-center gap-2 p-2 bg-muted rounded-md">
          <Label
            for="progressiveDuration"
            class="min-w-[70px] text-xs text-muted-foreground font-semibold">Duration</Label
          >
          <Input
            id="progressiveDuration"
            type="number"
            bind:value={localProgressiveDurationHours}
            disabled={isRunning}
            min="0.1"
            max="48"
            step="0.1"
            class="flex-1 max-w-[100px] h-9"
            onfocus={handleFocus}
            onblur={handleBlur}
            oninput={() => updateValue('progressiveDurationHours', localProgressiveDurationHours)}
          />
          <span class="text-xs text-muted-foreground font-semibold min-w-[40px]">hrs</span>
        </div>
      </div>
    {/if}
  </div>
</Card>
