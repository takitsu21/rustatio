<script>
  import Card from '$lib/components/ui/card.svelte';
  import { Timer, Upload, Download, TrendingUp, TrendingDown, Percent } from '@lucide/svelte';

  let { stats, formatBytes, formatDuration } = $props();

  // Calculate session ratio
  let sessionRatio = $derived(() => {
    if (stats.session_downloaded > 0) {
      return stats.session_uploaded / stats.session_downloaded;
    }
    return stats.session_uploaded > 0 ? Infinity : 0;
  });

  // Format ratio for display
  let ratioDisplay = $derived(() => {
    const ratio = sessionRatio();
    if (ratio === Infinity) return '∞';
    return ratio.toFixed(2);
  });
</script>

<Card class="p-4">
  <div class="flex items-center justify-between mb-4">
    <h2 class="text-primary text-lg font-semibold flex items-center gap-2">
      <Timer size={20} /> Session Stats
    </h2>
    {#if stats.elapsed_time}
      <div class="text-xs text-muted-foreground bg-muted px-2 py-1 rounded-md">
        {formatDuration(stats.elapsed_time?.secs || 0)}
      </div>
    {/if}
  </div>

  <!-- Transfer Stats -->
  <div class="bg-muted/50 rounded-lg border border-border overflow-hidden mb-3">
    <div class="grid grid-cols-2 divide-x divide-border">
      <!-- Uploaded -->
      <div class="p-3">
        <div class="flex items-center gap-1.5 text-xs text-muted-foreground mb-1">
          <Upload size={12} class="text-stat-upload" />
          Uploaded
        </div>
        <div class="text-xl font-bold text-stat-upload">
          {formatBytes(stats.session_uploaded)}
        </div>
      </div>
      <!-- Downloaded -->
      <div class="p-3">
        <div class="flex items-center gap-1.5 text-xs text-muted-foreground mb-1">
          <Download size={12} class="text-stat-download" />
          Downloaded
        </div>
        <div class="text-xl font-bold text-stat-download">
          {formatBytes(stats.session_downloaded)}
        </div>
      </div>
    </div>
  </div>

  <!-- Average Rates -->
  <div class="bg-muted/50 rounded-lg border border-border overflow-hidden mb-3">
    <div class="grid grid-cols-2 divide-x divide-border">
      <!-- Avg Upload -->
      <div class="p-3">
        <div class="flex items-center gap-1.5 text-xs text-muted-foreground mb-1">
          <TrendingUp size={12} class="text-stat-upload" />
          Avg Upload
        </div>
        <div class="text-lg font-semibold">
          {stats.average_upload_rate.toFixed(1)}
          <span class="text-xs font-normal text-muted-foreground">KB/s</span>
        </div>
      </div>
      <!-- Avg Download -->
      <div class="p-3">
        <div class="flex items-center gap-1.5 text-xs text-muted-foreground mb-1">
          <TrendingDown size={12} class="text-stat-download" />
          Avg Download
        </div>
        <div class="text-lg font-semibold">
          {stats.average_download_rate.toFixed(1)}
          <span class="text-xs font-normal text-muted-foreground">KB/s</span>
        </div>
      </div>
    </div>
  </div>

  <!-- Session Ratio -->
  <div class="bg-muted/50 rounded-lg border border-border p-3">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-1.5 text-xs text-muted-foreground">
        <Percent size={12} class="text-stat-ratio" />
        Session Ratio
      </div>
      <div class="text-xl font-bold text-stat-ratio">
        {ratioDisplay()}
      </div>
    </div>
    <!-- Visual ratio bar -->
    <div class="mt-2">
      <div class="w-full h-1.5 bg-background rounded-full overflow-hidden">
        <div
          class="h-full bg-stat-ratio rounded-full transition-all duration-300"
          style="width: {Math.min(sessionRatio() * 50, 100)}%"
        ></div>
      </div>
      <div class="flex justify-between mt-1">
        <span class="text-[10px] text-muted-foreground">0</span>
        <span class="text-[10px] text-muted-foreground">1.0</span>
        <span class="text-[10px] text-muted-foreground">2.0+</span>
      </div>
    </div>
  </div>
</Card>
