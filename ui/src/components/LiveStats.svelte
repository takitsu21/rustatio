<script>
  import Card from '$lib/components/ui/card.svelte';

  let { stats, formatDuration } = $props();
</script>

<Card class="p-5 border-2 border-primary shadow-lg shadow-primary/20">
  <h2 class="mb-3 text-primary text-lg font-semibold">⚡ Live Stats</h2>
  <div class="grid grid-cols-2 gap-3">
    <div class="bg-muted p-3 rounded-lg text-center border border-border">
      <span
        class="block text-[0.7rem] text-muted-foreground mb-1 uppercase tracking-wider font-semibold"
        >↑ Upload</span
      >
      <span class="block text-lg font-bold text-green-600 dark:text-green-500"
        >{stats.current_upload_rate.toFixed(1)} KB/s</span
      >
    </div>
    <div class="bg-muted p-3 rounded-lg text-center border border-border">
      <span
        class="block text-[0.7rem] text-muted-foreground mb-1 uppercase tracking-wider font-semibold"
        >↓ Download</span
      >
      <span class="block text-lg font-bold text-primary"
        >{stats.current_download_rate.toFixed(1)} KB/s</span
      >
    </div>
    <div class="bg-muted p-3 rounded-lg text-center border border-border">
      <span
        class="block text-[0.7rem] text-muted-foreground mb-1 uppercase tracking-wider font-semibold"
        >Ratio</span
      >
      <span class="block text-lg font-bold text-amber-500">{stats.ratio.toFixed(2)}</span>
    </div>
    <div class="bg-muted p-3 rounded-lg text-center border border-border">
      <span
        class="block text-[0.7rem] text-muted-foreground mb-1 uppercase tracking-wider font-semibold"
        >Time</span
      >
      <span class="block text-lg font-bold text-foreground"
        >{formatDuration(stats.elapsed_time?.secs || 0)}</span
      >
    </div>
  </div>
</Card>
