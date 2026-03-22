<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  interface DropdownItem {
    label: string;
    description?: string;
    icon?: string;
    onClick: () => void;
    danger?: boolean;
  }

  export let items: DropdownItem[] = [];
  export let align: 'left' | 'right' = 'right';

  let isOpen = false;
  let dropdown: HTMLDivElement;

  function toggle() {
    isOpen = !isOpen;
  }

  function close() {
    isOpen = false;
  }

  function handleItemClick(item: DropdownItem) {
    item.onClick();
    close();
  }

  function handleClickOutside(e: MouseEvent) {
    if (dropdown && !dropdown.contains(e.target as Node)) {
      close();
    }
  }

  onMount(() => {
    document.addEventListener('click', handleClickOutside);
  });

  onDestroy(() => {
    document.removeEventListener('click', handleClickOutside);
  });
</script>

<div bind:this={dropdown} class="relative inline-block">
  <div on:click={toggle}>
    <slot />
  </div>

  {#if isOpen}
    <div
      class="absolute top-[calc(100%+6px)] {align === 'right' ? 'right-0' : 'left-0'} z-50 min-w-[180px]"
      style="
        background: var(--bg-card);
        box-shadow: var(--neu-up);
        border-radius: var(--radius-md);
        padding: 6px;
        animation: scaleIn 0.15s ease-out;
      "
    >
      {#each items as item}
        <button
          on:click={() => handleItemClick(item)}
          class="w-full text-left px-3.5 py-2.5 rounded-lg transition-colors"
          style="
            background: transparent;
            border: none;
            cursor: pointer;
          "
          on:mouseenter={(e) => (e.target as HTMLElement).style.background = 'var(--accent-soft)'}
          on:mouseleave={(e) => (e.target as HTMLElement).style.background = 'transparent'}
        >
          <div
            class="text-sm font-medium"
            style="
              font-family: var(--sans);
              font-size: 13px;
              font-weight: 500;
              color: {item.danger ? 'var(--danger)' : 'var(--text-primary)'};
            "
          >
            {item.label}
          </div>
          {#if item.description}
            <div
              class="mt-0.5"
              style="
                font-size: 11px;
                color: var(--text-muted);
              "
            >
              {item.description}
            </div>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  @keyframes scaleIn {
    from {
      opacity: 0;
      transform: scale(0.95);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
