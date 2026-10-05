<script>
  import Card from '$lib/components/ui/card.svelte';
  import { Timer, Upload, Download, TrendingUp, TrendingDown, Percent } from '@lucide/svelte';

  let { stats, formatBytes, formatDuration } = $props();

  // Calculate session ratio
  let sessionRatio = $derived(() => {
    if ((stats.session_downloaded ?? 0) > 0) {
      return (stats.session_uploaded ?? 0) / stats.session_downloaded;
    }
    return (stats.session_uploaded ?? 0) > 0 ? Infinity : 0;
  });

  // Format ratio for display
  let ratioDisplay = $derived(() => {
    const ratio = sessionRatio();
    if (ratio === Infinity) return '∞';
    return ratio.toFixed(2);
  });
</script>

<Card>
  <div class="flex h-8 items-center gap-2 border-b border-border px-2.5">
    <Timer size={13} class="text-muted-foreground" />
    <span
      class="flex-1 text-[0.625rem] font-semibold uppercase tracking-[0.16em] text-muted-foreground"
    >
      Session
    </span>
    {#if stats.elapsed_time}
      <span class="tabular-nums text-[0.625rem] text-muted-foreground">
        {formatDuration(stats.elapsed_time?.secs || 0)}
      </span>
    {/if}
  </div>

  <!-- Transfer Stats -->
  <div class="grid grid-cols-2 border-b border-border">
    <div class="border-r border-border p-2">
      <div
        class="mb-0.5 flex items-center gap-1.5 text-[0.5625rem] uppercase tracking-wider text-muted-foreground"
      >
        <Upload size={10} class="text-stat-upload" />
        Uploaded
      </div>
      <div class="text-sm font-semibold tabular-nums text-stat-upload">
        {formatBytes(stats.session_uploaded)}
      </div>
    </div>
    <div class="p-2">
      <div
        class="mb-0.5 flex items-center gap-1.5 text-[0.5625rem] uppercase tracking-wider text-muted-foreground"
      >
        <Download size={10} class="text-stat-download" />
        Downloaded
      </div>
      <div class="text-sm font-semibold tabular-nums text-stat-download">
        {formatBytes(stats.session_downloaded)}
      </div>
    </div>
  </div>

  <!-- Average Rates -->
  <div class="grid grid-cols-2 border-b border-border">
    <div class="border-r border-border p-2">
      <div
        class="mb-0.5 flex items-center gap-1.5 text-[0.5625rem] uppercase tracking-wider text-muted-foreground"
      >
        <TrendingUp size={10} class="text-stat-upload" />
        Avg upload
      </div>
      <div class="text-[0.6875rem] tabular-nums text-foreground">
        {(stats.average_upload_rate ?? 0).toFixed(1)}
        <span class="text-[0.5625rem] text-muted-foreground">KB/s</span>
      </div>
    </div>
    <div class="p-2">
      <div
        class="mb-0.5 flex items-center gap-1.5 text-[0.5625rem] uppercase tracking-wider text-muted-foreground"
      >
        <TrendingDown size={10} class="text-stat-download" />
        Avg download
      </div>
      <div class="text-[0.6875rem] tabular-nums text-foreground">
        {(stats.average_download_rate ?? 0).toFixed(1)}
        <span class="text-[0.5625rem] text-muted-foreground">KB/s</span>
      </div>
    </div>
  </div>

  <!-- Session Ratio -->
  <div class="p-2">
    <div class="flex items-center justify-between">
      <span
        class="flex items-center gap-1.5 text-[0.5625rem] uppercase tracking-wider text-muted-foreground"
      >
        <Percent size={10} class="text-stat-ratio" />
        Session ratio
      </span>
      <span class="text-sm font-semibold tabular-nums text-stat-ratio">
        {ratioDisplay()}
      </span>
    </div>
    <div class="mt-1.5">
      <div class="h-1 w-full bg-muted">
        <div
          class="h-full bg-stat-ratio transition-[width] duration-300"
          style="width: {Math.min(sessionRatio() * 50, 100)}%"
        ></div>
      </div>
      <div class="mt-0.5 flex justify-between">
        <span class="text-[0.5625rem] tabular-nums text-muted-foreground">0</span>
        <span class="text-[0.5625rem] tabular-nums text-muted-foreground">1.0</span>
        <span class="text-[0.5625rem] tabular-nums text-muted-foreground">2.0+</span>
      </div>
    </div>
  </div>
</Card>
