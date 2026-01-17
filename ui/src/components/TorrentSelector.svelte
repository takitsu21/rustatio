<script>
  import Card from '$lib/components/ui/card.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import {
    FileText,
    FolderOpen,
    File,
    HardDrive,
    Puzzle,
    Package,
    Key,
    Globe,
    Files,
  } from '@lucide/svelte';

  let { torrent, selectTorrent, formatBytes } = $props();

  let showDetails = $state(false);
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

  let trackers = $derived(getAllTrackers(torrent));
</script>

<Card class="p-3">
  <h2 class="mb-3 text-primary text-lg font-semibold flex items-center gap-2">
    <FileText size={20} /> Torrent File
  </h2>
  <div class="flex flex-col gap-3.5">
    <input
      type="file"
      accept=".torrent"
      bind:this={fileInput}
      onchange={handleFileChange}
      class="hidden"
    />
    <Button onclick={handleFileSelect} class="w-full">
      {#snippet children()}
        <span class="flex items-center gap-2">
          {#if torrent}
            <File size={16} /> Change File
          {:else}
            <FolderOpen size={16} /> Select Torrent File
          {/if}
        </span>
      {/snippet}
    </Button>
    {#if torrent}
      <div
        class="bg-gradient-to-r from-primary/10 to-primary/5 dark:from-primary/20 dark:to-primary/10 p-4 rounded-lg border-l-4 border-primary border border-primary/30 dark:border-primary/50"
      >
        <div class="flex gap-2 mb-2 text-sm">
          <span class="font-bold text-primary min-w-[45px]">Name:</span>
          <span
            class="font-semibold text-foreground overflow-hidden text-ellipsis whitespace-nowrap"
            title={torrent.name}>{torrent.name}</span
          >
        </div>
        <div class="flex gap-2 mb-3 text-sm">
          <span class="font-bold text-primary min-w-[45px]">Size:</span>
          <span class="font-semibold text-foreground">{formatBytes(torrent.total_size)}</span>
        </div>
        <button
          class="bg-transparent border border-border text-muted-foreground px-3 py-1.5 rounded-md text-sm font-semibold cursor-pointer transition-all w-full mt-2 hover:bg-background hover:text-foreground hover:border-primary"
          onclick={() => (showDetails = !showDetails)}
        >
          {showDetails ? '▼' : '▶'}
          {showDetails ? 'Hide' : 'Show'} Details
        </button>
      </div>

      {#if showDetails}
        <div
          class="bg-gradient-to-br from-muted to-secondary/50 p-4 rounded-lg border border-primary/20 flex flex-col gap-4 animate-in slide-in-from-top-2 duration-200"
        >
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
            <div class="flex flex-col gap-1.5">
              <strong class="text-foreground text-sm font-semibold flex items-center gap-1.5"
                ><HardDrive size={14} /> Total Size:</strong
              >
              <span class="text-muted-foreground text-sm">{formatBytes(torrent.total_size)}</span>
            </div>
            <div class="flex flex-col gap-1.5">
              <strong class="text-foreground text-sm font-semibold flex items-center gap-1.5"
                ><Puzzle size={14} /> Pieces:</strong
              >
              <span class="text-muted-foreground text-sm"
                >{torrent.num_pieces?.toLocaleString() || 'N/A'}</span
              >
            </div>
            <div class="flex flex-col gap-1.5">
              <strong class="text-foreground text-sm font-semibold flex items-center gap-1.5"
                ><Package size={14} /> Piece Size:</strong
              >
              <span class="text-muted-foreground text-sm"
                >{torrent.piece_length ? formatBytes(torrent.piece_length) : 'N/A'}</span
              >
            </div>
            <div class="flex flex-col gap-1.5">
              <strong class="text-foreground text-sm font-semibold flex items-center gap-1.5"
                ><File size={14} /> Files:</strong
              >
              <span class="text-muted-foreground text-sm"
                >{torrent.files?.length || 1} file{(torrent.files?.length || 1) > 1
                  ? 's'
                  : ''}</span
              >
            </div>
          </div>

          <div class="flex flex-col gap-1.5 col-span-full">
            <strong class="text-foreground text-sm font-semibold flex items-center gap-1.5"
              ><Key size={14} /> Info Hash:</strong
            >
            <code
              class="bg-background text-primary px-2.5 py-1.5 rounded text-xs break-all font-mono block"
            >
              {torrent.info_hash
                ? Array.from(torrent.info_hash)
                    .map(b => b.toString(16).padStart(2, '0'))
                    .join('')
                : 'N/A'}
            </code>
          </div>

          {#if trackers.length > 0}
            <div class="flex flex-col gap-1.5 col-span-full">
              <strong class="text-foreground text-sm font-semibold flex items-center gap-1.5"
                ><Globe size={14} /> Tracker{trackers.length > 1 ? 's' : ''} ({trackers.length}):</strong
              >
              <div class="flex flex-col gap-1 mt-1 max-h-[120px] overflow-y-auto pr-1">
                {#each trackers as tracker, index (tracker)}
                  <div class="flex items-center gap-2">
                    {#if index === 0}
                      <span
                        class="inline-block px-1.5 py-0.5 rounded text-[0.65rem] font-semibold uppercase bg-primary text-primary-foreground flex-shrink-0"
                        >Primary</span
                      >
                    {:else}
                      <span class="text-xs text-muted-foreground w-10 flex-shrink-0"
                        >#{index + 1}</span
                      >
                    {/if}
                    <code
                      class="bg-background text-green-600 px-2 py-1 rounded text-[0.7rem] break-all font-mono flex-1 min-w-0"
                      >{tracker}</code
                    >
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          {#if torrent.files && torrent.files.length > 0 && torrent.files.length <= 10}
            <div class="flex flex-col gap-1.5 col-span-full">
              <strong class="text-foreground text-sm font-semibold flex items-center gap-1.5"
                ><Files size={14} /> File List:</strong
              >
              <div class="flex flex-col gap-2 mt-2 max-h-[300px] overflow-y-auto pr-2">
                {#each torrent.files as file (file.path)}
                  <div
                    class="flex justify-between items-center gap-4 p-2 bg-background rounded-md border border-border transition-all hover:border-primary hover:bg-secondary"
                  >
                    <span
                      class="text-foreground text-xs font-mono overflow-hidden text-ellipsis whitespace-nowrap flex-1"
                      >{file.path?.join('/') || 'Unknown'}</span
                    >
                    <span
                      class="text-muted-foreground text-xs font-semibold whitespace-nowrap px-2 py-1 bg-muted rounded"
                      >{formatBytes(file.length)}</span
                    >
                  </div>
                {/each}
              </div>
            </div>
          {:else if torrent.files && torrent.files.length > 10}
            <div class="flex flex-col gap-1.5 col-span-full">
              <strong class="text-foreground text-sm font-semibold flex items-center gap-1.5"
                ><Files size={14} /> Files:</strong
              >
              <span class="text-muted-foreground italic text-sm"
                >{torrent.files.length} files (too many to display)</span
              >
            </div>
          {/if}
        </div>
      {/if}
    {:else}
      <p class="text-muted-foreground italic text-center p-4">No torrent file selected</p>
    {/if}
  </div>
</Card>
