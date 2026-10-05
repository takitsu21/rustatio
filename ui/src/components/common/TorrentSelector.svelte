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

<Card>
  <div class="flex h-8 items-center gap-2 border-b border-border px-2.5">
    <FileText size={13} class="text-muted-foreground" />
    <span
      class="flex-1 text-[0.625rem] font-semibold uppercase tracking-[0.16em] text-muted-foreground"
    >
      Torrent
    </span>
    {#if torrent}
      <Button
        onclick={handleFileSelect}
        variant="outline"
        size="sm"
        class="h-6 px-2 text-[0.625rem]"
      >
        {#snippet children()}
          <FolderOpen size={11} /> Change
        {/snippet}
      </Button>
    {/if}
  </div>

  <input
    type="file"
    accept=".torrent"
    bind:this={fileInput}
    onchange={handleFileChange}
    class="hidden"
  />

  {#if torrent}
    <!-- Torrent loaded state -->
    <div class="p-2.5">
      <div class="flex items-center gap-2.5">
        <div
          class="flex h-8 w-8 flex-shrink-0 items-center justify-center border border-border bg-muted"
        >
          <File size={14} class="text-muted-foreground" />
        </div>
        <div class="min-w-0 flex-1">
          <div class="truncate text-xs font-medium text-foreground" title={torrent.name}>
            {torrent.name}
          </div>
          <div class="mt-0.5 flex items-center gap-2 text-[0.625rem] text-muted-foreground">
            <span class="tabular-nums">{formatBytes(torrent.total_size)}</span>
            <span>·</span>
            <span>
              {torrent.file_count || torrent.files?.length || 1} file{(torrent.file_count ||
                torrent.files?.length ||
                1) > 1
                ? 's'
                : ''}
            </span>
            <span>·</span>
            <span>{trackers.length} tracker{trackers.length !== 1 ? 's' : ''}</span>
          </div>
        </div>
      </div>

      <!-- Quick stats -->
      <div class="mt-2.5 grid grid-cols-4 border border-border">
        <div class="border-r border-border p-1.5">
          <div class="text-[0.5625rem] uppercase tracking-wider text-muted-foreground">Size</div>
          <div class="mt-0.5 text-[0.6875rem] tabular-nums text-foreground">
            {formatBytes(torrent.total_size)}
          </div>
        </div>
        <div class="border-r border-border p-1.5">
          <div class="text-[0.5625rem] uppercase tracking-wider text-muted-foreground">Pieces</div>
          <div class="mt-0.5 text-[0.6875rem] tabular-nums text-foreground">
            {torrent.num_pieces?.toLocaleString() || 'N/A'}
          </div>
        </div>
        <div class="border-r border-border p-1.5">
          <div class="text-[0.5625rem] uppercase tracking-wider text-muted-foreground">
            Piece size
          </div>
          <div class="mt-0.5 text-[0.6875rem] tabular-nums text-foreground">
            {torrent.piece_length ? formatBytes(torrent.piece_length) : 'N/A'}
          </div>
        </div>
        <div class="p-1.5">
          <div class="text-[0.5625rem] uppercase tracking-wider text-muted-foreground">Files</div>
          <div class="mt-0.5 text-[0.6875rem] tabular-nums text-foreground">
            {torrent.file_count || torrent.files?.length || 1}
          </div>
        </div>
      </div>

      <!-- Details toggle -->
      <button
        class="mt-2 flex w-full items-center justify-center gap-1.5 border border-border py-1 text-[0.625rem] text-muted-foreground transition-colors hover:bg-muted hover:text-foreground cursor-pointer"
        onclick={() => (showDetails = !showDetails)}
      >
        {#if showDetails}
          <ChevronDown size={12} />
        {:else}
          <ChevronRight size={12} />
        {/if}
        {showDetails ? 'Hide' : 'Show'} details
      </button>

      <!-- Expanded details -->
      {#if showDetails}
        <div class="mt-2 flex flex-col gap-2.5 border border-border p-2">
          <!-- Info Hash -->
          <div>
            <div
              class="mb-1 flex items-center gap-1.5 text-[0.625rem] uppercase tracking-wider text-muted-foreground"
            >
              <Key size={11} /> Info hash
            </div>
            <code
              class="block break-all border border-border bg-muted px-1.5 py-1 text-[0.625rem] text-foreground"
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
              <div
                class="mb-1 flex items-center gap-1.5 text-[0.625rem] uppercase tracking-wider text-muted-foreground"
              >
                <Globe size={11} /> Trackers ({trackers.length})
              </div>
              <div class="flex max-h-[6.875rem] flex-col gap-1 overflow-y-auto">
                {#each trackers as tracker, index (tracker)}
                  <div class="flex items-center gap-2 text-[0.625rem]">
                    {#if index === 0}
                      <span
                        class="flex-shrink-0 border border-primary/40 bg-primary/10 px-1 py-px text-[0.5625rem] font-semibold uppercase text-primary"
                      >
                        Primary
                      </span>
                    {:else}
                      <span class="w-8 flex-shrink-0 text-right tabular-nums text-muted-foreground"
                        >#{index + 1}</span
                      >
                    {/if}
                    <code class="min-w-0 flex-1 break-all text-stat-upload">
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
              <div
                class="mb-1 flex items-center gap-1.5 text-[0.625rem] uppercase tracking-wider text-muted-foreground"
              >
                <Files size={11} /> Files ({torrent.files.length})
              </div>
              {#if torrent.files.length <= 10}
                <div class="flex max-h-[9.375rem] flex-col gap-1 overflow-y-auto">
                  {#each torrent.files as file (file.path)}
                    <div
                      class="flex items-center justify-between gap-2 bg-muted p-1 text-[0.625rem]"
                    >
                      <span class="min-w-0 flex-1 truncate">
                        {file.path?.join('/') || 'Unknown'}
                      </span>
                      <span class="flex-shrink-0 tabular-nums text-muted-foreground">
                        {formatBytes(file.length)}
                      </span>
                    </div>
                  {/each}
                </div>
              {:else}
                <div class="text-[0.625rem] italic text-muted-foreground">
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
    <div class="p-2.5">
      <button
        onclick={handleFileSelect}
        ondragover={handleDragOver}
        ondragleave={handleDragLeave}
        ondrop={handleDrop}
        class="flex w-full flex-col items-center gap-2 border border-dashed p-5 transition-colors cursor-pointer
          {isDragging ? 'border-primary bg-primary/10' : 'border-border hover:bg-muted/50'}"
      >
        <Upload size={18} class={isDragging ? 'text-primary' : 'text-muted-foreground'} />
        <span class="text-xs font-medium">
          {isDragging ? 'Drop torrent file here' : 'Select torrent file'}
        </span>
        <span class="text-[0.625rem] text-muted-foreground">
          {isDragging ? 'Release to load' : 'Click to browse or drag and drop'}
        </span>
      </button>
    </div>
  {/if}
</Card>
