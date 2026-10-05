<script>
  import { AlertTriangle, CheckCircle2 } from '@lucide/svelte';
  import { cn } from '$lib/core/utils.js';

  let {
    open = $bindable(false),
    title = 'Confirm Action',
    message = '',
    cancelLabel = 'Cancel',
    secondaryLabel = '',
    confirmLabel = 'Confirm',
    kind = 'info',
    onCancel = () => {},
    onSecondary = () => {},
    onConfirm = () => {},
    disableCancel = false,
    disableSecondary = false,
    disableConfirm = false,
    closeOnBackdrop = true,
    closeOnEscape = true,
    closeOnCancel = true,
    closeOnSecondary = true,
    closeOnConfirm = true,
    showRememberChoice = false,
    rememberChoiceChecked = $bindable(false),
    titleId = 'confirm-dialog-title',
    zIndexClass = 'z-[80]',
  } = $props();

  function handleCancel() {
    if (disableCancel) return;
    if (closeOnCancel) open = false;
    onCancel();
  }

  function handleSecondary() {
    if (disableSecondary) return;
    if (closeOnSecondary) open = false;
    onSecondary();
  }

  function handleConfirm() {
    if (disableConfirm) return;
    if (closeOnConfirm) open = false;
    onConfirm();
  }

  function handleBackdropClick() {
    if (!closeOnBackdrop) return;
    handleCancel();
  }

  function handleOverlayKeydown(event) {
    if (event.key !== 'Escape' || !closeOnEscape) return;
    event.preventDefault();
    handleCancel();
  }

  let isDanger = $derived(kind === 'danger' || kind === 'warning');
</script>

{#if open}
  <div
    class={cn('fixed inset-0 bg-black/50 flex items-center justify-center p-4', zIndexClass)}
    onclick={handleBackdropClick}
    onkeydown={handleOverlayKeydown}
    role="dialog"
    aria-modal="true"
    aria-labelledby={titleId}
    tabindex="-1"
  >
    <div
      class="w-full max-w-md border border-border bg-card p-4 text-card-foreground"
      onclick={event => event.stopPropagation()}
      onkeydown={event => event.stopPropagation()}
      role="presentation"
    >
      <div class="mb-2.5 flex items-start gap-2.5">
        <div
          class={cn(
            'flex h-8 w-8 shrink-0 items-center justify-center border',
            isDanger ? 'border-stat-danger/40 bg-stat-danger/10' : 'border-primary/40 bg-primary/10'
          )}
        >
          {#if isDanger}
            <AlertTriangle size={15} class="text-stat-danger" />
          {:else}
            <CheckCircle2 size={15} class="text-primary" />
          {/if}
        </div>
        <h3
          id={titleId}
          class="mt-1 text-xs font-semibold uppercase tracking-wider text-foreground"
        >
          {title}
        </h3>
      </div>

      {#if message}
        <p class="text-xs leading-5 text-muted-foreground whitespace-pre-line">{message}</p>
      {/if}

      {#if showRememberChoice}
        <div class="mt-3 flex items-center gap-2">
          <label class="flex cursor-pointer items-center gap-2">
            <input
              type="checkbox"
              bind:checked={rememberChoiceChecked}
              class="h-3.5 w-3.5 cursor-pointer rounded-none border-input accent-primary"
            />
            <span class="text-xs text-foreground">Remember my choice</span>
          </label>
        </div>
      {/if}

      <div class="mt-4 flex justify-end gap-1.5">
        <button
          onclick={handleCancel}
          class="cursor-pointer border border-border px-2.5 py-1 text-[0.6875rem] font-medium text-foreground transition-colors hover:bg-muted disabled:opacity-60"
          disabled={disableCancel}
        >
          {cancelLabel}
        </button>
        {#if secondaryLabel}
          <button
            onclick={handleSecondary}
            class="cursor-pointer border border-border px-2.5 py-1 text-[0.6875rem] font-medium text-foreground transition-colors hover:bg-muted disabled:opacity-60"
            disabled={disableSecondary}
          >
            {secondaryLabel}
          </button>
        {/if}
        <button
          onclick={handleConfirm}
          class={cn(
            'cursor-pointer px-2.5 py-1 text-[0.6875rem] font-medium transition-colors disabled:opacity-60',
            isDanger
              ? 'bg-stat-danger text-white hover:bg-stat-danger/90'
              : 'bg-primary text-primary-foreground hover:bg-primary/90'
          )}
          disabled={disableConfirm}
        >
          {confirmLabel}
        </button>
      </div>
    </div>
  </div>
{/if}
