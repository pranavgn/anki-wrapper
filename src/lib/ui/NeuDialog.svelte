<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  export let isOpen: boolean = false;
  export let onClose: () => void;
  export let title: string = '';
  export let size: 'sm' | 'md' | 'lg' = 'md';

  let dialog: HTMLDivElement;

  const sizeClasses = {
    sm: 'max-w-sm',
    md: 'max-w-md',
    lg: 'max-w-lg'
  };

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && isOpen) {
      onClose();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
  });
</script>

{#if isOpen}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    style="background: var(--overlay);"
    on:click={handleBackdropClick}
    role="dialog"
    aria-modal="true"
    aria-labelledby="dialog-title"
  >
    <div
      bind:this={dialog}
      class="neu-raised w-full {sizeClasses[size]} mx-4 animate-modal-in"
      style="background: var(--bg-card); box-shadow: var(--neu-up); border-radius: var(--radius-md);"
    >
      {#if title}
        <div class="flex items-center justify-between p-6 pb-4">
          <h2
            id="dialog-title"
            class="text-2xl font-semibold"
            style="font-family: var(--serif); color: var(--text-primary);"
          >
            {title}
          </h2>
          <button
            on:click={onClose}
            class="neu-subtle flex items-center justify-center w-8 h-8 rounded-lg"
            style="background: var(--bg-card); box-shadow: var(--neu-subtle);"
            aria-label="Close dialog"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              class="h-4 w-4"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              style="color: var(--text-secondary);"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>
      {/if}
      <div class="px-6 pb-6">
        <slot />
      </div>
    </div>
  </div>
{/if}

<style>
  .animate-modal-in {
    animation: modalIn 0.2s ease-out;
  }

  @keyframes modalIn {
    from {
      opacity: 0;
      transform: scale(0.96) translateY(8px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }
</style>
