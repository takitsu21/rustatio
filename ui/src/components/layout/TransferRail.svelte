<script>
  import { getForwardedPort, getNetworkLabel, maskIp } from '$lib/core/network.js';
  import { formatBytes, formatRate } from '$lib/core/format.js';
  import { Loader2, Ban, AlertCircle, Lock, LockOpen, RefreshCw, PlugZap } from '@lucide/svelte';

  let {
    totals = null,
    networkStatus = null,
    networkStatusLoading = false,
    networkStatusError = null,
    onRefreshNetworkStatus = () => {},
  } = $props();

  let network = $derived(getNetworkLabel(networkStatus));
  let forwardedPort = $derived(getForwardedPort(networkStatus));
</script>

<footer
  class="flex h-7 shrink-0 items-center gap-4 border-t border-border bg-card px-3 text-[0.625rem] text-muted-foreground"
  aria-label="Global transfer status"
>
  <!-- Instance states -->
  <div class="flex items-center gap-3">
    {#if totals}
      {#if totals.running > 0}
        <span class="flex items-center gap-1.5" title="{totals.running} running">
          <span class="h-1.5 w-1.5 rounded-full bg-stat-upload"></span>
          <span class="tabular-nums text-foreground">{totals.running}</span>
          <span class="hidden sm:inline">running</span>
        </span>
      {/if}
      {#if totals.paused > 0}
        <span class="flex items-center gap-1.5" title="{totals.paused} paused">
          <span class="h-1.5 w-1.5 rounded-full bg-stat-ratio"></span>
          <span class="tabular-nums text-foreground">{totals.paused}</span>
          <span class="hidden sm:inline">paused</span>
        </span>
      {/if}
      <span class="flex items-center gap-1.5" title="{totals.total} instances">
        <span class="h-1.5 w-1.5 rounded-full bg-muted-foreground/60"></span>
        <span class="tabular-nums text-foreground">{totals.total}</span>
        <span class="hidden sm:inline">total</span>
      </span>
    {:else}
      <span>No instances</span>
    {/if}
  </div>

  <span class="h-3 w-px bg-border" aria-hidden="true"></span>

  <!-- Live rates -->
  <div class="flex items-center gap-4">
    <span class="flex items-center gap-1" title="Total upload rate">
      <span class="text-stat-upload">↑</span>
      <span class="tabular-nums text-stat-upload">{formatRate(totals?.uploadRate ?? 0)}</span>
    </span>
    <span class="flex items-center gap-1" title="Total download rate">
      <span class="text-stat-leecher">↓</span>
      <span class="tabular-nums text-stat-leecher">{formatRate(totals?.downloadRate ?? 0)}</span>
    </span>
    <span class="hidden items-center gap-1 md:flex" title="Cumulative ratio">
      <span>ratio</span>
      <span
        class="tabular-nums font-semibold {totals && totals.ratio >= 1
          ? 'text-stat-upload'
          : 'text-stat-ratio'}">{(totals?.ratio ?? 0).toFixed(2)}x</span
      >
    </span>
    <span class="hidden items-center gap-1 lg:flex" title="Cumulative transfer">
      <span class="text-stat-upload">{formatBytes(totals?.uploaded ?? 0)}</span>
      <span>/</span>
      <span class="text-stat-leecher">{formatBytes(totals?.downloaded ?? 0)}</span>
    </span>
  </div>

  <!-- Network status -->
  <div class="ml-auto flex items-center gap-3">
    {#if networkStatusLoading}
      <span class="flex items-center gap-1.5">
        <Loader2 size={11} class="animate-spin" />
        <span class="hidden lg:inline">Checking network…</span>
      </span>
    {:else if networkStatusError === 'unavailable'}
      <span class="flex items-center gap-1.5" title="Network status unavailable in this mode">
        <Ban size={11} />
        <span class="hidden lg:inline">IP hidden</span>
      </span>
    {:else if networkStatusError}
      <button
        class="flex items-center gap-1.5 text-destructive transition-colors hover:text-destructive/80 cursor-pointer"
        onclick={onRefreshNetworkStatus}
        title="Click to retry network status"
      >
        <AlertCircle size={11} />
        <span>Network error</span>
      </button>
    {:else if network}
      <button
        class="flex items-center gap-1.5 transition-colors hover:text-foreground cursor-pointer"
        onclick={onRefreshNetworkStatus}
        title="Refresh network status"
      >
        {#if !networkStatus || networkStatus.configured === false}
          <LockOpen size={11} class="text-stat-ratio" />
        {:else if networkStatus.is_vpn}
          <Lock size={11} class="text-stat-upload" />
        {:else}
          <LockOpen size={11} class="text-stat-ratio" />
        {/if}
        <span
          class="hidden max-w-[11.25rem] truncate lg:inline {networkStatus?.is_vpn
            ? 'text-stat-upload'
            : 'text-stat-ratio'}">{network.label}</span
        >
        {#if networkStatus?.ip}
          <span class="tabular-nums hidden xl:inline">{maskIp(networkStatus.ip)}</span>
        {/if}
        {#if forwardedPort}
          <span class="hidden items-center gap-1 xl:flex text-muted-foreground">
            <PlugZap size={10} class="text-stat-upload" />
            <span class="tabular-nums text-foreground">{forwardedPort}</span>
          </span>
        {/if}
        <RefreshCw size={10} class="opacity-50" />
      </button>
    {/if}
  </div>
</footer>
