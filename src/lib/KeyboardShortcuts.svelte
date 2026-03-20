<script lang="ts">
  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      onClose();
    }
  }

  function handleBackdropClick() {
    onClose();
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if isOpen}
  <!-- Backdrop -->
  <div
    class="fixed inset-0 bg-black/25 backdrop-blur-sm z-50 flex items-center justify-center transition-opacity duration-150"
    role="dialog"
    aria-modal="true"
    aria-labelledby="keyboard-shortcuts-title"
    onclick={handleBackdropClick}
    onkeydown={(e) => e.key === 'Escape' && handleBackdropClick()}
  >
    <!-- Modal -->
    <div
      class="bg-white rounded-3xl p-8 max-w-md w-full mx-4 shadow-2xl animate-in zoom-in-95 duration-180"
      role="document"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <div class="text-center mb-6">
        <h2 class="text-lg font-semibold text-text-primary">Keyboard Shortcuts</h2>
      </div>

      <div class="grid grid-cols-2 gap-4 text-sm">
        <div class="flex items-center gap-3">
          <kbd class="bg-bg-subtle border border-border px-2 py-0.5 rounded font-mono text-xs">Space</kbd>
          <span class="text-text-secondary">Show Answer</span>
        </div>
        <div class="flex items-center gap-3">
          <kbd class="bg-bg-subtle border border-border px-2 py-0.5 rounded font-mono text-xs">1</kbd>
          <span class="text-text-secondary">Again</span>
        </div>
        <div class="flex items-center gap-3">
          <kbd class="bg-bg-subtle border border-border px-2 py-0.5 rounded font-mono text-xs">2</kbd>
          <span class="text-text-secondary">Hard</span>
        </div>
        <div class="flex items-center gap-3">
          <kbd class="bg-bg-subtle border border-border px-2 py-0.5 rounded font-mono text-xs">3</kbd>
          <span class="text-text-secondary">Good</span>
        </div>
        <div class="flex items-center gap-3">
          <kbd class="bg-bg-subtle border border-border px-2 py-0.5 rounded font-mono text-xs">4</kbd>
          <span class="text-text-secondary">Easy</span>
        </div>
        <div class="flex items-center gap-3">
          <kbd class="bg-bg-subtle border border-border px-2 py-0.5 rounded font-mono text-xs">Ctrl+Enter</kbd>
          <span class="text-text-secondary">Save Card</span>
        </div>
        <div class="flex items-center gap-3">
          <kbd class="bg-bg-subtle border border-border px-2 py-0.5 rounded font-mono text-xs">?</kbd>
          <span class="text-text-secondary">Toggle Shortcuts</span>
        </div>
        <div class="flex items-center gap-3">
          <kbd class="bg-bg-subtle border border-border px-2 py-0.5 rounded font-mono text-xs">Esc</kbd>
          <span class="text-text-secondary">Close/Back</span>
        </div>
        <div class="flex items-center gap-3">
          <kbd class="bg-bg-subtle border border-border px-2 py-0.5 rounded font-mono text-xs">Ctrl+Z</kbd>
          <span class="text-text-secondary">Undo</span>
        </div>
        <div class="flex items-center gap-3">
          <kbd class="bg-bg-subtle border border-border px-2 py-0.5 rounded font-mono text-xs">⌘0</kbd>
          <span class="text-text-secondary">Remove Flag</span>
        </div>
        <div class="flex items-center gap-3">
          <kbd class="bg-bg-subtle border border-border px-2 py-0.5 rounded font-mono text-xs">⌘1</kbd>
          <span class="text-text-secondary">Flag Red</span>
        </div>
        <div class="flex items-center gap-3">
          <kbd class="bg-bg-subtle border border-border px-2 py-0.5 rounded font-mono text-xs">⌘2</kbd>
          <span class="text-text-secondary">Flag Orange</span>
        </div>
        <div class="flex items-center gap-3">
          <kbd class="bg-bg-subtle border border-border px-2 py-0.5 rounded font-mono text-xs">⌘3</kbd>
          <span class="text-text-secondary">Flag Green</span>
        </div>
        <div class="flex items-center gap-3">
          <kbd class="bg-bg-subtle border border-border px-2 py-0.5 rounded font-mono text-xs">⌘4</kbd>
          <span class="text-text-secondary">Flag Blue</span>
        </div>
        <div class="flex items-center gap-3">
          <kbd class="bg-bg-subtle border border-border px-2 py-0.5 rounded font-mono text-xs">⌘5</kbd>
          <span class="text-text-secondary">Flag Pink</span>
        </div>
        <div class="flex items-center gap-3">
          <kbd class="bg-bg-subtle border border-border px-2 py-0.5 rounded font-mono text-xs">⌘6</kbd>
          <span class="text-text-secondary">Flag Turquoise</span>
        </div>
      </div>
    </div>
  </div>
{/if}
