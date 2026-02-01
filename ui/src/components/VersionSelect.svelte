<script>
  import { ChevronDown } from '@lucide/svelte';

  let {
    versions = [],
    value = $bindable(),
    disabled = false,
    onchange,
    class: className = '',
  } = $props();

  let isOpen = $state(false);
  let containerRef = $state(null);

  // Ensure a version is always selected when versions are available
  $effect(() => {
    if (versions.length > 0 && (!value || !versions.includes(value))) {
      value = versions[0];
      onchange?.();
    }
  });

  function toggle() {
    if (!disabled) {
      isOpen = !isOpen;
    }
  }

  function select(version) {
    value = version;
    isOpen = false;
    onchange?.();
  }

  function handleKeydown(event) {
    if (event.key === 'Escape') {
      isOpen = false;
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      toggle();
    }
  }

  function handleOptionKeydown(event, version) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      select(version);
    }
  }

  // Close dropdown when clicking outside
  function handleClickOutside(event) {
    if (containerRef && !containerRef.contains(event.target)) {
      isOpen = false;
    }
  }

  $effect(() => {
    if (isOpen) {
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });
</script>

<div class="relative {className}" bind:this={containerRef}>
  <!-- Selected value button -->
  <button
    type="button"
    {disabled}
    onclick={toggle}
    onkeydown={handleKeydown}
    class="flex h-9 w-full items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
  >
    <span class="truncate">
      {#if value}
        {value}
      {:else}
        <span class="text-muted-foreground">Select version...</span>
      {/if}
    </span>
    <ChevronDown
      size={16}
      class="ml-2 flex-shrink-0 text-muted-foreground transition-transform {isOpen
        ? 'rotate-180'
        : ''}"
    />
  </button>

  <!-- Dropdown menu -->
  {#if isOpen}
    <div
      class="absolute z-50 mt-1 w-full rounded-md border border-border bg-popover shadow-md max-h-48 overflow-y-auto"
    >
      {#each versions as version (version)}
        <button
          type="button"
          onclick={() => select(version)}
          onkeydown={e => handleOptionKeydown(e, version)}
          class="flex w-full items-center px-3 py-2 text-sm hover:bg-muted transition-colors first:rounded-t-md last:rounded-b-md {version ===
          value
            ? 'bg-muted'
            : ''}"
        >
          {version}
        </button>
      {/each}
    </div>
  {/if}
</div>
