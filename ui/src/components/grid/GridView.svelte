<script>
  import { onDestroy } from 'svelte';
  import { api, listenToInstanceEvents } from '$lib/api.js';
  import { filteredGridInstances, gridActions, gridFilters, viewMode } from '$lib/gridStore.js';
  import { instanceActions } from '$lib/instanceStore.js';
  import { clearAllGridFilters } from '$lib/gridFilters.js';
  import GridFiltersPanel from './GridFiltersPanel.svelte';
  import GridToolbar from './GridToolbar.svelte';
  import GridTable from './GridTable.svelte';
  import GridImportDialog from './GridImportDialog.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import { Funnel, X, Upload } from '@lucide/svelte';

  let importDialogOpen = $state(false);
  let mobileFiltersOpen = $state(false);
  let networkStatus = $state(null);
  let networkStatusError = $state(null);

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

  // Debounce rapid instance events (e.g. during restoration) into a single fetch
  let debounceTimer = null;
  function debouncedFetch() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => gridActions.fetchSummaries(), 200);
  }

  // Start polling and SSE on mount
  gridActions.startPolling(3000);
  refreshNetworkStatus();

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

  function clearAllMobileFilters() {
    gridFilters.update(filters => clearAllGridFilters(filters));
  }

  function openMobileFilters() {
    mobileFiltersOpen = true;
  }

  function closeMobilePanels() {
    mobileFiltersOpen = false;
  }

  onDestroy(() => {
    gridActions.stopPolling();
    cleanupEvents();
    clearTimeout(debounceTimer);
  });
</script>

<div class="flex h-full min-h-0 flex-col gap-3 lg:flex-row">
  <div class="hidden lg:block">
    <GridFiltersPanel />
  </div>

  <div class="flex min-h-0 flex-1 flex-col gap-3 pb-20 lg:pb-0">
    <GridToolbar onImport={() => (importDialogOpen = true)} onOpenFilters={openMobileFilters} />

    {#if $filteredGridInstances.length === 0}
      <div
        class="flex-1 flex flex-col items-center justify-center gap-2 rounded-xl border border-border bg-card/40 text-muted-foreground"
      >
        <p class="text-sm">No instances found</p>
        <p class="text-xs">Import torrents or adjust filters to see instances here.</p>
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
    aria-label="Close grid mobile panel"
  ></button>
{/if}

{#if mobileFiltersOpen}
  <div
    class="fixed inset-x-0 bottom-0 z-50 max-h-[82vh] rounded-t-3xl border-t border-border bg-card shadow-2xl lg:hidden"
  >
    <div class="mx-auto mt-2 h-1.5 w-12 rounded-full bg-muted-foreground/30"></div>
    <div class="flex items-center justify-between gap-3 px-4 py-3 border-b border-border/70">
      <div>
        <div class="text-base font-semibold text-foreground">Filters</div>
        <div class="text-[11px] text-muted-foreground">
          {activeFiltersCount > 0
            ? `${activeFiltersCount} active filter${activeFiltersCount > 1 ? 's' : ''}`
            : 'Browse instances by facet'}
        </div>
      </div>
      <div class="flex items-center gap-1">
        {#if activeFiltersCount > 0}
          <Button
            onclick={clearAllMobileFilters}
            variant="ghost"
            size="sm"
            class="h-8 px-2 text-xs"
          >
            {#snippet children()}Clear{/snippet}
          </Button>
        {/if}
        <Button
          onclick={() => (mobileFiltersOpen = false)}
          variant="ghost"
          size="icon"
          class="h-8 w-8"
        >
          {#snippet children()}<X size={14} />{/snippet}
        </Button>
      </div>
    </div>

    <div class="overflow-y-auto px-4 py-3 pb-6">
      <GridFiltersPanel mobile={true} showHeader={false} />
    </div>
  </div>
{/if}

<div class="fixed inset-x-3 bottom-3 z-30 sm:hidden">
  <div class="rounded-2xl border border-border bg-card/95 p-1.5 shadow-2xl backdrop-blur-xl">
    <div class="grid grid-cols-2 gap-1.5">
      <button
        class="flex w-full flex-col items-center justify-center gap-1 rounded-xl px-3 py-2 text-[11px] text-muted-foreground transition-colors hover:bg-muted hover:text-foreground cursor-pointer"
        onclick={() => (importDialogOpen = true)}
      >
        <Upload size={16} />
        <span>Import</span>
      </button>

      <button
        class="relative flex w-full flex-col items-center justify-center gap-1 rounded-xl px-3 py-2 text-[11px] text-muted-foreground transition-colors hover:bg-muted hover:text-foreground cursor-pointer"
        onclick={() => {
          mobileFiltersOpen = !mobileFiltersOpen;
        }}
      >
        <Funnel size={16} />
        <span>Filters</span>
        {#if activeFiltersCount > 0}
          <span
            class="absolute right-3 top-1 inline-flex h-4 min-w-4 items-center justify-center rounded-full bg-primary px-1 text-[10px] font-semibold text-primary-foreground"
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
  vpnPortSyncEnabled={networkStatus?.vpn_port_sync_enabled ?? true}
  {networkStatusError}
  onRefreshNetworkStatus={refreshNetworkStatus}
/>
