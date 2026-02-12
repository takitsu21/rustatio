<script>
  import { onDestroy } from 'svelte';
  import { listenToInstanceEvents } from '$lib/api.js';
  import { filteredGridInstances, gridActions, viewMode } from '$lib/gridStore.js';
  import { instanceActions } from '$lib/instanceStore.js';
  import GridToolbar from './GridToolbar.svelte';
  import GridTable from './GridTable.svelte';
  import GridImportDialog from './GridImportDialog.svelte';

  let importDialogOpen = $state(false);

  // Debounce rapid instance events (e.g. during restoration) into a single fetch
  let debounceTimer = null;
  function debouncedFetch() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => gridActions.fetchSummaries(), 200);
  }

  // Start polling and SSE on mount
  gridActions.startPolling(3000);

  const cleanupEvents = listenToInstanceEvents(event => {
    if (event.type === 'created' || event.type === 'deleted' || event.type === 'state_changed') {
      debouncedFetch();
    }
  });

  async function handleContextAction(actionId, instance) {
    if (!instance) return;
    try {
      switch (actionId) {
        case 'start':
          await gridActions.startInstance(instance.id);
          break;
        case 'stop':
          await gridActions.stopInstance(instance.id);
          break;
        case 'pause':
          await gridActions.pauseInstance(instance.id);
          break;
        case 'resume':
          await gridActions.resumeInstance(instance.id);
          break;
        case 'edit': {
          const ensuredId = await instanceActions.ensureInstance(instance.id, instance);
          if (ensuredId) {
            instanceActions.selectInstance(ensuredId);
          }
          viewMode.set('standard');
          break;
        }
        case 'copy_hash':
          if (instance.infoHash) {
            await navigator.clipboard.writeText(instance.infoHash);
          }
          break;
        case 'delete':
          await gridActions.deleteInstance(instance.id);
          break;
      }
    } catch (error) {
      console.error(`Context action '${actionId}' failed:`, error);
    }
  }

  onDestroy(() => {
    gridActions.stopPolling();
    cleanupEvents();
    clearTimeout(debounceTimer);
  });
</script>

<div class="flex flex-col gap-3 h-full">
  <GridToolbar onImport={() => (importDialogOpen = true)} />

  {#if $filteredGridInstances.length === 0}
    <div class="flex-1 flex flex-col items-center justify-center gap-2 text-muted-foreground">
      <p class="text-sm">No instances found</p>
      <p class="text-xs">Import torrents or switch filters to see instances here.</p>
    </div>
  {:else}
    <GridTable data={$filteredGridInstances} oncontextaction={handleContextAction} />
  {/if}
</div>

<GridImportDialog bind:isOpen={importDialogOpen} />
