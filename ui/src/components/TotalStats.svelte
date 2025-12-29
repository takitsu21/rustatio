<script>
  import Card from '$lib/components/ui/card.svelte';

  let { stats, formatBytes } = $props();
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
      <span class="text-base text-foreground font-bold">{formatBytes(stats.uploaded)}</span>
    </div>
    <div
      class="flex justify-between items-center p-2.5 bg-muted rounded-md border border-border transition-all hover:translate-x-0.5 hover:border-primary"
    >
      <span class="text-xs text-muted-foreground uppercase tracking-wide font-semibold"
        >Total Downloaded ↓</span
      >
      <span class="text-base text-foreground font-bold">{formatBytes(stats.downloaded)}</span>
    </div>
    <div
      class="flex justify-between items-center p-2.5 bg-muted rounded-md border border-border transition-all hover:translate-x-0.5 hover:border-primary"
    >
      <span class="text-xs text-muted-foreground uppercase tracking-wide font-semibold">Ratio</span>
      <span class="text-lg text-green-600 dark:text-green-500 font-bold"
        >{stats.ratio.toFixed(2)}</span
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
