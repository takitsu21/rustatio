<script>
  import Checkbox from '$lib/components/ui/checkbox.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import { Percent, Upload, Download, Clock, Users, Pause, Settings, Shuffle } from '@lucide/svelte';

  let {
    stopAtRatioEnabled = $bindable(false),
    stopAtRatio = $bindable(2.0),
    randomizeRatio = $bindable(false),
    randomRatioRangePercent = $bindable(10),
    effectiveStopAtRatio = null,
    stopAtUploadedEnabled = $bindable(false),
    stopAtUploadedGB = $bindable(10),
    stopAtDownloadedEnabled = $bindable(false),
    stopAtDownloadedGB = $bindable(10),
    stopAtSeedTimeEnabled = $bindable(false),
    stopAtSeedTimeHours = $bindable(24),
    idleWhenNoLeechers = $bindable(false),
    idleWhenNoSeeders = $bindable(false),
    postStopAction = $bindable('idle'),
    completionPercent = 100,
    disabled = false,
    onchange,
  } = $props();

  let isLeecherMode = $derived(completionPercent < 100);
  let hasThresholdCondition = $derived(
    stopAtRatioEnabled || stopAtUploadedEnabled || stopAtDownloadedEnabled || stopAtSeedTimeEnabled
  );
</script>

<div class="bg-muted/50 rounded-lg border border-border overflow-hidden">
  <!-- Ratio -->
  <div
    class="flex items-center gap-3 p-3 border-b border-border {stopAtRatioEnabled
      ? 'bg-primary/5'
      : ''}"
  >
    <Checkbox
      id="stop-ratio"
      checked={stopAtRatioEnabled}
      {disabled}
      onchange={checked => {
        stopAtRatioEnabled = checked;
        onchange?.({ stopAtRatioEnabled: checked });
      }}
    />
    <Percent size={16} class={stopAtRatioEnabled ? 'text-primary' : 'text-muted-foreground'} />
    <Label for="stop-ratio" class="flex-1 cursor-pointer text-sm font-medium">Target Ratio</Label>
    {#if stopAtRatioEnabled}
      <div class="flex items-center gap-1">
        <Input
          type="number"
          bind:value={stopAtRatio}
          {disabled}
          min="0.1"
          max="100"
          step="0.1"
          class="w-20 h-8 text-center font-medium"
          placeholder="2.0"
          oninput={() => onchange?.({ stopAtRatio })}
        />
      </div>
    {:else}
      <span class="text-xs text-muted-foreground">disabled</span>
    {/if}
  </div>

  <!-- Randomize Ratio -->
  {#if stopAtRatioEnabled}
    <div class="border-b border-border bg-muted/30">
      <div class="flex items-center gap-3 px-3 py-2 pl-10">
        <Checkbox
          id="randomize-ratio"
          checked={randomizeRatio}
          {disabled}
          onchange={checked => {
            randomizeRatio = checked;
            onchange?.({ randomizeRatio: checked });
          }}
        />
        <Shuffle size={14} class={randomizeRatio ? 'text-primary' : 'text-muted-foreground'} />
        <Label for="randomize-ratio" class="flex-1 cursor-pointer text-xs font-medium">
          Randomize ratio for realistic behavior
        </Label>
      </div>
      {#if randomizeRatio}
        <div class="px-10 pb-2 flex items-center gap-3">
          <span class="text-xs text-muted-foreground whitespace-nowrap">Variance</span>
          <input
            type="range"
            bind:value={randomRatioRangePercent}
            {disabled}
            min="1"
            max="30"
            step="1"
            class="flex-1 h-1.5 rounded-lg cursor-pointer accent-primary"
            style="background: linear-gradient(to right, hsl(var(--primary)) {((randomRatioRangePercent -
              1) /
              29) *
              100}%, hsl(var(--muted)) {((randomRatioRangePercent - 1) / 29) * 100}%);"
            oninput={() => onchange?.({ randomRatioRangePercent })}
          />
          <span class="text-sm font-bold text-primary min-w-[4ch] text-right"
            >±{randomRatioRangePercent}%</span
          >
        </div>
        <div class="px-10 pb-2">
          <div class="text-xs text-muted-foreground">
            Range
            <span class="font-medium text-primary"
              >{(stopAtRatio * (1 - randomRatioRangePercent / 100)).toFixed(2)}</span
            >
            —
            <span class="font-medium text-primary"
              >{(stopAtRatio * (1 + randomRatioRangePercent / 100)).toFixed(2)}</span
            >
            {#if effectiveStopAtRatio != null}
              · Effective: <span class="font-semibold text-primary"
                >{effectiveStopAtRatio.toFixed(4)}</span
              >
            {/if}
          </div>
        </div>
      {/if}
    </div>
  {/if}

  <!-- Uploaded -->
  <div
    class="flex items-center gap-3 p-3 border-b border-border {stopAtUploadedEnabled
      ? 'bg-primary/5'
      : ''}"
  >
    <Checkbox
      id="stop-uploaded"
      checked={stopAtUploadedEnabled}
      {disabled}
      onchange={checked => {
        stopAtUploadedEnabled = checked;
        onchange?.({ stopAtUploadedEnabled: checked });
      }}
    />
    <Upload
      size={16}
      class={stopAtUploadedEnabled ? 'text-stat-upload' : 'text-muted-foreground'}
    />
    <Label for="stop-uploaded" class="flex-1 cursor-pointer text-sm font-medium">Max Upload</Label>
    {#if stopAtUploadedEnabled}
      <div class="flex items-center gap-1">
        <Input
          type="number"
          bind:value={stopAtUploadedGB}
          {disabled}
          min="0.1"
          step="0.1"
          class="w-20 h-8 text-center font-medium"
          placeholder="10"
          oninput={() => onchange?.({ stopAtUploadedGB })}
        />
        <span class="text-xs text-muted-foreground w-6">GB</span>
      </div>
    {:else}
      <span class="text-xs text-muted-foreground">disabled</span>
    {/if}
  </div>

  <!-- Downloaded -->
  <div
    class="flex items-center gap-3 p-3 border-b border-border {stopAtDownloadedEnabled
      ? 'bg-primary/5'
      : ''}"
  >
    <Checkbox
      id="stop-downloaded"
      checked={stopAtDownloadedEnabled}
      {disabled}
      onchange={checked => {
        stopAtDownloadedEnabled = checked;
        onchange?.({ stopAtDownloadedEnabled: checked });
      }}
    />
    <Download
      size={16}
      class={stopAtDownloadedEnabled ? 'text-stat-download' : 'text-muted-foreground'}
    />
    <Label for="stop-downloaded" class="flex-1 cursor-pointer text-sm font-medium"
      >Max Download</Label
    >
    {#if stopAtDownloadedEnabled}
      <div class="flex items-center gap-1">
        <Input
          type="number"
          bind:value={stopAtDownloadedGB}
          {disabled}
          min="0.1"
          step="0.1"
          class="w-20 h-8 text-center font-medium"
          placeholder="10"
          oninput={() => onchange?.({ stopAtDownloadedGB })}
        />
        <span class="text-xs text-muted-foreground w-6">GB</span>
      </div>
    {:else}
      <span class="text-xs text-muted-foreground">disabled</span>
    {/if}
  </div>

  <!-- Seed Time -->
  <div
    class="flex items-center gap-3 p-3 border-b border-border {stopAtSeedTimeEnabled
      ? 'bg-primary/5'
      : ''}"
  >
    <Checkbox
      id="stop-seedtime"
      checked={stopAtSeedTimeEnabled}
      {disabled}
      onchange={checked => {
        stopAtSeedTimeEnabled = checked;
        onchange?.({ stopAtSeedTimeEnabled: checked });
      }}
    />
    <Clock size={16} class={stopAtSeedTimeEnabled ? 'text-stat-ratio' : 'text-muted-foreground'} />
    <Label for="stop-seedtime" class="flex-1 cursor-pointer text-sm font-medium">Seed Time</Label>
    {#if stopAtSeedTimeEnabled}
      <div class="flex items-center gap-1">
        <Input
          type="number"
          bind:value={stopAtSeedTimeHours}
          {disabled}
          min="0.1"
          step="0.1"
          class="w-20 h-8 text-center font-medium"
          placeholder="24"
          oninput={() => onchange?.({ stopAtSeedTimeHours })}
        />
        <span class="text-xs text-muted-foreground w-6">hrs</span>
      </div>
    {:else}
      <span class="text-xs text-muted-foreground">disabled</span>
    {/if}
  </div>

  <!-- Idle when No Leechers -->
  <div
    class="flex items-center gap-3 p-3 border-b border-border {idleWhenNoLeechers
      ? 'bg-primary/5'
      : ''}"
  >
    <Checkbox
      id="idle-no-leechers"
      checked={idleWhenNoLeechers}
      {disabled}
      onchange={checked => {
        idleWhenNoLeechers = checked;
        onchange?.({ idleWhenNoLeechers: checked });
      }}
    />
    <Pause size={16} class={idleWhenNoLeechers ? 'text-purple-500' : 'text-muted-foreground'} />
    <Label for="idle-no-leechers" class="flex-1 cursor-pointer text-sm font-medium">
      Idle when no leechers
    </Label>
    {#if idleWhenNoLeechers}
      <span class="text-xs text-purple-500 font-medium">0 KB/s</span>
    {:else}
      <span class="text-xs text-muted-foreground">disabled</span>
    {/if}
  </div>

  <!-- Idle when No Seeders -->
  <div class="flex items-center gap-3 p-3 {idleWhenNoSeeders ? 'bg-primary/5' : ''}">
    <Checkbox
      id="idle-no-seeders"
      checked={idleWhenNoSeeders}
      {disabled}
      onchange={checked => {
        idleWhenNoSeeders = checked;
        onchange?.({ idleWhenNoSeeders: checked });
      }}
    />
    <Users size={16} class={idleWhenNoSeeders ? 'text-orange-500' : 'text-muted-foreground'} />
    <Label for="idle-no-seeders" class="flex-1 cursor-pointer text-sm font-medium">
      Idle when no seeders
    </Label>
    {#if idleWhenNoSeeders}
      {#if !isLeecherMode}
        <span class="text-xs text-orange-500 font-medium" title="Only works when completion < 100%"
          >0 KB/s</span
        >
      {:else}
        <span class="text-xs text-orange-500 font-medium">0 KB/s</span>
      {/if}
    {:else}
      <span class="text-xs text-muted-foreground">disabled</span>
    {/if}
  </div>

  <!-- Post-Stop Action -->
  {#if hasThresholdCondition}
    <div class="flex items-center gap-3 p-3 border-t border-border bg-muted/30">
      <Settings size={16} class="text-muted-foreground" />
      <Label for="post-stop-action" class="flex-1 text-sm font-medium"
        >When conditions are met</Label
      >
      <select
        id="post-stop-action"
        bind:value={postStopAction}
        {disabled}
        class="h-8 rounded-md border border-input bg-background px-2 text-xs font-medium focus:outline-none focus:ring-1 focus:ring-ring"
        onchange={() => onchange?.({ postStopAction })}
      >
        <option value="idle">Continue seeding (idle)</option>
        <option value="stop_seeding">Stop seeding</option>
        <option value="delete_instance">Delete instance</option>
      </select>
    </div>
  {/if}
</div>
