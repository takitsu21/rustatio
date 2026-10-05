<script>
  import Checkbox from '$lib/components/ui/checkbox.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import {
    Percent,
    Upload,
    Download,
    Clock,
    Users,
    Pause,
    Settings,
    Shuffle,
  } from '@lucide/svelte';

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

<div class="overflow-hidden border border-border bg-background">
  <!-- Ratio -->
  <div
    class="flex items-center gap-2 border-b border-border px-2 py-1.5 {stopAtRatioEnabled
      ? 'bg-primary/10'
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
    <Percent size={13} class={stopAtRatioEnabled ? 'text-primary' : 'text-muted-foreground'} />
    <Label for="stop-ratio" class="flex-1 cursor-pointer text-[0.6875rem] font-medium"
      >Target ratio</Label
    >
    {#if stopAtRatioEnabled}
      <div class="flex items-center gap-1">
        <Input
          type="number"
          bind:value={stopAtRatio}
          {disabled}
          min="0.1"
          max="100"
          step="0.1"
          class="h-7 w-16 text-center tabular-nums"
          placeholder="2.0"
          oninput={() => onchange?.({ stopAtRatio })}
        />
      </div>
    {:else}
      <span class="text-[0.625rem] text-muted-foreground">disabled</span>
    {/if}
  </div>

  <!-- Randomize Ratio -->
  {#if stopAtRatioEnabled}
    <div class="border-b border-border bg-muted/30">
      <div class="flex items-center gap-2 py-1.5 pl-8 pr-2">
        <Checkbox
          id="randomize-ratio"
          checked={randomizeRatio}
          {disabled}
          onchange={checked => {
            randomizeRatio = checked;
            onchange?.({ randomizeRatio: checked });
          }}
        />
        <Shuffle size={12} class={randomizeRatio ? 'text-primary' : 'text-muted-foreground'} />
        <Label for="randomize-ratio" class="flex-1 cursor-pointer text-[0.6875rem] font-medium">
          Randomize ratio for realistic behavior
        </Label>
      </div>
      {#if randomizeRatio}
        <div class="flex items-center gap-3 px-2 pb-1.5 pl-8">
          <span
            class="whitespace-nowrap text-[0.625rem] uppercase tracking-wider text-muted-foreground"
            >Variance</span
          >
          <input
            type="range"
            bind:value={randomRatioRangePercent}
            {disabled}
            min="1"
            max="30"
            step="1"
            class="h-1.5 flex-1 cursor-pointer appearance-none accent-primary"
            style="background: linear-gradient(to right, var(--color-primary) {((randomRatioRangePercent -
              1) /
              29) *
              100}%, var(--color-border) {((randomRatioRangePercent - 1) / 29) * 100}%);"
            oninput={() => onchange?.({ randomRatioRangePercent })}
          />
          <span
            class="min-w-[4ch] text-right text-[0.6875rem] font-semibold tabular-nums text-foreground"
            >±{randomRatioRangePercent}%</span
          >
        </div>
        <div class="px-2 pb-1.5 pl-8">
          <div class="text-[0.625rem] text-muted-foreground">
            Range
            <span class="font-medium tabular-nums text-stat-ratio"
              >{(stopAtRatio * (1 - randomRatioRangePercent / 100)).toFixed(2)}</span
            >
            –
            <span class="font-medium tabular-nums text-stat-ratio"
              >{(stopAtRatio * (1 + randomRatioRangePercent / 100)).toFixed(2)}</span
            >
            {#if effectiveStopAtRatio != null}
              · Effective: <span class="font-semibold tabular-nums text-stat-ratio"
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
    class="flex items-center gap-2 border-b border-border px-2 py-1.5 {stopAtUploadedEnabled
      ? 'bg-primary/10'
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
      size={13}
      class={stopAtUploadedEnabled ? 'text-stat-upload' : 'text-muted-foreground'}
    />
    <Label for="stop-uploaded" class="flex-1 cursor-pointer text-[0.6875rem] font-medium"
      >Max Upload</Label
    >
    {#if stopAtUploadedEnabled}
      <div class="flex items-center gap-1">
        <Input
          type="number"
          bind:value={stopAtUploadedGB}
          {disabled}
          min="0.1"
          step="0.1"
          class="h-7 w-16 text-center tabular-nums"
          placeholder="10"
          oninput={() => onchange?.({ stopAtUploadedGB })}
        />
        <span class="w-6 text-[0.625rem] text-muted-foreground">GB</span>
      </div>
    {:else}
      <span class="text-[0.625rem] text-muted-foreground">disabled</span>
    {/if}
  </div>

  <!-- Downloaded -->
  <div
    class="flex items-center gap-2 border-b border-border px-2 py-1.5 {stopAtDownloadedEnabled
      ? 'bg-primary/10'
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
      size={13}
      class={stopAtDownloadedEnabled ? 'text-stat-download' : 'text-muted-foreground'}
    />
    <Label for="stop-downloaded" class="flex-1 cursor-pointer text-[0.6875rem] font-medium"
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
          class="h-7 w-16 text-center tabular-nums"
          placeholder="10"
          oninput={() => onchange?.({ stopAtDownloadedGB })}
        />
        <span class="w-6 text-[0.625rem] text-muted-foreground">GB</span>
      </div>
    {:else}
      <span class="text-[0.625rem] text-muted-foreground">disabled</span>
    {/if}
  </div>

  <!-- Seed Time -->
  <div
    class="flex items-center gap-2 border-b border-border px-2 py-1.5 {stopAtSeedTimeEnabled
      ? 'bg-primary/10'
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
    <Clock size={13} class={stopAtSeedTimeEnabled ? 'text-stat-ratio' : 'text-muted-foreground'} />
    <Label for="stop-seedtime" class="flex-1 cursor-pointer text-[0.6875rem] font-medium"
      >Seed Time</Label
    >
    {#if stopAtSeedTimeEnabled}
      <div class="flex items-center gap-1">
        <Input
          type="number"
          bind:value={stopAtSeedTimeHours}
          {disabled}
          min="0.1"
          step="0.1"
          class="h-7 w-16 text-center tabular-nums"
          placeholder="24"
          oninput={() => onchange?.({ stopAtSeedTimeHours })}
        />
        <span class="w-6 text-[0.625rem] text-muted-foreground">hrs</span>
      </div>
    {:else}
      <span class="text-[0.625rem] text-muted-foreground">disabled</span>
    {/if}
  </div>

  <!-- Idle when No Leechers -->
  <div
    class="flex items-center gap-2 border-b border-border px-2 py-1.5 {idleWhenNoLeechers
      ? 'bg-primary/10'
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
    <Pause size={13} class={idleWhenNoLeechers ? 'text-violet-400' : 'text-muted-foreground'} />
    <Label for="idle-no-leechers" class="flex-1 cursor-pointer text-[0.6875rem] font-medium">
      Idle when no leechers
    </Label>
    {#if idleWhenNoLeechers}
      <span class="text-xs text-violet-400 font-medium">0 KB/s</span>
    {:else}
      <span class="text-[0.625rem] text-muted-foreground">disabled</span>
    {/if}
  </div>

  <!-- Idle when No Seeders -->
  <div class="flex items-center gap-2 px-2 py-1.5 {idleWhenNoSeeders ? 'bg-primary/10' : ''}">
    <Checkbox
      id="idle-no-seeders"
      checked={idleWhenNoSeeders}
      {disabled}
      onchange={checked => {
        idleWhenNoSeeders = checked;
        onchange?.({ idleWhenNoSeeders: checked });
      }}
    />
    <Users size={13} class={idleWhenNoSeeders ? 'text-stat-ratio' : 'text-muted-foreground'} />
    <Label for="idle-no-seeders" class="flex-1 cursor-pointer text-[0.6875rem] font-medium">
      Idle when no seeders
    </Label>
    {#if idleWhenNoSeeders}
      {#if !isLeecherMode}
        <span class="text-xs text-stat-ratio font-medium" title="Only works when completion < 100%"
          >0 KB/s</span
        >
      {:else}
        <span class="text-xs text-stat-ratio font-medium">0 KB/s</span>
      {/if}
    {:else}
      <span class="text-[0.625rem] text-muted-foreground">disabled</span>
    {/if}
  </div>

  <!-- Post-Stop Action -->
  {#if hasThresholdCondition}
    <div class="flex items-center gap-2 border-t border-border bg-muted/30 px-2 py-1.5">
      <Settings size={13} class="text-muted-foreground" />
      <Label for="post-stop-action" class="flex-1 text-[0.6875rem] font-medium"
        >When conditions are met</Label
      >
      <select
        id="post-stop-action"
        bind:value={postStopAction}
        {disabled}
        class="h-7 border border-input bg-background px-2 text-[0.6875rem] text-foreground focus:border-ring focus:outline-none"
        onchange={() => onchange?.({ postStopAction })}
      >
        <option value="idle">Continue (idle)</option>
        <option value="stop_seeding">Stop</option>
        <option value="delete_instance">Delete instance</option>
      </select>
    </div>
  {/if}
</div>
