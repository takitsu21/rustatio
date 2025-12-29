<script>
  import { cn } from '$lib/utils.js';

  let { statusMessage, statusType } = $props();

  const statusStyles = {
    idle: 'bg-gradient-to-r from-green-600 to-green-500 text-white border-green-500 shadow-lg shadow-green-500/30',
    running:
      'bg-gradient-to-r from-primary to-primary/90 text-primary-foreground border-primary shadow-lg shadow-primary/30',
    success:
      'bg-gradient-to-r from-green-600 to-green-500 text-white border-green-500 shadow-lg shadow-green-500/30',
    warning:
      'bg-gradient-to-r from-amber-500 to-amber-400 text-white border-amber-400 shadow-lg shadow-amber-400/30',
    error:
      'bg-gradient-to-r from-destructive to-destructive/90 text-destructive-foreground border-destructive shadow-lg shadow-destructive/30',
  };
</script>

<div
  class={cn(
    'max-w-7xl mx-auto mb-3 px-6 py-4 rounded-xl flex items-center gap-3 font-semibold transition-all border-2 backdrop-blur-sm',
    statusStyles[statusType] || statusStyles.idle
  )}
>
  <div
    class={cn('w-3 h-3 rounded-full flex-shrink-0', statusType === 'running' && 'animate-pulse')}
    style="background: currentColor;"
  ></div>
  <span class="flex-1 text-[15px]">{statusMessage}</span>
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
