<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';

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
  let menuEl: HTMLDivElement;
  let focusedIndex = -1;

  function toggle() {
    isOpen = !isOpen;
    if (isOpen) {
      focusedIndex = 0;
      tick().then(() => {
        focusItem(0);
      });
    }
  }

  function close() {
    isOpen = false;
    focusedIndex = -1;
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

  function handleKeydown(e: KeyboardEvent) {
    if (!isOpen) return;
    
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        focusedIndex = Math.min(focusedIndex + 1, items.length - 1);
        focusItem(focusedIndex);
        break;
      case 'ArrowUp':
        e.preventDefault();
        focusedIndex = Math.max(focusedIndex - 1, 0);
        focusItem(focusedIndex);
        break;
      case 'Enter':
      case ' ':
        e.preventDefault();
        if (focusedIndex >= 0 && focusedIndex < items.length) {
          handleItemClick(items[focusedIndex]);
        }
        break;
      case 'Escape':
        e.preventDefault();
        close();
        break;
    }
  }

  function focusItem(index: number) {
    if (menuEl) {
      const buttons = menuEl.querySelectorAll('button');
      if (buttons[index]) {
        (buttons[index] as HTMLElement).focus();
      }
    }
  }

  onMount(() => {
    document.addEventListener('click', handleClickOutside);
  });

  onDestroy(() => {
    document.removeEventListener('click', handleClickOutside);
  });
</script>

<div bind:this={dropdown} class="relative inline-block" onkeydown={handleKeydown}>
  <div onclick={toggle}>
    <slot />
  </div>

  {#if isOpen}
    <div
      bind:this={menuEl}
      class="absolute top-[calc(100%+6px)] {align === 'right' ? 'right-0' : 'left-0'} z-50 min-w-[180px]"
      role="menu"
      aria-orientation="vertical"
      style="
        background: var(--bg-card);
        box-shadow: var(--neu-up);
        border-radius: var(--radius-md);
        padding: 6px;
        animation: scaleIn 0.15s ease-out;
      "
    >
      {#each items as item, index}
        <button
          onclick={() => handleItemClick(item)}
          role="menuitem"
          tabindex={focusedIndex === index ? 0 : -1}
          class="w-full text-left px-3.5 py-2.5 rounded-lg transition-colors"
          style="
            background: {focusedIndex === index ? 'var(--accent-soft)' : 'transparent'};
            border: none;
            cursor: pointer;
          "
          onmouseenter={(e) => { (e.target as HTMLElement).style.background = 'var(--accent-soft)'; focusedIndex = index; }}
          onmouseleave={(e) => { if (focusedIndex === index) (e.target as HTMLElement).style.background = 'transparent'; }}
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
