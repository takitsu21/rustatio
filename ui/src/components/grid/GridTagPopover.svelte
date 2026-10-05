<script>
  import Button from '$lib/components/ui/button.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import TagBadge from './TagBadge.svelte';
  import { selectedIds, gridInstances, gridActions } from '$lib/grid/gridStore.js';
  import { Tag } from '@lucide/svelte';

  let { disabled = false } = $props();

  let open = $state(false);
  let newTag = $state('');
  let popoverEl = $state(null);

  // Collect all tags present on the selected instances
  let selectedTags = $derived.by(() => {
    const ids = $selectedIds;
    const tagCounts = new Map();
    for (const inst of $gridInstances) {
      if (!ids.has(inst.id)) continue;
      for (const tag of inst.tags || []) {
        tagCounts.set(tag, (tagCounts.get(tag) || 0) + 1);
      }
    }
    return [...tagCounts.entries()]
      .sort((a, b) => a[0].localeCompare(b[0]))
      .map(([tag, count]) => ({ tag, count }));
  });

  function toggle() {
    open = !open;
    if (open) newTag = '';
  }

  function handleClickOutside(e) {
    if (open && popoverEl && !popoverEl.contains(e.target)) {
      open = false;
    }
  }

  async function addTag() {
    const tag = newTag.trim();
    if (!tag) return;
    try {
      await gridActions.gridTag([tag], []);
      newTag = '';
    } catch (error) {
      console.error('Failed to add tag:', error);
    }
  }

  async function removeTag(tag) {
    try {
      await gridActions.gridTag([], [tag]);
    } catch (error) {
      console.error('Failed to remove tag:', error);
    }
  }
</script>

<svelte:window onclick={handleClickOutside} />

<div class="relative" bind:this={popoverEl}>
  <Button
    onclick={toggle}
    {disabled}
    size="icon"
    variant="outline"
    class="h-7 w-7"
    title="Manage tags"
    aria-label="Manage tags"
  >
    {#snippet children()}
      <Tag size={12} />
    {/snippet}
  </Button>

  {#if open}
    <div
      class="absolute left-0 top-full z-50 mt-1 w-64 border border-border bg-popover p-2.5"
      onclick={e => e.stopPropagation()}
      onkeydown={e => e.key === 'Escape' && (open = false)}
      role="dialog"
      tabindex="-1"
      aria-label="Manage tags"
    >
      <!-- Current tags on selected instances -->
      {#if selectedTags.length > 0}
        <div class="mb-2.5">
          <div
            class="mb-1.5 text-[0.625rem] font-semibold uppercase tracking-wider text-muted-foreground"
          >
            Current tags
          </div>
          <div class="flex flex-wrap gap-1">
            {#each selectedTags as { tag, count: _count } (tag)}
              <TagBadge {tag} removable onRemove={removeTag} />
            {/each}
          </div>
        </div>
      {/if}

      <!-- Add new tag -->
      <div class="flex items-center gap-1">
        <Input
          bind:value={newTag}
          placeholder="Add tag…"
          class="h-7 flex-1 text-[0.6875rem]"
          onkeydown={e => e.key === 'Enter' && addTag()}
        />
        <Button onclick={addTag} size="sm" variant="secondary" class="h-7 px-2 text-[0.6875rem]">
          {#snippet children()}
            Add
          {/snippet}
        </Button>
      </div>
    </div>
  {/if}
</div>
