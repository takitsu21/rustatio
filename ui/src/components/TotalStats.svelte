<script>
  import Card from '$lib/components/ui/card.svelte';
  import { TrendingUp } from '@lucide/svelte';

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
</script>

<Card class="p-5 border-2 border-primary shadow-lg shadow-primary/20">
  <h2 class="mb-3 text-primary text-lg font-semibold flex items-center gap-2">
    <TrendingUp size={20} /> Total Stats
  </h2>
  <div class="flex flex-col gap-2">
    <div
      class="flex justify-between items-center p-2.5 bg-muted rounded-md border border-border transition-all hover:translate-x-0.5 hover:border-primary"
    >
      <span class="text-xs text-muted-foreground uppercase tracking-wide font-semibold"
        >Total Uploaded ↑</span
      >
      <span class="text-base text-foreground font-bold">{formatBytes(totalUploaded())}</span>
    </div>
    <div
      class="flex justify-between items-center p-2.5 bg-muted rounded-md border border-border transition-all hover:translate-x-0.5 hover:border-primary"
    >
      <span class="text-xs text-muted-foreground uppercase tracking-wide font-semibold"
        >Total Downloaded ↓</span
      >
      <span class="text-base text-foreground font-bold">{formatBytes(totalDownloaded())}</span>
    </div>
    <div
      class="flex justify-between items-center p-2.5 bg-muted rounded-md border border-border transition-all hover:translate-x-0.5 hover:border-primary"
    >
      <span class="text-xs text-muted-foreground uppercase tracking-wide font-semibold">Ratio</span>
      <span class="text-lg text-green-600 dark:text-green-500 font-bold"
        >{cumulativeRatio().toFixed(2)}</span
      >
    </div>
    <div
      class="flex justify-between items-center p-2.5 bg-muted rounded-md border border-border transition-all hover:translate-x-0.5 hover:border-primary"
    >
      <span class="text-xs text-muted-foreground uppercase tracking-wide font-semibold">Peers</span>
      <span class="text-base text-foreground font-bold">{stats.seeders}↑ / {stats.leechers}↓</span>
    </div>
  </div>
</Card>
