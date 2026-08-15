<script>
  import { Menu } from '@lucide/svelte';
  import StatusBar from './StatusBar.svelte';

  let {
    onToggleSidebar,
    eyebrow = 'Standard',
    title = 'Session workspace',
    description = 'Select, configure, and monitor one torrent instance',
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
</script>

<header
  class="shrink-0 border-b border-border bg-card/95 py-2.5 pr-16 pl-3 backdrop-blur-xl md:px-4 md:pr-16 xl:pr-72"
>
  <div class="mx-auto flex max-w-[1500px] min-w-0 items-center gap-3">
    <button
      class="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-border bg-background text-muted-foreground hover:bg-muted hover:text-foreground lg:hidden"
      onclick={onToggleSidebar}
      aria-label="Open navigation"
    >
      <Menu size={18} />
    </button>

    <div class="min-w-0 flex-1">
      <div class="workspace-kicker mb-0.5">{eyebrow}</div>
      <h1 class="page-heading truncate">{title}</h1>
      <p class="page-description hidden truncate sm:block">{description}</p>
    </div>

    {#if showStatus}
      <div class="min-w-0 flex-[1.4]">
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
