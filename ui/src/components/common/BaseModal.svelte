<script>
  import { tick } from 'svelte';
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

  let overlay = $state(null);
  let previousFocus = null;
  const focusableSelector =
    'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

  $effect(() => {
    if (!open) return;
    previousFocus = document.activeElement;
    tick().then(() => {
      const first = overlay?.querySelector('[autofocus], ' + focusableSelector);
      (first || overlay)?.focus();
    });
    return () => {
      if (previousFocus instanceof HTMLElement) previousFocus.focus();
    };
  });

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

  function handlePanelKeydown(event) {
    if (event.key === 'Escape') {
      if (!closeOnEscape) return;
      event.preventDefault();
      requestClose();
      return;
    }
    if (event.key !== 'Tab') return;
    const focusable = [...(overlay?.querySelectorAll(focusableSelector) || [])];
    if (focusable.length === 0) {
      event.preventDefault();
      overlay?.focus();
      return;
    }
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }
</script>

{#if open}
  <div
    bind:this={overlay}
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
      onkeydown={handlePanelKeydown}
      role="presentation"
    >
      {@render children?.()}
    </div>
  </div>
{/if}
