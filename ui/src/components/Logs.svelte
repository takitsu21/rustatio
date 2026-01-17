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

  let logsContainer = $state();

  // Auto-scroll to bottom when new logs are added
  $effect(() => {
    if (logsContainer && logs.length > 0) {
      logsContainer.scrollTop = logsContainer.scrollHeight;
    }
  });

  function clearLogs() {
    logs = [];
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
  let logCounts = $derived({
    error: logs.filter(l => l.level === 'error').length,
    warn: logs.filter(l => l.level === 'warn').length,
    info: logs.filter(l => l.level === 'info').length,
    debug: logs.filter(l => l.level === 'debug').length,
  });
</script>

<Card class="p-3">
  <!-- Header -->
  <div class="flex items-center justify-between">
    <div class="flex items-center gap-3">
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
      <Label for="show-logs" class="cursor-pointer font-medium text-sm flex items-center gap-2">
        <Terminal size={16} class="text-muted-foreground" />
        Application Logs
      </Label>

      {#if logs.length > 0}
        <span class="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full">
          {logs.length}
        </span>
      {/if}
    </div>

    <div class="flex items-center gap-2">
      {#if showLogs && logs.length > 0}
        <!-- Log level counts -->
        <div class="hidden sm:flex items-center gap-1.5 mr-2">
          {#if logCounts.error > 0}
            <span
              class="flex items-center gap-1 text-xs text-stat-leecher bg-stat-leecher/10 px-1.5 py-0.5 rounded"
            >
              <AlertCircle size={10} />
              {logCounts.error}
            </span>
          {/if}
          {#if logCounts.warn > 0}
            <span
              class="flex items-center gap-1 text-xs text-stat-ratio bg-stat-ratio/10 px-1.5 py-0.5 rounded"
            >
              <AlertTriangle size={10} />
              {logCounts.warn}
            </span>
          {/if}
        </div>

        <button
          onclick={clearLogs}
          class="flex items-center gap-1.5 px-2 py-1 text-xs font-semibold bg-stat-danger hover:bg-stat-danger/90 text-white rounded shadow-sm border-0 transition-colors cursor-pointer"
        >
          <Trash2 size={12} />
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
        class="p-1 text-muted-foreground hover:text-foreground transition-colors cursor-pointer bg-transparent"
      >
        {#if showLogs}
          <ChevronDown size={16} />
        {:else}
          <ChevronRight size={16} />
        {/if}
      </button>
    </div>
  </div>

  <!-- Log content -->
  {#if showLogs}
    <div class="mt-3">
      <div
        class="bg-muted/50 border border-border rounded-lg overflow-hidden"
        bind:this={logsContainer}
      >
        {#if logs.length === 0}
          <div class="p-8 text-center">
            <Terminal size={32} class="text-muted-foreground mx-auto mb-2 opacity-30" />
            <p class="text-sm text-muted-foreground">No logs yet</p>
            <p class="text-xs text-muted-foreground/60 mt-1">Application events will appear here</p>
          </div>
        {:else}
          <div class="max-h-[250px] overflow-y-auto p-2 font-mono text-xs space-y-1">
            {#each logs as log, index (index)}
              {@const colors = getLogColors(log.level)}
              <div
                class="flex items-start gap-2 py-1.5 px-2 rounded {colors.bg} border {colors.border} transition-colors"
              >
                <!-- Level badge -->
                <span
                  class="flex-shrink-0 {colors.text} {colors.bg} px-1.5 py-0.5 rounded text-[10px] font-bold uppercase flex items-center gap-1"
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
                <span class="text-muted-foreground flex-shrink-0 tabular-nums">
                  {formatTimestamp(log.timestamp)}
                </span>

                <!-- Message -->
                <span class="flex-1 {colors.text} break-all">
                  {log.message}
                </span>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</Card>
