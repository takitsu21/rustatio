<script>
  import { ArrowLeft, Play, Pause, RefreshCw, Square } from '@lucide/svelte';
  import Button from '$lib/components/ui/button.svelte';
  import StatusBadge from '../common/StatusBadge.svelte';
  import { getInstanceState } from '$lib/core/status.js';
  import { viewMode } from '$lib/grid/gridStore.js';

  let {
    instance = null,
    startFaking = null,
    stopFaking = null,
    pauseFaking = null,
    resumeFaking = null,
    manualUpdate = null,
  } = $props();

  let state = $derived(getInstanceState(instance));
  let hasTorrent = $derived(Boolean(instance?.torrent));
  let isRunning = $derived(Boolean(instance?.isRunning));
  let isPaused = $derived(Boolean(instance?.isPaused));
  let name = $derived(instance?.torrent?.name || instance?.torrentPath || 'No torrent selected');
  let statusMessage = $derived(instance?.statusMessage || 'Select a torrent file to begin');
</script>

<header class="flex h-9 shrink-0 items-center gap-2 border-b border-border bg-card px-2">
  <Button
    onclick={() => viewMode.set('grid')}
    size="icon"
    variant="ghost"
    class="h-6 w-6"
    title="Back to instances"
    aria-label="Back to instances"
  >
    {#snippet children()}
      <ArrowLeft size={14} />
    {/snippet}
  </Button>

  <StatusBadge {state} size="xs" />

  <span class="min-w-0 max-w-[38ch] truncate text-xs font-medium text-foreground" title={name}>
    {name}
  </span>

  <span
    class="hidden min-w-0 truncate text-[0.625rem] text-muted-foreground md:inline"
    title={statusMessage}
  >
    {statusMessage}
  </span>

  <div class="ml-auto flex shrink-0 items-center gap-1">
    <Button
      onclick={startFaking}
      disabled={isRunning || !hasTorrent}
      size="icon"
      variant="outline"
      class="h-6 w-6 text-stat-upload"
      title="Start faking"
      aria-label="Start faking"
    >
      {#snippet children()}
        <Play size={12} fill="currentColor" />
      {/snippet}
    </Button>

    {#if isPaused}
      <Button
        onclick={resumeFaking}
        size="icon"
        variant="outline"
        class="h-6 w-6 text-stat-upload"
        title="Resume faking"
        aria-label="Resume faking"
      >
        {#snippet children()}
          <Play size={12} fill="currentColor" />
        {/snippet}
      </Button>
    {:else}
      <Button
        onclick={pauseFaking}
        disabled={!isRunning}
        size="icon"
        variant="outline"
        class="h-6 w-6 text-stat-ratio"
        title="Pause faking"
        aria-label="Pause faking"
      >
        {#snippet children()}
          <Pause size={12} fill="currentColor" />
        {/snippet}
      </Button>
    {/if}

    <Button
      onclick={manualUpdate}
      disabled={!isRunning || isPaused}
      size="icon"
      variant="outline"
      class="h-6 w-6"
      title="Update stats now"
      aria-label="Update stats now"
    >
      {#snippet children()}
        <RefreshCw size={12} />
      {/snippet}
    </Button>

    <Button
      onclick={stopFaking}
      disabled={!isRunning}
      size="icon"
      variant="outline"
      class="h-6 w-6 text-stat-danger"
      title="Stop faking"
      aria-label="Stop faking"
    >
      {#snippet children()}
        <Square size={11} fill="currentColor" />
      {/snippet}
    </Button>
  </div>
</header>
