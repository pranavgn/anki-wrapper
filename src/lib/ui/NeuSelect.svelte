<script lang="ts">
  import { tick, onMount } from 'svelte';

  type Option<T> = {
    value: T;
    label: string;
  };

  let {
    options = [],
    value = $bindable(),
    placeholder = 'Select...',
    size = 'md' as 'sm' | 'md' | 'lg',
    disabled = false,
    onchange = () => {},
  }: {
    options: Option<any>[];
    value?: any;
    placeholder?: string;
    size?: 'sm' | 'md' | 'lg';
    disabled?: boolean;
    onchange?: (value: any) => void;
  } = $props();

  let isOpen = $state(false);
  let triggerEl: HTMLButtonElement;
  let listEl: HTMLUListElement | null = $state(null);
  let focusedIndex = $state(-1);
  let containerEl: HTMLDivElement;

  onMount(() => {
    function handleDocClick(e: MouseEvent) {
      if (isOpen && !containerEl?.contains(e.target as Node)) {
        isOpen = false;
      }
    }
    document.addEventListener('click', handleDocClick);
    return () => document.removeEventListener('click', handleDocClick);
  });

  const sizeClasses: Record<string, string> = {
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2 text-base',
    lg: 'px-5 py-3 text-lg',
  };

  function toggle() {
    if (disabled) return;
    isOpen = !isOpen;
    if (isOpen) {
      focusedIndex = options.findIndex(opt => opt.value === value);
      tick().then(() => {
        listEl?.focus();
      });
    }
  }

  function close() {
    isOpen = false;
    triggerEl?.focus();
  }

  function selectOption(option: Option<any>) {
    value = option.value;
    onchange(option.value);
    close();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (disabled) return;

    switch (e.key) {
      case 'Enter':
      case ' ':
        e.preventDefault();
        if (isOpen) {
          if (focusedIndex >= 0 && focusedIndex < options.length) {
            selectOption(options[focusedIndex]);
          }
        } else {
          toggle();
        }
        break;
      case 'ArrowDown':
        e.preventDefault();
        if (!isOpen) {
          toggle();
        } else {
          focusedIndex = Math.min(focusedIndex + 1, options.length - 1);
        }
        break;
      case 'ArrowUp':
        e.preventDefault();
        if (isOpen) {
          focusedIndex = Math.max(focusedIndex - 1, 0);
        }
        break;
      case 'Escape':
        e.preventDefault();
        close();
        break;
      case 'Tab':
        close();
        break;
    }
  }

  function handleOptionKeydown(e: KeyboardEvent, option: Option<any>) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      selectOption(option);
    }
  }

  const selectedLabel = $derived(
    options.find(opt => opt.value === value)?.label ?? placeholder
  );
</script>

<div class="neu-select-container" class:is-disabled={disabled} bind:this={containerEl}>
  <button
    bind:this={triggerEl}
    type="button"
    class="neu-select-trigger {sizeClasses[size]}"
    class:is-open={isOpen}
    onclick={toggle}
    onkeydown={handleKeydown}
    aria-haspopup="listbox"
    aria-expanded={isOpen}
    {disabled}
  >
    <span class="neu-select-value">{selectedLabel}</span>
    <svg class="neu-select-chevron" viewBox="0 0 20 20" fill="currentColor">
      <path fill-rule="evenodd" d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z" clip-rule="evenodd" />
    </svg>
  </button>

  {#if isOpen}
    <ul
      bind:this={listEl}
      class="neu-select-list"
      role="listbox"
      tabindex="-1"
      onkeydown={handleKeydown}
    >
      {#each options as option, index}
        <li
          class="neu-select-option"
          class:is-focused={focusedIndex === index}
          class:is-selected={option.value === value}
          role="option"
          aria-selected={option.value === value}
          tabindex="0"
          onclick={() => selectOption(option)}
          onkeydown={(e) => handleOptionKeydown(e, option)}
        >
          {option.label}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .neu-select-container {
    position: relative;
    display: inline-block;
    width: 100%;
  }

  .neu-select-container.is-disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .neu-select-trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    background: var(--bg-card);
    box-shadow: var(--neu-down);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font-family: var(--sans);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .neu-select-trigger:hover:not(:disabled) {
    box-shadow: var(--neu-up);
  }

  .neu-select-trigger:focus {
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .neu-select-trigger.is-open {
    box-shadow: var(--neu-down);
  }

  .neu-select-value {
    flex: 1;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .neu-select-chevron {
    width: 1.25em;
    height: 1.25em;
    margin-left: 0.5em;
    flex-shrink: 0;
    color: var(--text-secondary);
    transition: transform 0.2s ease;
  }

  .neu-select-trigger.is-open .neu-select-chevron {
    transform: rotate(180deg);
  }

  .neu-select-list {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    z-index: 50;
    margin-top: 0.25rem;
    padding: 0.25rem 0;
    background: var(--bg-card);
    box-shadow: var(--neu-up);
    border-radius: var(--radius-md);
    list-style: none;
    max-height: 15rem;
    overflow-y: auto;
    animation: slideDown 0.15s ease-out;
  }

  @keyframes slideDown {
    from {
      opacity: 0;
      transform: translateY(-0.5rem);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .neu-select-option {
    padding: 0.5rem 1rem;
    cursor: pointer;
    transition: background 0.1s ease;
    color: var(--text-primary);
    font-family: var(--sans);
  }

  .neu-select-option:hover {
    background: var(--bg-subtle);
  }

  .neu-select-option.is-focused {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .neu-select-option.is-selected {
    background: var(--accent);
    color: white;
  }

  .neu-select-option.is-selected.is-focused {
    background: var(--accent);
    color: white;
  }
</style>
