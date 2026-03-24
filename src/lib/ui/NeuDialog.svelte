<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { Snippet } from 'svelte';

  let {
    isOpen = false,
    onClose,
    title = '',
    size = 'md' as 'sm' | 'md' | 'lg',
    children,
  }: {
    isOpen?: boolean;
    onClose: () => void;
    title?: string;
    size?: 'sm' | 'md' | 'lg';
    children?: Snippet;
  } = $props();

  const sizeClasses: Record<string, string> = {
    sm: 'max-w-sm',
    md: 'max-w-md',
    lg: 'max-w-xl',
    xl: 'max-w-2xl'
  };

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && isOpen) onClose();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }

  onMount(() => window.addEventListener('keydown', handleKeydown));
  onDestroy(() => window.removeEventListener('keydown', handleKeydown));
</script>

{#if isOpen}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    style="background: var(--overlay);"
    onclick={handleBackdropClick}
    role="dialog"
    aria-modal="true"
    aria-labelledby="dialog-title"
  >
    <div
      class="w-full {sizeClasses[size]} mx-4 animate-modal-in"
      style="background: var(--bg-card-raised, var(--bg-card)); box-shadow: 0 25px 50px -12px rgba(0,0,0,0.25), 0 0 0 1px var(--border); border-radius: var(--radius-md);"
    >
      {#if title}
        <div class="flex items-center justify-between p-6 pb-4">
          <h2
            id="dialog-title"
            style="font-family: var(--serif); color: var(--text-primary); font-size: 1.25rem; font-weight: 600; margin: 0;"
          >
            {title}
          </h2>
          <button
            onclick={onClose}
            class="neu-subtle neu-btn flex items-center justify-center w-8 h-8 rounded-lg"
            style="background: var(--bg-card);"
            aria-label="Close dialog"
          >
            <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" style="color: var(--text-secondary);">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      {/if}
      <div class="px-6 pb-6">
        {@render children?.()}
      </div>
    </div>
  </div>
{/if}

<style>
  .animate-modal-in {
    animation: modalIn 0.16s cubic-bezier(0.2, 0.8, 0.3, 1);
  }
</style>
