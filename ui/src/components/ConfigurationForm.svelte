<script>
  import Card from '$lib/components/ui/card.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Select from '$lib/components/ui/select.svelte';
  import Checkbox from '$lib/components/ui/checkbox.svelte';
  import {
    Settings,
    Shuffle,
    TrendingUp,
    ArrowUpDown,
    Clock,
    Upload,
    Download,
  } from '@lucide/svelte';
  import ClientIcon from './ClientIcon.svelte';
  import ClientSelect from './ClientSelect.svelte';

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
          <Select
            id="clientVersion"
            bind:value={localSelectedClientVersion}
            disabled={isRunning}
            onchange={() => updateValue('selectedClientVersion', localSelectedClientVersion)}
            class="h-9"
          >
            {#each clientVersions[localSelectedClient] || [] as version (version)}
              <option value={version}>{version}</option>
            {/each}
          </Select>
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
      <Label for="randomize" class="cursor-pointer font-medium flex items-center gap-2">
        <Shuffle size={16} class="text-muted-foreground" />
        Randomize rates for realistic behavior
      </Label>
    </div>

    {#if localRandomizeRates}
      <div class="bg-muted/50 rounded-lg border border-border overflow-hidden">
        <!-- Slider row -->
        <div class="p-4 flex items-center gap-4">
          <span class="text-sm text-muted-foreground whitespace-nowrap">Variance</span>
          <input
            id="randomRange"
            type="range"
            bind:value={localRandomRangePercent}
            disabled={isRunning}
            min="1"
            max="50"
            step="1"
            class="flex-1 h-2 rounded-lg cursor-pointer accent-primary"
            style="background: linear-gradient(to right, hsl(var(--primary)) {((localRandomRangePercent -
              1) /
              49) *
              100}%, hsl(var(--muted)) {((localRandomRangePercent - 1) / 49) * 100}%);"
            onfocus={handleFocus}
            onblur={handleBlur}
            oninput={() => updateValue('randomRangePercent', localRandomRangePercent)}
          />
          <span class="text-lg font-bold text-primary min-w-[4ch] text-right"
            >±{localRandomRangePercent}%</span
          >
        </div>

        <!-- Resulting ranges -->
        <div class="grid grid-cols-2 border-t border-border">
          <div class="p-3 border-r border-border">
            <div class="text-xs text-muted-foreground mb-1">↑ Upload Range</div>
            <div class="font-medium">
              <span class="text-muted-foreground"
                >{(localUploadRate * (1 - localRandomRangePercent / 100)).toFixed(1)}</span
              >
              <span class="text-muted-foreground mx-1">—</span>
              <span class="text-primary"
                >{(localUploadRate * (1 + localRandomRangePercent / 100)).toFixed(1)}</span
              >
              <span class="text-xs text-muted-foreground ml-1">KB/s</span>
            </div>
          </div>
          <div class="p-3">
            <div class="text-xs text-muted-foreground mb-1">↓ Download Range</div>
            <div class="font-medium">
              <span class="text-muted-foreground"
                >{(localDownloadRate * (1 - localRandomRangePercent / 100)).toFixed(1)}</span
              >
              <span class="text-muted-foreground mx-1">—</span>
              <span class="text-primary"
                >{(localDownloadRate * (1 + localRandomRangePercent / 100)).toFixed(1)}</span
              >
              <span class="text-xs text-muted-foreground ml-1">KB/s</span>
            </div>
          </div>
        </div>
      </div>
    {/if}
  </div>

  <!-- Progressive Rates -->
  <div class="mb-0">
    <div class="flex items-center gap-3 mb-3">
      <Checkbox
        id="progressive-enabled"
        checked={localProgressiveRatesEnabled}
        disabled={isRunning}
        onchange={checked => {
          localProgressiveRatesEnabled = checked;
          updateValue('progressiveRatesEnabled', checked);
        }}
      />
      <Label for="progressive-enabled" class="cursor-pointer font-medium flex items-center gap-2">
        <TrendingUp size={16} class="text-muted-foreground" />
        Progressive rate adjustment
      </Label>
    </div>

    {#if localProgressiveRatesEnabled}
      <div class="bg-muted/50 rounded-lg border border-border overflow-hidden">
        <!-- Duration slider -->
        <div class="p-4 flex items-center gap-4">
          <span class="text-sm text-muted-foreground whitespace-nowrap">Duration</span>
          <input
            id="progressiveDuration"
            type="range"
            bind:value={localProgressiveDurationHours}
            disabled={isRunning}
            min="0.5"
            max="24"
            step="0.5"
            class="flex-1 h-2 rounded-lg cursor-pointer accent-primary"
            style="background: linear-gradient(to right, hsl(var(--primary)) {((localProgressiveDurationHours -
              0.5) /
              23.5) *
              100}%, hsl(var(--muted)) {((localProgressiveDurationHours - 0.5) / 23.5) * 100}%);"
            onfocus={handleFocus}
            onblur={handleBlur}
            oninput={() => updateValue('progressiveDurationHours', localProgressiveDurationHours)}
          />
          <div class="flex items-center gap-1 min-w-[5ch]">
            <span class="text-lg font-bold text-primary">{localProgressiveDurationHours}</span>
            <span class="text-sm text-muted-foreground">hrs</span>
          </div>
        </div>

        <!-- Rate progression visualization -->
        <div class="grid grid-cols-2 border-t border-border">
          <!-- Upload progression -->
          <div class="p-3 border-r border-border">
            <div class="text-xs text-muted-foreground mb-2">↑ Upload</div>
            <div class="flex items-center gap-2">
              <div class="text-center">
                <div class="text-xs text-muted-foreground mb-0.5">Start</div>
                <div class="font-medium text-muted-foreground">{localUploadRate}</div>
              </div>
              <div class="flex-1 flex items-center gap-1 px-2">
                <div class="h-px flex-1 bg-border"></div>
                <TrendingUp size={14} class="text-primary" />
                <div class="h-px flex-1 bg-border"></div>
              </div>
              <div class="text-center">
                <div class="text-xs text-muted-foreground mb-0.5">Target</div>
                <Input
                  id="targetUpload"
                  type="number"
                  bind:value={localTargetUploadRate}
                  disabled={isRunning}
                  min="0"
                  step="0.1"
                  class="w-20 h-8 text-center font-medium"
                  onfocus={handleFocus}
                  onblur={handleBlur}
                  oninput={() => updateValue('targetUploadRate', localTargetUploadRate)}
                />
              </div>
            </div>
          </div>

          <!-- Download progression -->
          <div class="p-3">
            <div class="text-xs text-muted-foreground mb-2">↓ Download</div>
            <div class="flex items-center gap-2">
              <div class="text-center">
                <div class="text-xs text-muted-foreground mb-0.5">Start</div>
                <div class="font-medium text-muted-foreground">{localDownloadRate}</div>
              </div>
              <div class="flex-1 flex items-center gap-1 px-2">
                <div class="h-px flex-1 bg-border"></div>
                <TrendingUp size={14} class="text-primary" />
                <div class="h-px flex-1 bg-border"></div>
              </div>
              <div class="text-center">
                <div class="text-xs text-muted-foreground mb-0.5">Target</div>
                <Input
                  id="targetDownload"
                  type="number"
                  bind:value={localTargetDownloadRate}
                  disabled={isRunning}
                  min="0"
                  step="0.1"
                  class="w-20 h-8 text-center font-medium"
                  onfocus={handleFocus}
                  onblur={handleBlur}
                  oninput={() => updateValue('targetDownloadRate', localTargetDownloadRate)}
                />
              </div>
            </div>
          </div>
        </div>

        <!-- Summary -->
        <div
          class="px-4 py-2 bg-muted/50 border-t border-border text-xs text-muted-foreground text-center"
        >
          Rates will gradually adjust from starting values to targets over {localProgressiveDurationHours}
          hour{localProgressiveDurationHours !== 1 ? 's' : ''}
        </div>
      </div>
    {/if}
  </div>
</Card>
