<script>
  import { cn } from '$lib/utils.js';
  import { Play, Pause, Square, RefreshCw, Rocket, Moon } from '@lucide/svelte';

  let {
    statusMessage,
    statusType,
    statusIcon = null,
    embedded = false,
    isRunning = false,
    isPaused = false,
    startFaking = null,
    stopFaking = null,
    pauseFaking = null,
    resumeFaking = null,
    manualUpdate = null,
  } = $props();

  const statusMeta = {
    idle: {
      label: 'Ready',
      frame: 'border-stat-upload/20',
      chip: 'border-stat-upload/25 bg-stat-upload/10 text-stat-upload',
      dot: 'bg-stat-upload',
    },
    running: {
      label: 'Running',
      frame: 'border-primary/20',
      chip: 'border-primary/25 bg-primary/10 text-primary',
      dot: 'bg-primary',
    },
    paused: {
      label: 'Paused',
      frame: 'border-stat-ratio/20',
      chip: 'border-stat-ratio/25 bg-stat-ratio/10 text-stat-ratio',
      dot: 'bg-stat-ratio',
    },
    idling: {
      label: 'Idling',
      frame: 'border-stat-ratio/20',
      chip: 'border-stat-ratio/25 bg-stat-ratio/10 text-stat-ratio',
      dot: 'bg-stat-ratio',
    },
    success: {
      label: 'Done',
      frame: 'border-stat-upload/20',
      chip: 'border-stat-upload/25 bg-stat-upload/10 text-stat-upload',
      dot: 'bg-stat-upload',
    },
    warning: {
      label: 'Warning',
      frame: 'border-stat-ratio/20',
      chip: 'border-stat-ratio/25 bg-stat-ratio/10 text-stat-ratio',
      dot: 'bg-stat-ratio',
    },
    error: {
      label: 'Error',
      frame: 'border-destructive/20',
      chip: 'border-destructive/25 bg-destructive/10 text-destructive',
      dot: 'bg-destructive',
    },
  };

  const actionBase =
    'h-8 inline-flex items-center gap-1.5 rounded-full border px-3 text-xs font-semibold transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60 cursor-pointer';

  const embeddedActionBase =
    'inline-flex h-8 w-8 items-center justify-center rounded-xl border transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60 cursor-pointer';

  function getStatusMeta(type) {
    return statusMeta[type] || statusMeta.idle;
  }
</script>

<div
  class={cn(
    embedded
      ? 'bg-transparent px-0 py-0 shadow-none transition-colors'
      : 'rounded-2xl border bg-card/80 px-3 py-3 shadow-sm backdrop-blur-sm transition-colors',
    !embedded && getStatusMeta(statusType).frame
  )}
  role="status"
  aria-live="polite"
>
  <div
    class={cn(
      'flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between',
      embedded && 'gap-2 lg:gap-3'
    )}
  >
    <div class="min-w-0 flex flex-col gap-2 sm:flex-row sm:items-center sm:gap-3">
      <span
        class={cn(
          embedded
            ? 'inline-flex h-7 w-fit items-center gap-2 rounded-full border px-2.5 text-[10px] font-bold uppercase tracking-[0.24em]'
            : 'inline-flex w-fit items-center gap-2 rounded-full border px-3 py-1 text-[11px] font-bold uppercase tracking-[0.18em]',
          getStatusMeta(statusType).chip
        )}
      >
        {#if statusIcon === 'rocket'}
          <Rocket size={13} class="flex-shrink-0" />
        {:else if statusIcon === 'moon'}
          <Moon size={13} class="flex-shrink-0" />
        {:else if statusIcon === 'pause'}
          <Pause size={13} class="flex-shrink-0" fill="currentColor" />
        {:else}
          <span
            class={cn(
              'h-2 w-2 rounded-full flex-shrink-0',
              getStatusMeta(statusType).dot,
              statusType === 'running' && 'animate-pulse'
            )}
          ></span>
        {/if}
        <span>{getStatusMeta(statusType).label}</span>
      </span>

      <p
        class={cn(
          'min-w-0 leading-5 sm:truncate',
          embedded
            ? 'text-sm font-medium text-foreground/80'
            : 'text-sm font-medium text-foreground/90'
        )}
      >
        {statusMessage}
      </p>
    </div>

    {#if startFaking && stopFaking}
      <div class="flex flex-wrap items-center gap-2 lg:justify-end">
        {#if !isRunning}
          <button
            onclick={startFaking}
            aria-label="Start faking"
            title="Start"
            class={cn(
              embedded
                ? `${embeddedActionBase} border-stat-upload/30 bg-stat-upload text-white shadow-sm hover:bg-stat-upload/90`
                : `${actionBase} border-stat-upload/30 bg-stat-upload text-white hover:bg-stat-upload/90`
            )}
          >
            <Play size={13} fill="currentColor" />
            {#if !embedded}<span>Start</span>{/if}
          </button>
        {:else}
          {#if !isPaused}
            <button
              onclick={pauseFaking}
              aria-label="Pause faking"
              title="Pause"
              class={cn(
                embedded
                  ? `${embeddedActionBase} border-stat-ratio/30 bg-stat-ratio/10 text-stat-ratio hover:bg-stat-ratio/15`
                  : `${actionBase} border-stat-ratio/30 bg-stat-ratio/10 text-stat-ratio hover:bg-stat-ratio/15`
              )}
            >
              <Pause size={13} fill="currentColor" />
              {#if !embedded}<span>Pause</span>{/if}
            </button>
          {:else}
            <button
              onclick={resumeFaking}
              aria-label="Resume faking"
              title="Resume"
              class={cn(
                embedded
                  ? `${embeddedActionBase} border-primary/30 bg-primary/10 text-primary hover:bg-primary/15`
                  : `${actionBase} border-primary/30 bg-primary/10 text-primary hover:bg-primary/15`
              )}
            >
              <Play size={13} fill="currentColor" />
              {#if !embedded}<span>Resume</span>{/if}
            </button>
          {/if}
          <button
            onclick={manualUpdate}
            aria-label="Update stats"
            title="Update"
            class={cn(
              embedded
                ? `${embeddedActionBase} border-border/70 bg-background/65 text-muted-foreground hover:border-border hover:bg-secondary/60 hover:text-foreground`
                : `${actionBase} border-border/70 bg-background/65 text-muted-foreground hover:border-border hover:bg-secondary/60 hover:text-foreground`
            )}
          >
            <RefreshCw size={13} />
            {#if !embedded}<span>Update</span>{/if}
          </button>
          <button
            onclick={stopFaking}
            aria-label="Stop faking"
            title="Stop"
            class={cn(
              embedded
                ? `${embeddedActionBase} border-destructive/20 bg-destructive/10 text-destructive hover:bg-destructive/15`
                : `${actionBase} border-destructive/20 bg-destructive/10 text-destructive hover:bg-destructive/15`
            )}
          >
            <Square size={13} fill="currentColor" />
            {#if !embedded}<span>Stop</span>{/if}
          </button>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.5;
      transform: scale(1.2);
    }
  }

  .animate-pulse {
    animation: pulse 1.5s ease-in-out infinite;
  }
</style>
