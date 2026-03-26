<script>
  import { cn } from '$lib/utils.js';
  import { Loader2, Ban, AlertCircle, Lock, LockOpen, RefreshCw, PlugZap } from '@lucide/svelte';

  let {
    isCollapsed = false,
    networkStatus = null,
    networkStatusLoading = false,
    networkStatusError = null,
    onRefreshNetworkStatus = () => {},
  } = $props();

  function getForwardedPort(status) {
    return status?.forwarded_port ?? status?.forwardedPort ?? null;
  }

  function getPeerListenerPort(status) {
    return status?.peer_listener_port ?? status?.peerListenerPort ?? null;
  }

  function getPeerListenerError(status) {
    return status?.peer_listener_error ?? status?.peerListenerError ?? null;
  }

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
</script>

<div class={cn('px-3 py-2', isCollapsed && 'lg:px-2')}>
  {#if networkStatusLoading}
    <!-- Loading state -->
    <div class="flex items-center gap-2 text-xs text-muted-foreground">
      <Loader2 size={16} class="animate-spin" />
      <span class={cn(isCollapsed && 'lg:hidden')}>Checking...</span>
    </div>
  {:else if networkStatusError === 'unavailable'}
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
  {:else if networkStatusError}
    <!-- Error state -->
    <button
      class={cn(
        'flex items-center gap-2 text-xs text-destructive hover:text-destructive/80 transition-colors bg-transparent border-0 p-0 cursor-pointer',
        isCollapsed && 'lg:justify-center'
      )}
      onclick={onRefreshNetworkStatus}
      title="Click to retry"
    >
      <AlertCircle size={16} class="flex-shrink-0" />
      <span class={cn(isCollapsed && 'lg:hidden')}>Error</span>
    </button>
  {:else if networkStatus}
    <!-- Status display -->
    <div class="space-y-1.5">
      <!-- VPN Status indicator -->
      <div class={cn('flex items-center gap-2', isCollapsed && 'lg:justify-center')}>
        {#if networkStatus.is_vpn}
          <!-- VPN detected - green lock -->
          <Lock size={16} class="flex-shrink-0 text-stat-upload" />
          <div class={cn('flex-1 min-w-0', isCollapsed && 'lg:hidden')}>
            <div class="text-xs font-medium text-stat-upload truncate">
              {networkStatus.organization || 'VPN Active'}
            </div>
          </div>
        {:else}
          <!-- No VPN - yellow warning -->
          <LockOpen size={16} class="flex-shrink-0 text-stat-ratio" />
          <div class={cn('flex-1 min-w-0', isCollapsed && 'lg:hidden')}>
            <div class="text-xs font-medium text-stat-ratio">No VPN</div>
          </div>
        {/if}

        <!-- Refresh button -->
        <button
          class={cn(
            'p-1 rounded hover:bg-muted transition-colors bg-transparent border-0 cursor-pointer',
            isCollapsed && 'lg:hidden'
          )}
          onclick={onRefreshNetworkStatus}
          title="Refresh network status"
        >
          <RefreshCw size={12} class="text-muted-foreground hover:text-foreground" />
        </button>
      </div>

      <!-- IP and Location -->
      {#if !isCollapsed}
        <div class="text-[10px] text-muted-foreground pl-6">
          <div class="flex items-center gap-1.5">
            <span class="font-mono">{maskIp(networkStatus.ip)}</span>
            {#if networkStatus.country}
              <span class="text-muted-foreground/70">({networkStatus.country})</span>
            {/if}
          </div>
          {#if getForwardedPort(networkStatus)}
            <div class="mt-1 flex items-center gap-1.5">
              <PlugZap size={10} class="text-stat-upload" />
              <span>Forwarded port</span>
              <span class="font-mono text-foreground">{getForwardedPort(networkStatus)}</span>
            </div>
          {/if}
          {#if getPeerListenerPort(networkStatus)}
            <div class="mt-1 flex items-center gap-1.5">
              <PlugZap size={10} class="text-foreground" />
              <span>Listening</span>
              <span class="font-mono text-foreground">{getPeerListenerPort(networkStatus)}</span>
            </div>
          {/if}
          {#if getPeerListenerError(networkStatus)}
            <div class="mt-1 text-[10px] text-destructive">
              {getPeerListenerError(networkStatus)}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>
