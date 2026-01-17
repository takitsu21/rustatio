<script>
  import { api, getRunMode } from '$lib/api.js';
  import { cn } from '$lib/utils.js';
  import Button from '$lib/components/ui/button.svelte';
  import {
    FolderOpen,
    ChevronDown,
    Check,
    Clock,
    Copy,
    X,
    Trash2,
    RefreshCw,
  } from '@lucide/svelte';

  let { isCollapsed = false } = $props();

  let watchStatus = $state(null);
  let watchFiles = $state([]);
  let isLoading = $state(false);
  let isExpanded = $state(false);
  let error = $state(null);

  // Only show in server mode
  let isServerMode = $derived(getRunMode() === 'server');

  // Format bytes to human readable
  function formatBytes(bytes) {
    if (!bytes || bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  // Get status color
  function getStatusColor(status) {
    switch (status) {
      case 'loaded':
        return 'text-green-500';
      case 'pending':
        return 'text-amber-500';
      case 'duplicate':
        return 'text-blue-500';
      case 'invalid':
        return 'text-red-500';
      default:
        return 'text-muted-foreground';
    }
  }

  async function loadWatchData() {
    if (!isServerMode) return;

    isLoading = true;
    error = null;

    try {
      const [status, files] = await Promise.all([api.getWatchStatus(), api.listWatchFiles()]);

      watchStatus = status;
      watchFiles = files || [];
    } catch (e) {
      console.error('Failed to load watch folder data:', e);
      error = e.message;
    } finally {
      isLoading = false;
    }
  }

  async function handleDeleteFile(filename) {
    try {
      await api.deleteWatchFile(filename);
      // Refresh the list
      await loadWatchData();
    } catch (e) {
      console.error('Failed to delete file:', e);
    }
  }

  function toggleExpanded() {
    if (!isExpanded) {
      loadWatchData();
    }
    isExpanded = !isExpanded;
  }

  // Initial load when component mounts
  $effect(() => {
    if (isServerMode && isExpanded) {
      loadWatchData();
    }
  });
</script>

{#if isServerMode}
  <div class="border-t border-border">
    <!-- Header/Toggle -->
    <button
      onclick={toggleExpanded}
      class={cn(
        'w-full flex items-center gap-2 px-3 py-2 hover:bg-muted transition-colors text-sm font-medium text-muted-foreground hover:text-foreground',
        isCollapsed && 'lg:justify-center lg:px-2'
      )}
      title="Watch Folder"
    >
      <!-- Folder Icon -->
      <FolderOpen size={16} class="flex-shrink-0" />

      <span class={cn('flex-1 text-left', isCollapsed && 'lg:hidden lg:w-0 lg:opacity-0')}>
        Watch Folder
      </span>

      <!-- Badge with file count -->
      {#if !isCollapsed && watchStatus && watchStatus.file_count > 0}
        <span
          class="text-xs bg-primary/20 text-primary px-1.5 py-0.5 rounded"
          title="{watchStatus.file_count} files"
        >
          {watchStatus.file_count}
        </span>
      {/if}

      <!-- Expand/Collapse Arrow -->
      {#if !isCollapsed}
        <span class={cn('transition-transform duration-200', isExpanded && 'rotate-180')}>
          <ChevronDown size={12} />
        </span>
      {/if}
    </button>

    <!-- Expanded Content -->
    {#if isExpanded && !isCollapsed}
      <div class="px-3 pb-3 space-y-2">
        <!-- Status Info -->
        {#if watchStatus}
          <div class="text-xs text-muted-foreground bg-muted/50 rounded p-2">
            <div class="flex justify-between mb-1">
              <span>Directory:</span>
              <span class="font-mono text-foreground truncate ml-2" title={watchStatus.watch_dir}>
                {watchStatus.watch_dir}
              </span>
            </div>
            <div class="flex justify-between mb-1">
              <span>Auto-start:</span>
              <span class={watchStatus.auto_start ? 'text-green-500' : 'text-muted-foreground'}>
                {watchStatus.auto_start ? 'Enabled' : 'Disabled'}
              </span>
            </div>
            <div class="flex justify-between">
              <span>Status:</span>
              <span class={watchStatus.enabled ? 'text-green-500' : 'text-red-500'}>
                {watchStatus.enabled ? 'Watching' : 'Disabled'}
              </span>
            </div>
          </div>
        {/if}

        <!-- Error Message -->
        {#if error}
          <div class="text-xs text-red-500 bg-red-500/10 rounded p-2">
            {error}
          </div>
        {/if}

        <!-- File List -->
        {#if watchFiles.length > 0}
          <div class="space-y-1 max-h-48 overflow-y-auto">
            {#each watchFiles as file (file.filename)}
              <div
                class="flex items-center gap-2 p-2 bg-muted/30 rounded text-xs group"
                title={file.name || file.filename}
              >
                <!-- Status Icon -->
                <span class={cn('flex-shrink-0', getStatusColor(file.status))}>
                  {#if file.status === 'loaded'}
                    <Check size={12} />
                  {:else if file.status === 'pending'}
                    <Clock size={12} />
                  {:else if file.status === 'duplicate'}
                    <Copy size={12} />
                  {:else if file.status === 'invalid'}
                    <X size={12} />
                  {/if}
                </span>

                <!-- File Info -->
                <div class="flex-1 min-w-0">
                  <div class="truncate text-foreground" title={file.name || file.filename}>
                    {file.name || file.filename}
                  </div>
                  <div class="flex items-center gap-2 text-muted-foreground">
                    <span class={getStatusColor(file.status)}>{file.status}</span>
                    <span>{formatBytes(file.size)}</span>
                  </div>
                </div>

                <!-- Delete Button -->
                <button
                  onclick={() => handleDeleteFile(file.filename)}
                  class="flex-shrink-0 p-1 rounded hover:bg-destructive/20 opacity-0 group-hover:opacity-100 transition-opacity"
                  title="Delete file"
                >
                  <Trash2 size={12} class="text-muted-foreground hover:text-destructive" />
                </button>
              </div>
            {/each}
          </div>
        {:else if !isLoading}
          <div class="text-xs text-muted-foreground text-center py-2">
            No torrent files in watch folder
          </div>
        {/if}

        <!-- Refresh Button -->
        <Button
          onclick={loadWatchData}
          disabled={isLoading}
          size="sm"
          variant="outline"
          class="w-full gap-2"
        >
          {#snippet children()}
            <RefreshCw size={12} class={cn('transition-transform', isLoading && 'animate-spin')} />
            {isLoading ? 'Loading...' : 'Refresh'}
          {/snippet}
        </Button>
      </div>
    {/if}
  </div>
{/if}
