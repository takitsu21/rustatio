<script>
  import Card from '$lib/components/ui/card.svelte';

  let { stats, torrent, cumulativeUploaded, cumulativeDownloaded, formatBytes } = $props();

  // Total stats should show: cumulative (from previous sessions) + current session progress
  // Only use session_uploaded/session_downloaded (which start at 0) to show actual progress
  let totalUploaded = $derived(() => {
    const cumulativeBytes = (cumulativeUploaded || 0) * 1024 * 1024;
    const sessionBytes = stats.session_uploaded || 0;
    return cumulativeBytes + sessionBytes;
  });

  let totalDownloaded = $derived(() => {
    const cumulativeBytes = (cumulativeDownloaded || 0) * 1024 * 1024;
    const sessionBytes = stats.session_downloaded || 0;
    return cumulativeBytes + sessionBytes;
  });

  // Ratio = total_uploaded / torrent_total_size (not downloaded!)
  let cumulativeRatio = $derived(() => {
    const torrentSize = torrent?.total_size || 1;
    return totalUploaded() / torrentSize;
  });
</script>

<Card class="p-5 border-2 border-primary shadow-lg shadow-primary/20">
  <h2 class="mb-3 text-primary text-lg font-semibold">📈 Total Stats</h2>
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
