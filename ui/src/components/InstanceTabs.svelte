<script>
  import { instances, activeInstanceId, instanceActions } from '../lib/instanceStore.js';
  import { cn } from '$lib/utils.js';
  import Button from '$lib/components/ui/button.svelte';

  function getInstanceLabel(instance) {
    if (instance.torrent) {
      const name = instance.torrent.name;
      return name.length > 20 ? name.substring(0, 20) + '...' : name;
    }
    return `Instance ${instance.id}`;
  }

  function getInstanceStatus(instance) {
    if (instance.isRunning) {
      return instance.isPaused ? 'paused' : 'running';
    }
    return 'idle';
  }

  async function handleAddInstance() {
    try {
      await instanceActions.addInstance();
    } catch (error) {
      console.error('Failed to add instance:', error);
    }
  }

  async function handleRemoveInstance(event, id) {
    event.stopPropagation();
    event.preventDefault();

    try {
      await instanceActions.removeInstance(id);
    } catch (error) {
      console.error('Failed to remove instance:', error);
    }
  }

  function handleSelectInstance(id) {
    try {
      instanceActions.selectInstance(id);
    } catch (error) {
      console.error('Error switching instance:', error);
    }
  }
</script>

<div class="bg-background px-4 pt-2 mb-3">
  <div class="max-w-7xl mx-auto border-b-2 border-primary/10 pb-2">
    <div
      class="flex items-center gap-2 overflow-x-auto scrollbar-thin scrollbar-thumb-primary scrollbar-track-muted"
    >
      {#each $instances as instance (instance.id)}
        {@const status = getInstanceStatus(instance)}
        {@const isActive = $activeInstanceId === instance.id}
        <div
          class={cn(
            'flex items-center gap-2 px-3 py-2 rounded-t-lg text-sm font-semibold cursor-pointer transition-all whitespace-nowrap relative border-2 border-transparent border-b-0',
            isActive
              ? 'bg-card text-primary border-primary border-b-card -mb-0.5 pb-[calc(0.5rem+2px)]'
              : 'bg-muted text-muted-foreground hover:bg-card hover:text-foreground hover:border-border'
          )}
          role="button"
          tabindex="0"
          onclick={() => handleSelectInstance(instance.id)}
          onkeydown={e => e.key === 'Enter' && handleSelectInstance(instance.id)}
          title={instance.torrent ? instance.torrent.name : `Instance ${instance.id}`}
        >
          <span
            class={cn(
              'flex items-center justify-center flex-shrink-0',
              status === 'idle' && 'text-muted-foreground',
              status === 'running' && 'text-green-500 animate-pulse-slow',
              status === 'paused' && 'text-amber-500'
            )}
          >
            {#if status === 'running'}
              <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                <circle cx="12" cy="12" r="10" />
              </svg>
            {:else if status === 'paused'}
              <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                <rect x="6" y="4" width="4" height="16" />
                <rect x="14" y="4" width="4" height="16" />
              </svg>
            {:else}
              <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" opacity="0.3">
                <circle cx="12" cy="12" r="8" />
              </svg>
            {/if}
          </span>
          <span class="select-none flex-shrink-0">{getInstanceLabel(instance)}</span>
          {#if $instances.length > 1}
            <button
              class="flex items-center justify-center w-5 h-5 flex-shrink-0 p-0.5 ml-1 bg-transparent border-none rounded cursor-pointer transition-all outline-none relative z-10 group"
              onclick={e => handleRemoveInstance(e, instance.id)}
              title="Close instance"
              aria-label="Close instance"
            >
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2.5"
                stroke-linecap="round"
                class="text-muted-foreground group-hover:text-destructive transition-colors"
              >
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
              </svg>
            </button>
          {/if}
        </div>
      {/each}
      <Button
        onclick={handleAddInstance}
        size="icon"
        class="w-9 h-9 flex-shrink-0"
        title="Add new instance"
        aria-label="Add new instance"
      >
        {#snippet children()}
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.5"
            stroke-linecap="round"
          >
            <line x1="12" y1="5" x2="12" y2="19"></line>
            <line x1="5" y1="12" x2="19" y2="12"></line>
          </svg>
        {/snippet}
      </Button>
    </div>
  </div>
</div>

<style>
  @keyframes pulse-slow {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.6;
    }
  }

  .animate-pulse-slow {
    animation: pulse-slow 2s ease-in-out infinite;
  }

  /* Custom scrollbar styles */
  .scrollbar-thin {
    scrollbar-width: thin;
  }

  .scrollbar-thin::-webkit-scrollbar {
    height: 6px;
  }

  .scrollbar-track-muted::-webkit-scrollbar-track {
    background: hsl(var(--muted));
    border-radius: 3px;
  }

  .scrollbar-thumb-primary::-webkit-scrollbar-thumb {
    background: hsl(var(--primary));
    border-radius: 3px;
  }

  /* Close button hover effects */
  .group:hover {
    transform: scale(1.1);
  }

  .group:active {
    transform: scale(0.95);
  }

  @media (max-width: 768px) {
    .flex-shrink-0:not(button) {
      max-width: 120px;
      overflow: hidden;
      text-overflow: ellipsis;
    }
  }

  @media (max-width: 480px) {
    .flex-shrink-0:not(button) {
      max-width: 80px;
    }
  }
</style>
