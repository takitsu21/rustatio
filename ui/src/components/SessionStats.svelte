<script>
  import Card from '$lib/components/ui/card.svelte';

  let { stats, formatBytes } = $props();
</script>

<Card class="p-3">
  <h2 class="mb-3 text-primary text-lg font-semibold">⚡ Session Stats</h2>
  <div class="flex flex-col gap-2">
    <div
      class="flex justify-between items-center p-2.5 bg-muted rounded-md border border-border transition-all hover:translate-x-0.5 hover:border-primary"
    >
      <span class="text-xs text-muted-foreground uppercase tracking-wide font-semibold"
        >Session Uploaded ↑</span
      >
      <span class="text-base text-foreground font-bold">{formatBytes(stats.session_uploaded)}</span>
    </div>
    <div
      class="flex justify-between items-center p-2.5 bg-muted rounded-md border border-border transition-all hover:translate-x-0.5 hover:border-primary"
    >
      <span class="text-xs text-muted-foreground uppercase tracking-wide font-semibold"
        >Session Downloaded ↓</span
      >
      <span class="text-base text-foreground font-bold"
        >{formatBytes(stats.session_downloaded)}</span
      >
    </div>
    <div
      class="flex justify-between items-center p-2.5 bg-muted rounded-md border border-border transition-all hover:translate-x-0.5 hover:border-primary"
    >
      <span class="text-xs text-muted-foreground uppercase tracking-wide font-semibold"
        >Avg Upload Rate ↑</span
      >
      <span class="text-base text-foreground font-bold"
        >{stats.average_upload_rate.toFixed(1)} KB/s</span
      >
    </div>
    <div
      class="flex justify-between items-center p-2.5 bg-muted rounded-md border border-border transition-all hover:translate-x-0.5 hover:border-primary"
    >
      <span class="text-xs text-muted-foreground uppercase tracking-wide font-semibold"
        >Avg Download Rate ↓</span
      >
      <span class="text-base text-foreground font-bold"
        >{stats.average_download_rate.toFixed(1)} KB/s</span
      >
    </div>
  </div>
</Card>
