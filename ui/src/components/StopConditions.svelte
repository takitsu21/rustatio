<script>
  import Card from '$lib/components/ui/card.svelte';
  import Checkbox from '$lib/components/ui/checkbox.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Label from '$lib/components/ui/label.svelte';

  let {
    stopAtRatioEnabled,
    stopAtRatio,
    stopAtUploadedEnabled,
    stopAtUploadedGB,
    stopAtDownloadedEnabled,
    stopAtDownloadedGB,
    stopAtSeedTimeEnabled,
    stopAtSeedTimeHours,
    stopWhenNoLeechers,
    isRunning,
    onUpdate,
  } = $props();

  // Local state
  let localStopAtRatioEnabled = $state();
  let localStopAtRatio = $state();
  let localStopAtUploadedEnabled = $state();
  let localStopAtUploadedGB = $state();
  let localStopAtDownloadedEnabled = $state();
  let localStopAtDownloadedGB = $state();
  let localStopAtSeedTimeEnabled = $state();
  let localStopAtSeedTimeHours = $state();
  let localStopWhenNoLeechers = $state();

  // Track if we're currently editing to prevent external updates from interfering
  let isEditing = $state(false);
  let editTimeout;

  // Update local state when props change (only when not actively editing)
  $effect(() => {
    if (!isEditing) {
      localStopAtRatioEnabled = stopAtRatioEnabled;
      localStopAtRatio = stopAtRatio;
      localStopAtUploadedEnabled = stopAtUploadedEnabled;
      localStopAtUploadedGB = stopAtUploadedGB;
      localStopAtDownloadedEnabled = stopAtDownloadedEnabled;
      localStopAtDownloadedGB = stopAtDownloadedGB;
      localStopAtSeedTimeEnabled = stopAtSeedTimeEnabled;
      localStopAtSeedTimeHours = stopAtSeedTimeHours;
      localStopWhenNoLeechers = stopWhenNoLeechers;
    }
  });

  function updateValue(key, value) {
    isEditing = true;
    clearTimeout(editTimeout);

    if (onUpdate) {
      onUpdate({ [key]: value });
    }

    // Clear editing flag after a short delay
    editTimeout = setTimeout(() => {
      isEditing = false;
    }, 100);
  }
</script>

<Card class="p-3">
  <h2 class="mb-3 text-primary text-lg font-semibold">🎯 Stop Conditions</h2>
  <div class="flex flex-col gap-3">
    <div class="flex items-center gap-2 p-2 bg-muted rounded-md">
      <Checkbox
        id="stop-ratio"
        checked={localStopAtRatioEnabled}
        disabled={isRunning}
        onchange={checked => {
          localStopAtRatioEnabled = checked;
          updateValue('stopAtRatioEnabled', checked);
        }}
      />
      <Label
        for="stop-ratio"
        class="flex-1 cursor-pointer text-sm text-muted-foreground font-medium">Ratio</Label
      >
      {#if localStopAtRatioEnabled}
        <Input
          type="number"
          bind:value={localStopAtRatio}
          disabled={isRunning}
          min="0.1"
          max="100"
          step="0.1"
          class="w-24 h-9"
          placeholder="2.0"
          oninput={() => updateValue('stopAtRatio', localStopAtRatio)}
        />
      {/if}
    </div>

    <div class="flex items-center gap-2 p-2 bg-muted rounded-md">
      <Checkbox
        id="stop-uploaded"
        checked={localStopAtUploadedEnabled}
        disabled={isRunning}
        onchange={checked => {
          localStopAtUploadedEnabled = checked;
          updateValue('stopAtUploadedEnabled', checked);
        }}
      />
      <Label
        for="stop-uploaded"
        class="flex-1 cursor-pointer text-sm text-muted-foreground font-medium">Uploaded ↑</Label
      >
      {#if localStopAtUploadedEnabled}
        <Input
          type="number"
          bind:value={localStopAtUploadedGB}
          disabled={isRunning}
          min="0.1"
          step="0.1"
          class="w-24 h-9"
          placeholder="10"
          oninput={() => updateValue('stopAtUploadedGB', localStopAtUploadedGB)}
        />
        <span class="text-xs text-muted-foreground font-semibold min-w-[40px]">GB</span>
      {/if}
    </div>

    <div class="flex items-center gap-2 p-2 bg-muted rounded-md">
      <Checkbox
        id="stop-downloaded"
        checked={localStopAtDownloadedEnabled}
        disabled={isRunning}
        onchange={checked => {
          localStopAtDownloadedEnabled = checked;
          updateValue('stopAtDownloadedEnabled', checked);
        }}
      />
      <Label
        for="stop-downloaded"
        class="flex-1 cursor-pointer text-sm text-muted-foreground font-medium">Downloaded ↓</Label
      >
      {#if localStopAtDownloadedEnabled}
        <Input
          type="number"
          bind:value={localStopAtDownloadedGB}
          disabled={isRunning}
          min="0.1"
          step="0.1"
          class="w-24 h-9"
          placeholder="10"
          oninput={() => updateValue('stopAtDownloadedGB', localStopAtDownloadedGB)}
        />
        <span class="text-xs text-muted-foreground font-semibold min-w-[40px]">GB</span>
      {/if}
    </div>

    <div class="flex items-center gap-2 p-2 bg-muted rounded-md">
      <Checkbox
        id="stop-seedtime"
        checked={localStopAtSeedTimeEnabled}
        disabled={isRunning}
        onchange={checked => {
          localStopAtSeedTimeEnabled = checked;
          updateValue('stopAtSeedTimeEnabled', checked);
        }}
      />
      <Label
        for="stop-seedtime"
        class="flex-1 cursor-pointer text-sm text-muted-foreground font-medium">Seed Time</Label
      >
      {#if localStopAtSeedTimeEnabled}
        <Input
          type="number"
          bind:value={localStopAtSeedTimeHours}
          disabled={isRunning}
          min="0.1"
          step="0.1"
          class="w-24 h-9"
          placeholder="24"
          oninput={() => updateValue('stopAtSeedTimeHours', localStopAtSeedTimeHours)}
        />
        <span class="text-xs text-muted-foreground font-semibold min-w-[40px]">hrs</span>
      {/if}
    </div>

    <div class="flex items-center gap-2 p-2 bg-muted rounded-md">
      <Checkbox
        id="stop-no-leechers"
        checked={localStopWhenNoLeechers}
        disabled={isRunning}
        onchange={checked => {
          localStopWhenNoLeechers = checked;
          updateValue('stopWhenNoLeechers', checked);
        }}
      />
      <Label
        for="stop-no-leechers"
        class="flex-1 cursor-pointer text-sm text-muted-foreground font-medium">No Leechers</Label
      >
    </div>
  </div>
</Card>
