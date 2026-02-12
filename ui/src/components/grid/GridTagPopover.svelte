<script>
  import Button from '$lib/components/ui/button.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import TagBadge from './TagBadge.svelte';
  import { selectedIds, gridInstances, gridActions } from '$lib/gridStore.js';
  import { Tag } from '@lucide/svelte';

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
  <Button onclick={toggle} size="sm" variant="secondary" class="gap-1">
    {#snippet children()}
      <Tag size={12} />
      Tags
    {/snippet}
  </Button>

  {#if open}
    <div
      class="absolute top-full left-0 mt-1 z-50 w-64 bg-popover border border-border rounded-lg shadow-xl shadow-black/20 p-3"
      onclick={e => e.stopPropagation()}
      onkeydown={e => e.key === 'Escape' && (open = false)}
      role="dialog"
      tabindex="-1"
      aria-label="Manage tags"
    >
      <!-- Current tags on selected instances -->
      {#if selectedTags.length > 0}
        <div class="mb-3">
          <div class="text-xs font-medium text-muted-foreground mb-1.5">Current tags</div>
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
          placeholder="Add tag..."
          class="h-7 text-xs flex-1"
          onkeydown={e => e.key === 'Enter' && addTag()}
        />
        <Button onclick={addTag} size="sm" variant="secondary" class="h-7 px-2 text-xs">
          {#snippet children()}
            Add
          {/snippet}
        </Button>
      </div>
    </div>
  {/if}
</div>
