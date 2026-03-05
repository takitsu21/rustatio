<script>
  import { cn } from '$lib/utils.js';

  let {
    open = $bindable(false),
    onClose = () => {},
    closeOnBackdrop = true,
    closeOnEscape = true,
    zIndexClass = 'z-[80]',
    maxWidthClass = 'max-w-md',
    panelClass = '',
    overlayClass = '',
    titleId = '',
    children,
  } = $props();

  function requestClose() {
    open = false;
    onClose();
  }

  function handleBackdropClick() {
    if (!closeOnBackdrop) return;
    requestClose();
  }

  function handleOverlayKeydown(event) {
    if (event.key !== 'Escape' || !closeOnEscape) return;
    event.preventDefault();
    requestClose();
  }
</script>

{#if open}
  <div
    class={cn(
      'fixed inset-0 bg-black/50 flex items-center justify-center p-4',
      zIndexClass,
      overlayClass
    )}
    onclick={handleBackdropClick}
    onkeydown={handleOverlayKeydown}
    role="dialog"
    aria-modal="true"
    aria-labelledby={titleId || undefined}
    tabindex="-1"
  >
    <div
      class={cn(
        'bg-card text-card-foreground rounded-xl shadow-2xl w-full border border-border',
        maxWidthClass,
        panelClass
      )}
      onclick={event => event.stopPropagation()}
      onkeydown={event => event.stopPropagation()}
      role="presentation"
    >
      {@render children?.()}
    </div>
  </div>
{/if}
