<script>
  import { cn } from '$lib/utils.js';
  import { Play, Pause, Square, RefreshCw, Rocket } from '@lucide/svelte';

  let {
    statusMessage,
    statusType,
    statusIcon = null,
    isRunning = false,
    isPaused = false,
    startFaking = null,
    stopFaking = null,
    pauseFaking = null,
    resumeFaking = null,
    manualUpdate = null,
  } = $props();

  const statusStyles = {
    idle: 'bg-gradient-to-r from-stat-upload to-stat-upload/90 text-white border-stat-upload shadow-lg shadow-stat-upload/30',
    running:
      'bg-gradient-to-r from-primary to-primary/90 text-primary-foreground border-primary shadow-lg shadow-primary/30',
    success:
      'bg-gradient-to-r from-stat-upload to-stat-upload/90 text-white border-stat-upload shadow-lg shadow-stat-upload/30',
    warning:
      'bg-gradient-to-r from-stat-ratio to-stat-ratio/90 text-white border-stat-ratio shadow-lg shadow-stat-ratio/30',
    error:
      'bg-gradient-to-r from-destructive to-destructive/90 text-destructive-foreground border-destructive shadow-lg shadow-destructive/30',
  };
</script>

<div
  class={cn(
    'px-4 py-3 flex items-center gap-3 font-semibold transition-all border-2 backdrop-blur-sm',
    statusStyles[statusType] || statusStyles.idle
  )}
>
  {#if statusIcon === 'rocket'}
    <Rocket size={18} class="flex-shrink-0" />
  {:else if statusIcon === 'pause'}
    <Pause size={18} class="flex-shrink-0" fill="currentColor" />
  {:else}
    <div
      class={cn('w-3 h-3 rounded-full flex-shrink-0', statusType === 'running' && 'animate-pulse')}
      style="background: currentColor;"
    ></div>
  {/if}
  <span class="flex-1 text-[15px]">{statusMessage}</span>

  <!-- Control Buttons -->
  {#if startFaking && stopFaking}
    <div class="flex gap-2 items-center">
      {#if !isRunning}
        <button
          onclick={startFaking}
          class="px-3 py-1.5 rounded-md flex items-center gap-1.5 font-semibold text-sm transition-all bg-white hover:bg-gray-100 text-gray-900 shadow-lg shadow-black/15 border-0 cursor-pointer"
        >
          <Play size={14} fill="currentColor" />
          <span>Start</span>
        </button>
      {:else}
        {#if !isPaused}
          <button
            onclick={pauseFaking}
            class="px-3 py-1.5 rounded-md flex items-center gap-1.5 font-semibold text-sm transition-all bg-stat-ratio hover:bg-stat-ratio/90 text-white shadow-lg shadow-stat-ratio/25 border-0 cursor-pointer"
          >
            <Pause size={14} fill="currentColor" />
            <span>Pause</span>
          </button>
        {:else}
          <button
            onclick={resumeFaking}
            class="px-3 py-1.5 rounded-md flex items-center gap-1.5 font-semibold text-sm transition-all bg-white hover:bg-gray-100 text-gray-900 shadow-lg shadow-black/15 border-0 cursor-pointer"
          >
            <Play size={14} fill="currentColor" />
            <span>Resume</span>
          </button>
        {/if}
        <button
          onclick={manualUpdate}
          class="px-3 py-1.5 rounded-md flex items-center gap-1.5 font-semibold text-sm transition-all bg-gradient-to-r from-blue-600 to-blue-500 hover:from-blue-700 hover:to-blue-600 text-white shadow-lg shadow-blue-500/25 border-0 cursor-pointer"
        >
          <RefreshCw size={14} />
          <span>Update</span>
        </button>
        <button
          onclick={stopFaking}
          class="px-3 py-1.5 rounded-md flex items-center gap-1.5 font-semibold text-sm transition-all bg-stat-leecher hover:bg-stat-leecher/90 text-white shadow-lg shadow-stat-leecher/25 border-0 cursor-pointer"
        >
          <Square size={14} fill="currentColor" />
          <span>Stop</span>
        </button>
      {/if}
    </div>
  {/if}
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
