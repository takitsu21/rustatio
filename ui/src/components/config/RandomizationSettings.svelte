<script>
  import Checkbox from '$lib/components/ui/checkbox.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import { Shuffle } from '@lucide/svelte';

  let {
    enabled = $bindable(false),
    rangePercent = $bindable(20),
    uploadRate = 0,
    downloadRate = 0,
    disabled = false,
    onchange,
  } = $props();
</script>

<div>
  <div class="mb-2 flex items-center gap-2">
    <Checkbox
      id="randomize"
      checked={enabled}
      {disabled}
      onchange={checked => {
        enabled = checked;
        onchange?.({ randomizeRates: checked });
      }}
    />
    <Label
      for="randomize"
      class="flex cursor-pointer items-center gap-1.5 text-[0.6875rem] font-medium"
    >
      <Shuffle size={13} class="text-muted-foreground" />
      Randomize rates for realistic behavior
    </Label>
  </div>

  {#if enabled}
    <div class="overflow-hidden border border-border bg-background">
      <div class="flex items-center gap-3 p-2.5">
        <span
          class="whitespace-nowrap text-[0.625rem] uppercase tracking-wider text-muted-foreground"
          >Variance</span
        >
        <input
          id="randomRange"
          type="range"
          bind:value={rangePercent}
          {disabled}
          min="1"
          max="50"
          step="1"
          class="h-1.5 flex-1 cursor-pointer appearance-none accent-primary"
          style="background: linear-gradient(to right, var(--color-primary) {((rangePercent - 1) /
            49) *
            100}%, var(--color-border) {((rangePercent - 1) / 49) * 100}%);"
          oninput={() => onchange?.({ randomRangePercent: rangePercent })}
        />
        <span
          class="min-w-[4ch] text-right text-[0.6875rem] font-semibold tabular-nums text-foreground"
          >±{rangePercent}%</span
        >
      </div>

      <div class="grid grid-cols-2 border-t border-border">
        <div class="border-r border-border p-2">
          <div class="mb-0.5 text-[0.5625rem] uppercase tracking-wider text-muted-foreground">
            ↑ Upload range
          </div>
          <div class="text-[0.6875rem] tabular-nums">
            <span class="text-stat-upload"
              >{(uploadRate * (1 - rangePercent / 100)).toFixed(0)}</span
            >
            <span class="mx-1 text-muted-foreground">–</span>
            <span class="text-stat-upload"
              >{(uploadRate * (1 + rangePercent / 100)).toFixed(0)}</span
            >
            <span class="ml-1 text-[0.5625rem] text-muted-foreground">KB/s</span>
          </div>
        </div>
        <div class="p-2">
          <div class="mb-0.5 text-[0.5625rem] uppercase tracking-wider text-muted-foreground">
            ↓ Download range
          </div>
          <div class="text-[0.6875rem] tabular-nums">
            <span class="text-stat-download"
              >{(downloadRate * (1 - rangePercent / 100)).toFixed(0)}</span
            >
            <span class="mx-1 text-muted-foreground">–</span>
            <span class="text-stat-download"
              >{(downloadRate * (1 + rangePercent / 100)).toFixed(0)}</span
            >
            <span class="ml-1 text-[0.5625rem] text-muted-foreground">KB/s</span>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
