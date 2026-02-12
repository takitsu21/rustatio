<script>
  import Checkbox from '$lib/components/ui/checkbox.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import { TrendingUp } from '@lucide/svelte';

  let {
    enabled = $bindable(false),
    durationHours = $bindable(1),
    targetUploadRate = $bindable(500),
    targetDownloadRate = $bindable(0),
    uploadRate = 0,
    downloadRate = 0,
    disabled = false,
    onchange,
  } = $props();
</script>

<div>
  <div class="flex items-center gap-3 mb-3">
    <Checkbox
      id="progressive-enabled"
      checked={enabled}
      {disabled}
      onchange={checked => {
        enabled = checked;
        onchange?.({ progressiveRatesEnabled: checked });
      }}
    />
    <Label for="progressive-enabled" class="cursor-pointer font-medium flex items-center gap-2">
      <TrendingUp size={16} class="text-muted-foreground" />
      Progressive rate adjustment
    </Label>
  </div>

  {#if enabled}
    <div class="bg-muted/50 rounded-lg border border-border overflow-hidden">
      <div class="p-4 flex items-center gap-4">
        <span class="text-sm text-muted-foreground whitespace-nowrap">Duration</span>
        <input
          id="progressiveDuration"
          type="range"
          bind:value={durationHours}
          {disabled}
          min="0.5"
          max="24"
          step="0.5"
          class="flex-1 h-2 rounded-lg cursor-pointer accent-primary"
          style="background: linear-gradient(to right, hsl(var(--primary)) {((durationHours - 0.5) / 23.5) * 100}%, hsl(var(--muted)) {((durationHours - 0.5) / 23.5) * 100}%);"
          oninput={() => onchange?.({ progressiveDurationHours: durationHours })}
        />
        <div class="flex items-center gap-1 min-w-[5ch]">
          <span class="text-lg font-bold text-primary">{durationHours}</span>
          <span class="text-sm text-muted-foreground">hrs</span>
        </div>
      </div>

      <div class="grid grid-cols-2 border-t border-border">
        <div class="p-3 border-r border-border">
          <div class="text-xs text-muted-foreground mb-2">↑ Upload</div>
          <div class="flex items-center gap-2">
            <div class="text-center">
              <div class="text-xs text-muted-foreground mb-0.5">Start</div>
              <div class="font-medium text-muted-foreground">{uploadRate}</div>
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
                bind:value={targetUploadRate}
                {disabled}
                min="0"
                step="0.1"
                class="w-20 h-8 text-center font-medium"
                oninput={() => onchange?.({ targetUploadRate })}
              />
            </div>
          </div>
        </div>

        <div class="p-3">
          <div class="text-xs text-muted-foreground mb-2">↓ Download</div>
          <div class="flex items-center gap-2">
            <div class="text-center">
              <div class="text-xs text-muted-foreground mb-0.5">Start</div>
              <div class="font-medium text-muted-foreground">{downloadRate}</div>
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
                bind:value={targetDownloadRate}
                {disabled}
                min="0"
                step="0.1"
                class="w-20 h-8 text-center font-medium"
                oninput={() => onchange?.({ targetDownloadRate })}
              />
            </div>
          </div>
        </div>
      </div>

      <div class="px-4 py-2 bg-muted/50 border-t border-border text-xs text-muted-foreground text-center">
        Rates will gradually adjust from starting values to targets over {durationHours}
        hour{durationHours !== 1 ? 's' : ''}
      </div>
    </div>
  {/if}
</div>
