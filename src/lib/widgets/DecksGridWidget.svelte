<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { addToast } from "../toast";
  import { tick } from "svelte";

  interface DeckStat {
    id: number;
    name: string;
    short_name: string;
    level: number;
    new_count: number;
    learn_count: number;
    review_count: number;
    card_count: number;
    is_filtered: boolean;
    children?: DeckStat[];
  }

  let {
    onStudy = (deckId: number, deckName: string) => {},
    compact = false
  }: {
    onStudy?: (deckId: number, deckName: string) => void;
    compact?: boolean;
  } = $props();

  let decks: DeckStat[] = $state([]);
  let expandedDecks: Set<number> = $state(new Set());
  let isLoading = $state(false);
  let selectionMode = $state(false);
  let selectedDecks: Set<number> = $state(new Set());
  let selectMode = $state(false);
  
  // Keyboard navigation state
  let focusedDeckIndex = $state(-1);
  let gridEl: HTMLDivElement;

  // Drag-and-drop state
  let draggedDeckId: number | null = $state(null);
  let dragOverDeckId: number | null = $state(null);
  let dragOverRoot: boolean = $state(false);

  // Deck stats cache
  let lastDeckStatsTime = 0;
  const DECK_STATS_TTL = 3000; // 3 seconds

  function toggleDeckSelection(deckId: number) {
    if (selectedDecks.has(deckId)) {
      selectedDecks.delete(deckId);
    } else {
      selectedDecks.add(deckId);
    }
    selectedDecks = new Set(selectedDecks);
  }

  function selectAllDecks() {
    selectedDecks = new Set(decks.map(d => d.id));
  }

  function clearSelection() {
    selectedDecks = new Set();
    selectionMode = false;
  }

  function toggleSelectMode() {
    if (selectMode) {
      clearSelection();
    }
    selectMode = !selectMode;
  }

  async function exportSelectedDecks(includeScheduling: boolean) {
    if (selectedDecks.size === 0) {
      addToast("No decks selected", "error");
      return;
    }
    for (const deckId of selectedDecks) {
      await handleExportDeck(deckId, includeScheduling);
    }
  }

  async function handleExportDeck(deckId: number, includeScheduling: boolean) {
    try {
      const { exportDeckApkg, ImportError } = await import("../importer");
      await exportDeckApkg(deckId, includeScheduling);
      addToast("Deck exported successfully", "success");
    } catch (e) {
      if (e instanceof ImportError && e.isCancelled) return;
      addToast(e instanceof Error ? e.message : "Export failed", "error");
    }
  }

  // Drag-and-drop handlers
  function handleDragStart(e: DragEvent, deckId: number) {
    draggedDeckId = deckId;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('text/plain', String(deckId));
    }
  }

  function handleDragOver(e: DragEvent, targetDeckId: number) {
    if (draggedDeckId === null || draggedDeckId === targetDeckId) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    dragOverDeckId = targetDeckId;
  }

  function handleDragLeave() {
    dragOverDeckId = null;
  }

  async function handleDrop(e: DragEvent, targetDeckId: number) {
    e.preventDefault();
    dragOverDeckId = null;
    if (draggedDeckId === null || draggedDeckId === targetDeckId) return;

    try {
      await invoke('reparent_deck', {
        deckId: draggedDeckId,
        newParentId: targetDeckId
      });
      addToast('Deck moved successfully', 'success');
      await loadDeckStats(true);
    } catch (err) {
      addToast(`Failed to move deck: ${err}`, 'error');
    } finally {
      draggedDeckId = null;
    }
  }

  function handleRootDragOver(e: DragEvent) {
    if (draggedDeckId === null) return;
    e.preventDefault();
    dragOverRoot = true;
  }

  function handleRootDragLeave() {
    dragOverRoot = false;
  }

  async function handleRootDrop(e: DragEvent) {
    e.preventDefault();
    dragOverRoot = false;
    if (draggedDeckId === null) return;

    try {
      await invoke('reparent_deck', {
        deckId: draggedDeckId,
        newParentId: null
      });
      addToast('Deck moved to root', 'success');
      await loadDeckStats(true);
    } catch (err) {
      addToast(`Failed to move deck: ${err}`, 'error');
    } finally {
      draggedDeckId = null;
    }
  }

  function handleDragEnd() {
    draggedDeckId = null;
    dragOverDeckId = null;
    dragOverRoot = false;
  }

  // Derived total due cards
  const totalDue = $derived.by(() => {
    return decks.reduce((sum, deck) => sum + deck.new_count + deck.learn_count + deck.review_count, 0);
  });

  // Load deck stats on mount
  import { onMount } from "svelte";
  onMount(async () => {
    await loadDeckStats();
    window.addEventListener('refresh-decks', () => loadDeckStats(true));
    return () => {
      window.removeEventListener('refresh-decks', () => loadDeckStats(true));
    };
  });

  async function loadDeckStats(force = false) {
    const now = Date.now();
    if (!force && now - lastDeckStatsTime < DECK_STATS_TTL && decks.length > 0) return;
    
    isLoading = true;
    try {
      const result = await invoke<DeckStat[]>("get_all_decks");
      decks = result;
      lastDeckStatsTime = Date.now();
    } catch (error) {
      console.error("Error loading deck stats:", error);
      addToast(error instanceof Error ? error.message : "Failed to load decks", "error");
      decks = [];
    } finally {
      isLoading = false;
    }
  }

  function toggleDeckExpanded(deckId: number) {
    if (expandedDecks.has(deckId)) {
      expandedDecks.delete(deckId);
    } else {
      expandedDecks.add(deckId);
    }
    expandedDecks = new Set(expandedDecks);
  }

  function getDeckIndent(level: number): string {
    return `padding-left: ${level * 20}px`;
  }

  function shouldShowDeck(deck: DeckStat): boolean {
    // Show root level decks always
    if (deck.level <= 1) return true;
    
    // Check if any ancestor is collapsed
    let currentLevel = deck.level;
    for (const d of decks) {
      if (d.level < deck.level && d.level >= 1) {
        // Check if this is an ancestor
        const nameParts = deck.name.split('::');
        const ancestorName = nameParts.slice(0, d.level).join('::');
        if (d.name === ancestorName && !expandedDecks.has(d.id)) {
          return false;
        }
      }
    }
    return true;
  }

  async function handleDeckClick(deckId: number, deckName: string) {
    if (selectionMode) {
      toggleDeckSelection(deckId);
    } else {
      onStudy(deckId, deckName);
    }
  }

  // Keyboard navigation for deck grid
  function handleGridKeydown(e: KeyboardEvent) {
    const visibleDecks = decks.filter(d => shouldShowDeck(d));
    if (visibleDecks.length === 0) return;

    switch (e.key) {
      case 'ArrowRight':
        e.preventDefault();
        focusedDeckIndex = Math.min(focusedDeckIndex + 1, visibleDecks.length - 1);
        focusDeck(focusedDeckIndex);
        break;
      case 'ArrowLeft':
        e.preventDefault();
        focusedDeckIndex = Math.max(focusedDeckIndex - 1, 0);
        focusDeck(focusedDeckIndex);
        break;
      case 'ArrowDown':
        e.preventDefault();
        // Move to next row (assuming 3 columns)
        focusedDeckIndex = Math.min(focusedDeckIndex + 3, visibleDecks.length - 1);
        focusDeck(focusedDeckIndex);
        break;
      case 'ArrowUp':
        e.preventDefault();
        // Move to previous row (assuming 3 columns)
        focusedDeckIndex = Math.max(focusedDeckIndex - 3, 0);
        focusDeck(focusedDeckIndex);
        break;
      case 'Enter':
      case ' ':
        e.preventDefault();
        if (focusedDeckIndex >= 0 && focusedDeckIndex < visibleDecks.length) {
          const deck = visibleDecks[focusedDeckIndex];
          handleDeckClick(deck.id, deck.name);
        }
        break;
    }
  }

  function focusDeck(index: number) {
    if (gridEl) {
      const deckCards = gridEl.querySelectorAll('[data-deck-id]');
      if (deckCards[index]) {
        (deckCards[index] as HTMLElement).focus();
      }
    }
  }
</script>

<div class="decks-grid-widget">
  <div
    bind:this={gridEl}
    ondragover={handleRootDragOver}
    ondragleave={handleRootDragLeave}
    ondrop={handleRootDrop}
    onkeydown={handleGridKeydown}
    role="grid"
    tabindex="-1"
    aria-label="Deck grid"
    style="{dragOverRoot ? 'outline: 2px dashed var(--text-muted); outline-offset: 8px; border-radius: 16px;' : ''}"
  >
    {#if compact}
      <!-- Compact mode: Single-column list layout -->
      <div class="decks-list-compact" role="list" aria-label="Deck list">
        {#each decks.filter(d => shouldShowDeck(d)) as deck, index (deck.id)}
          <div
            data-deck-id={deck.id}
            class="deck-card-compact"
            style="
              background: var(--bg-card);
              box-shadow: var(--neu-subtle);
              border: 1px solid var(--border);
              border-radius: 10px;
              padding: 12px 14px;
              cursor: pointer;
              animation-delay: {index * 20}ms;
              {selectionMode && selectedDecks.has(deck.id) ? 'outline: 2px solid var(--accent); outline-offset: 2px;' : ''}
              {deck.level > 0 ? `margin-left: ${(deck.level - 1) * 16}px;` : ''}
              {dragOverDeckId === deck.id ? 'outline: 2px dashed var(--accent); outline-offset: 4px;' : ''}
              {draggedDeckId === deck.id ? 'opacity: 0.5;' : ''}
            "
            role="button"
            tabindex="0"
            aria-label="Deck: {deck.short_name || deck.name}. {deck.new_count} new, {deck.learn_count} learning, {deck.review_count} due for review."
            draggable="true"
            ondragstart={(e) => handleDragStart(e, deck.id)}
            ondragover={(e) => handleDragOver(e, deck.id)}
            ondragleave={handleDragLeave}
            ondrop={(e) => handleDrop(e, deck.id)}
            ondragend={handleDragEnd}
            onclick={() => {
              if (selectMode) {
                toggleDeckSelection(deck.id);
              } else {
                handleDeckClick(deck.id, deck.name);
              }
            }}
            onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && handleDeckClick(deck.id, deck.name)}
          >
            <div style="display: flex; justify-content: space-between; align-items: center;">
              <span style="font-family: var(--sans); font-size: 13px; font-weight: 600; color: var(--text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 160px;">
                {deck.short_name || deck.name}
              </span>
              <div style="display: flex; align-items: center; gap: 6px;">
                {#if deck.new_count > 0}
                  <span style="font-size: 11px; color: #3B82F6; font-family: var(--sans);">{deck.new_count}</span>
                {/if}
                {#if deck.learn_count > 0}
                  <span style="font-size: 11px; color: #EC4899; font-family: var(--sans);">{deck.learn_count}</span>
                {/if}
                {#if deck.review_count > 0}
                  <span style="font-size: 11px; color: #10B981; font-family: var(--sans);">{deck.review_count}</span>
                {/if}
              </div>
            </div>
          </div>
        {/each}

      </div>
    {:else}
      <!-- Non-compact mode: Grid layout (original) -->
      <div class="grid gap-7" style="grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));" role="grid" aria-label="Deck grid">
        {#each decks.filter(d => shouldShowDeck(d)) as deck, index (deck.id)}
          <div
            data-deck-id={deck.id}
            class="deck-card bg-bg-card rounded-2xl p-6 cursor-pointer relative"
            style="
              box-shadow: var(--neu-subtle);
              border: 1px solid var(--border);
              animation-delay: {index * 30}ms;
              {selectionMode && selectedDecks.has(deck.id) ? 'outline: 2px solid var(--accent); outline-offset: 2px;' : ''}
              {deck.level > 0 ? `margin-left: ${(deck.level - 1) * 20}px;` : ''}
              {dragOverDeckId === deck.id ? 'outline: 2px dashed var(--accent); outline-offset: 4px;' : ''}
              {draggedDeckId === deck.id ? 'opacity: 0.5;' : ''}
            "
            role="button"
            tabindex="0"
            aria-label="Deck: {deck.short_name || deck.name}. {deck.card_count} cards total. {deck.new_count} new, {deck.learn_count} learning, {deck.review_count} due for review."
            draggable="true"
            ondragstart={(e) => handleDragStart(e, deck.id)}
            ondragover={(e) => handleDragOver(e, deck.id)}
            ondragleave={handleDragLeave}
            ondrop={(e) => handleDrop(e, deck.id)}
            ondragend={handleDragEnd}
            onclick={() => {
              if (selectMode) {
                toggleDeckSelection(deck.id);
              } else {
                handleDeckClick(deck.id, deck.name);
              }
            }}
            onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && handleDeckClick(deck.id, deck.name)}
          >
            <!-- Selection checkbox -->
            {#if selectMode}
              <div
                class="absolute -top-2 -left-2 w-6 h-6 rounded-full flex items-center justify-center cursor-pointer"
                style="
                  background: {selectedDecks.has(deck.id) ? 'var(--accent)' : 'var(--bg-card)'};
                  box-shadow: var(--neu-subtle);
                "
                role="checkbox"
                aria-checked={selectedDecks.has(deck.id)}
                aria-label="Select {deck.short_name || deck.name}"
                tabindex="0"
                onclick={(e) => { e.stopPropagation(); toggleDeckSelection(deck.id); }}
                onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && toggleDeckSelection(deck.id)}
              >
                {#if selectedDecks.has(deck.id)}
                  <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: white;" aria-hidden="true">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
                  </svg>
                {/if}
              </div>
            {/if}

            <!-- Expand/Collapse button for nested decks -->
            {#if deck.level > 0}
              <button
                class="absolute top-4 left-2 w-6 h-6 flex items-center justify-center cursor-pointer"
                style="color: var(--text-secondary);"
                aria-label={expandedDecks.has(deck.id) ? "Collapse {deck.short_name || deck.name}" : "Expand {deck.short_name || deck.name}"}
                aria-expanded={expandedDecks.has(deck.id)}
                onclick={(e) => { e.stopPropagation(); toggleDeckExpanded(deck.id); }}
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  {#if expandedDecks.has(deck.id)}
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7" />
                  {:else}
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                  {/if}
                </svg>
              </button>
            {/if}

            <!-- Streak badge -->
            <div
              class="absolute top-4 right-4 neu-subtle px-2 py-1 rounded-lg"
              style="background: var(--bg-card); box-shadow: var(--neu-subtle);"
            >
              <span style="font-family: var(--sans); font-size: 12px; color: var(--success);">🔥 {deck.card_count}d</span>
            </div>

            <!-- Deck content -->
            <div class="flex items-center gap-3 mb-4">
              <span style="font-size: 30px;">📚</span>
              <div>
                <div class="flex items-center gap-2">
                  <h3 style="font-family: var(--serif); font-size: 20px; font-weight: 600; color: var(--text-primary); line-clamp: 2;">{deck.short_name || deck.name}</h3>
                  {#if deck.is_filtered}
                    <span class="px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider rounded-full" style="background: var(--accent-soft); color: var(--accent);">
                      Filtered
                    </span>
                  {/if}
                </div>
                <p style="font-family: var(--sans); font-size: 12px; color: var(--text-muted);">{deck.card_count} cards</p>
              </div>
            </div>

            <!-- Stats row -->
            <div class="flex items-center gap-4">
              {#if deck.new_count > 0}
                <div class="flex items-center gap-1.5">
                  <div class="w-[7px] h-[7px] rounded-full" style="background: #3B82F6;"></div>
                  <span style="font-family: var(--sans); font-size: 13px; color: var(--text-secondary);">{deck.new_count} new</span>
                </div>
              {/if}
              {#if deck.learn_count > 0}
                <div class="flex items-center gap-1.5">
                  <div class="w-[7px] h-[7px] rounded-full" style="background: #EC4899;"></div>
                  <span style="font-family: var(--sans); font-size: 13px; color: var(--text-secondary);">{deck.learn_count} due</span>
                </div>
              {/if}
              {#if deck.review_count > 0}
                <div class="flex items-center gap-1.5">
                  <div class="w-[7px] h-[7px] rounded-full" style="background: #10B981;"></div>
                  <span style="font-family: var(--sans); font-size: 13px; color: var(--text-secondary);">{deck.review_count} due</span>
                </div>
              {/if}
            </div>
          </div>
        {/each}

        <!-- New Deck Card -->
          <button
            aria-label="Create new deck"
            onclick={() => {
              // This would need to be handled by parent component
              // For now, just show a toast
              addToast("Create deck functionality moved to parent", "info");
            }}
            class="flex flex-col items-center justify-center cursor-pointer"
            style="
              background: var(--bg-base);
              box-shadow: var(--neu-down);
              border: 2px dashed var(--border);
              border-radius: var(--radius-md);
              padding: 30px 32px;
              min-height: 200px;
              animation: fadeUp 0.4s ease-out backwards;
              animation-delay: {decks.filter(d => shouldShowDeck(d)).length * 40}ms;
            "
          >
            <svg class="h-10 w-10 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
            </svg>
            <span style="font-family: var(--sans); font-size: 14px; font-weight: 500; color: var(--text-primary);">New Deck</span>
          </button>
      </div>
    {/if}
  </div>

  <!-- Floating Export Bar -->
  {#if selectMode && selectedDecks.size > 0}
    <div
      class="fixed bottom-6 left-1/2 -translate-x-1/2 z-40 flex items-center gap-3 px-5 py-3 rounded-2xl"
      style="
        background: var(--bg-card);
        box-shadow: 0 8px 32px rgba(0,0,0,0.15), var(--neu-up);
        border: 1px solid var(--border);
        animation: slideUp 0.25s cubic-bezier(0.2, 0.8, 0.3, 1);
      "
    >
      <span style="font-family: var(--sans); font-size: 13px; font-weight: 600; color: var(--text-primary);">
        {selectedDecks.size} selected
      </span>

      <div style="width: 1px; height: 20px; background: var(--border);"></div>

      <button
        onclick={() => exportSelectedDecks(false)}
        class="neu-btn px-3 py-1.5 rounded-lg cursor-pointer"
        style="background: var(--accent); color: white; font-family: var(--sans); font-size: 13px; font-weight: 500; border: none;"
      >
        Export .apkg
      </button>

      <button
        onclick={() => exportSelectedDecks(true)}
        class="neu-btn px-3 py-1.5 rounded-lg cursor-pointer"
        style="background: var(--bg-subtle); color: var(--text-primary); font-family: var(--sans); font-size: 13px; font-weight: 500; border: none;"
      >
        Export .colpkg
      </button>

      <button
        onclick={() => { clearSelection(); selectMode = false; }}
        class="neu-btn flex items-center justify-center w-8 h-8 rounded-lg cursor-pointer"
        style="background: var(--bg-subtle); border: none;"
        aria-label="Clear selection"
        title="Clear selection"
      >
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);" aria-hidden="true">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>
  {/if}
</div>

<style>
  .deck-card {
    opacity: 0;
    animation: deckFadeIn 0.25s cubic-bezier(0.2, 0.8, 0.3, 1) forwards;
    transition: transform 0.12s ease, box-shadow 0.12s ease;
  }

  .deck-card:hover {
    transform: translateY(-3px);
    box-shadow: var(--neu-up);
  }

  .decks-list-compact {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .deck-card-compact {
    opacity: 0;
    animation: deckFadeIn 0.25s cubic-bezier(0.2, 0.8, 0.3, 1) forwards;
    transition: transform 0.12s ease, box-shadow 0.12s ease;
  }

  .deck-card-compact:hover {
    transform: translateY(-2px);
    box-shadow: var(--neu-up);
  }

  @keyframes deckFadeIn {
    from {
      opacity: 0;
      transform: translateY(12px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes slideUp {
    from {
      opacity: 0;
      transform: translate(-50%, 20px);
    }
    to {
      opacity: 1;
      transform: translate(-50%, 0);
    }
  }
</style>
