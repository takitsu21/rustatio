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
  } from '$lib/gridStore.js';
  import { Play, Square, Pause, Trash2, Upload, Search, ChevronDown, Funnel } from '@lucide/svelte';

  let { onImport = () => {}, onOpenFilters = () => {} } = $props();

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
    <Button onclick={onImport} size="sm" class="hidden sm:inline-flex gap-1.5">
      {#snippet children()}
        <Upload size={14} />
        Import
      {/snippet}
    </Button>

    <Button
      onclick={onOpenFilters}
      size="icon"
      variant="outline"
      class="hidden sm:inline-flex lg:hidden h-9 w-9"
      title="Open filters"
      aria-label="Open filters"
    >
      {#snippet children()}
        <Funnel size={14} />
      {/snippet}
    </Button>

    <!-- Separator -->
    <div class="w-px h-6 bg-border"></div>

    <!-- Select dropdown -->
    <div class="relative" bind:this={selectMenuEl}>
      <Button onclick={handleSelectMenuClick} size="sm" variant="outline" class="gap-1 text-xs">
        {#snippet children()}
          Select
          <ChevronDown
            size={12}
            class={selectMenuOpen ? 'rotate-180 transition-transform' : 'transition-transform'}
          />
        {/snippet}
      </Button>

      {#if selectMenuOpen}
        <div
          class="absolute top-full left-0 mt-1 z-50 min-w-[180px] bg-popover border border-border rounded-lg shadow-xl shadow-black/20 py-1"
        >
          <button
            class="w-full text-left px-3 py-1.5 text-xs hover:bg-accent/80 transition-colors cursor-pointer"
            onclick={() => {
              gridActions.selectAll();
              selectMenuOpen = false;
            }}>Select All</button
          >
          <button
            class="w-full text-left px-3 py-1.5 text-xs hover:bg-accent/80 transition-colors cursor-pointer"
            onclick={() => {
              gridActions.deselectAll();
              selectMenuOpen = false;
            }}>Deselect All</button
          >
          <button
            class="w-full text-left px-3 py-1.5 text-xs hover:bg-accent/80 transition-colors cursor-pointer"
            onclick={() => {
              gridActions.invertSelection();
              selectMenuOpen = false;
            }}>Invert Selection</button
          >

          <div class="h-px bg-border/50 my-1 mx-2"></div>

          <button
            class="w-full text-left px-3 py-1.5 text-xs hover:bg-accent/80 transition-colors cursor-pointer"
            onclick={() => {
              gridActions.selectByState('running');
              selectMenuOpen = false;
            }}>All Running</button
          >
          <button
            class="w-full text-left px-3 py-1.5 text-xs hover:bg-accent/80 transition-colors cursor-pointer"
            onclick={() => {
              gridActions.selectByState('stopped');
              selectMenuOpen = false;
            }}>All Stopped</button
          >
          <button
            class="w-full text-left px-3 py-1.5 text-xs hover:bg-accent/80 transition-colors cursor-pointer"
            onclick={() => {
              gridActions.selectByState('paused');
              selectMenuOpen = false;
            }}>All Paused</button
          >

          {#if filteredTags.length > 0}
            <div class="h-px bg-border/50 my-1 mx-2"></div>
            <div class="px-3 py-1 text-xs text-muted-foreground font-medium">By Tag</div>
            {#each filteredTags as tag (tag)}
              <button
                class="w-full text-left px-3 py-1.5 text-xs hover:bg-accent/80 transition-colors cursor-pointer pl-5"
                onclick={() => {
                  gridActions.selectByTag(tag);
                  selectMenuOpen = false;
                }}>{tag}</button
              >
            {/each}
          {/if}
        </div>
      {/if}
    </div>

    <!-- Grid action buttons (only visible when selection exists) -->
    {#if selectionCount > 0}
      <div class="w-px h-6 bg-border"></div>

      <span class="text-xs text-muted-foreground">{selectionCount} selected</span>

      <Button
        onclick={handleStart}
        size="icon"
        variant="default"
        class="h-9 w-9"
        disabled={!canStart}
        title="Start selected"
        aria-label="Start selected"
      >
        {#snippet children()}
          <Play size={14} fill="currentColor" />
        {/snippet}
      </Button>

      <Button
        onclick={handleStop}
        size="icon"
        class="h-9 w-9 bg-stat-danger hover:bg-stat-danger/90 text-white"
        disabled={!canStop}
        title="Stop selected"
        aria-label="Stop selected"
      >
        {#snippet children()}
          <Square size={14} fill="currentColor" />
        {/snippet}
      </Button>

      <Button
        onclick={handlePause}
        size="icon"
        class="h-9 w-9 bg-stat-ratio hover:bg-stat-ratio/90 text-white"
        disabled={!canPause}
        title="Pause selected"
        aria-label="Pause selected"
      >
        {#snippet children()}
          <Pause size={14} fill="currentColor" />
        {/snippet}
      </Button>

      <Button
        onclick={handleResume}
        size="icon"
        variant="secondary"
        class="h-9 w-9"
        disabled={!canResume}
        title="Resume selected"
        aria-label="Resume selected"
      >
        {#snippet children()}
          <Play size={14} fill="currentColor" />
        {/snippet}
      </Button>

      <div class="w-px h-6 bg-border"></div>

      <GridTagPopover />

      <Button
        onclick={handleDelete}
        size="icon"
        variant="destructive"
        class="h-9 w-9"
        title="Delete selected"
        aria-label="Delete selected"
      >
        {#snippet children()}
          <Trash2 size={14} />
        {/snippet}
      </Button>
    {/if}

    <!-- Right side: total count -->
    <div class="ml-auto text-xs text-muted-foreground">
      {totalCount} instance{totalCount !== 1 ? 's' : ''}
    </div>
  </div>

  <!-- Bottom row: Search -->
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
