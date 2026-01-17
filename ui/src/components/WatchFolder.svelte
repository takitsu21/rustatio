<script>
  import { api, getRunMode } from '$lib/api.js';
  import { cn } from '$lib/utils.js';
  import Button from '$lib/components/ui/button.svelte';

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

  // Get status icon
  function getStatusIcon(status) {
    switch (status) {
      case 'loaded':
        return 'M5 13l4 4L19 7'; // Checkmark
      case 'pending':
        return 'M12 8v4l3 3'; // Clock
      case 'duplicate':
        return 'M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z'; // Copy
      case 'invalid':
        return 'M6 18L18 6M6 6l12 12'; // X
      default:
        return '';
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
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        class="flex-shrink-0"
      >
        <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
        ></path>
      </svg>

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
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          class={cn('transition-transform duration-200', isExpanded && 'rotate-180')}
        >
          <polyline points="6 9 12 15 18 9"></polyline>
        </svg>
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
                <svg
                  width="12"
                  height="12"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  class={cn('flex-shrink-0', getStatusColor(file.status))}
                >
                  <path d={getStatusIcon(file.status)}></path>
                </svg>

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
                  <svg
                    width="12"
                    height="12"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    class="text-muted-foreground hover:text-destructive"
                  >
                    <polyline points="3 6 5 6 21 6"></polyline>
                    <path
                      d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                    ></path>
                  </svg>
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
            <svg
              width="12"
              height="12"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              class={cn('transition-transform', isLoading && 'animate-spin')}
            >
              <path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"></path>
              <path d="M21 3v5h-5"></path>
            </svg>
            {isLoading ? 'Loading...' : 'Refresh'}
          {/snippet}
        </Button>
      </div>
    {/if}
  </div>
{/if}
