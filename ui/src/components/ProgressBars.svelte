<script>
  import Card from '$lib/components/ui/card.svelte';
  import { BarChart3 } from '@lucide/svelte';

  let {
    stats,
    stopAtRatioEnabled,
    stopAtRatio,
    stopAtUploadedEnabled,
    stopAtUploadedGB,
    stopAtDownloadedEnabled,
    stopAtDownloadedGB,
    stopAtSeedTimeEnabled,
    stopAtSeedTimeHours,
    formatBytes,
    formatDuration,
  } = $props();
</script>

<Card class="p-3">
  <h2 class="mb-3 text-primary text-lg font-semibold flex items-center gap-2">
    <BarChart3 size={20} /> Progress
  </h2>
  <div class="flex flex-col gap-3">
    {#if stopAtRatioEnabled && stats.ratio_progress >= 0}
      <div>
        <div class="flex justify-between items-center mb-2 flex-wrap gap-2">
          <span class="font-semibold text-foreground text-sm">Ratio</span>
          <span class="text-xs text-muted-foreground"
            >{stats.session_ratio.toFixed(2)} / {stopAtRatio}</span
          >
          {#if stats.eta_ratio}
            <span class="text-xs text-muted-foreground italic"
              >ETA {formatDuration(stats.eta_ratio.secs)}</span
            >
          {/if}
        </div>
        <div class="w-full h-5 bg-muted rounded-full overflow-hidden border border-border">
          <div
            class="h-full bg-gradient-to-r from-primary to-primary/90 transition-all duration-300 flex items-center justify-end pr-2"
            style="width: {stats.ratio_progress}%"
          >
            <span class="text-[0.7rem] text-white font-semibold pr-1"
              >{stats.ratio_progress.toFixed(0)}%</span
            >
          </div>
        </div>
      </div>
    {/if}

    {#if stopAtUploadedEnabled && stats.upload_progress >= 0}
      <div>
        <div class="flex justify-between items-center mb-2 flex-wrap gap-2">
          <span class="font-semibold text-foreground text-sm">Uploaded ↑</span>
          <span class="text-xs text-muted-foreground"
            >{formatBytes(stats.session_uploaded)} / {stopAtUploadedGB} GB</span
          >
          {#if stats.eta_uploaded}
            <span class="text-xs text-muted-foreground italic"
              >ETA {formatDuration(stats.eta_uploaded.secs)}</span
            >
          {/if}
        </div>
        <div class="w-full h-5 bg-muted rounded-full overflow-hidden border border-border">
          <div
            class="h-full bg-stat-upload transition-all duration-300 flex items-center justify-end pr-2"
            style="width: {stats.upload_progress}%"
          >
            <span class="text-[0.7rem] text-white font-semibold pr-1"
              >{stats.upload_progress.toFixed(0)}%</span
            >
          </div>
        </div>
      </div>
    {/if}

    {#if stopAtDownloadedEnabled && stats.download_progress >= 0}
      <div>
        <div class="flex justify-between items-center mb-2 flex-wrap gap-2">
          <span class="font-semibold text-foreground text-sm">Downloaded ↓</span>
          <span class="text-xs text-muted-foreground"
            >{formatBytes(stats.session_downloaded)} / {stopAtDownloadedGB} GB</span
          >
        </div>
        <div class="w-full h-5 bg-muted rounded-full overflow-hidden border border-border">
          <div
            class="h-full bg-gradient-to-r from-primary to-primary/90 transition-all duration-300 flex items-center justify-end pr-2"
            style="width: {stats.download_progress}%"
          >
            <span class="text-[0.7rem] text-white font-semibold pr-1"
              >{stats.download_progress.toFixed(0)}%</span
            >
          </div>
        </div>
      </div>
    {/if}

    {#if stopAtSeedTimeEnabled && stats.seed_time_progress >= 0}
      <div>
        <div class="flex justify-between items-center mb-2 flex-wrap gap-2">
          <span class="font-semibold text-foreground text-sm">⏱️ Seed Time</span>
          <span class="text-xs text-muted-foreground"
            >{formatDuration(stats.elapsed_time?.secs || 0)} / {stopAtSeedTimeHours}h</span
          >
          {#if stats.eta_seed_time}
            <span class="text-xs text-muted-foreground italic"
              >ETA {formatDuration(stats.eta_seed_time.secs)}</span
            >
          {/if}
        </div>
        <div class="w-full h-5 bg-muted rounded-full overflow-hidden border border-border">
          <div
            class="h-full bg-stat-ratio transition-all duration-300 flex items-center justify-end pr-2"
            style="width: {stats.seed_time_progress}%"
          >
            <span class="text-[0.7rem] text-white font-semibold pr-1"
              >{stats.seed_time_progress.toFixed(0)}%</span
            >
          </div>
        </div>
      </div>
    {/if}
  </div>
</Card>
