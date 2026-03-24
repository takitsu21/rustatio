<script>
  import { cn } from '$lib/utils.js';
  import TagBadge from './TagBadge.svelte';
  import { ChevronDown, Circle, LoaderCircle, Moon, Pause, Square } from '@lucide/svelte';

  let {
    options = [],
    value = $bindable(),
    placeholder = 'Select option',
    disabled = false,
    class: className = '',
    kind = 'state',
    onChange,
  } = $props();

  let isOpen = $state(false);
  let containerRef = $state(null);

  const iconMap = {
    circle: Circle,
    loader: LoaderCircle,
    moon: Moon,
    pause: Pause,
    square: Square,
  };

  const selected = $derived(options.find(option => option.value === value) || null);

  function toggle() {
    if (!disabled) {
      isOpen = !isOpen;
    }
  }

  function select(next) {
    value = next;
    isOpen = false;
    onChange?.(next);
  }

  function handleKeydown(event) {
    if (event.key === 'Escape') {
      isOpen = false;
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      toggle();
    }
  }

  function handleOptionKeydown(event, next) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      select(next);
    }
  }

  function handleClickOutside(event) {
    if (containerRef && !containerRef.contains(event.target)) {
      isOpen = false;
    }
  }

  function getIcon(option) {
    return option?.icon ? iconMap[option.icon] : null;
  }

  $effect(() => {
    if (isOpen) {
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });
</script>

<div class={cn('relative', className)} bind:this={containerRef}>
  <button
    type="button"
    {disabled}
    onclick={toggle}
    onkeydown={handleKeydown}
    class="flex h-8 w-full items-center justify-between rounded-md border border-input bg-background px-3 py-1.5 text-xs ring-offset-background transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
  >
    <span class="flex min-w-0 items-center gap-2 truncate">
      {#if selected}
        {@const SelectedIcon = getIcon(selected)}
        {#if kind === 'state'}
          {#if SelectedIcon}
            <SelectedIcon
              size={13}
              class={cn('flex-shrink-0', selected.tone, selected.spin && 'animate-spin')}
            />
          {/if}
          <span class="truncate">{selected.label}</span>
        {:else if selected.value}
          <TagBadge tag={selected.label} compact />
        {:else}
          <span class="truncate">{selected.label}</span>
        {/if}
      {:else}
        <span class="text-muted-foreground">{placeholder}</span>
      {/if}
    </span>
    <ChevronDown
      size={14}
      class="ml-2 flex-shrink-0 text-muted-foreground transition-transform {isOpen
        ? 'rotate-180'
        : ''}"
    />
  </button>

  {#if isOpen}
    <div
      class="absolute z-50 mt-1 w-full rounded-md border border-border bg-popover shadow-md max-h-56 overflow-y-auto"
    >
      {#each options as option (option.value)}
        {@const Icon = getIcon(option)}
        <button
          type="button"
          onclick={() => select(option.value)}
          onkeydown={event => handleOptionKeydown(event, option.value)}
          class={cn(
            'flex w-full items-center gap-2 px-3 py-2 text-xs transition-colors first:rounded-t-md last:rounded-b-md hover:bg-muted',
            option.value === value && 'bg-muted'
          )}
        >
          {#if kind === 'state'}
            {#if Icon}
              <Icon
                size={13}
                class={cn('flex-shrink-0', option.tone, option.spin && 'animate-spin')}
              />
            {/if}
            <span>{option.label}</span>
          {:else if option.value}
            <TagBadge tag={option.label} compact />
          {:else}
            <span class="text-muted-foreground">{option.label}</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>
