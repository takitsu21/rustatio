<script>
  import Card from '$lib/components/ui/card.svelte';
  import Checkbox from '$lib/components/ui/checkbox.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import { Target, Percent, Upload, Download, Clock, Users } from '@lucide/svelte';

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

  // Count active conditions
  let activeCount = $derived(
    [
      localStopAtRatioEnabled,
      localStopAtUploadedEnabled,
      localStopAtDownloadedEnabled,
      localStopAtSeedTimeEnabled,
      localStopWhenNoLeechers,
    ].filter(Boolean).length
  );
</script>

<Card class="p-3">
  <div class="flex items-center justify-between mb-3">
    <h2 class="text-primary text-lg font-semibold flex items-center gap-2">
      <Target size={20} /> Stop Conditions
    </h2>
    {#if activeCount > 0}
      <span class="text-xs bg-primary/10 text-primary px-2 py-1 rounded-full font-medium">
        {activeCount} active
      </span>
    {/if}
  </div>

  <div class="bg-muted/50 rounded-lg border border-border overflow-hidden">
    <!-- Ratio -->
    <div
      class="flex items-center gap-3 p-3 border-b border-border {localStopAtRatioEnabled
        ? 'bg-primary/5'
        : ''}"
    >
      <Checkbox
        id="stop-ratio"
        checked={localStopAtRatioEnabled}
        disabled={isRunning}
        onchange={checked => {
          localStopAtRatioEnabled = checked;
          updateValue('stopAtRatioEnabled', checked);
        }}
      />
      <Percent
        size={16}
        class={localStopAtRatioEnabled ? 'text-primary' : 'text-muted-foreground'}
      />
      <Label for="stop-ratio" class="flex-1 cursor-pointer text-sm font-medium">Target Ratio</Label>
      {#if localStopAtRatioEnabled}
        <div class="flex items-center gap-1">
          <Input
            type="number"
            bind:value={localStopAtRatio}
            disabled={isRunning}
            min="0.1"
            max="100"
            step="0.1"
            class="w-20 h-8 text-center font-medium"
            placeholder="2.0"
            oninput={() => updateValue('stopAtRatio', localStopAtRatio)}
          />
        </div>
      {:else}
        <span class="text-xs text-muted-foreground">disabled</span>
      {/if}
    </div>

    <!-- Uploaded -->
    <div
      class="flex items-center gap-3 p-3 border-b border-border {localStopAtUploadedEnabled
        ? 'bg-primary/5'
        : ''}"
    >
      <Checkbox
        id="stop-uploaded"
        checked={localStopAtUploadedEnabled}
        disabled={isRunning}
        onchange={checked => {
          localStopAtUploadedEnabled = checked;
          updateValue('stopAtUploadedEnabled', checked);
        }}
      />
      <Upload
        size={16}
        class={localStopAtUploadedEnabled ? 'text-stat-upload' : 'text-muted-foreground'}
      />
      <Label for="stop-uploaded" class="flex-1 cursor-pointer text-sm font-medium">
        Max Upload
      </Label>
      {#if localStopAtUploadedEnabled}
        <div class="flex items-center gap-1">
          <Input
            type="number"
            bind:value={localStopAtUploadedGB}
            disabled={isRunning}
            min="0.1"
            step="0.1"
            class="w-20 h-8 text-center font-medium"
            placeholder="10"
            oninput={() => updateValue('stopAtUploadedGB', localStopAtUploadedGB)}
          />
          <span class="text-xs text-muted-foreground w-6">GB</span>
        </div>
      {:else}
        <span class="text-xs text-muted-foreground">disabled</span>
      {/if}
    </div>

    <!-- Downloaded -->
    <div
      class="flex items-center gap-3 p-3 border-b border-border {localStopAtDownloadedEnabled
        ? 'bg-primary/5'
        : ''}"
    >
      <Checkbox
        id="stop-downloaded"
        checked={localStopAtDownloadedEnabled}
        disabled={isRunning}
        onchange={checked => {
          localStopAtDownloadedEnabled = checked;
          updateValue('stopAtDownloadedEnabled', checked);
        }}
      />
      <Download
        size={16}
        class={localStopAtDownloadedEnabled ? 'text-stat-download' : 'text-muted-foreground'}
      />
      <Label for="stop-downloaded" class="flex-1 cursor-pointer text-sm font-medium">
        Max Download
      </Label>
      {#if localStopAtDownloadedEnabled}
        <div class="flex items-center gap-1">
          <Input
            type="number"
            bind:value={localStopAtDownloadedGB}
            disabled={isRunning}
            min="0.1"
            step="0.1"
            class="w-20 h-8 text-center font-medium"
            placeholder="10"
            oninput={() => updateValue('stopAtDownloadedGB', localStopAtDownloadedGB)}
          />
          <span class="text-xs text-muted-foreground w-6">GB</span>
        </div>
      {:else}
        <span class="text-xs text-muted-foreground">disabled</span>
      {/if}
    </div>

    <!-- Seed Time -->
    <div
      class="flex items-center gap-3 p-3 border-b border-border {localStopAtSeedTimeEnabled
        ? 'bg-primary/5'
        : ''}"
    >
      <Checkbox
        id="stop-seedtime"
        checked={localStopAtSeedTimeEnabled}
        disabled={isRunning}
        onchange={checked => {
          localStopAtSeedTimeEnabled = checked;
          updateValue('stopAtSeedTimeEnabled', checked);
        }}
      />
      <Clock
        size={16}
        class={localStopAtSeedTimeEnabled ? 'text-stat-ratio' : 'text-muted-foreground'}
      />
      <Label for="stop-seedtime" class="flex-1 cursor-pointer text-sm font-medium">Seed Time</Label>
      {#if localStopAtSeedTimeEnabled}
        <div class="flex items-center gap-1">
          <Input
            type="number"
            bind:value={localStopAtSeedTimeHours}
            disabled={isRunning}
            min="0.1"
            step="0.1"
            class="w-20 h-8 text-center font-medium"
            placeholder="24"
            oninput={() => updateValue('stopAtSeedTimeHours', localStopAtSeedTimeHours)}
          />
          <span class="text-xs text-muted-foreground w-6">hrs</span>
        </div>
      {:else}
        <span class="text-xs text-muted-foreground">disabled</span>
      {/if}
    </div>

    <!-- No Leechers -->
    <div class="flex items-center gap-3 p-3 {localStopWhenNoLeechers ? 'bg-primary/5' : ''}">
      <Checkbox
        id="stop-no-leechers"
        checked={localStopWhenNoLeechers}
        disabled={isRunning}
        onchange={checked => {
          localStopWhenNoLeechers = checked;
          updateValue('stopWhenNoLeechers', checked);
        }}
      />
      <Users
        size={16}
        class={localStopWhenNoLeechers ? 'text-purple-500' : 'text-muted-foreground'}
      />
      <Label for="stop-no-leechers" class="flex-1 cursor-pointer text-sm font-medium">
        No Leechers
      </Label>
      {#if localStopWhenNoLeechers}
        <span class="text-xs text-primary font-medium">auto-stop</span>
      {:else}
        <span class="text-xs text-muted-foreground">disabled</span>
      {/if}
    </div>
  </div>
</Card>
