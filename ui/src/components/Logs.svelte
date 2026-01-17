<script>
  import Card from '$lib/components/ui/card.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Checkbox from '$lib/components/ui/checkbox.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import { ScrollText } from '@lucide/svelte';

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

  function getLogClass(level) {
    switch (level) {
      case 'error':
        return 'text-red-400';
      case 'warn':
        return 'text-yellow-400';
      case 'info':
        return 'text-blue-400';
      case 'debug':
        return 'text-gray-400';
      default:
        return 'text-foreground';
    }
  }

  function getLevelBadgeClass(level) {
    switch (level) {
      case 'error':
        return 'bg-red-600 text-white';
      case 'warn':
        return 'bg-amber-500 text-white';
      case 'info':
        return 'bg-primary text-primary-foreground';
      case 'debug':
        return 'bg-gray-600 text-white';
      default:
        return 'bg-muted text-muted-foreground';
    }
  }
</script>

<Card class="p-3">
  <div class="flex justify-between items-center mb-3">
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
      <Label for="show-logs" class="cursor-pointer font-semibold text-base flex items-center gap-2"
        ><ScrollText size={18} /> Show Application Logs</Label
      >
    </div>
    {#if showLogs && logs.length > 0}
      <Button onclick={clearLogs} variant="destructive" size="sm">
        {#snippet children()}
          Clear Logs
        {/snippet}
      </Button>
    {/if}
  </div>

  {#if showLogs}
    <div
      class="bg-muted border border-border rounded-lg p-4 h-[300px] overflow-y-auto font-mono text-sm leading-relaxed"
      bind:this={logsContainer}
    >
      {#if logs.length === 0}
        <div class="text-muted-foreground text-center py-8 italic">
          No logs yet. Application events will appear here...
        </div>
      {:else}
        {#each logs as log, index (index)}
          <div class="mb-1 p-1 rounded">
            <span class="text-muted-foreground mr-2">[{formatTimestamp(log.timestamp)}]</span>
            <span
              class="font-bold mr-2 px-1.5 py-0.5 rounded text-xs {getLevelBadgeClass(log.level)}"
              >{log.level.toUpperCase()}</span
            >
            <span class={getLogClass(log.level)}>{log.message}</span>
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</Card>
