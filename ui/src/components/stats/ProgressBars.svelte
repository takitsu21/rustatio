<script>
  import Card from '$lib/components/ui/card.svelte';
  import { BarChart3 } from '@lucide/svelte';

  let {
    stats,
    completionPercent = 100,
    torrentSize = 0,
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

  const isLeeching = $derived(completionPercent < 100);
  const torrentCompletion = $derived(stats?.torrent_completion ?? completionPercent);
  const torrentDownloaded = $derived(torrentSize > 0 ? torrentSize - (stats?.left ?? 0) : 0);
</script>

<Card>
  <div class="flex h-8 items-center gap-2 border-b border-border px-2.5">
    <BarChart3 size={13} class="text-muted-foreground" />
    <span
      class="flex-1 text-[0.625rem] font-semibold uppercase tracking-[0.16em] text-muted-foreground"
    >
      Progress
    </span>
  </div>

  <div class="flex flex-col gap-2.5 p-2.5">
    {#if isLeeching}
      <div>
        <div class="mb-1 flex flex-wrap items-center gap-2">
          <span class="flex-1 text-[0.6875rem] font-medium text-stat-leecher">Torrent download</span
          >
          <span class="tabular-nums text-[0.625rem] text-muted-foreground"
            >{formatBytes(torrentDownloaded)} / {formatBytes(torrentSize)}</span
          >
          {#if stats.eta_download_completion}
            <span class="tabular-nums text-[0.625rem] italic text-muted-foreground"
              >ETA {formatDuration(stats.eta_download_completion.secs)}</span
            >
          {/if}
        </div>
        <div class="flex items-center gap-2">
          <div class="h-1.5 min-w-0 flex-1 bg-muted">
            <div
              class="h-full bg-stat-leecher transition-[width] duration-300"
              style="width: {torrentCompletion}%"
            ></div>
          </div>
          <span class="w-9 shrink-0 text-right tabular-nums text-[0.625rem] text-foreground"
            >{torrentCompletion.toFixed(0)}%</span
          >
        </div>
        {#if torrentCompletion >= 100}
          <p class="mt-1 text-[0.625rem] text-stat-upload">Download complete — now seeding</p>
        {/if}
      </div>
    {/if}

    {#if stopAtRatioEnabled && stats.ratio_progress >= 0}
      <div>
        <div class="mb-1 flex flex-wrap items-center gap-2">
          <span class="flex-1 text-[0.6875rem] font-medium text-foreground">Ratio</span>
          <span class="tabular-nums text-[0.625rem] text-muted-foreground"
            >{(stats.ratio ?? 0).toFixed(2)} / {stopAtRatio}</span
          >
          {#if stats.eta_ratio}
            <span class="tabular-nums text-[0.625rem] italic text-muted-foreground"
              >ETA {formatDuration(stats.eta_ratio.secs)}</span
            >
          {/if}
        </div>
        <div class="flex items-center gap-2">
          <div class="h-1.5 min-w-0 flex-1 bg-muted">
            <div
              class="h-full bg-stat-ratio transition-[width] duration-300"
              style="width: {stats.ratio_progress}%"
            ></div>
          </div>
          <span class="w-9 shrink-0 text-right tabular-nums text-[0.625rem] text-foreground"
            >{(stats.ratio_progress ?? 0).toFixed(0)}%</span
          >
        </div>
      </div>
    {/if}

    {#if stopAtUploadedEnabled && stats.upload_progress >= 0}
      <div>
        <div class="mb-1 flex flex-wrap items-center gap-2">
          <span class="flex-1 text-[0.6875rem] font-medium text-foreground">Uploaded ↑</span>
          <span class="tabular-nums text-[0.625rem] text-muted-foreground"
            >{formatBytes(stats.session_uploaded)} / {stopAtUploadedGB} GB</span
          >
          {#if stats.eta_uploaded}
            <span class="tabular-nums text-[0.625rem] italic text-muted-foreground"
              >ETA {formatDuration(stats.eta_uploaded.secs)}</span
            >
          {/if}
        </div>
        <div class="flex items-center gap-2">
          <div class="h-1.5 min-w-0 flex-1 bg-muted">
            <div
              class="h-full bg-stat-upload transition-[width] duration-300"
              style="width: {stats.upload_progress}%"
            ></div>
          </div>
          <span class="w-9 shrink-0 text-right tabular-nums text-[0.625rem] text-foreground"
            >{(stats.upload_progress ?? 0).toFixed(0)}%</span
          >
        </div>
      </div>
    {/if}

    {#if stopAtDownloadedEnabled && stats.download_progress >= 0}
      <div>
        <div class="mb-1 flex flex-wrap items-center gap-2">
          <span class="flex-1 text-[0.6875rem] font-medium text-foreground">Downloaded ↓</span>
          <span class="tabular-nums text-[0.625rem] text-muted-foreground"
            >{formatBytes(stats.session_downloaded)} / {stopAtDownloadedGB} GB</span
          >
        </div>
        <div class="flex items-center gap-2">
          <div class="h-1.5 min-w-0 flex-1 bg-muted">
            <div
              class="h-full bg-stat-download transition-[width] duration-300"
              style="width: {stats.download_progress}%"
            ></div>
          </div>
          <span class="w-9 shrink-0 text-right tabular-nums text-[0.625rem] text-foreground"
            >{(stats.download_progress ?? 0).toFixed(0)}%</span
          >
        </div>
      </div>
    {/if}

    {#if stopAtSeedTimeEnabled && stats.seed_time_progress >= 0}
      <div>
        <div class="mb-1 flex flex-wrap items-center gap-2">
          <span class="flex-1 text-[0.6875rem] font-medium text-foreground">Seed time</span>
          <span class="tabular-nums text-[0.625rem] text-muted-foreground"
            >{formatDuration(stats.elapsed_time?.secs || 0)} / {stopAtSeedTimeHours}h</span
          >
          {#if stats.eta_seed_time}
            <span class="tabular-nums text-[0.625rem] italic text-muted-foreground"
              >ETA {formatDuration(stats.eta_seed_time.secs)}</span
            >
          {/if}
        </div>
        <div class="flex items-center gap-2">
          <div class="h-1.5 min-w-0 flex-1 bg-muted">
            <div
              class="h-full bg-primary transition-[width] duration-300"
              style="width: {stats.seed_time_progress}%"
            ></div>
          </div>
          <span class="w-9 shrink-0 text-right tabular-nums text-[0.625rem] text-foreground"
            >{(stats.seed_time_progress ?? 0).toFixed(0)}%</span
          >
        </div>
      </div>
    {/if}
  </div>
</Card>
