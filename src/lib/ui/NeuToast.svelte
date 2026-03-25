<script lang="ts">
  import { toasts } from '../toast';
  import type { Toast } from '../toast';

  const typeColors = {
    success: '#6B8F71',
    error: '#C0444A',
    warning: '#C49A4F',
    info: '#C4714F'
  };
</script>

<div
  class="fixed bottom-6 right-6 z-50 flex flex-col gap-3"
  style="pointer-events: none;"
  role="status"
  aria-live="polite"
  aria-atomic="false"
>
  {#each $toasts as toast (toast.id)}
    <div
      class="neu-raised flex items-stretch overflow-hidden"
      role={toast.type === 'error' ? 'alert' : 'status'}
      aria-live={toast.type === 'error' ? 'assertive' : 'polite'}
      style="
        background: var(--bg-card);
        box-shadow: var(--neu-up);
        border-radius: var(--radius-md);
        padding: 14px 20px;
        font-family: var(--sans);
        font-size: 14px;
        color: var(--text-primary);
        pointer-events: auto;
        opacity: {toast.visible ? 1 : 0};
        transform: {toast.visible ? 'translateY(0)' : 'translateY(10px)'};
        transition: opacity 0.3s ease, transform 0.3s ease;
      "
    >
      <div
        class="w-1 rounded-l-lg mr-4 -ml-5 -my-3.5"
        style="background: {typeColors[toast.type]};"
      ></div>
      <div class="flex items-center">
        {toast.message}
      </div>
    </div>
  {/each}
</div>
