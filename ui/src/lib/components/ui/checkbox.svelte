<script>
  import { cn } from '$lib/core/utils.js';

  let {
    class: className = '',
    checked = $bindable(false),
    disabled = false,
    id = undefined,
    onchange = undefined,
    ...props
  } = $props();

  function handleChange(event) {
    checked = event.target.checked;
    if (onchange) {
      onchange(checked);
    }
  }
</script>

<input
  type="checkbox"
  {id}
  {checked}
  onchange={handleChange}
  {disabled}
  class={cn(
    'peer h-3.5 w-3.5 shrink-0 cursor-pointer rounded-none border border-input accent-primary focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50',
    className
  )}
  {...props}
/>
