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
  <div class="flex items-center gap-3 mb-3">
    <Checkbox
      id="randomize"
      checked={enabled}
      {disabled}
      onchange={checked => {
        enabled = checked;
        onchange?.({ randomizeRates: checked });
      }}
    />
    <Label for="randomize" class="cursor-pointer font-medium flex items-center gap-2">
      <Shuffle size={16} class="text-muted-foreground" />
      Randomize rates for realistic behavior
    </Label>
  </div>

  {#if enabled}
    <div class="bg-muted/50 rounded-lg border border-border overflow-hidden">
      <div class="p-4 flex items-center gap-4">
        <span class="text-sm text-muted-foreground whitespace-nowrap">Variance</span>
        <input
          id="randomRange"
          type="range"
          bind:value={rangePercent}
          {disabled}
          min="1"
          max="50"
          step="1"
          class="flex-1 h-2 rounded-lg cursor-pointer accent-primary"
          style="background: linear-gradient(to right, hsl(var(--primary)) {((rangePercent - 1) / 49) * 100}%, hsl(var(--muted)) {((rangePercent - 1) / 49) * 100}%);"
          oninput={() => onchange?.({ randomRangePercent: rangePercent })}
        />
        <span class="text-lg font-bold text-primary min-w-[4ch] text-right">±{rangePercent}%</span>
      </div>

      <div class="grid grid-cols-2 border-t border-border">
        <div class="p-3 border-r border-border">
          <div class="text-xs text-muted-foreground mb-1">↑ Upload Range</div>
          <div class="font-medium">
            <span class="text-muted-foreground">{(uploadRate * (1 - rangePercent / 100)).toFixed(0)}</span>
            <span class="text-muted-foreground mx-1">—</span>
            <span class="text-primary">{(uploadRate * (1 + rangePercent / 100)).toFixed(0)}</span>
            <span class="text-xs text-muted-foreground ml-1">KB/s</span>
          </div>
        </div>
        <div class="p-3">
          <div class="text-xs text-muted-foreground mb-1">↓ Download Range</div>
          <div class="font-medium">
            <span class="text-muted-foreground">{(downloadRate * (1 - rangePercent / 100)).toFixed(0)}</span>
            <span class="text-muted-foreground mx-1">—</span>
            <span class="text-primary">{(downloadRate * (1 + rangePercent / 100)).toFixed(0)}</span>
            <span class="text-xs text-muted-foreground ml-1">KB/s</span>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
