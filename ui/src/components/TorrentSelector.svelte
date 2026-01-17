<script>
  import Card from '$lib/components/ui/card.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import {
    FileText,
    FolderOpen,
    File,
    Key,
    Globe,
    Files,
    ChevronDown,
    ChevronRight,
    Upload,
  } from '@lucide/svelte';

  let { torrent, selectTorrent, formatBytes } = $props();

  let showDetails = $state(false);
  let isDragging = $state(false);
  let fileInput;

  // Check if running in Tauri
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  function getAllTrackers(torrent) {
    if (!torrent) return [];
    const trackers = new Set();
    if (torrent.announce) trackers.add(torrent.announce);
    if (torrent.announce_list && Array.isArray(torrent.announce_list)) {
      torrent.announce_list.forEach(tier => {
        if (Array.isArray(tier)) {
          tier.forEach(url => trackers.add(url));
        }
      });
    }
    return Array.from(trackers);
  }

  async function handleFileSelect() {
    if (isTauri) {
      // Use Tauri file dialog
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Torrent', extensions: ['torrent'] }],
      });
      if (selected) {
        await selectTorrent(selected);
      }
    } else {
      // Use HTML5 file input
      fileInput.click();
    }
  }

  async function handleFileChange(event) {
    const file = event.target.files?.[0];
    if (file) {
      await selectTorrent(file);
    }
  }

  function handleDragOver(event) {
    event.preventDefault();
    event.stopPropagation();
    isDragging = true;
  }

  function handleDragLeave(event) {
    event.preventDefault();
    event.stopPropagation();
    isDragging = false;
  }

  async function handleDrop(event) {
    event.preventDefault();
    event.stopPropagation();
    isDragging = false;

    const files = event.dataTransfer?.files;
    if (files && files.length > 0) {
      const file = files[0];
      // Check if it's a .torrent file
      if (file.name.endsWith('.torrent') || file.type === 'application/x-bittorrent') {
        await selectTorrent(file);
      } else {
        alert('Please drop a .torrent file');
      }
    }
  }

  let trackers = $derived(getAllTrackers(torrent));
</script>

<Card class="p-3">
  <h2 class="mb-3 text-primary text-lg font-semibold flex items-center gap-2">
    <FileText size={20} /> Torrent File
  </h2>

  <input
    type="file"
    accept=".torrent"
    bind:this={fileInput}
    onchange={handleFileChange}
    class="hidden"
  />

  {#if torrent}
    <!-- Torrent loaded state -->
    <div class="bg-muted/50 rounded-lg border border-border overflow-hidden">
      <!-- Main info row -->
      <div class="p-3 flex items-center gap-3">
        <div
          class="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center flex-shrink-0"
        >
          <File size={20} class="text-primary" />
        </div>
        <div class="flex-1 min-w-0">
          <div class="font-medium text-sm truncate" title={torrent.name}>
            {torrent.name}
          </div>
          <div class="text-xs text-muted-foreground flex items-center gap-2 mt-0.5">
            <span>{formatBytes(torrent.total_size)}</span>
            <span class="text-border">•</span>
            <span
              >{torrent.files?.length || 1} file{(torrent.files?.length || 1) > 1 ? 's' : ''}</span
            >
            <span class="text-border">•</span>
            <span>{trackers.length} tracker{trackers.length !== 1 ? 's' : ''}</span>
          </div>
        </div>
        <Button onclick={handleFileSelect} variant="outline" class="h-8 px-3 text-xs">
          {#snippet children()}
            <span class="flex items-center gap-1.5">
              <FolderOpen size={14} /> Change
            </span>
          {/snippet}
        </Button>
      </div>

      <!-- Quick stats -->
      <div class="grid grid-cols-4 border-t border-border">
        <div class="p-2 text-center border-r border-border">
          <div class="text-xs text-muted-foreground mb-0.5">Size</div>
          <div class="text-sm font-medium">{formatBytes(torrent.total_size)}</div>
        </div>
        <div class="p-2 text-center border-r border-border">
          <div class="text-xs text-muted-foreground mb-0.5">Pieces</div>
          <div class="text-sm font-medium">{torrent.num_pieces?.toLocaleString() || 'N/A'}</div>
        </div>
        <div class="p-2 text-center border-r border-border">
          <div class="text-xs text-muted-foreground mb-0.5">Piece Size</div>
          <div class="text-sm font-medium">
            {torrent.piece_length ? formatBytes(torrent.piece_length) : 'N/A'}
          </div>
        </div>
        <div class="p-2 text-center">
          <div class="text-xs text-muted-foreground mb-0.5">Files</div>
          <div class="text-sm font-medium">{torrent.files?.length || 1}</div>
        </div>
      </div>

      <!-- Details toggle -->
      <button
        class="w-full p-2 flex items-center justify-center gap-2 text-xs text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors border-t border-border cursor-pointer bg-transparent"
        onclick={() => (showDetails = !showDetails)}
      >
        {#if showDetails}
          <ChevronDown size={14} />
        {:else}
          <ChevronRight size={14} />
        {/if}
        {showDetails ? 'Hide' : 'Show'} Details
      </button>

      <!-- Expanded details -->
      {#if showDetails}
        <div class="border-t border-border p-3 flex flex-col gap-3 bg-background/50">
          <!-- Info Hash -->
          <div>
            <div class="text-xs text-muted-foreground mb-1.5 flex items-center gap-1.5">
              <Key size={12} /> Info Hash
            </div>
            <code
              class="bg-muted text-primary px-2 py-1.5 rounded text-xs break-all font-mono block"
            >
              {torrent.info_hash
                ? Array.from(torrent.info_hash)
                    .map(b => b.toString(16).padStart(2, '0'))
                    .join('')
                : 'N/A'}
            </code>
          </div>

          <!-- Trackers -->
          {#if trackers.length > 0}
            <div>
              <div class="text-xs text-muted-foreground mb-1.5 flex items-center gap-1.5">
                <Globe size={12} /> Trackers ({trackers.length})
              </div>
              <div class="flex flex-col gap-1 max-h-[100px] overflow-y-auto">
                {#each trackers as tracker, index (tracker)}
                  <div class="flex items-center gap-2 text-xs">
                    {#if index === 0}
                      <span
                        class="px-1.5 py-0.5 rounded text-[0.6rem] font-medium uppercase bg-primary text-primary-foreground flex-shrink-0"
                      >
                        Primary
                      </span>
                    {:else}
                      <span class="text-muted-foreground w-12 flex-shrink-0 text-right"
                        >#{index + 1}</span
                      >
                    {/if}
                    <code class="text-stat-upload break-all font-mono flex-1 min-w-0">
                      {tracker}
                    </code>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <!-- File List -->
          {#if torrent.files && torrent.files.length > 0}
            <div>
              <div class="text-xs text-muted-foreground mb-1.5 flex items-center gap-1.5">
                <Files size={12} /> Files ({torrent.files.length})
              </div>
              {#if torrent.files.length <= 10}
                <div class="flex flex-col gap-1 max-h-[150px] overflow-y-auto">
                  {#each torrent.files as file (file.path)}
                    <div
                      class="flex items-center justify-between gap-2 p-1.5 bg-muted rounded text-xs"
                    >
                      <span class="font-mono truncate flex-1 min-w-0">
                        {file.path?.join('/') || 'Unknown'}
                      </span>
                      <span class="text-muted-foreground flex-shrink-0">
                        {formatBytes(file.length)}
                      </span>
                    </div>
                  {/each}
                </div>
              {:else}
                <div class="text-xs text-muted-foreground italic">
                  {torrent.files.length} files (too many to display)
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {:else}
    <!-- Empty state with drag and drop -->
    <button
      onclick={handleFileSelect}
      ondragover={handleDragOver}
      ondragleave={handleDragLeave}
      ondrop={handleDrop}
      class="w-full p-6 border-2 border-dashed rounded-lg flex flex-col items-center gap-3 cursor-pointer transition-all group
        {isDragging
        ? 'border-primary bg-primary/10'
        : 'border-border bg-muted/30 hover:bg-muted/50 hover:border-primary/50'}"
    >
      <div
        class="w-12 h-12 rounded-full flex items-center justify-center transition-colors
        {isDragging ? 'bg-primary/20' : 'bg-muted group-hover:bg-primary/10'}"
      >
        <Upload
          size={24}
          class="transition-colors {isDragging
            ? 'text-primary'
            : 'text-muted-foreground group-hover:text-primary'}"
        />
      </div>
      <div class="text-center">
        <div class="font-medium text-sm mb-1">
          {isDragging ? 'Drop torrent file here' : 'Select Torrent File'}
        </div>
        <div class="text-xs text-muted-foreground">
          {isDragging ? 'Release to load' : 'Click to browse or drag and drop'}
        </div>
      </div>
    </button>
  {/if}
</Card>
