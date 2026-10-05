<script>
  import { cn } from '$lib/core/utils.js';
  import { getGridLivePeers, getGridLiveRate } from '$lib/grid/gridMetrics.js';
  import { getTrackerIssue } from '$lib/core/status.js';
  import { formatBytes, formatRate } from '$lib/core/format.js';
  import { getZoom } from '$lib/core/zoomStore.svelte.js';
  import { getWindow } from '$lib/core/virtual.js';
  import ConfirmDialog from '../common/ConfirmDialog.svelte';
  import StatusBadge from '../common/StatusBadge.svelte';
  import { selectedIds, gridActions, gridSort } from '$lib/grid/gridStore.js';
  import TagBadge from './TagBadge.svelte';
  import {
    ArrowUp,
    ArrowDown,
    ArrowUpDown,
    Play,
    Pause,
    Trash2,
    Copy,
    Pencil,
    AlertTriangle,
    Maximize2,
    Square,
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
    items.push({
      id: 'edit',
      label: 'Open detail',
      icon: Pencil,
      color: 'text-foreground',
    });
    items.push({
      id: 'copy_hash',
      label: 'Copy info hash',
      icon: Copy,
      color: 'text-muted-foreground',
    });
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

  const ROW_HEIGHT_REM = 1.75;
  const BUFFER_ROWS = 10;

  // Rows and columns are defined in rem so the interface zoom scales them with the text.
  // Virtualization math needs the rendered pixel height, re-measured when zoom changes.
  let rootFontPx = $derived.by(() => {
    getZoom();
    if (typeof document === 'undefined') return 16;
    const value = parseFloat(getComputedStyle(document.documentElement).fontSize);
    return Number.isFinite(value) ? value : 16;
  });
  let rowHeight = $derived(ROW_HEIGHT_REM * rootFontPx);

  let scrollContainer = $state(null);
  let scrollTop = $state(0);
  let containerHeight = $state(0);

  function onScroll(e) {
    scrollTop = e.target.scrollTop;
  }

  let view = $derived(
    getWindow({
      total: data.length,
      scrollTop,
      viewportHeight: containerHeight,
      rowHeight,
      buffer: BUFFER_ROWS,
    })
  );
  let visibleData = $derived(data.slice(view.startIndex, view.endIndex));
  let offsetY = $derived(view.offsetY);
  let totalHeight = $derived(view.totalHeight);

  function getIssueMessage(instance) {
    return getTrackerIssue(instance)?.statusMessage || null;
  }

  function getPrimaryAction(state) {
    switch (state?.toLowerCase()) {
      case 'stopped':
        return { id: 'start', icon: Play, title: 'Start', color: 'text-stat-upload' };
      case 'running':
      case 'idle':
        return { id: 'pause', icon: Pause, title: 'Pause', color: 'text-stat-ratio' };
      case 'paused':
        return { id: 'resume', icon: Play, title: 'Resume', color: 'text-stat-upload' };
      case 'starting':
        return { id: 'stop', icon: Square, title: 'Stop', color: 'text-stat-danger' };
      default:
        return null;
    }
  }

  const columns = [
    { id: 'select', header: '', width: 1.5, sortable: false },
    { id: 'name', header: 'Name', width: 14, sortable: true },
    { id: 'state', header: 'State', width: 5, sortable: true },
    { id: 'progress', header: 'Progress', width: 5.75, sortable: true },
    { id: 'totalSize', header: 'Size', width: 4.25, sortable: true },
    { id: 'uploaded', header: 'UL', width: 4.25, sortable: true },
    { id: 'downloaded', header: 'DL', width: 4.25, sortable: true },
    { id: 'ratio', header: 'Ratio', width: 3, sortable: true },
    { id: 'currentUploadRate', header: 'UL Rate', width: 4.75, sortable: true },
    { id: 'currentDownloadRate', header: 'DL Rate', width: 4.75, sortable: true },
    { id: 'seeders', header: 'S/L', width: 3.5, sortable: true },
    { id: 'tags', header: 'Tags', width: 4.75, sortable: false },
    { id: 'actions', header: '', width: 3, sortable: false },
  ];

  function setItemSelected(id, shouldSelect) {
    selectedIds.update(s => {
      const next = new Set(s);
      if (shouldSelect) {
        next.add(id);
      } else {
        next.delete(id);
      }
      return next;
    });
  }

  function setRangeSelected(fromIdx, toIdx, shouldSelect) {
    if (fromIdx == null || toIdx == null) return;

    const start = Math.max(0, Math.min(fromIdx, toIdx));
    const end = Math.min(data.length - 1, Math.max(fromIdx, toIdx));

    selectedIds.update(s => {
      const next = new Set(s);
      for (let i = start; i <= end; i++) {
        const id = data[i]?.id;
        if (id == null) continue;
        if (shouldSelect) {
          next.add(id);
        } else {
          next.delete(id);
        }
      }
      return next;
    });
  }

  function handleRowClick(e, instance) {
    const currentIndex = data.findIndex(d => d.id === instance.id);
    if (currentIndex < 0) return;

    const shouldSelect = !$selectedIds.has(instance.id);
    if (e.shiftKey && lastSelectedIndex !== null && lastSelectedIndex !== currentIndex) {
      setRangeSelected(lastSelectedIndex, currentIndex, shouldSelect);
    } else {
      setItemSelected(instance.id, shouldSelect);
    }
    lastSelectedIndex = currentIndex;
  }

  function handleCheckboxChange(e, instance) {
    const currentIndex = data.findIndex(d => d.id === instance.id);
    if (currentIndex < 0) return;

    const shouldSelect = Boolean(e.currentTarget.checked);
    if (e.shiftKey && lastSelectedIndex !== null && lastSelectedIndex !== currentIndex) {
      setRangeSelected(lastSelectedIndex, currentIndex, shouldSelect);
    } else {
      setItemSelected(instance.id, shouldSelect);
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

  $effect(() => {
    if (lastSelectedIndex === null) return;
    if (lastSelectedIndex >= data.length) {
      lastSelectedIndex = null;
    }
  });
</script>

<svelte:window onmousedown={handleWindowMousedown} onkeydown={handleWindowKeydown} />

<div
  bind:this={scrollContainer}
  bind:clientHeight={containerHeight}
  class="min-h-0 min-w-0 flex-1 overflow-auto"
  onscroll={onScroll}
>
  <table class="w-full table-fixed border-separate border-spacing-0 text-xs">
    <thead class="sticky top-0 z-20">
      <tr>
        {#each columns as col (col.id)}
          <th
            class={cn(
              'whitespace-nowrap border-b border-border bg-card px-2 py-1 text-left text-[0.625rem] font-semibold uppercase tracking-wider text-muted-foreground',
              col.sortable && 'cursor-pointer select-none hover:text-foreground',
              col.id === 'select' && 'sticky left-0 z-30',
              col.id === 'name' && 'sticky left-[1.5rem] z-30 border-r'
            )}
            style="width: {col.width}rem"
            onclick={() => handleSort(col)}
          >
            {#if col.id === 'select'}
              <input
                type="checkbox"
                checked={allSelected}
                indeterminate={someSelected}
                onchange={handleSelectAll}
                class="h-3.5 w-3.5 cursor-pointer rounded-none border-input accent-primary"
              />
            {:else if col.header}
              <div class="flex items-center gap-1">
                <span>{col.header}</span>
                {#if $gridSort.column === col.id && $gridSort.direction === 'asc'}
                  <ArrowUp size={11} />
                {:else if $gridSort.column === col.id && $gridSort.direction === 'desc'}
                  <ArrowDown size={11} />
                {:else if col.sortable}
                  <ArrowUpDown size={11} class="opacity-30" />
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
          {@const completionPct = instance.torrentCompletion ?? 100}
          {@const issueMessage = getIssueMessage(instance)}
          {@const liveRateUp = getGridLiveRate(instance.state, instance.currentUploadRate)}
          {@const liveRateDown = getGridLiveRate(instance.state, instance.currentDownloadRate)}
          {@const livePeers = getGridLivePeers(instance.state, instance.seeders, instance.leechers)}
          {@const primary = getPrimaryAction(instance.state)}
          <tr
            class={cn(
              'group cursor-pointer bg-card transition-colors hover:bg-muted',
              isSelected && 'bg-selected'
            )}
            style="height: {ROW_HEIGHT_REM}rem"
            onclick={e => handleRowClick(e, instance)}
            oncontextmenu={e => openContextMenu(e, instance)}
          >
            <!-- Select -->
            <td
              class="sticky left-0 z-10 border-b border-border/60 bg-inherit whitespace-nowrap px-2 py-1"
            >
              <input
                type="checkbox"
                checked={isSelected}
                onclick={e => e.stopPropagation()}
                onchange={e => handleCheckboxChange(e, instance)}
                class="h-3.5 w-3.5 cursor-pointer rounded-none border-input accent-primary"
              />
            </td>

            <!-- Name -->
            <td
              class="sticky left-[1.5rem] z-10 select-none overflow-hidden whitespace-nowrap border-b border-r border-border/60 bg-inherit px-2 py-1"
              ondblclick={e => {
                e.preventDefault();
                e.stopPropagation();
                oncontextaction('edit', instance);
              }}
            >
              <span class="inline-flex max-w-full items-center gap-1.5">
                <span class="truncate font-medium text-foreground" title={instance.name}>
                  {instance.name}
                </span>
                {#if issueMessage}
                  <span
                    class="flex-shrink-0 text-stat-ratio"
                    title={issueMessage}
                    aria-label={issueMessage}
                  >
                    <AlertTriangle size={11} />
                  </span>
                {/if}
              </span>
            </td>

            <!-- State -->
            <td class="whitespace-nowrap border-b border-border/60 px-2 py-1">
              <StatusBadge state={instance.state} size="xs" />
            </td>

            <!-- Progress -->
            <td class="whitespace-nowrap border-b border-border/60 px-2 py-1">
              <div class="flex items-center gap-1.5" title="{completionPct.toFixed(1)}%">
                <div class="h-1.5 min-w-0 flex-1 bg-muted">
                  <div
                    class={cn(
                      'h-full',
                      completionPct >= 100 ? 'bg-stat-upload' : 'bg-stat-leecher'
                    )}
                    style="width: {Math.min(completionPct, 100)}%"
                  ></div>
                </div>
                <span
                  class="w-8 shrink-0 text-right tabular-nums text-[0.625rem] text-muted-foreground"
                >
                  {completionPct.toFixed(0)}%
                </span>
              </div>
            </td>

            <!-- Size -->
            <td
              class="whitespace-nowrap border-b border-border/60 px-2 py-1 tabular-nums text-muted-foreground"
            >
              {formatBytes(instance.totalSize)}
            </td>

            <!-- Uploaded -->
            <td
              class="whitespace-nowrap border-b border-border/60 px-2 py-1 tabular-nums text-stat-upload"
            >
              {formatBytes(instance.uploaded)}
            </td>

            <!-- Downloaded -->
            <td
              class="whitespace-nowrap border-b border-border/60 px-2 py-1 tabular-nums text-stat-leecher"
            >
              {formatBytes(instance.downloaded)}
            </td>

            <!-- Ratio -->
            <td class="whitespace-nowrap border-b border-border/60 px-2 py-1">
              <span
                class={cn(
                  'tabular-nums font-semibold',
                  (instance.ratio || 0) >= 1 ? 'text-stat-upload' : 'text-stat-ratio'
                )}
              >
                {instance.ratio?.toFixed(2) || '0.00'}
              </span>
            </td>

            <!-- UL Rate -->
            <td
              class="overflow-hidden whitespace-nowrap border-b border-border/60 px-2 py-1 tabular-nums text-stat-upload"
            >
              {formatRate(liveRateUp)}
            </td>

            <!-- DL Rate -->
            <td
              class="overflow-hidden whitespace-nowrap border-b border-border/60 px-2 py-1 tabular-nums text-stat-leecher"
            >
              {formatRate(liveRateDown)}
            </td>

            <!-- Seeders / Leechers -->
            <td class="whitespace-nowrap border-b border-border/60 px-2 py-1 tabular-nums">
              <span class="text-stat-upload">{livePeers.seeders ?? '-'}</span>
              <span class="text-muted-foreground">/</span>
              <span class="text-stat-leecher">{livePeers.leechers ?? '-'}</span>
            </td>

            <!-- Tags -->
            <td class="whitespace-nowrap border-b border-border/60 px-2 py-1">
              <div class="flex max-w-[4.25rem] items-center gap-0.5 overflow-hidden">
                {#each (instance.tags || []).slice(0, 2) as tag (tag)}
                  <TagBadge {tag} compact />
                {/each}
                {#if (instance.tags || []).length > 2}
                  <span class="text-[0.625rem] text-muted-foreground"
                    >+{instance.tags.length - 2}</span
                  >
                {/if}
              </div>
            </td>

            <!-- Actions -->
            <td class="whitespace-nowrap border-b border-border/60 px-1 py-1">
              <div
                class="flex items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100"
              >
                {#if primary}
                  <button
                    class={cn(
                      'inline-flex h-5 w-5 items-center justify-center transition-colors hover:bg-muted cursor-pointer',
                      primary.color
                    )}
                    title={primary.title}
                    aria-label={primary.title}
                    onclick={e => {
                      e.stopPropagation();
                      oncontextaction(primary.id, instance);
                    }}
                  >
                    <primary.icon size={11} fill="currentColor" />
                  </button>
                {/if}
                <button
                  class="inline-flex h-5 w-5 items-center justify-center text-muted-foreground transition-colors hover:bg-muted hover:text-foreground cursor-pointer"
                  title="Open detail"
                  aria-label="Open detail"
                  onclick={e => {
                    e.stopPropagation();
                    oncontextaction('edit', instance);
                  }}
                >
                  <Maximize2 size={11} />
                </button>
              </div>
            </td>
          </tr>
        {/each}

        <!-- Bottom spacer for remaining rows -->
        <tr
          style="height: {totalHeight - offsetY - visibleData.length * rowHeight}px"
          aria-hidden="true"
        >
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
    class="fixed z-50 min-w-[11.25rem] border border-border bg-popover py-1"
    style={ctxStyle}
    role="menu"
    tabindex="-1"
    onmousedown={e => e.stopPropagation()}
  >
    <div class="mb-1 border-b border-border px-2.5 py-1">
      <span
        class="block max-w-[10rem] truncate text-[0.625rem] text-muted-foreground"
        title={ctxInstance.name}
      >
        {ctxInstance.name}
      </span>
    </div>

    {#each ctxActions as action, i (i)}
      {#if action === null}
        <div class="mx-2 my-1 h-px bg-border"></div>
      {:else}
        {@const Icon = action.icon}
        <button
          class={cn(
            'flex w-full items-center gap-2.5 px-2.5 py-1 text-[0.6875rem] text-foreground transition-colors',
            'hover:bg-muted focus:bg-muted focus:outline-none cursor-pointer'
          )}
          role="menuitem"
          onclick={() => handleCtxAction(action.id)}
        >
          <Icon size={13} class={action.color} />
          <span class={action.id === 'delete' ? 'text-stat-danger' : ''}>{action.label}</span>
        </button>
      {/if}
    {/each}
  </div>
{/if}

<ConfirmDialog
  bind:open={deleteConfirmVisible}
  title="Delete Instance"
  message={`${deleteTarget?.name || ''}\n\nThis action cannot be undone.`}
  confirmLabel="Delete"
  kind="danger"
  titleId="delete-confirm-title"
  onCancel={cancelDelete}
  onConfirm={confirmDelete}
/>
