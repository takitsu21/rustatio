<script>
  import Card from '$lib/components/ui/card.svelte';
  import { Trophy, Upload, Download, Percent, Users, ArrowUp, ArrowDown } from '@lucide/svelte';

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

<Card class="p-4 border-2 border-primary/50 bg-gradient-to-br from-primary/5 to-transparent">
  <div class="flex items-center justify-between mb-4">
    <h2 class="text-primary text-lg font-semibold flex items-center gap-2">
      <Trophy size={20} /> Total Stats
    </h2>
    <div
      class="flex items-center gap-1 text-xs bg-primary/10 text-primary px-2 py-1 rounded-md font-medium"
    >
      <Percent size={12} />
      {cumulativeRatio().toFixed(2)}
    </div>
  </div>

  <!-- Main Ratio Display -->
  <div class="bg-muted/50 rounded-lg border border-border p-4 mb-3 text-center">
    <div class="text-xs text-muted-foreground mb-1">Cumulative Ratio</div>
    <div class="text-4xl font-bold {ratioColor()}">
      {cumulativeRatio().toFixed(2)}
    </div>
    <!-- Visual ratio bar -->
    <div class="mt-3 max-w-48 mx-auto">
      <div class="w-full h-2 bg-background rounded-full overflow-hidden">
        <div
          class="h-full {ratioBgColor()} rounded-full transition-all duration-300"
          style="width: {Math.min(cumulativeRatio() * 50, 100)}%"
        ></div>
      </div>
      <div class="flex justify-between mt-1">
        <span class="text-[10px] text-muted-foreground">0</span>
        <span class="text-[10px] text-muted-foreground">1.0</span>
        <span class="text-[10px] text-muted-foreground">2.0+</span>
      </div>
    </div>
  </div>

  <!-- Transfer Stats -->
  <div class="bg-muted/50 rounded-lg border border-border overflow-hidden mb-3">
    <div class="grid grid-cols-2 divide-x divide-border">
      <!-- Total Uploaded -->
      <div class="p-3">
        <div class="flex items-center gap-1.5 text-xs text-muted-foreground mb-1">
          <Upload size={12} class="text-stat-upload" />
          Total Uploaded
        </div>
        <div class="text-xl font-bold text-stat-upload">
          {formatBytes(totalUploaded())}
        </div>
      </div>
      <!-- Total Downloaded -->
      <div class="p-3">
        <div class="flex items-center gap-1.5 text-xs text-muted-foreground mb-1">
          <Download size={12} class="text-stat-download" />
          Total Downloaded
        </div>
        <div class="text-xl font-bold text-stat-download">
          {formatBytes(totalDownloaded())}
        </div>
      </div>
    </div>
  </div>

  <!-- Peers -->
  <div class="bg-muted/50 rounded-lg border border-border p-3">
    <div class="flex items-center gap-1.5 text-xs text-muted-foreground mb-2">
      <Users size={12} />
      Connected Peers
    </div>
    <div class="grid grid-cols-2 gap-3">
      <div class="flex items-center gap-2">
        <div class="w-8 h-8 rounded-full bg-stat-upload/10 flex items-center justify-center">
          <ArrowUp size={14} class="text-stat-upload" />
        </div>
        <div>
          <div class="text-lg font-bold">{stats.seeders}</div>
          <div class="text-[10px] text-muted-foreground">Seeders</div>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <div class="w-8 h-8 rounded-full bg-stat-danger/10 flex items-center justify-center">
          <ArrowDown size={14} class="text-stat-danger" />
        </div>
        <div>
          <div class="text-lg font-bold">{stats.leechers}</div>
          <div class="text-[10px] text-muted-foreground">Leechers</div>
        </div>
      </div>
    </div>
  </div>
</Card>
