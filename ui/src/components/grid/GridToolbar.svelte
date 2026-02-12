<script>
  import Button from '$lib/components/ui/button.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Select from '$lib/components/ui/select.svelte';
  import GridTagPopover from './GridTagPopover.svelte';
  import {
    selectedIds,
    gridFilters,
    gridActions,
    allTags,
    gridInstances,
    filteredGridInstances,
  } from '$lib/gridStore.js';
  import {
    Play,
    Square,
    Pause,
    Trash2,
    Upload,
    Search,
    ChevronDown,
    AlertTriangle,
  } from '@lucide/svelte';

  let { onImport = () => {} } = $props();

  let selectionCount = $derived($selectedIds.size);
  let totalCount = $derived($gridInstances.length);

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

  async function handleDelete() {
    deleteConfirmVisible = true;
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

  function handleStateFilter(e) {
    gridFilters.update(f => ({ ...f, stateFilter: e.target.value }));
  }

  function handleTagFilter(e) {
    gridFilters.update(f => ({ ...f, tagFilter: e.target.value }));
  }

  function handleSelectMenuClick(e) {
    e.stopPropagation();
    selectMenuOpen = !selectMenuOpen;
  }

  function handleSelectMenuOutside(e) {
    if (selectMenuOpen && selectMenuEl && !selectMenuEl.contains(e.target)) {
      selectMenuOpen = false;
    }
  }
</script>

<svelte:window onclick={handleSelectMenuOutside} />

<div class="flex flex-col gap-3">
  <!-- Top row: Import + Grid actions -->
  <div class="flex items-center gap-2 flex-wrap">
    <!-- Import button -->
    <Button onclick={onImport} size="sm" class="gap-1.5">
      {#snippet children()}
        <Upload size={14} />
        Import
      {/snippet}
    </Button>

    <!-- Separator -->
    <div class="w-px h-6 bg-border"></div>

    <!-- Select dropdown -->
    <div class="relative" bind:this={selectMenuEl}>
      <Button onclick={handleSelectMenuClick} size="sm" variant="outline" class="gap-1 text-xs">
        {#snippet children()}
          Select
          <ChevronDown size={12} class={selectMenuOpen ? 'rotate-180 transition-transform' : 'transition-transform'} />
        {/snippet}
      </Button>

      {#if selectMenuOpen}
        <div class="absolute top-full left-0 mt-1 z-50 min-w-[180px] bg-popover border border-border rounded-lg shadow-xl shadow-black/20 py-1">
          <button
            class="w-full text-left px-3 py-1.5 text-xs hover:bg-accent/80 transition-colors cursor-pointer"
            onclick={() => { gridActions.selectAll(); selectMenuOpen = false; }}
          >Select All</button>
          <button
            class="w-full text-left px-3 py-1.5 text-xs hover:bg-accent/80 transition-colors cursor-pointer"
            onclick={() => { gridActions.deselectAll(); selectMenuOpen = false; }}
          >Deselect All</button>
          <button
            class="w-full text-left px-3 py-1.5 text-xs hover:bg-accent/80 transition-colors cursor-pointer"
            onclick={() => { gridActions.invertSelection(); selectMenuOpen = false; }}
          >Invert Selection</button>

          <div class="h-px bg-border/50 my-1 mx-2"></div>

          <button
            class="w-full text-left px-3 py-1.5 text-xs hover:bg-accent/80 transition-colors cursor-pointer"
            onclick={() => { gridActions.selectByState('running'); selectMenuOpen = false; }}
          >All Running</button>
          <button
            class="w-full text-left px-3 py-1.5 text-xs hover:bg-accent/80 transition-colors cursor-pointer"
            onclick={() => { gridActions.selectByState('stopped'); selectMenuOpen = false; }}
          >All Stopped</button>
          <button
            class="w-full text-left px-3 py-1.5 text-xs hover:bg-accent/80 transition-colors cursor-pointer"
            onclick={() => { gridActions.selectByState('paused'); selectMenuOpen = false; }}
          >All Paused</button>

          {#if filteredTags.length > 0}
            <div class="h-px bg-border/50 my-1 mx-2"></div>
            <div class="px-3 py-1 text-xs text-muted-foreground font-medium">By Tag</div>
            {#each filteredTags as tag (tag)}
              <button
                class="w-full text-left px-3 py-1.5 text-xs hover:bg-accent/80 transition-colors cursor-pointer pl-5"
                onclick={() => { gridActions.selectByTag(tag); selectMenuOpen = false; }}
              >{tag}</button>
            {/each}
          {/if}
        </div>
      {/if}
    </div>

    <!-- Grid action buttons (only visible when selection exists) -->
    {#if selectionCount > 0}
      <div class="w-px h-6 bg-border"></div>

      <span class="text-xs text-muted-foreground">{selectionCount} selected</span>

      <Button onclick={handleStart} size="sm" variant="default" class="gap-1" disabled={!canStart}>
        {#snippet children()}
          <Play size={12} fill="currentColor" />
          Start
        {/snippet}
      </Button>

      <Button onclick={handleStop} size="sm" class="gap-1 bg-stat-danger hover:bg-stat-danger/90 text-white" disabled={!canStop}>
        {#snippet children()}
          <Square size={12} fill="currentColor" />
          Stop
        {/snippet}
      </Button>

      <Button onclick={handlePause} size="sm" class="gap-1 bg-stat-ratio hover:bg-stat-ratio/90 text-white" disabled={!canPause}>
        {#snippet children()}
          <Pause size={12} fill="currentColor" />
          Pause
        {/snippet}
      </Button>

      <Button onclick={handleResume} size="sm" variant="secondary" class="gap-1" disabled={!canResume}>
        {#snippet children()}
          <Play size={12} fill="currentColor" />
          Resume
        {/snippet}
      </Button>

      <div class="w-px h-6 bg-border"></div>

      <GridTagPopover />

      <Button onclick={handleDelete} size="sm" variant="destructive" class="gap-1">
        {#snippet children()}
          <Trash2 size={12} />
          Delete
        {/snippet}
      </Button>
    {/if}

    <!-- Right side: total count -->
    <div class="ml-auto text-xs text-muted-foreground">
      {totalCount} instance{totalCount !== 1 ? 's' : ''}
    </div>
  </div>

  <!-- Bottom row: Search + Filters -->
  <div class="flex items-center gap-2">
    <div class="relative flex-1 max-w-sm">
      <Search size={14} class="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground" />
      <Input
        value={$gridFilters.search}
        oninput={handleSearchInput}
        placeholder="Search by name, hash, or tag..."
        class="h-8 pl-8 text-xs"
      />
    </div>

    <Select value={$gridFilters.stateFilter} onchange={handleStateFilter} class="h-8 w-32 text-xs py-1">
      {#snippet children()}
        <option value="all">All States</option>
        <option value="starting">Starting</option>
        <option value="running">Running</option>
        <option value="stopping">Stopping</option>
        <option value="paused">Paused</option>
        <option value="idle">Idle</option>
        <option value="stopped">Stopped</option>
      {/snippet}
    </Select>

    {#if $allTags.length > 0}
      <Select value={$gridFilters.tagFilter} onchange={handleTagFilter} class="h-8 w-32 text-xs py-1">
        {#snippet children()}
          <option value="">All Tags</option>
          {#each $allTags as tag (tag)}
            <option value={tag}>{tag}</option>
          {/each}
        {/snippet}
      </Select>
    {/if}
  </div>
</div>

{#if deleteConfirmVisible && selectionCount > 0}
  <div
    class="fixed inset-0 bg-black/50 z-[60] flex items-center justify-center p-4"
    onclick={cancelDelete}
    onkeydown={e => e.key === 'Escape' && cancelDelete()}
    role="dialog"
    aria-modal="true"
    aria-labelledby="grid-delete-confirm-title"
    tabindex="-1"
  >
    <div
      class="bg-card text-card-foreground rounded-xl shadow-2xl max-w-sm w-full p-5 border border-border"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="presentation"
    >
      <div class="flex items-center gap-3 mb-3">
        <div class="w-10 h-10 bg-stat-danger/10 rounded-lg flex items-center justify-center shrink-0">
          <AlertTriangle size={20} class="text-stat-danger" />
        </div>
        <div>
          <h3 id="grid-delete-confirm-title" class="text-sm font-semibold text-foreground">Delete {selectionCount} Instance{selectionCount !== 1 ? 's' : ''}</h3>
          <p class="text-xs text-muted-foreground mt-0.5">
            This will permanently delete the selected instance{selectionCount !== 1 ? 's' : ''}.
          </p>
        </div>
      </div>
      <p class="text-xs text-muted-foreground mb-4">This action cannot be undone.</p>
      <div class="flex justify-end gap-2">
        <button
          onclick={cancelDelete}
          class="px-3 py-1.5 text-xs font-medium rounded-md border border-border hover:bg-muted transition-colors cursor-pointer"
        >
          Cancel
        </button>
        <button
          onclick={confirmDelete}
          class="px-3 py-1.5 text-xs font-medium rounded-md bg-stat-danger text-white hover:bg-stat-danger/90 transition-colors cursor-pointer"
        >
          Delete
        </button>
      </div>
    </div>
  </div>
{/if}
