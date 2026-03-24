<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { addToast } from "./toast";
  import DeckOptions from "./DeckOptions.svelte";
  import NeuDropdown from "./ui/NeuDropdown.svelte";
  import {
    pickAndImportApkg,
    pickAndImportColpkg,
    pickAndImportText,
    exportDeckApkg,
    ImportError,
  } from "./importer";

  // Collection status prop
  type CollectionStatus = "loading" | "ready" | "error";
  let { 
    collectionStatus = "loading",
    onStudy = (deckId: number, deckName: string) => {} 
  }: { 
    collectionStatus?: CollectionStatus;
    onStudy?: (deckId: number, deckName: string) => void;
  } = $props();

  // Deck stats type
  type DeckStat = {
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
  };

  // Import log type
  type ImportLog = {
    notes_added: number;
    notes_updated: number;
    notes_skipped: number;
    decks_added: string[];
  };

  // State for decks and new deck creation
  let decks: DeckStat[] = $state([]);
  let expandedDecks: Set<number> = $state(new Set());
  let isCreatingDeck = $state(false);
  let newDeckName = $state("");
  let isLoading = $state(true);
  let isImporting = $state(false);
  let showImportMenu = $state(false);
  let importMenuRef: HTMLDivElement | null = $state(null);
  let openDeckMenuId: number | null = $state(null);
  let selectedDecks: Set<number> = $state(new Set());
  let optionsDeckId: number | null = $state(null);
  let selectionMode = $state(false);
  let selectMode = $state(false);
  
  // Drag-and-drop state
  let draggedDeckId: number | null = $state(null);
  let dragOverDeckId: number | null = $state(null);
  let dragOverRoot: boolean = $state(false);
  
  // Deck stats cache
  let lastDeckStatsTime = 0;
  const DECK_STATS_TTL = 3000; // 3 seconds
  
  // Custom Study modal state
  let showCustomStudy = $state(false);
  let customStudyName = $state("");
  let customStudyQuery = $state("is:due");
  let customStudyLimit = $state(50);
  let customStudyOrder = $state(0);
  let orderLabels: string[] = $state([]);
  let isCreatingFilteredDeck = $state(false);

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
    openDeckMenuId = null;
    try {
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

  function handleClickOutside(event: MouseEvent) {
    if (importMenuRef && !importMenuRef.contains(event.target as Node)) {
      showImportMenu = false;
    }
  }

  // Derived total due cards
  const totalDue = $derived.by(() => {
    return decks.reduce((sum, deck) => sum + deck.new_count + deck.learn_count + deck.review_count, 0);
  });

  // Load deck stats on mount
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
      const result = await invoke<DeckStat[]>("get_deck_stats");
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

  async function handleImportApkg() {
    showImportMenu = false;
    try {
      const result = await pickAndImportApkg();
      await loadDeckStats();
      addToast(`Imported ${result.notes_added} new cards, ${result.notes_updated} updated`, "success");
    } catch (error) {
      if (error instanceof ImportError && error.isCancelled) return;
      console.error("Error importing apkg:", error);
      addToast(error instanceof Error ? error.message : "Failed to import package", "error");
    } finally {
      isImporting = false;
    }
  }

  async function handleImportColpkg() {
    showImportMenu = false;
    try {
      await pickAndImportColpkg();
      addToast("Collection imported successfully", "success");
    } catch (error) {
      if (error instanceof ImportError && error.isCancelled) return;
      console.error("Error importing colpkg:", error);
      addToast(error instanceof Error ? error.message : "Failed to import collection", "error");
    } finally {
      isImporting = false;
    }
  }

  async function handleImportText() {
    showImportMenu = false;
    try {
      const result = await pickAndImportText({
        deckId: 1,
        notetypeName: "Basic",
        delimiter: ",",
        htmlEnabled: false,
        duplicatePolicy: "ignore",
      });
      await loadDeckStats();
      addToast(`Imported ${result.notes_added} new cards, ${result.notes_updated} updated`, "success");
    } catch (error) {
      if (error instanceof ImportError && error.isCancelled) return;
      console.error("Error importing text:", error);
      addToast(error instanceof Error ? error.message : "Failed to import text file", "error");
    } finally {
      isImporting = false;
    }
  }

  function handleCreateDeckClick() {
    isCreatingDeck = true;
  }

  async function handleCreateDeckSubmit() {
    if (!newDeckName.trim()) return;
    
    try {
      await invoke("create_deck", { name: newDeckName.trim() });
      await loadDeckStats();
      isCreatingDeck = false;
      newDeckName = "";
    } catch (error) {
      console.error("Error creating deck:", error);
      addToast(error instanceof Error ? error.message : "Failed to create deck", "error");
    }
  }

  function handleCancelCreateDeck() {
    isCreatingDeck = false;
    newDeckName = "";
  }

  async function openCustomStudyModal() {
    showCustomStudy = true;
    // Load order labels if not already loaded
    if (orderLabels.length === 0) {
      try {
        const result = await invoke<{ labels: string[] }>("get_filtered_deck_order_labels");
        orderLabels = result.labels;
      } catch (error) {
        console.error("Error loading order labels:", error);
        // Use default labels
        orderLabels = [
          "Oldest reviewed first",
          "Random",
          "Increasing intervals",
          "Decreasing intervals",
          "Most lapses",
          "Added order",
          "Due date",
          "Latest added first",
          "Ascending retrievability",
          "Descending retrievability",
          "Relative overdueness"
        ];
      }
    }
  }

  function closeCustomStudyModal() {
    showCustomStudy = false;
    customStudyName = "";
    customStudyQuery = "is:due";
    customStudyLimit = 50;
    customStudyOrder = 0;
    isCreatingFilteredDeck = false;
  }

  async function handleCreateFilteredDeck() {
    if (!customStudyName.trim()) {
      addToast("Please enter a deck name", "error");
      return;
    }
    if (!customStudyQuery.trim()) {
      addToast("Please enter a search query", "error");
      return;
    }
    
    isCreatingFilteredDeck = true;
    try {
      const deckId = await invoke<number>("create_filtered_deck", {
        name: customStudyName.trim(),
        searchQuery: customStudyQuery.trim(),
        limit: customStudyLimit,
        order: customStudyOrder
      });
      await loadDeckStats();
      closeCustomStudyModal();
      addToast("Custom Study deck created", "success");
      // Navigate to study the newly created filtered deck
      onStudy(deckId, customStudyName.trim());
    } catch (error) {
      console.error("Error creating filtered deck:", error);
      addToast(error instanceof Error ? error.message : "Failed to create filtered deck", "error");
    } finally {
      isCreatingFilteredDeck = false;
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      handleCreateDeckSubmit();
    } else if (event.key === "Escape") {
      handleCancelCreateDeck();
    }
  }

  async function handleDeckClick(deckId: number, deckName: string) {
    if (selectionMode) {
      toggleDeckSelection(deckId);
    } else {
      onStudy(deckId, deckName);
    }
  }

  const importDropdownItems = [
    {
      label: "Import .apkg",
      description: "Anki deck package",
      onClick: handleImportApkg
    },
    {
      label: "Import .colpkg",
      description: "Full collection",
      onClick: handleImportColpkg
    },
    {
      label: "Import text/CSV",
      description: "Text file import",
      onClick: handleImportText
    }
  ];
</script>

<div class="max-w-[860px] mx-auto px-9 pt-13 pb-13" style="animation: fadeUp 0.4s ease-out;">
  <!-- Header Row -->
  <div class="flex items-center justify-between mb-12">
    <div>
      <h1 style="font-family: var(--serif); font-size: 36px; font-weight: 600; color: var(--text-primary); letter-spacing: -0.02em; margin-bottom: 4px;">Your Decks</h1>
      <p style="font-family: var(--sans); font-size: 14px; color: var(--text-secondary);">{totalDue} cards due today across {decks.length} {decks.length === 1 ? 'deck' : 'decks'}</p>
    </div>
    <div class="flex items-center gap-2">
      <NeuDropdown items={importDropdownItems}>
        <button
          class="neu-subtle neu-btn flex items-center gap-2 px-3.5 py-1.5 rounded-lg cursor-pointer"
          style="background: var(--bg-card); box-shadow: var(--neu-subtle);"
        >
          <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
          </svg>
          <span style="font-family: var(--sans); font-size: 13px; color: var(--text-secondary);">Import</span>
        </button>
      </NeuDropdown>
      <button
        onclick={toggleSelectMode}
        class="neu-subtle neu-btn px-4 py-2 rounded-lg cursor-pointer flex items-center gap-2"
        style="background: {selectMode ? 'var(--accent)' : 'var(--bg-card)'}; color: {selectMode ? 'white' : 'var(--text-secondary)'};"
      >
        {#if selectMode}
          <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
          </svg>
          <span style="font-family: var(--sans); font-size: 13px; font-weight: 500;">Done</span>
        {:else}
          <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
          </svg>
          <span style="font-family: var(--sans); font-size: 13px; font-weight: 500;">Select</span>
        {/if}
      </button>
    </div>
  </div>

  <!-- Deck Grid -->
  <div
    ondragover={handleRootDragOver}
    ondragleave={handleRootDragLeave}
    ondrop={handleRootDrop}
    style="{dragOverRoot ? 'outline: 2px dashed var(--text-muted); outline-offset: 8px; border-radius: 16px;' : ''}"
  >
  <div class="grid gap-7" style="grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));">
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
            onclick={(e) => { e.stopPropagation(); toggleDeckSelection(deck.id); }}
          >
            {#if selectedDecks.has(deck.id)}
              <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: white;">
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
            onclick={(e) => { e.stopPropagation(); toggleDeckExpanded(deck.id); }}
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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

    <!-- Loading skeletons -->
    {#if isLoading}
      {#each Array(3) as _, index}
        <div
          class="neu-raised"
          style="
            background: var(--bg-card);
            box-shadow: var(--neu-up);
            border-radius: var(--radius-md);
            padding: 30px 32px;
            animation: fadeUp 0.4s ease-out backwards;
            animation-delay: {index * 40}ms;
          "
        >
          <div class="skeleton h-6 w-3/4 mb-4"></div>
          <div class="flex gap-2 mb-6">
            <div class="skeleton h-6 w-12 rounded-full"></div>
            <div class="skeleton h-6 w-16 rounded-full"></div>
          </div>
          <div class="skeleton h-10 w-full rounded-xl"></div>
        </div>
      {/each}
    {/if}

    <!-- New Deck Card -->
    {#if !isCreatingDeck && !isLoading}
      <button
        onclick={handleCreateDeckClick}
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
        <svg class="h-10 w-10 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        <span style="font-family: var(--sans); font-size: 14px; font-weight: 500; color: var(--text-primary);">New Deck</span>
      </button>
    {:else if isCreatingDeck}
      <div
        class="flex flex-col items-center justify-center"
        style="
          background: var(--bg-card);
          box-shadow: var(--neu-down);
          border: 2px dashed var(--accent);
          border-radius: var(--radius-md);
          padding: 30px 32px;
          min-height: 200px;
          animation: fadeUp 0.4s ease-out;
        "
      >
        <div class="w-full mb-4">
          <input
            type="text"
            bind:value={newDeckName}
            onkeydown={handleKeyDown}
            class="w-full px-4 py-2 rounded-xl text-text-primary placeholder:text-text-muted focus:outline-none"
            style="
              background: var(--bg-subtle);
              box-shadow: var(--neu-down);
              border: none;
              font-family: var(--sans);
            "
            placeholder="Deck name..."
          />
        </div>
        <div class="flex gap-3 w-full">
          <button
            onclick={handleCreateDeckSubmit}
            class="flex-1 px-4 py-2 rounded-xl cursor-pointer"
            style="
              background: var(--accent);
              color: white;
              font-family: var(--sans);
              font-size: 14px;
              font-weight: 500;
              border: none;
            "
          >
            Create
          </button>
          <button
            onclick={handleCancelCreateDeck}
            class="px-4 py-2 rounded-xl cursor-pointer"
            style="
              background: var(--bg-subtle);
              color: var(--text-primary);
              font-family: var(--sans);
              font-size: 14px;
              font-weight: 500;
              border: none;
            "
          >
            Cancel
          </button>
        </div>
      </div>
    {/if}
  </div>
  </div>

  {#if optionsDeckId !== null}
    {@const selectedDeck = decks.find(d => d.id === optionsDeckId)}
    {#if selectedDeck}
      <DeckOptions 
        deckId={optionsDeckId} 
        deckName={selectedDeck.name} 
        isFiltered={selectedDeck.is_filtered}
        onClose={() => optionsDeckId = null} 
      />
    {/if}
  {/if}

  <!-- Custom Study Modal -->
  {#if showCustomStudy}
    <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div class="bg-bg-card rounded-2xl p-6 w-full max-w-lg shadow-xl animate-in fade-in zoom-in-95">
        <h2 class="text-xl font-bold text-text-primary mb-4">Custom Study</h2>
        
        <div class="space-y-4">
          <!-- Deck Name -->
          <div>
            <label class="block text-sm font-medium text-text-secondary mb-1">Deck Name</label>
            <input
              type="text"
              bind:value={customStudyName}
              class="w-full px-4 py-2 bg-bg-subtle rounded-xl text-text-primary placeholder:text-text-secondary focus:outline-none focus:ring-2 focus:ring-accent/50 transition-all"
              placeholder="My Custom Study Deck"
            />
          </div>

          <!-- Search Query -->
          <div>
            <label class="block text-sm font-medium text-text-secondary mb-1">
              Search Query
              <span class="text-xs font-normal text-text-secondary ml-2">(e.g., deck:MyDeck tag:exam is:due)</span>
            </label>
            <input
              type="text"
              bind:value={customStudyQuery}
              class="w-full px-4 py-2 bg-bg-subtle rounded-xl text-text-primary placeholder:text-text-secondary focus:outline-none focus:ring-2 focus:ring-accent/50 transition-all font-mono text-sm"
              placeholder="is:due"
            />
            <div class="mt-2 text-xs text-text-secondary">
              <p class="font-medium mb-1">Search tips:</p>
              <ul class="list-disc list-inside space-y-0.5">
                <li><code class="bg-bg-subtle px-1 rounded">deck:Name</code> - cards in deck "Name"</li>
                <li><code class="bg-bg-subtle px-1 rounded">tag:tagname</code> - cards with tag</li>
                <li><code class="bg-bg-subtle px-1 rounded">is:due</code> - cards due for review</li>
                <li><code class="bg-bg-subtle px-1 rounded">is:new</code> - new cards</li>
                <li><code class="bg-bg-subtle px-1 rounded">rated:7:1</code> - answered within 7 days</li>
                <li><code class="bg-bg-subtle px-1 rounded">prop:ivl>30</code> - interval over 30 days</li>
              </ul>
            </div>
          </div>

          <!-- Card Limit -->
          <div>
            <label class="block text-sm font-medium text-text-secondary mb-1">Card Limit</label>
            <input
              type="number"
              bind:value={customStudyLimit}
              min="1"
              max="9999"
              class="w-full px-4 py-2 bg-bg-subtle rounded-xl text-text-primary placeholder:text-text-secondary focus:outline-none focus:ring-2 focus:ring-accent/50 transition-all"
            />
          </div>

          <!-- Sort Order -->
          <div>
            <label class="block text-sm font-medium text-text-secondary mb-1">Sort Order</label>
            <select
              bind:value={customStudyOrder}
              class="w-full px-4 py-2 bg-bg-subtle rounded-xl text-text-primary focus:outline-none focus:ring-2 focus:ring-accent/50 transition-all"
            >
              {#each orderLabels as label, index}
                <option value={index}>{label}</option>
              {/each}
            </select>
          </div>
        </div>

        <div class="flex gap-3 mt-6">
          <button
            onclick={handleCreateFilteredDeck}
            disabled={isCreatingFilteredDeck}
            class="flex-1 px-4 py-2 bg-accent text-white rounded-xl hover:bg-accent/90 transition-colors text-sm font-medium disabled:opacity-50 cursor-pointer"
          >
            {isCreatingFilteredDeck ? 'Creating...' : 'Create'}
          </button>
          <button
            onclick={closeCustomStudyModal}
            class="px-4 py-2 bg-bg-subtle text-text-primary rounded-xl hover:bg-bg-subtle/80 transition-colors text-sm font-medium cursor-pointer"
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  {/if}

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
        title="Clear selection"
      >
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);">
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
</style>
