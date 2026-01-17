<script>
  import { api } from '$lib/api.js';
  import { cn } from '$lib/utils.js';
  import { onMount } from 'svelte';
  import { Loader2, Ban, AlertCircle, Lock, LockOpen, RefreshCw } from '@lucide/svelte';

  let { isCollapsed = false } = $props();

  let status = $state(null);
  let loading = $state(false);
  let error = $state(null);

  // Mask IP address for privacy (show first and last octets)
  function maskIp(ip) {
    if (!ip) return '---';
    const parts = ip.split('.');
    if (parts.length === 4) {
      return `${parts[0]}.***.***.${parts[3]}`;
    }
    // IPv6 or other format
    return ip.substring(0, 8) + '...';
  }

  async function fetchStatus() {
    loading = true;
    error = null;
    try {
      const result = await api.getNetworkStatus();
      if (result) {
        status = result;
      } else {
        // null means unavailable (e.g., CORS blocked on WASM)
        error = 'unavailable';
      }
    } catch (e) {
      error = e.message || 'Failed to fetch';
      console.error('Network status error:', e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    fetchStatus();
  });
</script>

<div class={cn('px-3 py-2', isCollapsed && 'lg:px-2')}>
  {#if loading}
    <!-- Loading state -->
    <div class="flex items-center gap-2 text-xs text-muted-foreground">
      <Loader2 size={16} class="animate-spin" />
      <span class={cn(isCollapsed && 'lg:hidden')}>Checking...</span>
    </div>
  {:else if error === 'unavailable'}
    <!-- Unavailable (CORS blocked) -->
    <div
      class={cn(
        'flex items-center gap-2 text-xs text-muted-foreground/50',
        isCollapsed && 'lg:justify-center'
      )}
      title="Network status unavailable in this mode"
    >
      <Ban size={16} class="flex-shrink-0 opacity-50" />
      <span class={cn(isCollapsed && 'lg:hidden')}>IP hidden</span>
    </div>
  {:else if error}
    <!-- Error state -->
    <button
      class={cn(
        'flex items-center gap-2 text-xs text-destructive hover:text-destructive/80 transition-colors bg-transparent border-0 p-0 cursor-pointer',
        isCollapsed && 'lg:justify-center'
      )}
      onclick={fetchStatus}
      title="Click to retry"
    >
      <AlertCircle size={16} class="flex-shrink-0" />
      <span class={cn(isCollapsed && 'lg:hidden')}>Error</span>
    </button>
  {:else if status}
    <!-- Status display -->
    <div class="space-y-1.5">
      <!-- VPN Status indicator -->
      <div class={cn('flex items-center gap-2', isCollapsed && 'lg:justify-center')}>
        {#if status.is_vpn}
          <!-- VPN detected - green lock -->
          <Lock size={16} class="flex-shrink-0 text-green-500" />
          <div class={cn('flex-1 min-w-0', isCollapsed && 'lg:hidden')}>
            <div class="text-xs font-medium text-green-500 truncate">
              {status.vpn_provider || 'VPN Active'}
              <span class="text-muted-foreground font-normal">(Experimental)</span>
            </div>
          </div>
        {:else}
          <!-- No VPN - yellow warning -->
          <LockOpen size={16} class="flex-shrink-0 text-amber-500" />
          <div class={cn('flex-1 min-w-0', isCollapsed && 'lg:hidden')}>
            <div class="text-xs font-medium text-amber-500">
              No VPN <span class="text-muted-foreground font-normal">(Experimental)</span>
            </div>
          </div>
        {/if}

        <!-- Refresh button -->
        <button
          class={cn(
            'p-1 rounded hover:bg-muted transition-colors bg-transparent border-0 cursor-pointer',
            isCollapsed && 'lg:hidden'
          )}
          onclick={fetchStatus}
          title="Refresh network status"
        >
          <RefreshCw size={12} class="text-muted-foreground hover:text-foreground" />
        </button>
      </div>

      <!-- IP and Location -->
      {#if !isCollapsed}
        <div class="text-[10px] text-muted-foreground pl-6 space-y-0.5">
          <div class="flex items-center gap-1.5">
            <span class="font-mono">{maskIp(status.ip)}</span>
            {#if status.country}
              <span class="text-muted-foreground/70">({status.country})</span>
            {/if}
          </div>
          {#if status.org && !status.is_vpn}
            <div class="truncate text-muted-foreground/60" title={status.org}>
              {status.org}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>
