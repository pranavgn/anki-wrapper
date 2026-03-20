<script lang="ts">
  import { toasts } from "./toast";
  import { fly, fade } from "svelte/transition";
  import { fly_if_enabled, fade_if_enabled } from "./animate";

  let toastList: Array<{ id: number; message: string; type: 'success' | 'error' | 'warning'; visible: boolean }> = [];

  const unsubscribe = toasts.subscribe(value => {
    toastList = value;
  });

  // Cleanup subscription on destroy
  import { onDestroy } from "svelte";
  onDestroy(unsubscribe);
</script>

{#if toastList.length > 0}
  <div class="fixed bottom-4 left-1/2 transform -translate-x-1/2 z-50 pointer-events-none">
    {#each toastList as toast (toast.id)}
      <div
        class="mb-2 px-5 py-3 rounded-full shadow-lg text-sm font-medium pointer-events-auto flex items-center gap-2"
        class:bg-success={toast.type === 'success'}
        class:bg-danger={toast.type === 'error'}
        class:bg-warning={toast.type === 'warning'}
        in:fly={fly_if_enabled({ y: 12 })}
        out:fade={fade_if_enabled({})}
      >
        {#if toast.type === 'success'}
          <svg class="h-4 w-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
          </svg>
        {:else if toast.type === 'warning'}
          <svg class="h-4 w-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
        {:else}
          <svg class="h-4 w-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
        {/if}
        <span>{toast.message}</span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .bg-success {
    background-color: #D1FAE5;
    color: #065F46;
  }

  .bg-danger {
    background-color: #FEE2E2;
    color: #991B1B;
  }

  .bg-warning {
    background-color: #FEF3C7;
    color: #92400E;
  }
</style>
