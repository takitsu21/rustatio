<script>
  import { onDestroy } from 'svelte';
  import { api, getRunMode, listenToInstanceEvents } from '$lib/api.js';
  import {
    filteredGridInstances,
    gridActions,
    gridFilters,
    selectedIds,
    viewMode,
  } from '$lib/grid/gridStore.js';
  import { instanceActions, instances } from '$lib/core/instanceStore.js';
  import { clearAllGridFilters } from '$lib/grid/gridFilters.js';
  import GridFiltersPanel from './GridFiltersPanel.svelte';
  import GridToolbar from './GridToolbar.svelte';
  import GridTable from './GridTable.svelte';
  import GridImportDialog from './GridImportDialog.svelte';
  import GridBulkEditDialog from './GridBulkEditDialog.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import { Funnel, X, Upload, Pencil } from '@lucide/svelte';

  let importDialogOpen = $state(false);
  let bulkEditDialogOpen = $state(false);
  let mobileFiltersOpen = $state(false);
  let networkStatus = $state(null);
  let networkStatusError = $state(null);
  let clientInfos = $state([]);

  // Filters rail visibility (desktop), persisted between sessions
  const FILTERS_VISIBLE_KEY = 'rustatio-grid-filters-visible';

  function loadFiltersVisible() {
    try {
      return localStorage.getItem(FILTERS_VISIBLE_KEY) !== 'false';
    } catch {
      return true;
    }
  }

  let filtersVisible = $state(loadFiltersVisible());

  function toggleFilters() {
    filtersVisible = !filtersVisible;
    try {
      localStorage.setItem(FILTERS_VISIBLE_KEY, String(filtersVisible));
    } catch {
      // localStorage unavailable
    }
  }

  let clients = $derived(clientInfos.map(c => ({ id: c.id, name: c.name })));
  let clientVersions = $derived(Object.fromEntries(clientInfos.map(c => [c.id, c.versions])));
  let selectedSummaries = $derived(
    $filteredGridInstances.filter(instance => $selectedIds.has(instance.id))
  );

  function isNetworkConfigured(status) {
    return status?.configured !== false;
  }

  function getVpnPortSyncEnabled(status) {
    return isNetworkConfigured(status) && (status?.vpn_port_sync_enabled ?? true);
  }

  let activeFiltersCount = $derived(
    ($gridFilters.stateFilter !== 'all' ? 1 : 0) +
      $gridFilters.tagFilter.length +
      $gridFilters.trackerFilter.length
  );

  async function refreshNetworkStatus() {
    networkStatusError = null;
    try {
      networkStatus = await api.getNetworkStatus();
    } catch (error) {
      networkStatus = null;
      networkStatusError = error.message || 'Failed to fetch';
    }
  }

  async function refreshClientInfos() {
    try {
      clientInfos = (await api.getClientInfos()) || [];
    } catch {
      clientInfos = [];
    }
  }

  // Debounce rapid instance events (e.g. during restoration) into a single fetch
  let debounceTimer = null;
  function debouncedFetch() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => gridActions.fetchSummaries(), 200);
  }

  // Start polling and SSE on mount. In server mode summaries arrive over SSE.
  if (getRunMode() === 'server') {
    gridActions.fetchSummaries();
  } else {
    gridActions.startPolling(3000);
  }
  refreshNetworkStatus();
  refreshClientInfos();

  const cleanupEvents = listenToInstanceEvents(event => {
    if (event.type === 'summaries') {
      gridActions.applySummaries(event.instances);
    } else if (
      event.type === 'created' ||
      event.type === 'deleted' ||
      event.type === 'state_changed'
    ) {
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

  function clearAllMobileFilters() {
    gridFilters.update(filters => clearAllGridFilters(filters));
  }

  function openMobileFilters() {
    mobileFiltersOpen = true;
  }

  function closeMobilePanels() {
    mobileFiltersOpen = false;
  }

  async function handleBulkApply(entries, mergedInstances = []) {
    const result = await api.bulkUpdateConfigs(entries);
    if (result?.failed?.length > 0) {
      const first = result.failed[0];
      throw new Error(first?.error || 'Failed to update selected instances.');
    }

    for (const instance of mergedInstances) {
      instanceActions.updateInstance(String(instance.id), instance);
    }

    // Hydrate all updated instances from one backend read instead of one call per instance.
    await instanceActions.ensureInstances(entries.map(entry => String(entry.id)));
    await gridActions.fetchSummaries();
  }

  function openBulkEdit() {
    bulkEditDialogOpen = true;
  }

  onDestroy(() => {
    gridActions.stopPolling();
    cleanupEvents();
    clearTimeout(debounceTimer);
  });
</script>

<div class="flex min-h-0 flex-1">
  <!-- Filters rail (desktop) -->
  {#if filtersVisible}
    <div class="hidden min-h-0 lg:flex">
      <GridFiltersPanel />
    </div>
  {/if}

  <!-- Workspace -->
  <div class="flex min-h-0 min-w-0 flex-1 flex-col pb-16 sm:pb-0">
    <GridToolbar
      onImport={() => (importDialogOpen = true)}
      onOpenFilters={openMobileFilters}
      onBulkEdit={openBulkEdit}
      {filtersVisible}
      onToggleFilters={toggleFilters}
    />

    {#if $filteredGridInstances.length === 0}
      <div class="flex flex-1 flex-col items-center justify-center gap-2 text-muted-foreground">
        <p class="text-xs">No instances found</p>
        <p class="text-[0.6875rem]">Import torrents or adjust filters to see instances here.</p>
      </div>
    {:else}
      <GridTable data={$filteredGridInstances} oncontextaction={handleContextAction} />
    {/if}
  </div>
</div>

{#if mobileFiltersOpen}
  <button
    class="fixed inset-0 z-40 bg-black/55 lg:hidden"
    onclick={closeMobilePanels}
    aria-label="Close filters panel"
  ></button>
{/if}

{#if mobileFiltersOpen}
  <div
    class="fixed inset-x-0 bottom-0 z-50 flex max-h-[82vh] flex-col border-t border-border bg-card lg:hidden"
  >
    <div class="mx-auto mt-1.5 h-1 w-10 rounded-full bg-muted-foreground/30"></div>
    <div class="flex items-center justify-between gap-3 border-b border-border px-3 py-2">
      <div class="text-xs font-semibold text-foreground">
        Filters
        {#if activeFiltersCount > 0}
          <span class="ml-1 text-primary">({activeFiltersCount})</span>
        {/if}
      </div>
      <div class="flex items-center gap-1">
        {#if activeFiltersCount > 0}
          <Button
            onclick={clearAllMobileFilters}
            variant="ghost"
            size="sm"
            class="h-7 px-2 text-[0.6875rem]"
          >
            {#snippet children()}Clear{/snippet}
          </Button>
        {/if}
        <Button
          onclick={() => (mobileFiltersOpen = false)}
          variant="ghost"
          size="icon"
          class="h-7 w-7"
          title="Close filters"
          aria-label="Close filters"
        >
          {#snippet children()}<X size={13} />{/snippet}
        </Button>
      </div>
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto overscroll-y-contain px-3 py-2 pb-6">
      <GridFiltersPanel mobile={true} showHeader={false} />
    </div>
  </div>
{/if}

<!-- Mobile action bar -->
<div class="fixed inset-x-3 bottom-10 z-30 sm:hidden">
  <div class="border border-border bg-card p-1">
    <div class="grid grid-cols-3 gap-1">
      <button
        class="flex w-full flex-col items-center justify-center gap-1 rounded-sm px-3 py-2 text-[0.625rem] text-muted-foreground transition-colors hover:bg-muted hover:text-foreground cursor-pointer"
        onclick={() => (importDialogOpen = true)}
      >
        <Upload size={15} />
        <span>Import</span>
      </button>

      <button
        class="flex w-full flex-col items-center justify-center gap-1 rounded-sm px-3 py-2 text-[0.625rem] text-muted-foreground transition-colors hover:bg-muted hover:text-foreground cursor-pointer disabled:pointer-events-none disabled:opacity-50"
        onclick={openBulkEdit}
        disabled={$selectedIds.size === 0}
      >
        <Pencil size={15} />
        <span>Edit</span>
      </button>

      <button
        class="relative flex w-full flex-col items-center justify-center gap-1 rounded-sm px-3 py-2 text-[0.625rem] text-muted-foreground transition-colors hover:bg-muted hover:text-foreground cursor-pointer"
        onclick={() => {
          mobileFiltersOpen = !mobileFiltersOpen;
        }}
      >
        <Funnel size={15} />
        <span>Filters</span>
        {#if activeFiltersCount > 0}
          <span
            class="absolute right-3 top-1 inline-flex h-4 min-w-4 items-center justify-center bg-primary px-1 text-[0.5625rem] font-semibold text-primary-foreground"
          >
            {activeFiltersCount}
          </span>
        {/if}
      </button>
    </div>
  </div>
</div>

<GridImportDialog
  bind:isOpen={importDialogOpen}
  vpnPortSyncVisible={true}
  currentForwardedPort={networkStatus?.forwarded_port ?? networkStatus?.forwardedPort ?? null}
  networkStatusConfigured={isNetworkConfigured(networkStatus)}
  vpnPortSyncEnabled={getVpnPortSyncEnabled(networkStatus)}
  {networkStatusError}
  onRefreshNetworkStatus={refreshNetworkStatus}
/>

<GridBulkEditDialog
  bind:isOpen={bulkEditDialogOpen}
  selectedIds={[...$selectedIds]}
  {selectedSummaries}
  fallbackInstances={$instances.filter(instance => $selectedIds.has(instance.id))}
  {clients}
  {clientVersions}
  currentForwardedPort={networkStatus?.forwarded_port ?? networkStatus?.forwardedPort ?? null}
  vpnPortSyncVisible={true}
  networkStatusConfigured={isNetworkConfigured(networkStatus)}
  vpnPortSyncEnabled={getVpnPortSyncEnabled(networkStatus)}
  {networkStatusError}
  isServerMode={getRunMode() === 'server'}
  onApply={handleBulkApply}
/>
