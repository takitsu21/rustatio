<script>
  import { ChevronDown } from '@lucide/svelte';
  import ClientIcon from './ClientIcon.svelte';

  let {
    clients,
    value = $bindable(),
    disabled = false,
    onchange,
    class: className = '',
  } = $props();

  let isOpen = $state(false);
  let containerRef = $state(null);

  const selectedClient = $derived(clients.find(c => c.id === value));

  function toggle() {
    if (!disabled) {
      isOpen = !isOpen;
    }
  }

  function select(clientId) {
    value = clientId;
    isOpen = false;
    onchange?.();
  }

  function handleKeydown(event) {
    if (event.key === 'Escape') {
      isOpen = false;
    } else if (event.key === 'Enter' || event.key === ' ') {
      toggle();
    }
  }

  function handleOptionKeydown(event, clientId) {
    if (event.key === 'Enter' || event.key === ' ') {
      select(clientId);
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
    <span class="flex items-center gap-2">
      {#if selectedClient}
        <ClientIcon clientId={selectedClient.id} size={16} />
        <span>{selectedClient.name}</span>
      {:else}
        <span class="text-muted-foreground">Select client...</span>
      {/if}
    </span>
    <ChevronDown
      size={16}
      class="text-muted-foreground transition-transform {isOpen ? 'rotate-180' : ''}"
    />
  </button>

  <!-- Dropdown menu -->
  {#if isOpen}
    <div class="absolute z-50 mt-1 w-full rounded-md border border-border bg-popover shadow-md">
      {#each clients as client (client.id)}
        <button
          type="button"
          onclick={() => select(client.id)}
          onkeydown={e => handleOptionKeydown(e, client.id)}
          class="flex w-full items-center gap-2 px-3 py-2 text-sm hover:bg-muted transition-colors first:rounded-t-md last:rounded-b-md {client.id ===
          value
            ? 'bg-muted'
            : ''}"
        >
          <ClientIcon clientId={client.id} size={16} />
          <span>{client.name}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>
