<script>
  import { onDestroy, onMount, untrack } from 'svelte';
  import { api, getRunMode } from '$lib/api.js';
  import { getTrackerIssue, getInstanceState } from '$lib/core/status.js';
  import { instances, activeInstanceId, instanceActions } from '$lib/core/instanceStore.js';
  import { viewMode } from '$lib/grid/gridStore.js';
  import { getStateMeta, getTone } from '$lib/core/statusMeta.js';
  import { getWindow, getScrollTopForIndex } from '$lib/core/virtual.js';
  import { getZoom } from '$lib/core/zoomStore.svelte.js';
  import { cn } from '$lib/core/utils.js';
  import Button from '$lib/components/ui/button.svelte';
  import ConfirmDialog from '../common/ConfirmDialog.svelte';
  import {
    PanelLeftClose,
    Play,
    Square,
    Plus,
    Pause,
    Search,
    X,
    FolderOpen,
    FolderSearch,
    Settings,
    Github,
    LayoutGrid,
    AlertTriangle,
  } from '@lucide/svelte';

  /* global __APP_VERSION__ */
  let appVersion = $state(__APP_VERSION__);
  const repository = 'https://github.com/takitsu21/rustatio';
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  let {
    onStartAll = () => {},
    onStopAll = () => {},
    onPauseAll = () => {},
    onResumeAll = () => {},
    onOpenSettings = () => {},
    isOpen = $bindable(false),
    isCollapsed = $bindable(false),
  } = $props();

  let forceDeleteConfirmVisible = $state(false);
  let forceDeleteTarget = $state(null);
  let forceDeleteBusy = $state(false);
  let isGridMode = $derived($viewMode === 'grid');
  // When opened via mobile overlay (isOpen=true), always treat as expanded
  let isCompact = $derived(isOpen ? false : isCollapsed);
  let isWatchMode = $derived($viewMode === 'watch');
  let isWatchRuntime = $derived(getRunMode() === 'server' || getRunMode() === 'desktop');
  let watchCount = $state(null);
  let watchLoadedCount = $state(null);
  let watchInterval = null;

  // Instance list virtualization + search
  const LIST_BUFFER_ROWS = 8;
  let search = $state('');
  let listEl = $state(null);
  let listScrollTop = $state(0);
  let listViewportH = $state(0);

  let rootFontPx = $derived.by(() => {
    getZoom();
    if (typeof document === 'undefined') return 16;
    const value = parseFloat(getComputedStyle(document.documentElement).fontSize);
    return Number.isFinite(value) ? value : 16;
  });
  let rowHeight = $derived(1.75 * rootFontPx);

  let filteredInstances = $derived.by(() => {
    const query = search.trim().toLowerCase();
    if (!query) return $instances;
    return $instances.filter(instance => getInstanceLabel(instance).toLowerCase().includes(query));
  });

  let view = $derived(
    getWindow({
      total: filteredInstances.length,
      scrollTop: listScrollTop,
      viewportHeight: listViewportH,
      rowHeight,
      buffer: LIST_BUFFER_ROWS,
    })
  );
  let visibleInstances = $derived(filteredInstances.slice(view.startIndex, view.endIndex));
  let bottomSpacer = $derived(
    Math.max(0, view.totalHeight - view.offsetY - visibleInstances.length * rowHeight)
  );

  function handleListScroll(event) {
    listScrollTop = event.target.scrollTop;
  }

  let lastActiveId = null;

  // Selection changes scroll the active instance into view and clear a filter
  // that would hide it (e.g. a freshly created instance).
  // Deliberately untracked beyond the active id: data updates must never move the list.
  $effect(() => {
    const activeId = $activeInstanceId;
    if (activeId === lastActiveId) return;
    lastActiveId = activeId;
    if (activeId == null) return;

    untrack(() => {
      if (search && !filteredInstances.some(inst => inst.id === activeId)) {
        search = '';
      }
      scrollInstanceIntoView(activeId);
    });
  });

  function scrollInstanceIntoView(id) {
    if (!listEl) return;
    const index = filteredInstances.findIndex(inst => inst.id === id);
    if (index < 0) return;

    const target = getScrollTopForIndex({
      index,
      currentScrollTop: listEl.scrollTop,
      viewportHeight: listEl.clientHeight,
      rowHeight,
    });

    if (target !== listEl.scrollTop) {
      listEl.scrollTop = target;
      listScrollTop = target;
    }
  }

  // The search box is unavailable while collapsed
  $effect(() => {
    if (isCompact && search) {
      search = '';
    }
  });

  let hasMultipleInstancesWithTorrents = $derived(
    $instances.filter(inst => inst.torrent).length > 1
  );

  let hasRunningInstances = $derived($instances.some(inst => inst.isRunning));

  let hasStoppedInstancesWithTorrents = $derived(
    $instances.some(inst => inst.torrent && !inst.isRunning)
  );

  let hasPausedInstances = $derived($instances.some(inst => inst.isRunning && inst.isPaused));

  let hasUnpausedRunningInstances = $derived(
    $instances.some(inst => inst.isRunning && !inst.isPaused)
  );

  async function loadWatchStatus() {
    if (!isWatchRuntime) return;
    try {
      const status = await api.getWatchStatus();
      watchCount = status?.file_count ?? 0;
      watchLoadedCount = status?.loaded_count ?? 0;
    } catch (error) {
      console.warn('Failed to load watch status:', error);
      watchCount = null;
      watchLoadedCount = null;
    }
  }

  onMount(() => {
    if (isWatchRuntime) {
      loadWatchStatus();
      watchInterval = setInterval(loadWatchStatus, 30000);
    }

    if (isTauri) {
      import('@tauri-apps/api/app')
        .then(mod => mod.getVersion())
        .then(version => {
          appVersion = version;
        })
        .catch(error => {
          console.error('Failed to get app version:', error);
        });
    }
  });

  onDestroy(() => {
    if (watchInterval) {
      clearInterval(watchInterval);
      watchInterval = null;
    }
  });

  function getInstanceLabel(instance) {
    if (instance.torrent) {
      return instance.torrent.name;
    }
    return `Instance ${instance.id}`;
  }

  function getIssueMessage(instance) {
    return getTrackerIssue(instance.stats)?.statusMessage || null;
  }

  function getRowTitle(instance) {
    return instance.torrent ? instance.torrent.name : `Instance ${instance.id}`;
  }

  async function handleAddInstance() {
    try {
      await instanceActions.addInstance();
    } catch (error) {
      console.error('Failed to add instance:', error);
    }
  }

  async function handleRemoveInstance(event, id) {
    event.stopPropagation();
    event.preventDefault();

    try {
      await instanceActions.removeInstance(id);
    } catch (error) {
      console.error('Failed to remove instance:', error);
    }
  }

  function handleForceRemoveInstance(event, id, name) {
    event.stopPropagation();
    event.preventDefault();

    forceDeleteTarget = {
      id,
      name: name || 'this instance',
    };
    forceDeleteConfirmVisible = true;
  }

  function cancelForceRemoveInstance() {
    if (forceDeleteBusy) return;
    forceDeleteConfirmVisible = false;
    forceDeleteTarget = null;
  }

  async function confirmForceRemoveInstance() {
    if (!forceDeleteTarget || forceDeleteBusy) return;
    forceDeleteBusy = true;

    try {
      await instanceActions.removeInstance(forceDeleteTarget.id, true); // force=true
    } catch (error) {
      console.error('Failed to force remove instance:', error);
    } finally {
      forceDeleteBusy = false;
      forceDeleteConfirmVisible = false;
      forceDeleteTarget = null;
    }
  }

  function handleSelectInstance(id) {
    try {
      instanceActions.selectInstance(id);
      viewMode.set('standard');
    } catch (error) {
      console.error('Error switching instance:', error);
    }
  }
</script>

<!-- Mobile Overlay -->
{#if isOpen}
  <button
    class="fixed inset-0 z-40 border-0 bg-black/50 p-0 lg:hidden cursor-default"
    onclick={() => (isOpen = false)}
    aria-label="Close sidebar"
  ></button>
{/if}

<aside
  class={cn(
    'flex h-dvh flex-col overflow-hidden border-r border-border bg-card transition-[width,transform] duration-200 ease-out lg:h-screen',
    'fixed top-0 z-50 lg:sticky lg:z-auto',
    // Mobile: slide in/out
    isOpen ? 'translate-x-0' : '-translate-x-full lg:translate-x-0',
    // Desktop: collapse/expand
    !isOpen && isCollapsed ? 'w-60 lg:w-12' : 'w-60 lg:w-52'
  )}
>
  <!-- Header -->
  <div class="flex h-9 shrink-0 items-center gap-1 border-b border-border px-1.5">
    <span
      class={cn(
        'flex-1 px-1 text-[0.625rem] font-semibold uppercase tracking-[0.16em] text-muted-foreground',
        isCompact && 'lg:hidden'
      )}
    >
      Rustatio
    </span>
    <button
      class="hidden h-6 w-6 items-center justify-center text-muted-foreground transition-colors hover:bg-muted hover:text-foreground lg:inline-flex cursor-pointer"
      onclick={() => (isCollapsed = !isCollapsed)}
      title={isCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
      aria-label={isCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
    >
      <span class={cn('block transition-transform duration-200', isCollapsed && 'rotate-180')}>
        <PanelLeftClose size={14} />
      </span>
    </button>
  </div>

  <!-- Navigation -->
  <nav class="shrink-0 space-y-0.5 border-b border-border p-1.5">
    <button
      class={cn(
        'relative flex w-full items-center gap-2 px-2 py-1.5 text-xs transition-colors cursor-pointer',
        isGridMode
          ? 'bg-muted text-foreground'
          : 'text-muted-foreground hover:bg-muted/60 hover:text-foreground',
        isCompact && 'lg:justify-center lg:px-0'
      )}
      onclick={() => viewMode.set('grid')}
      title="Instances"
    >
      <span
        class={cn(
          'absolute bottom-1 left-0 top-1 w-0.5',
          isGridMode ? 'bg-primary' : 'bg-transparent'
        )}
        aria-hidden="true"
      ></span>
      <LayoutGrid size={14} class="flex-shrink-0" />
      <span class={cn('flex-1 text-left', isCompact && 'lg:hidden')}>Instances</span>
      {#if $instances.length > 0}
        <span
          class={cn('tabular-nums text-[0.625rem] text-muted-foreground', isCompact && 'lg:hidden')}
          >{$instances.length}</span
        >
      {/if}
    </button>

    <button
      class={cn(
        'relative flex w-full items-center gap-2 px-2 py-1.5 text-xs transition-colors cursor-pointer',
        isWatchMode
          ? 'bg-muted text-foreground'
          : 'text-muted-foreground hover:bg-muted/60 hover:text-foreground',
        !isWatchRuntime && 'cursor-not-allowed opacity-50',
        isCompact && 'lg:justify-center lg:px-0'
      )}
      onclick={() => (isWatchRuntime ? viewMode.set('watch') : null)}
      title="Watch folder"
      disabled={!isWatchRuntime}
      aria-disabled={!isWatchRuntime}
    >
      <span
        class={cn(
          'absolute bottom-1 left-0 top-1 w-0.5',
          isWatchMode ? 'bg-primary' : 'bg-transparent'
        )}
        aria-hidden="true"
      ></span>
      <FolderSearch size={14} class="flex-shrink-0" />
      <span class={cn('flex-1 text-left', isCompact && 'lg:hidden')}>Watch</span>
      {#if isWatchRuntime && watchCount !== null && watchCount > 0}
        <span
          class={cn(
            'inline-flex h-4 min-w-4 items-center justify-center px-1 text-[0.625rem] font-semibold leading-none',
            (watchLoadedCount ?? 0) > 0
              ? 'bg-primary text-primary-foreground'
              : 'border border-stat-ratio/40 text-stat-ratio',
            isCompact && 'lg:hidden'
          )}
          title="{watchLoadedCount ?? 0} loaded · {watchCount} watch files"
        >
          {watchLoadedCount ?? 0}
        </span>
      {/if}
    </button>

    <button
      class={cn(
        'flex w-full items-center gap-2 px-2 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground cursor-pointer',
        isCompact && 'lg:justify-center lg:px-0'
      )}
      onclick={onOpenSettings}
      title="Settings"
    >
      <Settings size={14} class="flex-shrink-0" />
      <span class={cn('flex-1 text-left', isCompact && 'lg:hidden')}>Settings</span>
    </button>
  </nav>

  <!-- Instances -->
  <div class="shrink-0 border-b border-border">
    <div
      class={cn(
        'flex items-center justify-between gap-1 px-2.5 py-1.5',
        isCompact && 'lg:justify-center lg:px-0'
      )}
    >
      <span class={cn('flex items-center gap-2', isCompact && 'lg:hidden')}>
        <span
          class="text-[0.625rem] font-semibold uppercase tracking-[0.16em] text-muted-foreground"
          >Instances</span
        >
        {#if search.trim()}
          <span class="tabular-nums text-[0.625rem] text-muted-foreground">
            {filteredInstances.length} / {$instances.length}
          </span>
        {/if}
      </span>
      <button
        class="inline-flex h-5 w-5 items-center justify-center text-muted-foreground transition-colors hover:bg-muted hover:text-foreground cursor-pointer"
        onclick={handleAddInstance}
        title="New instance"
        aria-label="New instance"
      >
        <Plus size={13} strokeWidth={2.5} />
      </button>
    </div>

    {#if !isCompact}
      <div class="px-1.5 pb-1.5">
        <div class="relative">
          <Search
            size={12}
            class="absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground"
          />
          <input
            value={search}
            oninput={e => (search = e.target.value)}
            placeholder="Search instances…"
            class="h-7 w-full border border-input bg-background pl-6.5 pr-6 text-[0.6875rem] text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none"
          />
          {#if search}
            <button
              class="absolute right-1.5 top-1/2 -translate-y-1/2 text-muted-foreground transition-colors hover:text-foreground cursor-pointer"
              onclick={() => (search = '')}
              title="Clear search"
              aria-label="Clear search"
            >
              <X size={11} />
            </button>
          {/if}
        </div>
      </div>
    {/if}
  </div>

  <!-- Instance list -->
  <div
    class="min-h-0 flex-1 overflow-y-auto overscroll-y-contain"
    bind:this={listEl}
    bind:clientHeight={listViewportH}
    onscroll={handleListScroll}
  >
    {#if filteredInstances.length === 0}
      <div class={cn('px-3 py-4 text-[0.6875rem] text-muted-foreground', isCompact && 'lg:hidden')}>
        {search.trim()
          ? 'No instances match your search.'
          : 'No instances yet. Import torrents in the Instances view.'}
      </div>
    {/if}

    {#if view.offsetY > 0}
      <div style="height: {view.offsetY}px" aria-hidden="true"></div>
    {/if}

    {#each visibleInstances as instance (instance.id)}
      {@const state = getInstanceState(instance)}
      {@const meta = getStateMeta(state)}
      {@const tone = getTone(meta.tone)}
      {@const isActive = $activeInstanceId === instance.id}
      {@const issueMessage = getIssueMessage(instance)}

      <div
        class={cn(
          'group flex h-7 w-full items-center gap-2 border-l-2 px-2 py-1.5 text-left transition-colors cursor-pointer',
          isActive ? 'border-l-primary bg-muted' : 'border-l-transparent hover:bg-muted/60',
          isCompact && 'lg:justify-center lg:px-0'
        )}
        onclick={() => handleSelectInstance(instance.id)}
        onkeydown={e => e.key === 'Enter' && handleSelectInstance(instance.id)}
        role="button"
        tabindex="0"
        title="{getRowTitle(instance)} · {meta.label}"
      >
        <span class={cn('h-1.5 w-1.5 flex-shrink-0 rounded-full', tone.dot)} aria-label={meta.label}
        ></span>

        <span
          class={cn(
            'min-w-0 flex-1 truncate text-xs',
            isActive ? 'font-medium text-foreground' : 'text-muted-foreground',
            isCompact && 'lg:hidden'
          )}
        >
          {getInstanceLabel(instance)}
        </span>

        {#if issueMessage && !isCompact}
          <span
            class="flex-shrink-0 text-stat-ratio"
            title={issueMessage}
            aria-label={issueMessage}
          >
            <AlertTriangle size={11} />
          </span>
        {/if}

        {#if !isCompact && instance.stats && instance.stats.ratio > 0}
          <span
            class="flex-shrink-0 tabular-nums text-[0.625rem] font-semibold {instance.stats.ratio >=
            1
              ? 'text-stat-upload'
              : 'text-stat-ratio'}"
            title="Current ratio"
          >
            {instance.stats.ratio.toFixed(2)}x
          </span>
        {/if}

        {#if !isCompact}
          {#if instance.source !== 'watch_folder'}
            <button
              class="flex-shrink-0 p-0.5 text-muted-foreground opacity-0 transition-colors hover:text-stat-danger group-hover:opacity-100 focus-visible:opacity-100 cursor-pointer"
              onclick={e => handleRemoveInstance(e, instance.id)}
              title="Close instance"
              aria-label="Close instance"
            >
              <X size={12} strokeWidth={2.5} />
            </button>
          {:else}
            <button
              class="flex-shrink-0 p-0.5 text-muted-foreground opacity-0 transition-colors hover:text-stat-danger group-hover:opacity-100 cursor-pointer"
              onclick={e =>
                handleForceRemoveInstance(e, instance.id, instance.torrent?.name || instance.name)}
              title="Force delete (file may be missing)"
              aria-label="Force delete instance"
            >
              <FolderOpen size={11} />
            </button>
          {/if}
        {/if}
      </div>
    {/each}

    {#if bottomSpacer > 0}
      <div style="height: {bottomSpacer}px" aria-hidden="true"></div>
    {/if}
  </div>

  <!-- Bulk actions -->
  {#if hasMultipleInstancesWithTorrents}
    <div class="shrink-0 border-t border-border p-1.5">
      <div class={cn('grid grid-cols-4 gap-1', isCompact && 'lg:grid-cols-1')}>
        <Button
          onclick={onStartAll}
          disabled={!hasStoppedInstancesWithTorrents}
          size="icon"
          class="h-6 w-full text-stat-upload"
          variant="outline"
          title="Start all instances"
          aria-label="Start all instances"
        >
          {#snippet children()}
            <Play size={11} fill="currentColor" />
          {/snippet}
        </Button>

        <Button
          onclick={onPauseAll}
          disabled={!hasUnpausedRunningInstances}
          size="icon"
          class="h-6 w-full text-stat-ratio"
          variant="outline"
          title="Pause all running instances"
          aria-label="Pause all running instances"
        >
          {#snippet children()}
            <Pause size={11} fill="currentColor" />
          {/snippet}
        </Button>

        <Button
          onclick={onResumeAll}
          disabled={!hasPausedInstances}
          size="icon"
          class="h-6 w-full text-stat-upload"
          variant="outline"
          title="Resume all paused instances"
          aria-label="Resume all paused instances"
        >
          {#snippet children()}
            <Play size={11} fill="currentColor" strokeWidth={2.5} />
          {/snippet}
        </Button>

        <Button
          onclick={onStopAll}
          disabled={!hasRunningInstances}
          size="icon"
          class="h-6 w-full text-stat-danger"
          variant="outline"
          title="Stop all instances"
          aria-label="Stop all instances"
        >
          {#snippet children()}
            <Square size={11} fill="currentColor" />
          {/snippet}
        </Button>
      </div>
    </div>
  {/if}

  <!-- Footer -->
  <div class="shrink-0 border-t border-border p-1.5">
    <a
      href={repository}
      target="_blank"
      rel="noopener noreferrer"
      class={cn(
        'flex items-center gap-2 px-1.5 py-1 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground',
        isCompact && 'lg:justify-center lg:px-0'
      )}
      title="View Rustatio on GitHub · v{appVersion}"
    >
      <Github size={13} class="flex-shrink-0" />
      <span class={cn('text-[0.625rem]', isCompact && 'lg:hidden')}>
        v<span class="italic">{appVersion}</span>
      </span>
    </a>
  </div>
</aside>

<ConfirmDialog
  bind:open={forceDeleteConfirmVisible}
  title="Force Delete Instance"
  message={`Force delete "${forceDeleteTarget?.name || 'this instance'}"? This instance was created from the watch folder but the torrent file may no longer exist.`}
  cancelLabel="Cancel"
  confirmLabel={forceDeleteBusy ? 'Deleting...' : 'Force Delete'}
  kind="danger"
  titleId="force-delete-title"
  onCancel={cancelForceRemoveInstance}
  onConfirm={confirmForceRemoveInstance}
  disableCancel={forceDeleteBusy}
  disableConfirm={forceDeleteBusy}
  closeOnBackdrop={!forceDeleteBusy}
  closeOnEscape={!forceDeleteBusy}
/>
