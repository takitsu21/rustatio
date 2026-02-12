<script>
  import { cn } from '$lib/utils.js';
  import { X } from '@lucide/svelte';

  let {
    tag,
    removable = false,
    compact = false,
    onRemove = undefined,
    class: className = '',
  } = $props();

  const colors = [
    'bg-blue-500/20 text-blue-400 border-blue-500/30',
    'bg-emerald-500/20 text-emerald-400 border-emerald-500/30',
    'bg-violet-500/20 text-violet-400 border-violet-500/30',
    'bg-amber-500/20 text-amber-400 border-amber-500/30',
    'bg-rose-500/20 text-rose-400 border-rose-500/30',
    'bg-cyan-500/20 text-cyan-400 border-cyan-500/30',
    'bg-pink-500/20 text-pink-400 border-pink-500/30',
    'bg-teal-500/20 text-teal-400 border-teal-500/30',
  ];

  function hashString(str) {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      hash = ((hash << 5) - hash + str.charCodeAt(i)) | 0;
    }
    return Math.abs(hash);
  }

  let colorClass = $derived(colors[hashString(tag) % colors.length]);
</script>

<span
  class={cn(
    'inline-flex items-center gap-1 rounded-full font-medium border',
    compact ? 'px-1.5 py-0 text-[10px]' : 'px-2 py-0.5 text-xs',
    colorClass,
    className
  )}
>
  {tag}
  {#if removable && onRemove}
    <button
      onclick={e => {
        e.stopPropagation();
        onRemove(tag);
      }}
      class="p-0 border-0 bg-transparent cursor-pointer hover:opacity-70 flex items-center"
      aria-label="Remove tag {tag}"
    >
      <X size={10} />
    </button>
  {/if}
</span>
