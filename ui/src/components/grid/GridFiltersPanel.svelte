<script>
  let { mobile = false, showHeader = true } = $props();

  import { cn } from '$lib/utils.js';
  import Input from '$lib/components/ui/input.svelte';
  import Checkbox from '$lib/components/ui/checkbox.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import TagBadge from './TagBadge.svelte';
  import TrackerBadge from './TrackerBadge.svelte';
  import {
    gridFilters,
    stateFilterEntries,
    tagFilterEntries,
    trackerFilterEntries,
  } from '$lib/gridStore.js';
  import { UNTAGGED_FILTER_VALUE, clearAllGridFilters } from '$lib/gridFilters.js';
  import { getGridStateMeta } from '$lib/gridFilterOptions.js';
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
</script>

<div class={cn('w-full', !mobile && 'lg:w-[280px] lg:flex-shrink-0')}>
  <div
    class={cn(
      'space-y-3',
      mobile ? '' : 'rounded-xl border border-border bg-card/60 p-3 shadow-sm backdrop-blur-sm'
    )}
  >
    {#if showHeader}
      <div class="mb-3 flex items-center justify-between gap-3 px-1">
        <div>
          <div class="text-base font-semibold text-foreground">Filters</div>
          <div class="text-[11px] text-muted-foreground">
            {activeFiltersCount > 0
              ? `${activeFiltersCount} active filter${activeFiltersCount > 1 ? 's' : ''}`
              : 'Browse instances by facet'}
          </div>
        </div>
        {#if activeFiltersCount > 0}
          <Button onclick={clearAll} variant="ghost" size="sm" class="h-7 px-2 text-[11px]">
            {#snippet children()}Clear all{/snippet}
          </Button>
        {/if}
      </div>
    {/if}

    <div class="space-y-3">
      <section class="rounded-xl border border-border/70 bg-background/40">
        <button
          class="flex w-full items-center justify-between px-3 py-2.5 text-left cursor-pointer"
          onclick={() => toggleSection('status')}
        >
          <div class="text-sm font-semibold text-foreground">Status</div>
          <ChevronDown
            size={14}
            class={cn(
              'text-muted-foreground transition-transform',
              !sections.status && '-rotate-90'
            )}
          />
        </button>

        {#if sections.status}
          <div class="space-y-1 border-t border-border/70 px-2 py-2">
            {#each $stateFilterEntries as entry (entry.value)}
              {@const option = getStateMeta(entry.value)}
              {@const Icon = iconMap[option.icon]}
              <button
                class={cn(
                  'flex w-full items-center gap-2 rounded-lg px-2 py-2 text-xs transition-colors cursor-pointer',
                  $gridFilters.stateFilter === entry.value
                    ? 'bg-primary/10 text-foreground font-medium'
                    : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                )}
                onclick={() => setStateFilter(entry.value)}
              >
                <span class={option.tone}>
                  {#if Icon}
                    <Icon
                      size={13}
                      class={cn(option.spin && 'animate-spin')}
                      fill={entry.value === 'running' || entry.value === 'idle'
                        ? 'currentColor'
                        : 'none'}
                    />
                  {/if}
                </span>
                <span class="flex-1 text-left">{option.label}</span>
                <span class="font-mono tabular-nums">{entry.count}</span>
              </button>
            {/each}
          </div>
        {/if}
      </section>

      <section class="rounded-xl border border-border/70 bg-background/40">
        <button
          class="flex w-full items-center justify-between px-3 py-2.5 text-left cursor-pointer"
          onclick={() => toggleSection('tags')}
        >
          <div class="text-sm font-semibold text-foreground">Tags</div>
          <ChevronDown
            size={14}
            class={cn('text-muted-foreground transition-transform', !sections.tags && '-rotate-90')}
          />
        </button>

        {#if sections.tags}
          <div class="space-y-2 border-t border-border/70 px-2 py-2">
            <div class="relative">
              <Search
                size={12}
                class="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
              />
              <Input
                value={$gridFilters.tagSearch}
                oninput={updateTagSearch}
                placeholder="Search tags..."
                class="h-8 pl-7 text-xs"
              />
            </div>

            <div class="space-y-1 max-h-56 overflow-y-auto pr-1">
              {#if $tagFilterEntries.length === 0}
                <div class="px-2 py-3 text-xs text-muted-foreground">No tags found.</div>
              {:else}
                {#each $tagFilterEntries as entry (entry.value)}
                  <label
                    class={cn(
                      'flex w-full items-center gap-2 rounded-lg px-2 py-2 text-xs cursor-pointer transition-colors',
                      selectedTags.has(entry.value)
                        ? 'bg-primary/10 text-foreground font-medium'
                        : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                    )}
                  >
                    <Checkbox
                      checked={selectedTags.has(entry.value)}
                      onchange={() => toggleTag(entry.value)}
                    />
                    <div class="flex-1 min-w-0 text-left">
                      {#if entry.value === UNTAGGED_FILTER_VALUE}
                        <span class="italic text-muted-foreground">{entry.label}</span>
                      {:else}
                        <TagBadge tag={entry.label} compact />
                      {/if}
                    </div>
                    <span class="font-mono tabular-nums">{entry.count}</span>
                  </label>
                {/each}
              {/if}
            </div>
          </div>
        {/if}
      </section>

      <section class="rounded-xl border border-border/70 bg-background/40">
        <button
          class="flex w-full items-center justify-between px-3 py-2.5 text-left cursor-pointer"
          onclick={() => toggleSection('trackers')}
        >
          <div class="text-sm font-semibold text-foreground">Trackers</div>
          <ChevronDown
            size={14}
            class={cn(
              'text-muted-foreground transition-transform',
              !sections.trackers && '-rotate-90'
            )}
          />
        </button>

        {#if sections.trackers}
          <div class="space-y-2 border-t border-border/70 px-2 py-2">
            <div class="relative">
              <Search
                size={12}
                class="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
              />
              <Input
                value={$gridFilters.trackerSearch}
                oninput={updateTrackerSearch}
                placeholder="Search trackers..."
                class="h-8 pl-7 text-xs"
              />
            </div>

            <div class="space-y-1 max-h-64 overflow-y-auto pr-1">
              {#if $trackerFilterEntries.length === 0}
                <div class="px-2 py-3 text-xs text-muted-foreground">No trackers found.</div>
              {:else}
                {#each $trackerFilterEntries as entry (entry.value)}
                  <label
                    class={cn(
                      'flex w-full items-center gap-2 rounded-lg px-2 py-2 text-xs cursor-pointer transition-colors',
                      selectedTrackers.has(entry.value)
                        ? 'bg-primary/10 text-foreground font-medium'
                        : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                    )}
                  >
                    <Checkbox
                      checked={selectedTrackers.has(entry.value)}
                      onchange={() => toggleTracker(entry.value)}
                    />
                    <TrackerBadge
                      tracker={entry.label}
                      iconUrl={entry.iconUrl}
                      initial={entry.initial}
                    />
                    <span class="flex-1 min-w-0 truncate text-left">{entry.label}</span>
                    <span class="font-mono tabular-nums">{entry.count}</span>
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
