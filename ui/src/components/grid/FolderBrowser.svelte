<script>
  import Button from '$lib/components/ui/button.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import { api } from '$lib/api.js';
  import { Folder, FileText, ArrowUp, ChevronRight, X } from '@lucide/svelte';

  let { isOpen = $bindable(false), onSelect } = $props();

  let currentPath = $state('/');
  let pathInput = $state('/');
  let entries = $state([]);
  let parentPath = $state(null);
  let loading = $state(false);
  let error = $state('');
  let torrentCount = $derived(entries.filter(e => !e.is_dir).length);

  $effect(() => {
    if (isOpen) {
      browse(currentPath);
    }
  });

  async function browse(path) {
    loading = true;
    error = '';
    try {
      const result = await api.browseFolders(path);
      currentPath = result.path;
      pathInput = result.path;
      parentPath = result.parent || null;
      entries = result.entries || [];
    } catch (e) {
      error = e.message || 'Failed to browse directory';
    } finally {
      loading = false;
    }
  }

  function navigateTo(path) {
    browse(path);
  }

  function goUp() {
    if (parentPath) {
      browse(parentPath);
    }
  }

  function handlePathSubmit(e) {
    e.preventDefault();
    if (pathInput.trim()) {
      browse(pathInput.trim());
    }
  }

  function selectFolder() {
    onSelect?.(currentPath);
    isOpen = false;
  }

  function handleBackdropClick(event) {
    if (event.target === event.currentTarget) {
      close();
    }
  }

  function close() {
    isOpen = false;
  }

  function formatSize(bytes) {
    if (bytes == null) return '';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }
</script>

{#if isOpen}
  <div
    class="fixed inset-0 bg-black/50 z-[60] flex items-center justify-center p-4"
    onclick={handleBackdropClick}
    onkeydown={e => e.key === 'Escape' && close()}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
  >
    <div class="bg-card border border-border rounded-xl shadow-2xl w-full max-w-lg max-h-[70vh] flex flex-col">
      <!-- Header -->
      <div class="flex items-center justify-between p-3 border-b border-border">
        <h3 class="text-sm font-semibold text-foreground">Browse Server Folders</h3>
        <button
          onclick={close}
          class="p-1 rounded-md hover:bg-muted bg-transparent border-0 cursor-pointer"
        >
          <X size={16} class="text-muted-foreground" />
        </button>
      </div>

      <!-- Path bar -->
      <form onsubmit={handlePathSubmit} class="flex gap-2 p-3 border-b border-border">
        <button
          type="button"
          onclick={goUp}
          disabled={!parentPath}
          class="p-1.5 rounded-md border border-input bg-background hover:bg-muted disabled:opacity-40 cursor-pointer disabled:cursor-default"
          title="Go to parent directory"
        >
          <ArrowUp size={14} />
        </button>
        <Input
          bind:value={pathInput}
          placeholder="/path/to/directory"
          class="flex-1 text-xs h-8"
        />
        <Button size="sm" type="submit" variant="secondary">
          {#snippet children()}Go{/snippet}
        </Button>
      </form>

      <!-- Entries list -->
      <div class="flex-1 overflow-y-auto min-h-0">
        {#if loading}
          <div class="flex items-center justify-center p-8 text-muted-foreground text-sm">
            Loading...
          </div>
        {:else if error}
          <div class="p-4 text-destructive text-sm">{error}</div>
        {:else if entries.length === 0}
          <div class="flex items-center justify-center p-8 text-muted-foreground text-sm">
            Empty directory
          </div>
        {:else}
          <div class="divide-y divide-border">
            {#each entries as entry (entry.path)}
              {#if entry.is_dir}
                <button
                  class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-muted text-left bg-transparent border-0 cursor-pointer text-sm"
                  ondblclick={() => navigateTo(entry.path)}
                  onclick={() => navigateTo(entry.path)}
                >
                  <Folder size={14} class="text-primary shrink-0" />
                  <span class="truncate text-foreground">{entry.name}</span>
                  <ChevronRight size={12} class="text-muted-foreground ml-auto shrink-0" />
                </button>
              {:else}
                <div class="w-full flex items-center gap-2 px-3 py-1.5 text-sm opacity-60">
                  <FileText size={14} class="text-muted-foreground shrink-0" />
                  <span class="truncate text-muted-foreground">{entry.name}</span>
                  {#if entry.size != null}
                    <span class="text-xs text-muted-foreground ml-auto shrink-0">{formatSize(entry.size)}</span>
                  {/if}
                </div>
              {/if}
            {/each}
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-between p-3 border-t border-border">
        <span class="text-xs text-muted-foreground">
          {torrentCount > 0 ? `${torrentCount} .torrent file(s) found` : 'No .torrent files in this directory'}
        </span>
        <div class="flex gap-2">
          <Button onclick={close} size="sm" variant="secondary">
            {#snippet children()}Cancel{/snippet}
          </Button>
          <Button onclick={selectFolder} size="sm">
            {#snippet children()}Select Folder{/snippet}
          </Button>
        </div>
      </div>
    </div>
  </div>
{/if}
