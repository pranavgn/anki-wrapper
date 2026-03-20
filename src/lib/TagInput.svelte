<script lang="ts">
  // Props using Svelte 5 runes
  interface Props {
    tags?: string[];
    suggestions?: string[];
    placeholder?: string;
  }

  let { tags = $bindable([]), suggestions = [], placeholder = "Add tag..." }: Props = $props();

  // Internal state
  let inputValue = $state("");
  let showSuggestions = $state(false);
  let selectedIndex = $state(-1);
  let inputElement: HTMLInputElement | null = $state(null);
  let containerElement: HTMLDivElement | null = $state(null);

  // Filter suggestions to exclude already added tags
  let filteredSuggestions = $derived(
    suggestions.filter(
      (s: string) =>
        !tags.includes(s.toLowerCase()) &&
        s.toLowerCase().includes(inputValue.toLowerCase())
    )
  );

  // Add a new tag
  function addTag(tag: string) {
    const normalizedTag = tag.trim().toLowerCase();
    if (normalizedTag && !tags.includes(normalizedTag)) {
      tags = [...tags, normalizedTag];
    }
    inputValue = "";
    showSuggestions = false;
    selectedIndex = -1;
    inputElement?.focus();
  }

  // Remove a tag by index
  function removeTag(index: number) {
    tags = tags.filter((_: string, i: number) => i !== index);
    inputElement?.focus();
  }

  // Handle keyboard events
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" || event.key === ",") {
      event.preventDefault();
      if (inputValue.trim()) {
        addTag(inputValue);
      }
    } else if (event.key === "Backspace" && !inputValue && tags.length > 0) {
      // Remove last tag when backspace is pressed on empty input
      removeTag(tags.length - 1);
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      if (filteredSuggestions.length > 0) {
        showSuggestions = true;
        selectedIndex = Math.min(selectedIndex + 1, filteredSuggestions.length - 1);
      }
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      if (filteredSuggestions.length > 0) {
        showSuggestions = true;
        selectedIndex = Math.max(selectedIndex - 1, 0);
      }
    } else if (event.key === "Escape") {
      showSuggestions = false;
      selectedIndex = -1;
    }
  }

  // Handle input focus
  function handleFocus() {
    showSuggestions = inputValue.length > 0 || suggestions.length > 0;
  }

  // Handle input blur with delay to allow click events on suggestions
  function handleBlur() {
    setTimeout(() => {
      showSuggestions = false;
      selectedIndex = -1;
    }, 150);
  }

  // Handle suggestion click
  function handleSuggestionClick(suggestion: string) {
    addTag(suggestion);
  }
</script>

<div class="relative" bind:this={containerElement}>
  <!-- Tag Input Container -->
  <div
    class="flex flex-wrap gap-2 min-h-[44px] p-3 bg-bg-subtle border border-border rounded-xl focus-within:border-accent focus-within:outline-none transition-colors"
  >
    <!-- Render existing tags as pills -->
    {#each tags as tag, index}
      <span class="inline-flex items-center gap-1 bg-accent-soft text-accent text-sm rounded-full px-3 py-1 font-medium">
        {tag}
        <button
          type="button"
          onclick={() => removeTag(index)}
          class="ml-1 hover:text-accent/70 focus:outline-none"
          aria-label="Remove tag {tag}"
        >
          <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </span>
    {/each}

    <!-- Input field -->
    <input
      type="text"
      bind:this={inputElement}
      bind:value={inputValue}
      onkeydown={handleKeydown}
      onfocus={handleFocus}
      onblur={handleBlur}
      class="flex-grow min-w-[120px] bg-transparent border-none outline-none text-text-primary placeholder:text-text-secondary/70 text-sm"
      {placeholder}
    />
  </div>

  <!-- Suggestions Dropdown -->
  {#if showSuggestions && filteredSuggestions.length > 0}
    <div class="absolute z-50 w-full bg-white rounded-xl shadow-lg border border-border max-h-[200px] overflow-y-auto mt-1">
      {#each filteredSuggestions as suggestion, index}
        <button
          type="button"
          onclick={() => handleSuggestionClick(suggestion)}
          class="w-full px-4 py-2 text-sm text-left hover:bg-bg-subtle cursor-pointer transition-colors {index === selectedIndex ? 'bg-bg-subtle' : ''}"
        >
          {suggestion}
        </button>
      {/each}
    </div>
  {/if}
</div>
