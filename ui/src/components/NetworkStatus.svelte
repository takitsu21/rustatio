<script>
  import { api } from '$lib/api.js';
  import { cn } from '$lib/utils.js';
  import { onMount } from 'svelte';

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
      <svg
        class="animate-spin h-4 w-4"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
      >
        <circle cx="12" cy="12" r="10" stroke-opacity="0.25"></circle>
        <path d="M12 2a10 10 0 0 1 10 10" stroke-opacity="0.75"></path>
      </svg>
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
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        class="flex-shrink-0 opacity-50"
      >
        <circle cx="12" cy="12" r="10"></circle>
        <line x1="4.93" y1="4.93" x2="19.07" y2="19.07"></line>
      </svg>
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
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        class="flex-shrink-0"
      >
        <circle cx="12" cy="12" r="10"></circle>
        <line x1="12" y1="8" x2="12" y2="12"></line>
        <line x1="12" y1="16" x2="12.01" y2="16"></line>
      </svg>
      <span class={cn(isCollapsed && 'lg:hidden')}>Error</span>
    </button>
  {:else if status}
    <!-- Status display -->
    <div class="space-y-1.5">
      <!-- VPN Status indicator -->
      <div class={cn('flex items-center gap-2', isCollapsed && 'lg:justify-center')}>
        {#if status.is_vpn}
          <!-- VPN detected - green lock -->
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            class="flex-shrink-0 text-green-500"
          >
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect>
            <path d="M7 11V7a5 5 0 0 1 10 0v4"></path>
          </svg>
          <div class={cn('flex-1 min-w-0', isCollapsed && 'lg:hidden')}>
            <div class="text-xs font-medium text-green-500 truncate">
              {status.vpn_provider || 'VPN Active'}
              <span class="text-muted-foreground font-normal">(Experimental)</span>
            </div>
          </div>
        {:else}
          <!-- No VPN - yellow warning -->
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            class="flex-shrink-0 text-amber-500"
          >
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect>
            <path d="M7 11V7a5 5 0 0 1 9.9-1"></path>
          </svg>
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
          <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            class="text-muted-foreground hover:text-foreground"
          >
            <path d="M21 2v6h-6"></path>
            <path d="M3 12a9 9 0 0 1 15-6.7L21 8"></path>
            <path d="M3 22v-6h6"></path>
            <path d="M21 12a9 9 0 0 1-15 6.7L3 16"></path>
          </svg>
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
