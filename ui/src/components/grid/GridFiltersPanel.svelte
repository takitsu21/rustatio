<script>
  let { mobile = false, showHeader = true } = $props();

  import { cn } from '$lib/core/utils.js';
  import Input from '$lib/components/ui/input.svelte';
  import Checkbox from '$lib/components/ui/checkbox.svelte';
  import TagBadge from './TagBadge.svelte';
  import TrackerBadge from './TrackerBadge.svelte';
  import {
    gridFilters,
    stateFilterEntries,
    tagFilterEntries,
    trackerFilterEntries,
  } from '$lib/grid/gridStore.js';
  import { UNTAGGED_FILTER_VALUE, clearAllGridFilters } from '$lib/grid/gridFilters.js';
  import { getGridStateMeta } from '$lib/grid/gridFilterOptions.js';
  import { Search, ChevronDown, Circle, Pause, Moon, Square, LoaderCircle } from '@lucide/svelte';

  const iconMap = {
    circle: Circle,
    pause: Pause,
    moon: Moon,
    square: Square,
    loader: LoaderCircle,
  };

  let sections = $state({
    status: true,
    tags: true,
    trackers: true,
  });

  let activeFiltersCount = $derived(
    ($gridFilters.stateFilter !== 'all' ? 1 : 0) +
      $gridFilters.tagFilter.length +
      $gridFilters.trackerFilter.length
  );

  let selectedTags = $derived(new Set($gridFilters.tagFilter));
  let selectedTrackers = $derived(new Set($gridFilters.trackerFilter));

  function toggleSection(key) {
    sections = { ...sections, [key]: !sections[key] };
  }

  function setStateFilter(value) {
    gridFilters.update(filters => ({
      ...filters,
      stateFilter: filters.stateFilter === value ? 'all' : value,
    }));
  }

  function toggleTag(tag) {
    gridFilters.update(filters => ({
      ...filters,
      tagFilter: filters.tagFilter.includes(tag)
        ? filters.tagFilter.filter(value => value !== tag)
        : [...filters.tagFilter, tag],
    }));
  }

  function toggleTracker(tracker) {
    gridFilters.update(filters => ({
      ...filters,
      trackerFilter: filters.trackerFilter.includes(tracker)
        ? filters.trackerFilter.filter(value => value !== tracker)
        : [...filters.trackerFilter, tracker],
    }));
  }

  function updateTagSearch(event) {
    gridFilters.update(filters => ({ ...filters, tagSearch: event.target.value }));
  }

  function updateTrackerSearch(event) {
    gridFilters.update(filters => ({ ...filters, trackerSearch: event.target.value }));
  }

  function clearAll() {
    gridFilters.update(filters => clearAllGridFilters(filters));
  }

  function getStateMeta(value) {
    return getGridStateMeta(value);
  }

  const sectionHeaderClass =
    'flex w-full items-center gap-1.5 px-2.5 py-2 text-left text-[0.625rem] font-semibold uppercase tracking-[0.14em] text-muted-foreground transition-colors hover:text-foreground cursor-pointer';
</script>

<div
  class={cn(
    'w-full',
    !mobile &&
      'lg:flex lg:w-60 lg:flex-shrink-0 lg:flex-col lg:border-r lg:border-border lg:bg-card'
  )}
>
  <div class={cn('flex min-h-0 flex-col', !mobile && 'lg:flex-1')}>
    {#if showHeader}
      <div class="flex h-9 shrink-0 items-center gap-2 border-b border-border px-2.5">
        <span
          class="flex-1 text-[0.625rem] font-semibold uppercase tracking-[0.16em] text-muted-foreground"
          >Filters</span
        >
        {#if activeFiltersCount > 0}
          <button
            class="text-[0.625rem] text-primary transition-colors hover:text-primary/80 cursor-pointer"
            onclick={clearAll}
            title="Clear all filters"
          >
            Clear ({activeFiltersCount})
          </button>
        {/if}
      </div>
    {/if}

    <div class={cn('min-h-0', !mobile && 'lg:flex-1 lg:overflow-y-auto')}>
      <!-- Status -->
      <section class="border-b border-border">
        <button class={sectionHeaderClass} onclick={() => toggleSection('status')}>
          <ChevronDown
            size={11}
            class={cn('transition-transform', !sections.status && '-rotate-90')}
          />
          <span class="flex-1">Status</span>
        </button>

        {#if sections.status}
          <div class="pb-1.5">
            {#if $stateFilterEntries.length === 0}
              <div class="px-2.5 py-1.5 text-[0.6875rem] text-muted-foreground">No states.</div>
            {:else}
              {#each $stateFilterEntries as entry (entry.value)}
                {@const option = getStateMeta(entry.value)}
                {@const Icon = iconMap[option.icon]}
                {@const isActive = $gridFilters.stateFilter === entry.value}
                <button
                  class={cn(
                    'flex h-6 w-full items-center gap-2 px-2.5 text-[0.6875rem] transition-colors cursor-pointer',
                    isActive
                      ? 'bg-primary/15 text-foreground'
                      : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                  )}
                  onclick={() => setStateFilter(entry.value)}
                >
                  <span class={option.tone}>
                    {#if Icon}
                      <Icon
                        size={11}
                        class={cn(option.spin && 'animate-spin')}
                        fill={entry.value === 'running' || entry.value === 'idle'
                          ? 'currentColor'
                          : 'none'}
                      />
                    {/if}
                  </span>
                  <span class="flex-1 truncate text-left">{option.label}</span>
                  <span class="tabular-nums text-[0.625rem] text-muted-foreground"
                    >{entry.count}</span
                  >
                </button>
              {/each}
            {/if}
          </div>
        {/if}
      </section>

      <!-- Tags -->
      <section class="border-b border-border">
        <button class={sectionHeaderClass} onclick={() => toggleSection('tags')}>
          <ChevronDown
            size={11}
            class={cn('transition-transform', !sections.tags && '-rotate-90')}
          />
          <span class="flex-1">Tags</span>
          {#if $gridFilters.tagFilter.length > 0}
            <span class="tabular-nums text-primary">{$gridFilters.tagFilter.length}</span>
          {/if}
        </button>

        {#if sections.tags}
          <div class="space-y-1.5 px-2 pb-2">
            <div class="relative">
              <Search
                size={11}
                class="absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground"
              />
              <Input
                value={$gridFilters.tagSearch}
                oninput={updateTagSearch}
                placeholder="Search tags…"
                class="h-7 pl-6 text-[0.6875rem]"
              />
            </div>

            <div class={cn('space-y-px', mobile ? '' : 'max-h-56 overflow-y-auto')}>
              {#if $tagFilterEntries.length === 0}
                <div class="px-1 py-1.5 text-[0.6875rem] text-muted-foreground">No tags found.</div>
              {:else}
                {#each $tagFilterEntries as entry (entry.value)}
                  {@const isActive = selectedTags.has(entry.value)}
                  <label
                    class={cn(
                      'flex h-6 items-center gap-2 px-1 text-[0.6875rem] transition-colors cursor-pointer',
                      isActive
                        ? 'bg-primary/15 text-foreground'
                        : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                    )}
                  >
                    <Checkbox
                      checked={isActive}
                      onchange={() => toggleTag(entry.value)}
                      class="h-3 w-3"
                    />
                    <div class="min-w-0 flex-1 text-left">
                      {#if entry.value === UNTAGGED_FILTER_VALUE}
                        <span class="italic text-muted-foreground">{entry.label}</span>
                      {:else}
                        <TagBadge tag={entry.label} compact />
                      {/if}
                    </div>
                    <span class="tabular-nums text-[0.625rem] text-muted-foreground"
                      >{entry.count}</span
                    >
                  </label>
                {/each}
              {/if}
            </div>
          </div>
        {/if}
      </section>

      <!-- Trackers -->
      <section class="border-b border-border">
        <button class={sectionHeaderClass} onclick={() => toggleSection('trackers')}>
          <ChevronDown
            size={11}
            class={cn('transition-transform', !sections.trackers && '-rotate-90')}
          />
          <span class="flex-1">Trackers</span>
          {#if $gridFilters.trackerFilter.length > 0}
            <span class="tabular-nums text-primary">{$gridFilters.trackerFilter.length}</span>
          {/if}
        </button>

        {#if sections.trackers}
          <div class="space-y-1.5 px-2 pb-2">
            <div class="relative">
              <Search
                size={11}
                class="absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground"
              />
              <Input
                value={$gridFilters.trackerSearch}
                oninput={updateTrackerSearch}
                placeholder="Search trackers…"
                class="h-7 pl-6 text-[0.6875rem]"
              />
            </div>

            <div class={cn('space-y-px pr-1', mobile ? '' : 'max-h-64 overflow-y-auto')}>
              {#if $trackerFilterEntries.length === 0}
                <div class="px-1 py-1.5 text-[0.6875rem] text-muted-foreground">
                  No trackers found.
                </div>
              {:else}
                {#each $trackerFilterEntries as entry (entry.value)}
                  {@const isActive = selectedTrackers.has(entry.value)}
                  <label
                    class={cn(
                      'flex h-6 items-center gap-2 px-1 text-[0.6875rem] transition-colors cursor-pointer',
                      isActive
                        ? 'bg-primary/15 text-foreground'
                        : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                    )}
                  >
                    <Checkbox
                      checked={isActive}
                      onchange={() => toggleTracker(entry.value)}
                      class="h-3 w-3"
                    />
                    <TrackerBadge
                      tracker={entry.label}
                      iconUrl={entry.iconUrl}
                      initial={entry.initial}
                    />
                    <span class="min-w-0 flex-1 truncate text-left">{entry.label}</span>
                    <span class="tabular-nums text-[0.625rem] text-muted-foreground"
                      >{entry.count}</span
                    >
                  </label>
                {/each}
              {/if}
            </div>
          </div>
        {/if}
      </section>
    </div>
  </div>
</div>
