<script>
  import Card from '$lib/components/ui/card.svelte';
  import Checkbox from '$lib/components/ui/checkbox.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import {
    Terminal,
    Trash2,
    AlertCircle,
    AlertTriangle,
    Info,
    Bug,
    ChevronDown,
    ChevronRight,
  } from '@lucide/svelte';

  let { logs = $bindable([]), showLogs = $bindable(false), onUpdate } = $props();

  let scrollContainer = $state();
  let isAtBottom = $state(true);

  // Check if user is at the bottom of the scroll container
  function handleScroll() {
    if (!scrollContainer) return;
    const threshold = 10; // pixels from bottom to consider "at bottom"
    const { scrollTop, scrollHeight, clientHeight } = scrollContainer;
    isAtBottom = scrollHeight - scrollTop - clientHeight < threshold;
  }

  // Auto-scroll to bottom when new logs are added, but only if user is at bottom
  $effect(() => {
    if (scrollContainer && logs.length > 0 && isAtBottom) {
      // Use requestAnimationFrame to ensure DOM has updated
      requestAnimationFrame(() => {
        if (scrollContainer) {
          scrollContainer.scrollTop = scrollContainer.scrollHeight;
        }
      });
    }
  });

  function clearLogs() {
    logs = [];
    isAtBottom = true; // Reset to auto-scroll after clearing
  }

  function formatTimestamp(timestamp) {
    const date = new Date(timestamp);
    return date.toLocaleTimeString('en-US', { hour12: false });
  }

  function getLogColors(level) {
    switch (level) {
      case 'error':
        return {
          text: 'text-stat-leecher',
          bg: 'bg-stat-leecher/10',
          border: 'border-stat-leecher/20',
        };
      case 'warn':
        return { text: 'text-stat-ratio', bg: 'bg-stat-ratio/10', border: 'border-stat-ratio/20' };
      case 'info':
        return {
          text: 'text-stat-download',
          bg: 'bg-stat-download/10',
          border: 'border-stat-download/20',
        };
      case 'debug':
        return { text: 'text-muted-foreground', bg: 'bg-muted', border: 'border-border' };
      default:
        return { text: 'text-foreground', bg: 'bg-muted', border: 'border-border' };
    }
  }

  // Count logs by level
  let logCounts = $derived(
    logs.reduce(
      (acc, l) => {
        if (l.level in acc) acc[l.level]++;
        return acc;
      },
      { error: 0, warn: 0, info: 0, debug: 0 }
    )
  );
</script>

<Card>
  <!-- Header -->
  <div class="flex h-8 items-center gap-2 border-b border-border px-2.5">
    <Checkbox
      id="show-logs"
      bind:checked={showLogs}
      onchange={checked => {
        showLogs = checked;
        if (onUpdate) {
          onUpdate({ showLogs: checked });
        }
      }}
    />
    <Label
      for="show-logs"
      class="flex cursor-pointer items-center gap-1.5 text-[0.625rem] font-semibold uppercase tracking-[0.16em] text-muted-foreground"
    >
      <Terminal size={12} />
      Logs
    </Label>

    {#if logs.length > 0}
      <span class="tabular-nums text-[0.625rem] text-muted-foreground">{logs.length}</span>
    {/if}

    <div class="ml-auto flex items-center gap-1.5">
      {#if showLogs && logs.length > 0}
        <!-- Log level counts -->
        <div class="mr-1 hidden items-center gap-1.5 sm:flex">
          {#if logCounts.error > 0}
            <span class="flex items-center gap-1 text-[0.625rem] text-stat-leecher">
              <AlertCircle size={10} />
              <span class="tabular-nums">{logCounts.error}</span>
            </span>
          {/if}
          {#if logCounts.warn > 0}
            <span class="flex items-center gap-1 text-[0.625rem] text-stat-ratio">
              <AlertTriangle size={10} />
              <span class="tabular-nums">{logCounts.warn}</span>
            </span>
          {/if}
        </div>

        <button
          onclick={clearLogs}
          class="flex cursor-pointer items-center gap-1 border border-stat-danger/40 px-1.5 py-0.5 text-[0.625rem] text-stat-danger transition-colors hover:bg-stat-danger/10"
        >
          <Trash2 size={11} />
          Clear
        </button>
      {/if}

      <button
        onclick={() => {
          showLogs = !showLogs;
          if (onUpdate) {
            onUpdate({ showLogs });
          }
        }}
        class="flex h-6 w-6 cursor-pointer items-center justify-center text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
        title={showLogs ? 'Collapse logs' : 'Expand logs'}
        aria-label={showLogs ? 'Collapse logs' : 'Expand logs'}
      >
        {#if showLogs}
          <ChevronDown size={13} />
        {:else}
          <ChevronRight size={13} />
        {/if}
      </button>
    </div>
  </div>

  <!-- Log content -->
  {#if showLogs}
    <div class="border-t border-border">
      {#if logs.length === 0}
        <div class="p-6 text-center">
          <Terminal size={20} class="mx-auto mb-1.5 text-muted-foreground opacity-30" />
          <p class="text-[0.6875rem] text-muted-foreground">No logs yet</p>
          <p class="mt-0.5 text-[0.625rem] text-muted-foreground/60">
            Application events will appear here
          </p>
        </div>
      {:else}
        <div
          bind:this={scrollContainer}
          onscroll={handleScroll}
          class="max-h-[15.625rem] space-y-px overflow-y-auto p-1.5 text-[0.6875rem]"
        >
          {#each logs as log, index (index)}
            {@const colors = getLogColors(log.level)}
            <div class="flex items-start gap-2 border-l-2 {colors.border} bg-muted/30 px-1.5 py-1">
              <!-- Level badge -->
              <span
                class="flex flex-shrink-0 items-center gap-1 text-[0.5625rem] font-bold uppercase {colors.text}"
              >
                {#if log.level === 'error'}
                  <AlertCircle size={10} />
                {:else if log.level === 'warn'}
                  <AlertTriangle size={10} />
                {:else if log.level === 'info'}
                  <Info size={10} />
                {:else}
                  <Bug size={10} />
                {/if}
                {log.level}
              </span>

              <!-- Timestamp -->
              <span class="flex-shrink-0 tabular-nums text-muted-foreground">
                {formatTimestamp(log.timestamp)}
              </span>

              <!-- Message -->
              <span class="flex-1 break-all {colors.text}">
                {log.message}
              </span>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</Card>
