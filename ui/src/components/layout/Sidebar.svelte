<script>
  import { instances, activeInstanceId, instanceActions } from '$lib/instanceStore.js';
  import { viewMode, gridInstances, selectedIds, gridFilters } from '$lib/gridStore.js';
  import { cn } from '$lib/utils.js';
  import Button from '$lib/components/ui/button.svelte';
  import AboutDialog from './AboutDialog.svelte';
  import SettingsDialog from './SettingsDialog.svelte';
  import NetworkStatus from './NetworkStatus.svelte';
  import WatchFolder from './WatchFolder.svelte';
  import {
    PanelLeftClose,
    Play,
    Square,
    Plus,
    Circle,
    Pause,
    X,
    FolderOpen,
    Settings,
    Info,
    Moon,
    List,
    LayoutGrid,
    CheckCircle,
    LoaderCircle,
  } from '@lucide/svelte';

  let {
    onStartAll = () => {},
    onStopAll = () => {},
    onPauseAll = () => {},
    onResumeAll = () => {},
    isOpen = $bindable(false),
    isCollapsed = $bindable(false),
  } = $props();

  let showAbout = $state(false);
  let showSettings = $state(false);
  let isGridMode = $derived($viewMode === 'grid');

  // Derived state
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

  // Grid mode aggregate stats
  let gridStats = $derived(() => {
    let totalUploaded = 0;
    let totalDownloaded = 0;
    let totalSize = 0;
    let totalUploadRate = 0;
    let totalDownloadRate = 0;
    let downloadingCount = 0;
    const stateCounts = {};

    for (const inst of $gridInstances) {
      totalUploaded += inst.uploaded || 0;
      totalDownloaded += inst.downloaded || 0;
      totalSize += inst.totalSize || 0;
      totalUploadRate += inst.currentUploadRate || 0;
      totalDownloadRate += inst.currentDownloadRate || 0;
      if ((inst.torrentCompletion ?? 100) < 100) downloadingCount++;
      const s = inst.state || 'stopped';
      stateCounts[s] = (stateCounts[s] || 0) + 1;
    }

    const ratio = totalDownloaded > 0 ? totalUploaded / totalDownloaded : 0;
    return {
      totalUploaded,
      totalDownloaded,
      totalSize,
      totalUploadRate,
      totalDownloadRate,
      ratio,
      stateCounts,
      total: $gridInstances.length,
      downloadingCount,
    };
  });

  // Grid selection aggregate stats
  let selectionStats = $derived(() => {
    const ids = $selectedIds;
    if (ids.size === 0) return null;
    let uploaded = 0;
    let downloaded = 0;
    let size = 0;
    for (const inst of $gridInstances) {
      if (ids.has(inst.id)) {
        uploaded += inst.uploaded || 0;
        downloaded += inst.downloaded || 0;
        size += inst.totalSize || 0;
      }
    }
    return { count: ids.size, uploaded, downloaded, size };
  });

  let activeStateFilter = $derived($gridFilters.stateFilter);

  const stateConfig = [
    { key: 'running', label: 'Running', icon: Circle, color: 'text-stat-upload' },
    { key: 'paused', label: 'Paused', icon: Pause, color: 'text-stat-ratio' },
    { key: 'idle', label: 'Idle', icon: Moon, color: 'text-violet-500' },
    { key: 'stopped', label: 'Stopped', icon: Square, color: 'text-muted-foreground' },
    { key: 'starting', label: 'Starting', icon: LoaderCircle, color: 'text-primary' },
    { key: 'stopping', label: 'Stopping', icon: LoaderCircle, color: 'text-stat-danger' },
  ];

  const quickFilters = [
    { key: 'all', label: 'All' },
    { key: 'running', label: 'Running' },
    { key: 'stopped', label: 'Stopped' },
    { key: 'paused', label: 'Paused' },
  ];

  function setStateFilter(state) {
    gridFilters.update(f => ({ ...f, stateFilter: state }));
  }

  function formatRate(rate) {
    if (!rate || rate === 0) return '0 KB/s';
    if (rate >= 1000) return (rate / 1024).toFixed(1) + ' MB/s';
    return rate.toFixed(1) + ' KB/s';
  }

  function formatRateCompact(rate) {
    if (!rate || rate === 0) return '0';
    if (rate >= 1000) return (rate / 1024).toFixed(0) + 'M';
    return rate.toFixed(0) + 'K';
  }

  // Total stats across all instances
  let totalStats = $derived(() => {
    let totalUploaded = 0;
    let totalDownloaded = 0;
    let runningCount = 0;

    for (const inst of $instances) {
      if (inst.stats) {
        totalUploaded += inst.stats.uploaded || 0;
        totalDownloaded += inst.stats.downloaded || 0;
      }
      if (inst.isRunning) runningCount++;
    }

    return {
      uploaded: totalUploaded,
      downloaded: totalDownloaded,
      running: runningCount,
      total: $instances.length,
    };
  });

  // Format bytes to human readable
  function formatBytes(bytes) {
    if (!bytes || bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  // Format bytes compact (for sidebar)
  function formatBytesCompact(bytes) {
    if (!bytes || bytes === 0) return '0';
    const k = 1024;
    const sizes = ['B', 'K', 'M', 'G', 'T'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + sizes[i];
  }

  // Get progress percentage for stop conditions
  function getStopConditionProgress(instance) {
    if (!instance.stats) return null;

    const stats = instance.stats;
    let maxProgress = 0;
    let activeCondition = null;

    // Check ratio progress
    if (instance.stopAtRatioEnabled && instance.stopAtRatio > 0) {
      const progress = Math.min(100, (stats.session_ratio / instance.stopAtRatio) * 100);
      if (progress > maxProgress) {
        maxProgress = progress;
        activeCondition = 'ratio';
      }
    }

    // Check uploaded progress
    if (instance.stopAtUploadedEnabled && instance.stopAtUploadedGB > 0) {
      const targetBytes = instance.stopAtUploadedGB * 1024 * 1024 * 1024;
      const progress = Math.min(100, (stats.session_uploaded / targetBytes) * 100);
      if (progress > maxProgress) {
        maxProgress = progress;
        activeCondition = 'uploaded';
      }
    }

    // Check downloaded progress
    if (instance.stopAtDownloadedEnabled && instance.stopAtDownloadedGB > 0) {
      const targetBytes = instance.stopAtDownloadedGB * 1024 * 1024 * 1024;
      const progress = Math.min(100, (stats.session_downloaded / targetBytes) * 100);
      if (progress > maxProgress) {
        maxProgress = progress;
        activeCondition = 'downloaded';
      }
    }

    // Check seed time progress
    if (instance.stopAtSeedTimeEnabled && instance.stopAtSeedTimeHours > 0) {
      const targetSeconds = instance.stopAtSeedTimeHours * 3600;
      const elapsedSeconds = stats.elapsed_time?.secs || 0;
      const progress = Math.min(100, (elapsedSeconds / targetSeconds) * 100);
      if (progress > maxProgress) {
        maxProgress = progress;
        activeCondition = 'time';
      }
    }

    if (activeCondition) {
      return { progress: maxProgress, condition: activeCondition };
    }
    return null;
  }

  function getInstanceLabel(instance) {
    if (instance.torrent) {
      const name = instance.torrent.name;
      return name.length > 20 ? name.substring(0, 20) + '...' : name;
    }
    return `Instance ${instance.id}`;
  }

  function getInstanceStatus(instance) {
    if (instance.isRunning) {
      if (instance.isPaused) return 'paused';
      if (instance.stats?.is_idling) return 'idling';
      return 'running';
    }
    return 'idle';
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

  async function handleForceRemoveInstance(event, id, name) {
    event.stopPropagation();
    event.preventDefault();

    const confirmed = confirm(
      `Force delete "${name || 'this instance'}"?\n\n` +
        'This instance was created from the watch folder but the torrent file may no longer exist. ' +
        'Click OK to permanently remove it.'
    );

    if (!confirmed) return;

    try {
      await instanceActions.removeInstance(id, true); // force=true
    } catch (error) {
      console.error('Failed to force remove instance:', error);
    }
  }

  function handleSelectInstance(id) {
    try {
      instanceActions.selectInstance(id);
    } catch (error) {
      console.error('Error switching instance:', error);
    }
  }

  function handleStartAll() {
    onStartAll();
  }

  function handleStopAll() {
    onStopAll();
  }

  function handlePauseAll() {
    onPauseAll();
  }

  function handleResumeAll() {
    onResumeAll();
  }
</script>

<!-- Mobile Overlay -->
{#if isOpen}
  <button
    class="fixed inset-0 bg-black/50 z-40 lg:hidden border-0 p-0 cursor-default"
    onclick={() => (isOpen = false)}
    aria-label="Close sidebar"
  ></button>
{/if}

<aside
  class={cn(
    'bg-card border-r border-border flex flex-col h-screen transition-all duration-300 ease-in-out',
    'fixed lg:sticky top-0 z-50 lg:z-auto',
    // Mobile: slide in/out
    isOpen ? 'translate-x-0' : '-translate-x-full lg:translate-x-0',
    // Desktop: collapse/expand
    isCollapsed ? 'w-16' : 'w-64'
  )}
>
  <!-- Sidebar Header -->
  <div class="p-4 border-b border-border">
    <div class="flex items-center justify-between mb-3">
      <h2
        class={cn(
          'text-lg font-semibold text-foreground transition-opacity duration-200',
          isCollapsed && 'lg:opacity-0 lg:w-0 lg:overflow-hidden'
        )}
      >
        {isGridMode ? 'Grid Mode' : 'Instances'}
      </h2>

      <!-- Desktop Toggle Button -->
      <button
        class="hidden lg:block p-1 rounded hover:bg-muted"
        onclick={() => (isCollapsed = !isCollapsed)}
        title={isCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
      >
        <span class={cn('transition-transform duration-300 block', isCollapsed && 'rotate-180')}>
          <PanelLeftClose size={16} />
        </span>
      </button>
    </div>

    <!-- View Mode Toggle -->
    {#if !isCollapsed}
      <div class="flex gap-1 bg-muted/50 rounded-lg p-1 mb-3">
        <button
          class={cn(
            'flex-1 flex items-center justify-center gap-1.5 px-2 py-1.5 rounded-md text-xs font-medium transition-colors border-0 cursor-pointer',
            !isGridMode
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground bg-transparent'
          )}
          onclick={() => viewMode.set('standard')}
          title="Standard view - manage individual instances"
        >
          <List size={12} />
          Standard
        </button>
        <button
          class={cn(
            'flex-1 flex items-center justify-center gap-1.5 px-2 py-1.5 rounded-md text-xs font-medium transition-colors border-0 cursor-pointer',
            isGridMode
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground bg-transparent'
          )}
          onclick={() => viewMode.set('grid')}
          title="Grid view - manage many instances at once"
        >
          <LayoutGrid size={12} />
          Grid
        </button>
      </div>
    {/if}

    <!-- Standard Mode Content -->
    {#if !isGridMode}
    <!-- Total Stats Summary -->
    {#if !isCollapsed && (totalStats().uploaded > 0 || totalStats().downloaded > 0)}
      <div class="mb-3 p-2 bg-muted/50 rounded-lg text-xs">
        <div class="flex justify-between text-muted-foreground mb-1">
          <span>Total Uploaded</span>
          <span class="font-semibold text-stat-upload">↑ {formatBytes(totalStats().uploaded)}</span>
        </div>
        <div class="flex justify-between text-muted-foreground mb-1">
          <span>Total Downloaded</span>
          <span class="font-semibold text-stat-leecher"
            >↓ {formatBytes(totalStats().downloaded)}</span
          >
        </div>
        <div class="flex justify-between text-muted-foreground">
          <span>Running</span>
          <span class="font-semibold text-foreground"
            >{totalStats().running}/{totalStats().total}</span
          >
        </div>
      </div>
    {/if}

    <!-- Bulk Actions -->
    {#if hasMultipleInstancesWithTorrents}
      <div class={cn('flex flex-wrap gap-2 mb-3', isCollapsed && 'lg:flex-col')}>
        <Button
          onclick={handleStartAll}
          disabled={!hasStoppedInstancesWithTorrents}
          size="sm"
          variant="default"
          class={cn('gap-1', isCollapsed ? 'lg:w-full lg:px-2' : 'flex-1')}
          title="Start all instances"
        >
          {#snippet children()}
            <Play size={12} fill="currentColor" />
            <span class={cn(isCollapsed && 'lg:hidden')}>Start All</span>
          {/snippet}
        </Button>

        <Button
          onclick={handleStopAll}
          disabled={!hasRunningInstances}
          size="sm"
          class={cn(
            'gap-1 bg-stat-danger hover:bg-stat-danger/90 text-white shadow-lg shadow-stat-danger/25',
            isCollapsed ? 'lg:w-full lg:px-2' : 'flex-1'
          )}
          title="Stop all instances"
        >
          {#snippet children()}
            <Square size={12} fill="currentColor" />
            <span class={cn(isCollapsed && 'lg:hidden')}>Stop All</span>
          {/snippet}
        </Button>

        <Button
          onclick={handlePauseAll}
          disabled={!hasUnpausedRunningInstances}
          size="sm"
          class={cn(
            'gap-1 bg-stat-ratio hover:bg-stat-ratio/90 text-white shadow-lg shadow-stat-ratio/25',
            isCollapsed ? 'lg:w-full lg:px-2' : 'flex-1'
          )}
          title="Pause all running instances"
        >
          {#snippet children()}
            <Pause size={12} fill="currentColor" />
            <span class={cn(isCollapsed && 'lg:hidden')}>Pause All</span>
          {/snippet}
        </Button>

        <Button
          onclick={handleResumeAll}
          disabled={!hasPausedInstances}
          size="sm"
          variant="secondary"
          class={cn('gap-1', isCollapsed ? 'lg:w-full lg:px-2' : 'flex-1')}
          title="Resume all paused instances"
        >
          {#snippet children()}
            <Play size={12} fill="currentColor" />
            <span class={cn(isCollapsed && 'lg:hidden')}>Resume All</span>
          {/snippet}
        </Button>
      </div>
    {/if}

    <!-- Add Instance Button -->
    <Button
      onclick={handleAddInstance}
      size="sm"
      class={cn('w-full gap-2', isCollapsed && 'lg:px-2')}
      title="Add new instance"
    >
      {#snippet children()}
        <Plus size={16} strokeWidth={2.5} />
        <span class={cn(isCollapsed && 'lg:hidden')}>New Instance</span>
      {/snippet}
    </Button>
    {/if}
  </div>

  <!-- Instance List (standard mode only) -->
  {#if !isGridMode}
  <div class="flex-1 overflow-y-auto min-h-0">
    {#each $instances as instance (instance.id)}
      {@const status = getInstanceStatus(instance)}
      {@const isActive = $activeInstanceId === instance.id}
      {@const stopProgress = getStopConditionProgress(instance)}

      <div
        class={cn(
          'w-full px-4 py-3 border-l-4 transition-all text-left cursor-pointer',
          isActive ? 'bg-muted border-l-primary' : 'border-l-transparent hover:bg-muted/50',
          isCollapsed ? 'lg:px-2' : ''
        )}
        onclick={() => handleSelectInstance(instance.id)}
        onkeydown={e => e.key === 'Enter' && handleSelectInstance(instance.id)}
        role="button"
        tabindex="0"
        title={instance.torrent ? instance.torrent.name : `Instance ${instance.id}`}
      >
        <div class="flex items-center justify-between gap-2">
          <div
            class={cn('flex items-center gap-2 min-w-0 flex-1', isCollapsed && 'lg:justify-center')}
          >
            <!-- Status Indicator -->
            <span
              class={cn(
                'flex-shrink-0',
                status === 'idle' && 'text-muted-foreground',
                status === 'running' && 'text-stat-upload animate-pulse-slow',
                status === 'idling' && 'text-violet-500',
                status === 'paused' && 'text-stat-ratio'
              )}
            >
              {#if status === 'running'}
                <Circle size={10} fill="currentColor" />
              {:else if status === 'idling'}
                <Moon size={10} fill="currentColor" />
              {:else if status === 'paused'}
                <Pause size={10} fill="currentColor" />
              {:else}
                <Circle size={10} fill="currentColor" class="opacity-30" />
              {/if}
            </span>

            <!-- Instance Name -->
            <span
              class={cn(
                'text-sm truncate transition-opacity duration-200',
                isActive ? 'font-semibold text-foreground' : 'text-muted-foreground',
                isCollapsed && 'lg:hidden lg:w-0 lg:opacity-0'
              )}
            >
              {getInstanceLabel(instance)}
            </span>
          </div>

          <!-- Ratio Badge (when running or has stats) -->
          {#if !isCollapsed && instance.stats && instance.stats.ratio > 0}
            <span
              class={cn(
                'flex-shrink-0 text-xs font-bold px-1.5 py-0.5 rounded',
                instance.stats.ratio >= 1
                  ? 'bg-stat-upload/20 text-stat-upload'
                  : 'bg-stat-ratio/20 text-stat-ratio'
              )}
              title="Current ratio"
            >
              {instance.stats.ratio.toFixed(2)}x
            </span>
          {/if}

          <!-- Close Button -->
          {#if !isCollapsed && instance.source !== 'watch_folder'}
            <button
              class="flex-shrink-0 p-1 rounded hover:bg-destructive/20 group bg-transparent border-0 cursor-pointer"
              onclick={e => handleRemoveInstance(e, instance.id)}
              title="Close instance"
              aria-label="Close instance"
            >
              <X
                size={12}
                strokeWidth={2.5}
                class="text-muted-foreground group-hover:text-destructive transition-colors"
              />
            </button>
          {:else if instance.source === 'watch_folder' && !isCollapsed}
            <!-- Watch folder instance: show folder icon + force delete button -->
            <div class="flex items-center gap-1">
              <span class="flex-shrink-0 text-muted-foreground" title="From watch folder">
                <FolderOpen size={12} />
              </span>
              <button
                class="flex-shrink-0 p-1 rounded hover:bg-destructive/20 group bg-transparent border-0 cursor-pointer opacity-50 hover:opacity-100"
                onclick={e => handleForceRemoveInstance(e, instance.id, instance.name)}
                title="Force delete (file may be missing)"
                aria-label="Force delete instance"
              >
                <X
                  size={10}
                  strokeWidth={2.5}
                  class="text-muted-foreground group-hover:text-destructive transition-colors"
                />
              </button>
            </div>
          {/if}
        </div>

        <!-- Stats Row (when not collapsed and has stats) -->
        {#if !isCollapsed && instance.stats && instance.isRunning}
          <div class="mt-1.5 flex items-center gap-3 text-xs text-muted-foreground pl-5">
            <span class="text-stat-upload" title="Session uploaded">
              ↑ {formatBytesCompact(instance.stats.session_uploaded)}
            </span>
            <span class="text-stat-leecher" title="Session downloaded">
              ↓ {formatBytesCompact(instance.stats.session_downloaded)}
            </span>
            {#if instance.stats.current_upload_rate > 0}
              <span class="text-muted-foreground/70" title="Upload speed">
                {instance.stats.current_upload_rate.toFixed(1)} KB/s
              </span>
            {/if}
          </div>
        {/if}

        <!-- Progress Bar (when stop condition is active) -->
        {#if !isCollapsed && stopProgress && instance.isRunning}
          <div class="mt-2 pl-5">
            <div class="h-1 bg-muted rounded-full overflow-hidden">
              <div
                class={cn(
                  'h-full rounded-full transition-all duration-300',
                  stopProgress.progress >= 100
                    ? 'bg-stat-upload'
                    : stopProgress.progress >= 75
                      ? 'bg-stat-ratio'
                      : 'bg-primary'
                )}
                style="width: {Math.min(100, stopProgress.progress)}%"
              ></div>
            </div>
            <div class="mt-0.5 text-[10px] text-muted-foreground/70">
              {stopProgress.progress.toFixed(0)}% to target
            </div>
          </div>
        {/if}
      </div>
    {/each}
  </div>
  {:else}
    <!-- Grid Mode Sidebar Content -->
    <div class="flex-1 overflow-y-auto min-h-0">
      {#if isCollapsed}
        <!-- Collapsed: compact aggregate rates -->
        <div class="flex flex-col items-center gap-1 py-3 px-1">
          <span class="text-stat-upload text-[10px] font-bold" title="Total upload rate">
            ↑{formatRateCompact(gridStats().totalUploadRate)}
          </span>
          <span class="text-stat-leecher text-[10px] font-bold" title="Total download rate">
            ↓{formatRateCompact(gridStats().totalDownloadRate)}
          </span>
          {#if gridStats().total > 0}
            <span class="text-muted-foreground text-[10px] mt-1" title="{gridStats().total} instances">
              {gridStats().total}
            </span>
          {/if}
        </div>
      {:else}
        <div class="p-3 space-y-3">

          <!-- Aggregate Rate Display -->
          <div class="p-3 bg-muted/50 rounded-lg">
            <div class="flex items-center justify-between mb-2">
              <span class="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">Live Rates</span>
              {#if gridStats().stateCounts['running'] > 0}
                <span class="flex items-center gap-1 text-[10px] text-stat-upload">
                  <Circle size={6} fill="currentColor" class="animate-pulse-slow" />
                  {gridStats().stateCounts['running']} active
                </span>
              {/if}
            </div>
            <div class="flex items-baseline gap-2">
              <span class="text-lg font-bold text-stat-upload">↑ {formatRate(gridStats().totalUploadRate)}</span>
            </div>
            <div class="flex items-baseline gap-2 mt-0.5">
              <span class="text-lg font-bold text-stat-leecher">↓ {formatRate(gridStats().totalDownloadRate)}</span>
            </div>
          </div>

          <!-- Aggregate Stats Summary -->
          {#if gridStats().total > 0}
            <div class="p-2.5 bg-muted/50 rounded-lg text-xs space-y-1.5">
              <div class="flex justify-between text-muted-foreground">
                <span>Uploaded</span>
                <span class="font-semibold text-stat-upload">↑ {formatBytes(gridStats().totalUploaded)}</span>
              </div>
              <div class="flex justify-between text-muted-foreground">
                <span>Downloaded</span>
                <span class="font-semibold text-stat-leecher">↓ {formatBytes(gridStats().totalDownloaded)}</span>
              </div>
              <div class="flex justify-between text-muted-foreground">
                <span>Ratio</span>
                <span class={cn(
                  'font-bold',
                  gridStats().ratio >= 1 ? 'text-stat-upload' : 'text-stat-ratio'
                )}>
                  {gridStats().ratio.toFixed(2)}x
                </span>
              </div>
              <div class="flex justify-between text-muted-foreground">
                <span>Total Size</span>
                <span class="font-semibold text-foreground">{formatBytes(gridStats().totalSize)}</span>
              </div>
              <div class="flex justify-between text-muted-foreground">
                <span>Instances</span>
                <span class="font-semibold text-foreground">{gridStats().total}</span>
              </div>
              {#if gridStats().downloadingCount > 0}
                <div class="flex justify-between text-muted-foreground">
                  <span>Downloading</span>
                  <span class="font-semibold text-stat-leecher">{gridStats().downloadingCount}</span>
                </div>
              {/if}
            </div>
          {/if}

          <!-- State Breakdown -->
          {#if gridStats().total > 0}
            <div class="space-y-0.5">
              <span class="text-[10px] font-medium text-muted-foreground uppercase tracking-wider px-1">States</span>
              {#each stateConfig as sc (sc.key)}
                {@const count = gridStats().stateCounts[sc.key] || 0}
                {#if count > 0}
                  {@const StateIcon = sc.icon}
                  <button
                    class={cn(
                      'w-full flex items-center gap-2 px-2 py-1.5 rounded-md text-xs transition-colors border-0 cursor-pointer',
                      activeStateFilter === sc.key
                        ? 'bg-primary/10 text-foreground font-medium'
                        : 'text-muted-foreground hover:bg-muted hover:text-foreground bg-transparent'
                    )}
                    onclick={() => setStateFilter(activeStateFilter === sc.key ? 'all' : sc.key)}
                    title="Filter by {sc.label}"
                  >
                    <span class={sc.color}>
                      <StateIcon size={12} fill={sc.key === 'running' || sc.key === 'idle' ? 'currentColor' : 'none'} />
                    </span>
                    <span class="flex-1 text-left">{sc.label}</span>
                    <span class="font-mono font-semibold tabular-nums">{count}</span>
                  </button>
                {/if}
              {/each}
            </div>
          {/if}

          <!-- Quick State Filters -->
          <div class="space-y-1">
            <span class="text-[10px] font-medium text-muted-foreground uppercase tracking-wider px-1">Filter</span>
            <div class="flex flex-wrap gap-1">
              {#each quickFilters as qf (qf.key)}
                <button
                  class={cn(
                    'px-2 py-1 rounded-md text-[11px] font-medium transition-colors border-0 cursor-pointer',
                    activeStateFilter === qf.key
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-muted/70 text-muted-foreground hover:bg-muted hover:text-foreground'
                  )}
                  onclick={() => setStateFilter(qf.key)}
                >
                  {qf.label}
                </button>
              {/each}
            </div>
          </div>

          <!-- Selection Summary -->
          {#if selectionStats()}
            <div class="p-2.5 bg-primary/5 border border-primary/20 rounded-lg text-xs space-y-1.5">
              <div class="flex items-center gap-1.5 text-primary font-semibold text-[11px]">
                <CheckCircle size={12} />
                {selectionStats().count} selected
              </div>
              <div class="flex justify-between text-muted-foreground">
                <span>Size</span>
                <span class="font-semibold text-foreground">{formatBytes(selectionStats().size)}</span>
              </div>
              <div class="flex justify-between text-muted-foreground">
                <span>Uploaded</span>
                <span class="font-semibold text-stat-upload">↑ {formatBytes(selectionStats().uploaded)}</span>
              </div>
              <div class="flex justify-between text-muted-foreground">
                <span>Downloaded</span>
                <span class="font-semibold text-stat-leecher">↓ {formatBytes(selectionStats().downloaded)}</span>
              </div>
            </div>
          {/if}

        </div>
      {/if}
    </div>
  {/if}

  <!-- Footer with Network Status and About Button -->
  <div class="border-t border-border p-3 space-y-2">
    <!-- Network Status -->
    <NetworkStatus {isCollapsed} />

    <!-- Watch Folder (server mode only) -->
    <WatchFolder {isCollapsed} />

    <!-- Settings Button -->
    <button
      onclick={() => (showSettings = true)}
      class={cn(
        'w-full flex items-center gap-2 px-3 py-2 rounded-lg hover:bg-muted transition-colors text-sm font-medium text-muted-foreground hover:text-foreground',
        isCollapsed && 'lg:justify-center lg:px-2'
      )}
      title="Settings"
    >
      <Settings size={16} class="flex-shrink-0" />
      <span
        class={cn(
          'transition-opacity duration-200',
          isCollapsed && 'lg:hidden lg:w-0 lg:opacity-0'
        )}
      >
        Settings
      </span>
    </button>

    <!-- About Button -->
    <button
      onclick={() => (showAbout = true)}
      class={cn(
        'w-full flex items-center gap-2 px-3 py-2 rounded-lg hover:bg-muted transition-colors text-sm font-medium text-muted-foreground hover:text-foreground',
        isCollapsed && 'lg:justify-center lg:px-2'
      )}
      title="About Rustatio"
    >
      <Info size={16} class="flex-shrink-0" />
      <span
        class={cn(
          'transition-opacity duration-200',
          isCollapsed && 'lg:hidden lg:w-0 lg:opacity-0'
        )}
      >
        About
      </span>
    </button>
  </div>
</aside>

<!-- About Dialog -->
<AboutDialog bind:isOpen={showAbout} />

<!-- Settings Dialog -->
<SettingsDialog bind:isOpen={showSettings} />

<style>
  @keyframes pulse-slow {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.6;
    }
  }

  .animate-pulse-slow {
    animation: pulse-slow 2s ease-in-out infinite;
  }
</style>
