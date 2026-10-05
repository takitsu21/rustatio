<script>
  import Button from '$lib/components/ui/button.svelte';
  import ConfirmDialog from '../common/ConfirmDialog.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import GridTagPopover from './GridTagPopover.svelte';
  import {
    selectedIds,
    gridFilters,
    gridActions,
    gridInstances,
    filteredGridInstances,
  } from '$lib/grid/gridStore.js';
  import {
    Play,
    Square,
    Pause,
    Trash2,
    Upload,
    Search,
    X,
    Funnel,
    ListChecks,
    Pencil,
    MoreHorizontal,
    PanelLeft,
    PanelLeftClose,
  } from '@lucide/svelte';

  let {
    onImport = () => {},
    onOpenFilters = () => {},
    onBulkEdit = () => {},
    filtersVisible = true,
    onToggleFilters = () => {},
  } = $props();

  let selectionCount = $derived($selectedIds.size);
  let totalCount = $derived($gridInstances.length);
  let visibleCount = $derived($filteredGridInstances.length);
  let isFiltered = $derived(visibleCount !== totalCount);

  // Determine which bulk actions are valid based on selected instances' states
  let canStart = $derived.by(() => {
    if ($selectedIds.size === 0) return false;
    return $gridInstances.some(i => $selectedIds.has(i.id) && i.state?.toLowerCase() === 'stopped');
  });

  let canStop = $derived.by(() => {
    if ($selectedIds.size === 0) return false;
    return $gridInstances.some(i => {
      if (!$selectedIds.has(i.id)) return false;
      const s = i.state?.toLowerCase();
      return s === 'running' || s === 'idle' || s === 'paused' || s === 'starting';
    });
  });

  let canPause = $derived.by(() => {
    if ($selectedIds.size === 0) return false;
    return $gridInstances.some(i => {
      if (!$selectedIds.has(i.id)) return false;
      const s = i.state?.toLowerCase();
      return s === 'running' || s === 'idle';
    });
  });

  let canResume = $derived.by(() => {
    if ($selectedIds.size === 0) return false;
    return $gridInstances.some(i => $selectedIds.has(i.id) && i.state?.toLowerCase() === 'paused');
  });

  let deleteConfirmVisible = $state(false);
  let selectMenuOpen = $state(false);
  let selectMenuEl = $state(null);
  let overflowOpen = $state(false);
  let overflowEl = $state(null);

  // Unique tags across filtered instances for "Select by tag" submenu
  let filteredTags = $derived.by(() => {
    const tagSet = new Set();
    for (const inst of $filteredGridInstances) {
      for (const tag of inst.tags || []) tagSet.add(tag);
    }
    return [...tagSet].sort();
  });

  async function handleStart() {
    try {
      await gridActions.gridStart();
    } catch (error) {
      console.error('Grid start failed:', error);
    }
  }

  async function handleStop() {
    try {
      await gridActions.gridStop();
    } catch (error) {
      console.error('Grid stop failed:', error);
    }
  }

  async function handlePause() {
    try {
      await gridActions.gridPause();
    } catch (error) {
      console.error('Grid pause failed:', error);
    }
  }

  async function handleResume() {
    try {
      await gridActions.gridResume();
    } catch (error) {
      console.error('Grid resume failed:', error);
    }
  }

  function handleDelete() {
    deleteConfirmVisible = true;
    overflowOpen = false;
  }

  async function confirmDelete() {
    deleteConfirmVisible = false;
    try {
      await gridActions.gridDelete();
    } catch (error) {
      console.error('Grid delete failed:', error);
    }
  }

  function cancelDelete() {
    deleteConfirmVisible = false;
  }

  function handleSearchInput(e) {
    gridFilters.update(f => ({ ...f, search: e.target.value }));
  }

  function handleSelectMenuClick(e) {
    e.stopPropagation();
    selectMenuOpen = !selectMenuOpen;
    overflowOpen = false;
  }

  function handleOverflowClick(e) {
    e.stopPropagation();
    overflowOpen = !overflowOpen;
    selectMenuOpen = false;
  }

  function handleOutside(e) {
    if (selectMenuOpen && selectMenuEl && !selectMenuEl.contains(e.target)) {
      selectMenuOpen = false;
    }
    if (overflowOpen && overflowEl && !overflowEl.contains(e.target)) {
      overflowOpen = false;
    }
  }

  function runOverflow(action) {
    overflowOpen = false;
    switch (action) {
      case 'start':
        handleStart();
        break;
      case 'pause':
        handlePause();
        break;
      case 'resume':
        handleResume();
        break;
      case 'stop':
        handleStop();
        break;
      case 'select-all':
        gridActions.selectAll();
        break;
      case 'deselect-all':
        gridActions.deselectAll();
        break;
      case 'invert':
        gridActions.invertSelection();
        break;
      case 'select-running':
        gridActions.selectByState('running');
        break;
      case 'select-stopped':
        gridActions.selectByState('stopped');
        break;
      case 'select-paused':
        gridActions.selectByState('paused');
        break;
      case 'edit':
        onBulkEdit();
        break;
      case 'delete':
        handleDelete();
        break;
      default:
        break;
    }
  }

  const menuItemClass =
    'flex w-full items-center gap-2 px-2.5 py-1 text-left text-[0.6875rem] text-muted-foreground transition-colors hover:bg-muted hover:text-foreground cursor-pointer disabled:pointer-events-none disabled:opacity-40';
</script>

<svelte:window onclick={handleOutside} />

<div class="flex min-h-9 shrink-0 items-center gap-2 border-b border-border bg-card px-2 py-1">
  <!-- Search -->
  <div class="relative min-w-0 max-w-[17.5rem] flex-1">
    <Search size={12} class="absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground" />
    <Input
      value={$gridFilters.search}
      oninput={handleSearchInput}
      placeholder="Search instances…"
      class="h-7 pl-6.5 pr-6 text-[0.6875rem]"
    />
    {#if $gridFilters.search}
      <button
        class="absolute right-1.5 top-1/2 -translate-y-1/2 text-muted-foreground transition-colors hover:text-foreground cursor-pointer"
        onclick={() => gridFilters.update(f => ({ ...f, search: '' }))}
        title="Clear search"
        aria-label="Clear search"
      >
        <X size={11} />
      </button>
    {/if}
  </div>

  <!-- Filters rail toggle (desktop) -->
  <Button
    onclick={onToggleFilters}
    size="icon"
    variant="outline"
    class="hidden h-7 w-7 lg:inline-flex"
    title={filtersVisible ? 'Hide filters' : 'Show filters'}
    aria-label={filtersVisible ? 'Hide filters' : 'Show filters'}
  >
    {#snippet children()}
      {#if filtersVisible}
        <PanelLeftClose size={12} />
      {:else}
        <PanelLeft size={12} />
      {/if}
    {/snippet}
  </Button>

  <!-- Mobile filters -->
  <Button
    onclick={onOpenFilters}
    size="icon"
    variant="outline"
    class="h-7 w-7 lg:hidden"
    title="Open filters"
    aria-label="Open filters"
  >
    {#snippet children()}
      <Funnel size={12} />
    {/snippet}
  </Button>

  <!-- Right cluster -->
  <div class="ml-auto flex min-w-0 shrink-0 items-center gap-1">
    {#if selectionCount > 0}
      <span
        class="mr-1 hidden whitespace-nowrap tabular-nums text-[0.625rem] text-muted-foreground sm:inline"
      >
        {selectionCount} selected
      </span>
    {/if}

    <!-- Bulk actions (md+; collapsed into the overflow menu below) -->
    <div class="hidden items-center gap-0.5 md:flex">
      <Button
        onclick={handleStart}
        disabled={!canStart}
        size="icon"
        variant="outline"
        class="h-7 w-7 text-stat-upload"
        title="Start selected"
        aria-label="Start selected"
      >
        {#snippet children()}
          <Play size={12} fill="currentColor" />
        {/snippet}
      </Button>

      <Button
        onclick={handlePause}
        disabled={!canPause}
        size="icon"
        variant="outline"
        class="h-7 w-7 text-stat-ratio"
        title="Pause selected"
        aria-label="Pause selected"
      >
        {#snippet children()}
          <Pause size={12} fill="currentColor" />
        {/snippet}
      </Button>

      <Button
        onclick={handleResume}
        disabled={!canResume}
        size="icon"
        variant="outline"
        class="h-7 w-7 text-stat-upload"
        title="Resume selected"
        aria-label="Resume selected"
      >
        {#snippet children()}
          <Play size={12} fill="currentColor" strokeWidth={2.5} />
        {/snippet}
      </Button>

      <Button
        onclick={handleStop}
        disabled={!canStop}
        size="icon"
        variant="outline"
        class="h-7 w-7 text-stat-danger"
        title="Stop selected"
        aria-label="Stop selected"
      >
        {#snippet children()}
          <Square size={11} fill="currentColor" />
        {/snippet}
      </Button>
    </div>

    <span class="mx-0.5 hidden h-4 w-px bg-border md:block" aria-hidden="true"></span>

    <!-- Tags -->
    <GridTagPopover disabled={selectionCount === 0} />

    <!-- Edit / delete / selection tools (xl+) -->
    <div class="hidden items-center gap-0.5 xl:flex">
      <Button
        onclick={onBulkEdit}
        disabled={selectionCount === 0}
        size="icon"
        variant="outline"
        class="h-7 w-7"
        title="Edit selected"
        aria-label="Edit selected"
      >
        {#snippet children()}
          <Pencil size={12} />
        {/snippet}
      </Button>

      <Button
        onclick={handleDelete}
        disabled={selectionCount === 0}
        size="icon"
        variant="outline"
        class="h-7 w-7 text-stat-danger"
        title="Delete selected"
        aria-label="Delete selected"
      >
        {#snippet children()}
          <Trash2 size={12} />
        {/snippet}
      </Button>

      <span class="mx-0.5 h-4 w-px bg-border" aria-hidden="true"></span>

      <div class="relative" bind:this={selectMenuEl}>
        <Button
          onclick={handleSelectMenuClick}
          size="icon"
          variant="outline"
          class="h-7 w-7"
          title="Selection tools"
          aria-label="Selection tools"
        >
          {#snippet children()}
            <ListChecks size={12} />
          {/snippet}
        </Button>

        {#if selectMenuOpen}
          <div
            class="absolute left-0 top-full z-50 mt-1 min-w-[10.625rem] border border-border bg-popover py-1"
            role="menu"
          >
            <button
              class={menuItemClass}
              onclick={() => {
                gridActions.selectAll();
                selectMenuOpen = false;
              }}>Select all</button
            >
            <button
              class={menuItemClass}
              onclick={() => {
                gridActions.deselectAll();
                selectMenuOpen = false;
              }}>Deselect all</button
            >
            <button
              class={menuItemClass}
              onclick={() => {
                gridActions.invertSelection();
                selectMenuOpen = false;
              }}>Invert selection</button
            >

            <div class="mx-2 my-1 h-px bg-border"></div>

            <button
              class={menuItemClass}
              onclick={() => {
                gridActions.selectByState('running');
                selectMenuOpen = false;
              }}>All running</button
            >
            <button
              class={menuItemClass}
              onclick={() => {
                gridActions.selectByState('stopped');
                selectMenuOpen = false;
              }}>All stopped</button
            >
            <button
              class={menuItemClass}
              onclick={() => {
                gridActions.selectByState('paused');
                selectMenuOpen = false;
              }}>All paused</button
            >

            {#if filteredTags.length > 0}
              <div class="mx-2 my-1 h-px bg-border"></div>
              <div
                class="px-2.5 py-1 text-[0.625rem] uppercase tracking-wider text-muted-foreground"
              >
                By tag
              </div>
              <div class="max-h-40 overflow-y-auto">
                {#each filteredTags as tag (tag)}
                  <button
                    class={menuItemClass}
                    onclick={() => {
                      gridActions.selectByTag(tag);
                      selectMenuOpen = false;
                    }}>{tag}</button
                  >
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <!-- Overflow (below xl) -->
    <div class="relative xl:hidden" bind:this={overflowEl}>
      <Button
        onclick={handleOverflowClick}
        size="icon"
        variant="outline"
        class="h-7 w-7"
        title="More actions"
        aria-label="More actions"
      >
        {#snippet children()}
          <MoreHorizontal size={13} />
        {/snippet}
      </Button>

      {#if overflowOpen}
        <div
          class="absolute right-0 top-full z-50 mt-1 min-w-[12rem] border border-border bg-popover py-1"
          role="menu"
        >
          <div class="px-2.5 py-1 text-[0.625rem] uppercase tracking-wider text-muted-foreground">
            Selected
          </div>
          <button class={menuItemClass} disabled={!canStart} onclick={() => runOverflow('start')}>
            <Play size={12} class="text-stat-upload" fill="currentColor" /> Start
          </button>
          <button class={menuItemClass} disabled={!canPause} onclick={() => runOverflow('pause')}>
            <Pause size={12} class="text-stat-ratio" fill="currentColor" /> Pause
          </button>
          <button class={menuItemClass} disabled={!canResume} onclick={() => runOverflow('resume')}>
            <Play size={12} class="text-stat-upload" fill="currentColor" /> Resume
          </button>
          <button class={menuItemClass} disabled={!canStop} onclick={() => runOverflow('stop')}>
            <Square size={11} class="text-stat-danger" fill="currentColor" /> Stop
          </button>
          <button
            class={menuItemClass}
            disabled={selectionCount === 0}
            onclick={() => runOverflow('edit')}
          >
            <Pencil size={12} /> Edit selected
          </button>
          <button
            class="{menuItemClass} text-stat-danger"
            disabled={selectionCount === 0}
            onclick={() => runOverflow('delete')}
          >
            <Trash2 size={12} /> Delete selected
          </button>

          <div class="mx-2 my-1 h-px bg-border"></div>

          <div class="px-2.5 py-1 text-[0.625rem] uppercase tracking-wider text-muted-foreground">
            Select
          </div>
          <button class={menuItemClass} onclick={() => runOverflow('select-all')}>Select all</button
          >
          <button class={menuItemClass} onclick={() => runOverflow('deselect-all')}
            >Deselect all</button
          >
          <button class={menuItemClass} onclick={() => runOverflow('invert')}
            >Invert selection</button
          >
          <button class={menuItemClass} onclick={() => runOverflow('select-running')}
            >All running</button
          >
          <button class={menuItemClass} onclick={() => runOverflow('select-stopped')}
            >All stopped</button
          >
          <button class={menuItemClass} onclick={() => runOverflow('select-paused')}
            >All paused</button
          >
        </div>
      {/if}
    </div>

    <Button
      onclick={onImport}
      size="sm"
      variant="default"
      class="h-7 gap-1.5 px-2 text-[0.6875rem]"
      title="Import torrents"
    >
      {#snippet children()}
        <Upload size={12} />
        <span class="hidden sm:inline">Import</span>
      {/snippet}
    </Button>

    <span
      class="ml-1 hidden whitespace-nowrap tabular-nums text-[0.625rem] text-muted-foreground xl:inline"
    >
      {#if isFiltered}
        {visibleCount} / {totalCount}
      {:else}
        {totalCount} instance{totalCount !== 1 ? 's' : ''}
      {/if}
    </span>
  </div>
</div>

<ConfirmDialog
  bind:open={deleteConfirmVisible}
  title={`Delete ${selectionCount} Instance${selectionCount !== 1 ? 's' : ''}`}
  message={`This will permanently delete the selected instance${selectionCount !== 1 ? 's' : ''}.\n\nThis action cannot be undone.`}
  confirmLabel="Delete"
  kind="danger"
  titleId="grid-delete-confirm-title"
  onCancel={cancelDelete}
  onConfirm={confirmDelete}
/>
