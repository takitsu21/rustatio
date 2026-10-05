<script>
  import Card from '$lib/components/ui/card.svelte';
  import { Trophy, Upload, Download, Users, ArrowUp, ArrowDown } from '@lucide/svelte';

  let { stats, torrent, formatBytes } = $props();

  // Total stats from backend (cumulative across all sessions)
  let totalUploaded = $derived(() => {
    return stats?.uploaded || 0;
  });

  let totalDownloaded = $derived(() => {
    return stats?.downloaded || 0;
  });

  // Use ratio from backend, or calculate as uploaded/torrent_size if downloaded is 0
  let cumulativeRatio = $derived(() => {
    // If backend provides ratio, use it
    if (stats?.ratio !== undefined && stats.ratio > 0) {
      return stats.ratio;
    }
    // Fallback: calculate as uploaded / torrent_size (common for seeders)
    const torrentSize = torrent?.total_size || 0;
    if (torrentSize > 0) {
      return totalUploaded() / torrentSize;
    }
    return 0;
  });

  // Determine ratio status color (theme-aware with better contrast)
  let ratioColor = $derived(() => {
    const ratio = cumulativeRatio();
    if (ratio >= 2) return 'text-stat-upload';
    if (ratio >= 1) return 'text-stat-ratio';
    return 'text-stat-danger';
  });

  let ratioBgColor = $derived(() => {
    const ratio = cumulativeRatio();
    if (ratio >= 2) return 'bg-stat-upload';
    if (ratio >= 1) return 'bg-stat-ratio';
    return 'bg-stat-danger';
  });
</script>

<Card>
  <div class="flex h-8 items-center gap-2 border-b border-border px-2.5">
    <Trophy size={13} class="text-muted-foreground" />
    <span
      class="flex-1 text-[0.625rem] font-semibold uppercase tracking-[0.16em] text-muted-foreground"
    >
      Total
    </span>
    <span class="tabular-nums text-[0.625rem] font-semibold {ratioColor()}">
      {cumulativeRatio().toFixed(2)}x
    </span>
  </div>

  <!-- Main Ratio Display -->
  <div class="border-b border-border p-2.5 text-center">
    <div class="text-[0.5625rem] uppercase tracking-wider text-muted-foreground">
      Cumulative ratio
    </div>
    <div class="mt-0.5 text-2xl font-bold tabular-nums {ratioColor()}">
      {cumulativeRatio().toFixed(2)}
    </div>
    <!-- Visual ratio bar -->
    <div class="mx-auto mt-2 max-w-48">
      <div class="h-1.5 w-full bg-muted">
        <div
          class="h-full {ratioBgColor()} transition-[width] duration-300"
          style="width: {Math.min(cumulativeRatio() * 50, 100)}%"
        ></div>
      </div>
      <div class="mt-0.5 flex justify-between">
        <span class="text-[0.5625rem] tabular-nums text-muted-foreground">0</span>
        <span class="text-[0.5625rem] tabular-nums text-muted-foreground">1.0</span>
        <span class="text-[0.5625rem] tabular-nums text-muted-foreground">2.0+</span>
      </div>
    </div>
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
        {formatBytes(totalUploaded())}
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
        {formatBytes(totalDownloaded())}
      </div>
    </div>
  </div>

  <!-- Peers -->
  <div class="p-2">
    <div
      class="mb-1.5 flex items-center gap-1.5 text-[0.5625rem] uppercase tracking-wider text-muted-foreground"
    >
      <Users size={10} />
      Connected peers
    </div>
    <div class="grid grid-cols-2 gap-2">
      <div class="flex items-center gap-2">
        <span
          class="flex h-6 w-6 items-center justify-center border border-stat-upload/40 bg-stat-upload/10"
        >
          <ArrowUp size={11} class="text-stat-upload" />
        </span>
        <div>
          <div class="text-sm font-semibold tabular-nums text-foreground">{stats.seeders ?? 0}</div>
          <div class="text-[0.5625rem] text-muted-foreground">Seeders</div>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <span
          class="flex h-6 w-6 items-center justify-center border border-stat-leecher/40 bg-stat-leecher/10"
        >
          <ArrowDown size={11} class="text-stat-leecher" />
        </span>
        <div>
          <div class="text-sm font-semibold tabular-nums text-foreground">
            {stats.leechers ?? 0}
          </div>
          <div class="text-[0.5625rem] text-muted-foreground">Leechers</div>
        </div>
      </div>
    </div>
  </div>
</Card>
