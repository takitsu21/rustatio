<script>
  import { cn } from '$lib/utils.js';
  import { selectedIds, gridActions, gridSort } from '$lib/gridStore.js';
  import TagBadge from './TagBadge.svelte';
  import {
    ArrowUp,
    ArrowDown,
    ArrowUpDown,
    Circle,
    Pause,
    Moon,
    Square,
    LoaderCircle,
    Play,
    Trash2,
    Copy,
    Pencil,
    AlertTriangle,
  } from '@lucide/svelte';

  let { data = [], oncontextaction = () => {} } = $props();

  // Context menu state
  let ctxVisible = $state(false);
  let ctxX = $state(0);
  let ctxY = $state(0);
  let ctxInstance = $state(null);
  let menuEl = $state(null);

  // Shift+click range select
  let lastSelectedIndex = $state(null);

  // Delete confirmation dialog state
  let deleteConfirmVisible = $state(false);
  let deleteTarget = $state(null);

  function openContextMenu(e, instance) {
    e.preventDefault();
    e.stopPropagation();
    ctxX = e.clientX;
    ctxY = e.clientY;
    ctxInstance = instance;
    ctxVisible = true;
  }

  function closeContextMenu() {
    ctxVisible = false;
    ctxInstance = null;
  }

  function handleCtxAction(actionId) {
    const inst = ctxInstance;
    closeContextMenu();
    if (actionId === 'delete') {
      deleteTarget = inst;
      deleteConfirmVisible = true;
      return;
    }
    oncontextaction(actionId, inst);
  }

  function confirmDelete() {
    const inst = deleteTarget;
    deleteConfirmVisible = false;
    deleteTarget = null;
    oncontextaction('delete', inst);
  }

  function cancelDelete() {
    deleteConfirmVisible = false;
    deleteTarget = null;
  }

  function getCtxActions(inst) {
    if (!inst) return [];
    const state = inst.state?.toLowerCase();
    const items = [];

    if (state === 'stopped') {
      items.push({ id: 'start', label: 'Start', icon: Play, color: 'text-stat-upload' });
    }
    if (state === 'running' || state === 'idle') {
      items.push({ id: 'pause', label: 'Pause', icon: Pause, color: 'text-stat-ratio' });
    }
    if (state === 'paused') {
      items.push({ id: 'resume', label: 'Resume', icon: Play, color: 'text-stat-upload' });
    }
    if (state === 'running' || state === 'idle' || state === 'paused' || state === 'starting') {
      items.push({ id: 'stop', label: 'Stop', icon: Square, color: 'text-stat-danger' });
    }

    items.push(null); // separator
    items.push({ id: 'edit', label: 'Edit in Standard View', icon: Pencil, color: 'text-foreground' });
    items.push({ id: 'copy_hash', label: 'Copy Info Hash', icon: Copy, color: 'text-muted-foreground' });
    items.push(null); // separator
    items.push({ id: 'delete', label: 'Delete', icon: Trash2, color: 'text-stat-danger' });

    return items;
  }

  let ctxActions = $derived(getCtxActions(ctxInstance));

  // Adjust position to stay within viewport
  let ctxStyle = $derived.by(() => {
    if (!ctxVisible) return '';
    const menuWidth = 180;
    const menuHeight = ctxActions.length * 32 + 40;
    const adjX = ctxX + menuWidth > window.innerWidth ? ctxX - menuWidth : ctxX;
    const adjY = ctxY + menuHeight > window.innerHeight ? ctxY - menuHeight : ctxY;
    return `left: ${adjX}px; top: ${adjY}px;`;
  });

  // Close on click outside / escape
  function handleWindowMousedown(e) {
    if (ctxVisible && menuEl && !menuEl.contains(e.target)) {
      closeContextMenu();
    }
  }

  function handleWindowKeydown(e) {
    if (ctxVisible && e.key === 'Escape') {
      closeContextMenu();
    }
  }

  const ROW_HEIGHT = 28;
  const BUFFER_ROWS = 10;

  let scrollContainer = $state(null);
  let scrollTop = $state(0);
  let containerHeight = $state(0);

  function onScroll(e) {
    scrollTop = e.target.scrollTop;
  }

  let totalHeight = $derived(data.length * ROW_HEIGHT);
  let startIndex = $derived(Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - BUFFER_ROWS));
  let visibleCount = $derived(Math.ceil(containerHeight / ROW_HEIGHT) + BUFFER_ROWS * 2);
  let endIndex = $derived(Math.min(data.length, startIndex + visibleCount));
  let visibleData = $derived(data.slice(startIndex, endIndex));
  let offsetY = $derived(startIndex * ROW_HEIGHT);

  function formatBytes(bytes) {
    if (!bytes || bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  function formatRate(rate) {
    if (!rate || rate === 0) return '-';
    if (rate >= 1000) return (rate / 1024).toFixed(1) + ' MB/s';
    return rate.toFixed(1) + ' KB/s';
  }

  function getStateIcon(state) {
    switch (state?.toLowerCase()) {
      case 'starting':
      case 'stopping':
        return LoaderCircle;
      case 'running':
        return Circle;
      case 'paused':
        return Pause;
      case 'idle':
        return Moon;
      case 'stopped':
        return Square;
      default:
        return Circle;
    }
  }

  function getStateColor(state) {
    switch (state?.toLowerCase()) {
      case 'starting':
      case 'stopping':
        return 'text-stat-ratio';
      case 'running':
        return 'text-stat-upload';
      case 'paused':
        return 'text-stat-ratio';
      case 'idle':
        return 'text-muted-foreground';
      case 'stopped':
        return 'text-muted-foreground';
      default:
        return 'text-muted-foreground';
    }
  }

  const columns = [
    { id: 'select', header: '', width: 32, sortable: false },
    { id: 'name', header: 'Name', width: 300, sortable: true },
    { id: 'totalSize', header: 'Size', width: 80, sortable: true },
    { id: 'progress', header: 'Progress', width: 110, sortable: true },
    { id: 'state', header: 'State', width: 70, sortable: true },
    { id: 'tags', header: 'Tags', width: 120, sortable: false },
    { id: 'uploaded', header: 'UL', width: 80, sortable: true },
    { id: 'downloaded', header: 'DL', width: 80, sortable: true },
    { id: 'ratio', header: 'Ratio', width: 55, sortable: true },
    { id: 'currentUploadRate', header: 'UL Rate', width: 90, sortable: true },
    { id: 'currentDownloadRate', header: 'DL Rate', width: 90, sortable: true },
    { id: 'seeders', header: 'S/L', width: 65, sortable: true },
  ];

  function handleRowClick(e, instance) {
    const currentIndex = data.findIndex(d => d.id === instance.id);
    if (e.shiftKey && lastSelectedIndex !== null && lastSelectedIndex !== currentIndex) {
      gridActions.selectRange(lastSelectedIndex, currentIndex);
    } else {
      gridActions.toggleSelect(instance.id);
    }
    lastSelectedIndex = currentIndex;
  }

  function handleSelectAll() {
    const allIds = data.map(d => d.id);
    const currentSelected = $selectedIds;
    const everySelected = allIds.length > 0 && allIds.every(id => currentSelected.has(id));

    if (everySelected) {
      gridActions.deselectAll();
    } else {
      gridActions.selectAll();
    }
  }

  function handleSort(col) {
    if (!col.sortable) return;
    gridActions.toggleSort(col.id);
  }

  let allSelected = $derived(data.length > 0 && data.every(d => $selectedIds.has(d.id)));
  let someSelected = $derived(data.some(d => $selectedIds.has(d.id)) && !allSelected);
</script>

<svelte:window onmousedown={handleWindowMousedown} onkeydown={handleWindowKeydown} />

<div
  bind:this={scrollContainer}
  bind:clientHeight={containerHeight}
  class="overflow-auto flex-1 border border-border rounded-lg"
  onscroll={onScroll}
>
  <table class="w-full text-xs table-fixed">
    <thead class="bg-muted/50 sticky top-0 z-10">
      <tr>
        {#each columns as col (col.id)}
          <th
            class={cn(
              'px-2 py-1 text-left text-[10px] font-semibold text-muted-foreground uppercase tracking-wider whitespace-nowrap',
              col.sortable && 'cursor-pointer select-none hover:text-foreground'
            )}
            style="width: {col.width}px"
            onclick={() => handleSort(col)}
          >
            {#if col.id === 'select'}
              <input
                type="checkbox"
                checked={allSelected}
                indeterminate={someSelected}
                onchange={handleSelectAll}
                class="h-3.5 w-3.5 rounded border-input accent-primary cursor-pointer"
              />
            {:else}
              <div class="flex items-center gap-1">
                <span>{col.header}</span>
                {#if $gridSort.column === col.id && $gridSort.direction === 'asc'}
                  <ArrowUp size={12} />
                {:else if $gridSort.column === col.id && $gridSort.direction === 'desc'}
                  <ArrowDown size={12} />
                {:else if col.sortable}
                  <ArrowUpDown size={12} class="opacity-30" />
                {/if}
              </div>
            {/if}
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#if data.length === 0}
        <tr>
          <td colspan={columns.length} class="px-2 py-12 text-center text-muted-foreground">
            No instances found. Import torrents to get started.
          </td>
        </tr>
      {:else}
        <!-- Spacer row for virtual scroll offset -->
        <tr style="height: {offsetY}px" aria-hidden="true">
          <td colspan={columns.length}></td>
        </tr>

        {#each visibleData as instance (instance.id)}
          {@const isSelected = $selectedIds.has(instance.id)}
          {@const StateIcon = getStateIcon(instance.state)}
          {@const completionPct = instance.torrentCompletion ?? 100}
          <tr
            class={cn(
              'border-t border-border/50 transition-colors cursor-pointer',
              isSelected ? 'bg-primary/10' : 'hover:bg-muted/30'
            )}
            style="height: {ROW_HEIGHT}px"
            onclick={(e) => handleRowClick(e, instance)}
            oncontextmenu={(e) => openContextMenu(e, instance)}
          >
            <!-- Select -->
            <td class="px-2 py-1 whitespace-nowrap">
              <input
                type="checkbox"
                checked={isSelected}
                onclick={e => e.stopPropagation()}
                onchange={() => gridActions.toggleSelect(instance.id)}
                class="h-3.5 w-3.5 rounded border-input accent-primary cursor-pointer"
              />
            </td>

            <!-- Name -->
            <td class="px-2 py-1 whitespace-nowrap overflow-hidden">
              <span class="text-foreground font-medium truncate" title={instance.name}>
                {instance.name}
              </span>
            </td>

            <!-- Size -->
            <td class="px-2 py-1 whitespace-nowrap">
              <span class="text-muted-foreground">{formatBytes(instance.totalSize)}</span>
            </td>

            <!-- Progress -->
            <td class="px-0.5 py-1 whitespace-nowrap">
              <div class="relative h-4 bg-muted rounded overflow-hidden" title="{completionPct.toFixed(1)}%">
                <div
                  class={cn(
                    'absolute inset-y-0 left-0 rounded transition-all',
                    completionPct >= 100 ? 'bg-stat-upload/60' : 'bg-stat-leecher/60'
                  )}
                  style="width: {Math.min(completionPct, 100)}%"
                ></div>
                <span class="absolute inset-0 flex items-center justify-center text-[10px] font-semibold text-foreground">
                  {completionPct.toFixed(1)}%
                </span>
              </div>
            </td>

            <!-- State -->
            <td class="px-2 py-1 whitespace-nowrap">
              <span class={cn('flex items-center gap-1', getStateColor(instance.state))}>
                {#if instance.state?.toLowerCase() === 'starting' || instance.state?.toLowerCase() === 'stopping'}
                  <StateIcon size={11} class="animate-spin" />
                {:else}
                  <StateIcon size={11} fill="currentColor" />
                {/if}
                <span class="capitalize">{instance.state}</span>
              </span>
            </td>

            <!-- Tags -->
            <td class="px-2 py-1 whitespace-nowrap">
              <div class="flex items-center gap-0.5 overflow-hidden max-w-[110px]">
                {#each (instance.tags || []).slice(0, 2) as tag (tag)}
                  <TagBadge {tag} compact />
                {/each}
                {#if (instance.tags || []).length > 2}
                  <span class="text-[10px] text-muted-foreground">+{instance.tags.length - 2}</span>
                {/if}
              </div>
            </td>

            <!-- Uploaded -->
            <td class="px-2 py-1 whitespace-nowrap">
              <span class="text-stat-upload">{formatBytes(instance.uploaded)}</span>
            </td>

            <!-- Downloaded -->
            <td class="px-2 py-1 whitespace-nowrap">
              <span class="text-stat-leecher">{formatBytes(instance.downloaded)}</span>
            </td>

            <!-- Ratio -->
            <td class="px-2 py-1 whitespace-nowrap">
              <span
                class={cn(
                  'font-semibold',
                  (instance.ratio || 0) >= 1 ? 'text-stat-upload' : 'text-stat-ratio'
                )}
              >
                {instance.ratio?.toFixed(2) || '0.00'}
              </span>
            </td>

            <!-- UL Rate -->
            <td class="px-2 py-1 whitespace-nowrap overflow-hidden">
              <span class="text-stat-upload">{formatRate(instance.currentUploadRate)}</span>
            </td>

            <!-- DL Rate -->
            <td class="px-2 py-1 whitespace-nowrap overflow-hidden">
              <span class="text-stat-leecher">{formatRate(instance.currentDownloadRate)}</span>
            </td>

            <!-- Seeders / Leechers -->
            <td class="px-2 py-1 whitespace-nowrap">
              <span class="text-stat-upload">{instance.seeders ?? '-'}</span>
              <span class="text-muted-foreground">/</span>
              <span class="text-stat-leecher">{instance.leechers ?? '-'}</span>
            </td>
          </tr>
        {/each}

        <!-- Bottom spacer for remaining rows -->
        <tr style="height: {totalHeight - offsetY - visibleData.length * ROW_HEIGHT}px" aria-hidden="true">
          <td colspan={columns.length}></td>
        </tr>
      {/if}
    </tbody>
  </table>
</div>

<!-- Context menu -->
{#if ctxVisible && ctxInstance}
  <div
    bind:this={menuEl}
    class="fixed z-50 min-w-[180px] rounded-lg border border-border bg-popover shadow-xl shadow-black/20 py-1"
    style={ctxStyle}
    role="menu"
    tabindex="-1"
    onmousedown={(e) => e.stopPropagation()}
  >
    <div class="px-3 py-1.5 border-b border-border/50 mb-1">
      <span class="text-xs text-muted-foreground font-medium truncate block max-w-[160px]" title={ctxInstance.name}>
        {ctxInstance.name}
      </span>
    </div>

    {#each ctxActions as action, i (i)}
      {#if action === null}
        <div class="h-px bg-border/50 my-1 mx-2"></div>
      {:else}
        {@const Icon = action.icon}
        <button
          class={cn(
            'w-full flex items-center gap-2.5 px-3 py-1.5 text-xs text-foreground',
            'hover:bg-accent/80 transition-colors cursor-pointer',
            'focus:outline-none focus:bg-accent/80'
          )}
          role="menuitem"
          onclick={() => handleCtxAction(action.id)}
        >
          <Icon size={14} class={action.color} />
          <span class={action.id === 'delete' ? 'text-stat-danger' : ''}>{action.label}</span>
        </button>
      {/if}
    {/each}
  </div>
{/if}

<!-- Delete confirmation dialog -->
{#if deleteConfirmVisible && deleteTarget}
  <div
    class="fixed inset-0 bg-black/50 z-[60] flex items-center justify-center p-4"
    onclick={cancelDelete}
    onkeydown={e => e.key === 'Escape' && cancelDelete()}
    role="dialog"
    aria-modal="true"
    aria-labelledby="delete-confirm-title"
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
          <h3 id="delete-confirm-title" class="text-sm font-semibold text-foreground">Delete Instance</h3>
          <p class="text-xs text-muted-foreground mt-0.5 break-all">
            {deleteTarget.name}
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
