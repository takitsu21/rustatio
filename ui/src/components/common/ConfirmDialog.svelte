<script>
  import { AlertTriangle, CheckCircle2 } from '@lucide/svelte';
  import { cn } from '$lib/utils.js';

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
      class="bg-card text-card-foreground rounded-xl shadow-2xl max-w-md w-full border border-border p-5"
      onclick={event => event.stopPropagation()}
      onkeydown={event => event.stopPropagation()}
      role="presentation"
    >
      <div class="flex items-start gap-3 mb-3">
        <div
          class={cn(
            'w-10 h-10 rounded-lg flex items-center justify-center shrink-0',
            isDanger ? 'bg-stat-danger/10' : 'bg-primary/10'
          )}
        >
          {#if isDanger}
            <AlertTriangle size={20} class="text-stat-danger" />
          {:else}
            <CheckCircle2 size={20} class="text-primary" />
          {/if}
        </div>
        <h3 id={titleId} class="text-base font-semibold text-foreground mt-1">{title}</h3>
      </div>

      {#if message}
        <p class="text-sm text-muted-foreground whitespace-pre-line">{message}</p>
      {/if}

      {#if showRememberChoice}
        <div class="mt-4 flex items-center gap-2">
          <label class="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              bind:checked={rememberChoiceChecked}
              class="w-4 h-4 rounded border-border text-primary focus:ring-primary/50"
            />
            <span class="text-sm text-foreground">Remember my choice</span>
          </label>
        </div>
      {/if}

      <div class="flex justify-end gap-2 mt-5">
        <button
          onclick={handleCancel}
          class="px-3 py-1.5 text-xs font-medium rounded-md border border-border hover:bg-muted transition-colors cursor-pointer disabled:opacity-60"
          disabled={disableCancel}
        >
          {cancelLabel}
        </button>
        {#if secondaryLabel}
          <button
            onclick={handleSecondary}
            class="px-3 py-1.5 text-xs font-medium rounded-md border border-border hover:bg-muted transition-colors cursor-pointer disabled:opacity-60"
            disabled={disableSecondary}
          >
            {secondaryLabel}
          </button>
        {/if}
        <button
          onclick={handleConfirm}
          class={cn(
            'px-3 py-1.5 text-xs font-medium rounded-md transition-colors cursor-pointer disabled:opacity-60',
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
