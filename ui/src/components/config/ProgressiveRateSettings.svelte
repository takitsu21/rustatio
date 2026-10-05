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
  <div class="mb-2 flex items-center gap-2">
    <Checkbox
      id="progressive-enabled"
      checked={enabled}
      {disabled}
      onchange={checked => {
        enabled = checked;
        onchange?.({ progressiveRatesEnabled: checked });
      }}
    />
    <Label
      for="progressive-enabled"
      class="flex cursor-pointer items-center gap-1.5 text-[0.6875rem] font-medium"
    >
      <TrendingUp size={13} class="text-muted-foreground" />
      Progressive rate adjustment
    </Label>
  </div>

  {#if enabled}
    <div class="overflow-hidden border border-border bg-background">
      <div class="flex items-center gap-3 p-2.5">
        <span
          class="whitespace-nowrap text-[0.625rem] uppercase tracking-wider text-muted-foreground"
          >Duration</span
        >
        <input
          id="progressiveDuration"
          type="range"
          bind:value={durationHours}
          {disabled}
          min="0.5"
          max="24"
          step="0.5"
          class="h-1.5 flex-1 cursor-pointer appearance-none accent-primary"
          style="background: linear-gradient(to right, var(--color-primary) {((durationHours -
            0.5) /
            23.5) *
            100}%, var(--color-border) {((durationHours - 0.5) / 23.5) * 100}%);"
          oninput={() => onchange?.({ progressiveDurationHours: durationHours })}
        />
        <div class="flex min-w-[5ch] items-center gap-1">
          <span class="text-[0.6875rem] font-semibold tabular-nums text-foreground"
            >{durationHours}</span
          >
          <span class="text-[0.625rem] text-muted-foreground">hrs</span>
        </div>
      </div>

      <div class="grid grid-cols-2 border-t border-border">
        <div class="border-r border-border p-2">
          <div class="mb-1.5 text-[0.5625rem] uppercase tracking-wider text-stat-upload">
            ↑ Upload
          </div>
          <div class="flex items-center gap-2">
            <div class="text-center">
              <div class="mb-0.5 text-[0.5625rem] uppercase tracking-wider text-muted-foreground">
                Start
              </div>
              <div class="text-[0.6875rem] tabular-nums text-muted-foreground">{uploadRate}</div>
            </div>
            <div class="flex flex-1 items-center gap-1 px-1">
              <div class="h-px flex-1 bg-border"></div>
              <TrendingUp size={12} class="text-muted-foreground" />
              <div class="h-px flex-1 bg-border"></div>
            </div>
            <div class="text-center">
              <div class="mb-0.5 text-[0.5625rem] uppercase tracking-wider text-muted-foreground">
                Target
              </div>
              <Input
                id="targetUpload"
                type="number"
                bind:value={targetUploadRate}
                {disabled}
                min="0"
                step="0.1"
                class="h-7 w-16 text-center tabular-nums"
                oninput={() => onchange?.({ targetUploadRate })}
              />
            </div>
          </div>
        </div>

        <div class="p-2">
          <div class="mb-1.5 text-[0.5625rem] uppercase tracking-wider text-stat-download">
            ↓ Download
          </div>
          <div class="flex items-center gap-2">
            <div class="text-center">
              <div class="mb-0.5 text-[0.5625rem] uppercase tracking-wider text-muted-foreground">
                Start
              </div>
              <div class="text-[0.6875rem] tabular-nums text-muted-foreground">{downloadRate}</div>
            </div>
            <div class="flex flex-1 items-center gap-1 px-1">
              <div class="h-px flex-1 bg-border"></div>
              <TrendingUp size={12} class="text-muted-foreground" />
              <div class="h-px flex-1 bg-border"></div>
            </div>
            <div class="text-center">
              <div class="mb-0.5 text-[0.5625rem] uppercase tracking-wider text-muted-foreground">
                Target
              </div>
              <Input
                id="targetDownload"
                type="number"
                bind:value={targetDownloadRate}
                {disabled}
                min="0"
                step="0.1"
                class="h-7 w-16 text-center tabular-nums"
                oninput={() => onchange?.({ targetDownloadRate })}
              />
            </div>
          </div>
        </div>
      </div>

      <div
        class="border-t border-border bg-muted/30 px-3 py-1.5 text-center text-[0.625rem] text-muted-foreground"
      >
        Rates gradually adjust from starting values to targets over {durationHours}
        hour{durationHours !== 1 ? 's' : ''}
      </div>
    </div>
  {/if}
</div>
