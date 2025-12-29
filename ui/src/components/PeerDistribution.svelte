<script>
  import Card from '$lib/components/ui/card.svelte';

  let { stats } = $props();
</script>

<Card class="p-3">
  <h2 class="mb-3 text-primary text-lg font-semibold">🌐 Peer Distribution</h2>
  <div class="flex flex-col items-center gap-6 p-4">
    {#if stats}
      {@const total = stats.seeders + stats.leechers}
      {@const seederPercent = total > 0 ? (stats.seeders / total) * 100 : 50}
      {@const leecherPercent = total > 0 ? (stats.leechers / total) * 100 : 50}
      {@const seederAngle = (seederPercent / 100) * 360}

      <svg viewBox="0 0 200 200" class="max-w-[250px] w-full h-auto">
        <defs>
          <filter id="shadow">
            <feDropShadow dx="0" dy="2" stdDeviation="3" flood-opacity="0.3" />
          </filter>
        </defs>

        {#if total > 0}
          <!-- Seeders slice (green) -->
          {@const seederPath =
            seederAngle >= 360
              ? 'M 100 100 m 0 -90 a 90 90 0 1 1 0 180 a 90 90 0 1 1 0 -180'
              : `M 100 100 L 100 10 A 90 90 0 ${seederAngle > 180 ? 1 : 0} 1 ${100 + 90 * Math.sin((seederAngle * Math.PI) / 180)} ${100 - 90 * Math.cos((seederAngle * Math.PI) / 180)} Z`}

          <path d={seederPath} fill="#10b981" filter="url(#shadow)" />

          <!-- Leechers slice (red) -->
          {#if leecherPercent > 0}
            {@const leecherPath = `M 100 100 L ${100 + 90 * Math.sin((seederAngle * Math.PI) / 180)} ${100 - 90 * Math.cos((seederAngle * Math.PI) / 180)} A 90 90 0 ${leecherPercent > 50 ? 1 : 0} 1 100 10 Z`}
            <path d={leecherPath} fill="#ef4444" filter="url(#shadow)" />
          {/if}

          <!-- Center circle (donut effect) -->
          <circle cx="100" cy="100" r="50" class="fill-card" />

          <!-- Center text -->
          <text
            x="100"
            y="95"
            text-anchor="middle"
            class="fill-foreground"
            font-size="24"
            font-weight="bold">{total}</text
          >
          <text
            x="100"
            y="115"
            text-anchor="middle"
            class="fill-foreground opacity-70"
            font-size="12">Total Peers</text
          >
        {:else}
          <circle cx="100" cy="100" r="90" class="fill-muted" />
          <circle cx="100" cy="100" r="50" class="fill-card" />
          <text x="100" y="105" text-anchor="middle" class="fill-foreground" font-size="14"
            >No data</text
          >
        {/if}
      </svg>

      <div class="flex gap-8 flex-wrap justify-center">
        <div class="flex items-center gap-3">
          <div class="w-5 h-5 bg-green-600 rounded flex-shrink-0"></div>
          <div class="flex flex-col gap-1">
            <div class="text-xs text-muted-foreground uppercase tracking-wider font-semibold">
              Seeders
            </div>
            <div class="text-base text-foreground font-bold">
              {stats.seeders} ({seederPercent.toFixed(1)}%)
            </div>
          </div>
        </div>
        <div class="flex items-center gap-3">
          <div class="w-5 h-5 bg-red-500 rounded flex-shrink-0"></div>
          <div class="flex flex-col gap-1">
            <div class="text-xs text-muted-foreground uppercase tracking-wider font-semibold">
              Leechers
            </div>
            <div class="text-base text-foreground font-bold">
              {stats.leechers} ({leecherPercent.toFixed(1)}%)
            </div>
          </div>
        </div>
      </div>
    {/if}
  </div>
</Card>
