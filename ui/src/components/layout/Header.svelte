<script>
  import { Menu } from '@lucide/svelte';
  import { cn } from '$lib/utils.js';
  import StatusBar from './StatusBar.svelte';

  let {
    onToggleSidebar,
    showStatus = false,
    statusMessage = 'Select a torrent file to begin',
    statusType = 'warning',
    statusIcon = null,
    isRunning = false,
    isPaused = false,
    startFaking = null,
    stopFaking = null,
    pauseFaking = null,
    resumeFaking = null,
    manualUpdate = null,
  } = $props();

  const statusFrameClass = {
    idle: 'border-stat-upload/20',
    running: 'border-primary/20',
    paused: 'border-stat-ratio/20',
    idling: 'border-stat-ratio/20',
    success: 'border-stat-upload/20',
    warning: 'border-stat-ratio/20',
    error: 'border-destructive/20',
  };

  function getStatusFrameClass(type) {
    return statusFrameClass[type] || 'border-border/55';
  }
</script>

<header class="border-b border-border/60 bg-background/92 px-3 py-3 backdrop-blur-xl">
  <div class="mx-auto flex max-w-7xl flex-col gap-3">
    <div class="flex min-w-0 items-start justify-between gap-3 lg:items-center">
      <button
        class="lg:hidden rounded-md bg-primary p-2 text-primary-foreground shadow-lg"
        onclick={onToggleSidebar}
        aria-label="Toggle menu"
      >
        <Menu size={20} />
      </button>

      <div class="flex min-w-0 flex-1 items-center gap-3">
        <img src="/favicon-32x32.png" alt="Rustatio" width="28" height="28" class="rounded" />
        <div class="min-w-0">
          <h1 class="truncate text-2xl font-bold tracking-tight text-foreground">Rustatio</h1>
          <p class="mt-0.5 text-xs text-muted-foreground">Modern BitTorrent Ratio Faker</p>
        </div>
      </div>
    </div>

    {#if showStatus}
      <div
        class={cn(
          'min-w-0 rounded-2xl border bg-card/25 px-3 py-2.5 transition-colors',
          getStatusFrameClass(statusType)
        )}
      >
        <StatusBar
          embedded={true}
          {statusMessage}
          {statusType}
          {statusIcon}
          {isRunning}
          {isPaused}
          {startFaking}
          {stopFaking}
          {pauseFaking}
          {resumeFaking}
          {manualUpdate}
        />
      </div>
    {/if}
  </div>
</header>
