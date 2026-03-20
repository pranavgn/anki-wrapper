<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { addToast } from "./toast";
  import DeckOptions from "./DeckOptions.svelte";
  import {
    pickAndImportApkg,
    pickAndImportColpkg,
    pickAndImportText,
    exportDeckApkg,
    ImportError,
  } from "./importer";

  // Collection status prop
  type CollectionStatus = "loading" | "ready" | "error";
  let { collectionStatus = "loading" }: { collectionStatus?: CollectionStatus } = $props();

  // Deck stats type
  type DeckStat = {
    id: number;
    name: string;
    new_cards: number;
    learn_cards: number;
    review_cards: number;
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
  let isCreatingDeck = $state(false);
  let newDeckName = $state("");
  let isLoading = $state(true);
  let isImporting = $state(false);
  let showImportMenu = $state(false);
  let importMenuRef: HTMLDivElement | null = $state(null);
  let openDeckMenuId: number | null = $state(null);
  let selectedDecks: Set<number> = $state(new Set());
  let optionsDeckId: number | null = $state(null);

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

  function handleClickOutside(event: MouseEvent) {
    if (importMenuRef && !importMenuRef.contains(event.target as Node)) {
      showImportMenu = false;
    }
  }

  // Derived greeting based on time
  const greeting = $derived.by(() => {
    const hour = new Date().getHours();
    if (hour < 12) return "Good morning";
    if (hour < 18) return "Good afternoon";
    return "Good evening";
  });

  // Derived total due cards
  const totalDue = $derived.by(() => {
    return decks.reduce((sum, deck) => sum + deck.new_cards + deck.learn_cards + deck.review_cards, 0);
  });

  // Load deck stats on mount
  onMount(async () => {
    await loadDeckStats();
    window.addEventListener('refresh-decks', loadDeckStats);
    return () => {
      window.removeEventListener('refresh-decks', loadDeckStats);
    };
  });

  async function loadDeckStats() {
    isLoading = true;
    try {
      const result = await invoke<DeckStat[]>("get_deck_stats");
      decks = result;
    } catch (error) {
      console.error("Error loading deck stats:", error);
      addToast(error instanceof Error ? error.message : "Failed to load decks", "error");
      decks = [];
    } finally {
      isLoading = false;
    }
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
        deck_id: 1,
        notetype_name: "Basic",
        delimiter: ",",
        html_enabled: false,
        duplicate_policy: "ignore",
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

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      handleCreateDeckSubmit();
    } else if (event.key === "Escape") {
      handleCancelCreateDeck();
    }
  }

  async function handleDeckClick(deckId: number, deckName: string) {
    // Add tactile feedback
    const deckElement = document.querySelector(`[data-deck-id="${deckId}"]`);
    if (deckElement) {
      deckElement.classList.add('pressed');
      await new Promise(resolve => setTimeout(resolve, 80));
      deckElement.classList.remove('pressed');
    }
    // Navigate to study mode
    // This would typically be handled by the parent component
    console.log(`Starting review for deck: ${deckName}`);
  }
</script>

<div class="max-w-5xl mx-auto">
  <!-- Header with greeting -->
  <div class="mb-8">
    <h1 class="text-2xl font-semibold text-text-primary mb-2">{greeting}</h1>
    <p class="text-text-secondary">
      {totalDue} cards due today across {decks.length} {decks.length === 1 ? 'deck' : 'decks'}
    </p>
  </div>

  <!-- Deck Grid -->
  {#if selectedDecks.size > 0}
    <div class="flex items-center gap-4 mb-4 p-3 bg-accent-soft rounded-xl">
      <span class="text-sm text-text-primary font-medium">{selectedDecks.size} deck{selectedDecks.size > 1 ? 's' : ''} selected</span>
      <button
        class="px-3 py-1.5 text-sm bg-bg-card border border-border rounded-lg hover:bg-bg-subtle transition-colors cursor-pointer"
        onclick={clearSelection}
      >
        Clear
      </button>
      <button
        class="px-3 py-1.5 text-sm bg-accent text-white rounded-lg hover:bg-accent/90 transition-colors cursor-pointer"
        onclick={() => exportSelectedDecks(false)}
      >
        Export Selected
      </button>
    </div>
  {/if}
  
  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 mb-8">
    {#each decks as deck, index (deck.id)}
      <div
        data-deck-id={deck.id}
        class="bg-bg-card border border-border rounded-2xl p-6 transition-all duration-200 hover:-translate-y-0.5 hover:shadow-lg animate-in fade-in slide-in-from-bottom-4 cursor-pointer relative"
        style={`animation-delay: ${index * 40}ms`}
        role="button"
        tabindex="0"
        onclick={() => handleDeckClick(deck.id, deck.name)}
        onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && handleDeckClick(deck.id, deck.name)}
      >
        <!-- Checkbox -->
        <label class="absolute top-4 left-4 cursor-pointer" onclick={(e) => e.stopPropagation()}>
          <input
            type="checkbox"
            checked={selectedDecks.has(deck.id)}
            onchange={() => toggleDeckSelection(deck.id)}
            class="w-5 h-5 rounded border-border text-accent focus:ring-accent/30"
          />
        </label>
        
        <div class="flex justify-between items-start mb-4 pt-6">
          <h3 class="text-lg font-semibold text-text-primary line-clamp-2">{deck.name}</h3>
          <div class="relative">
            <button
              class="p-1.5 text-text-secondary hover:text-text-primary rounded-lg hover:bg-bg-subtle transition-colors cursor-pointer"
              aria-label="Deck options"
              onclick={(e) => { e.stopPropagation(); openDeckMenuId = openDeckMenuId === deck.id ? null : deck.id; }}
            >
              <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 12h.01M12 12h.01M19 12h.01M6 12a1 1 0 11-2 0 1 1 0 012 0zm7 0a1 1 0 11-2 0 1 1 0 012 0zm7 0a1 1 0 11-2 0 1 1 0 012 0z" />
              </svg>
            </button>
            {#if openDeckMenuId === deck.id}
              <div class="absolute right-0 mt-1 w-48 bg-bg-card border border-border rounded-xl shadow-lg z-20 overflow-hidden">
                <button
                  class="w-full px-4 py-2.5 text-left text-sm text-text-primary hover:bg-bg-subtle transition-colors cursor-pointer flex items-center gap-2"
                  onclick={(e) => { e.stopPropagation(); optionsDeckId = deck.id; openDeckMenuId = null; }}
                >
                  <svg class="h-4 w-4 text-text-secondary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                  </svg>
                  Deck Options
                </button>
                <button
                  class="w-full px-4 py-2.5 text-left text-sm text-text-primary hover:bg-bg-subtle transition-colors cursor-pointer flex items-center gap-2"
                  onclick={(e) => { e.stopPropagation(); handleExportDeck(deck.id, false); }}
                >
                  <svg class="h-4 w-4 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4 8l-4-4m0 0L8 16m4-4v12" />
                  </svg>
                  Export Deck (.apkg)
                </button>
                <button
                  class="w-full px-4 py-2.5 text-left text-sm text-text-primary hover:bg-bg-subtle transition-colors cursor-pointer flex items-center gap-2"
                  onclick={(e) => { e.stopPropagation(); handleExportDeck(deck.id, true); }}
                >
                  <svg class="h-4 w-4 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4 8l-4-4m0 0L8 16m4-4v12" />
                  </svg>
                  Export with scheduling
                </button>
              </div>
            {/if}
          </div>
        </div>
        
        <div class="flex flex-wrap gap-2 mb-6">
          {#if deck.new_cards > 0}
            <span class="inline-flex items-center px-2 py-1 bg-[#DBEAFE] text-[#1D4ED8] text-xs font-medium rounded-full">
              New: {deck.new_cards}
            </span>
          {/if}
          {#if deck.learn_cards > 0}
            <span class="inline-flex items-center px-2 py-1 bg-[#FCE7F3] text-[#BE185D] text-xs font-medium rounded-full">
              Learn: {deck.learn_cards}
            </span>
          {/if}
          {#if deck.review_cards > 0}
            <span class="inline-flex items-center px-2 py-1 bg-[#D1FAE5] text-[#065F46] text-xs font-medium rounded-full">
              Review: {deck.review_cards}
            </span>
          {/if}
        </div>
        
        {#if deck.new_cards === 0 && deck.learn_cards === 0 && deck.review_cards === 0}
          <div class="text-text-secondary text-sm text-center py-2">
            All caught up ✓
          </div>
        {:else}
          <button
            class="w-full px-4 py-2 bg-accent text-white rounded-xl hover:bg-accent/90 transition-colors text-sm font-medium cursor-pointer"
          >
            Study Now
          </button>
        {/if}
      </div>
    {/each}

    <!-- Loading skeletons -->
    {#if isLoading}
      {#each Array(3) as _, index}
        <div class="bg-bg-card border border-border rounded-2xl p-6 animate-in fade-in slide-in-from-bottom-4" style={`animation-delay: ${index * 40}ms`}>
          <div class="skeleton h-5 w-3/4 mb-4"></div>
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
        class="bg-bg-card border-2 border-dashed border-border rounded-2xl p-6 flex flex-col items-center justify-center h-full min-h-[200px] transition-all duration-200 hover:border-accent/50 hover:bg-accent-soft/20 cursor-pointer"
      >
        <svg class="h-10 w-10 text-text-secondary mb-3 transition-colors hover:text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        <span class="text-text-primary font-medium">+ New Deck</span>
      </button>
    {:else if isCreatingDeck}
      <div class="bg-bg-card border-2 border-dashed border-accent rounded-2xl p-6 flex flex-col items-center justify-center h-full min-h-[200px] animate-in fade-in zoom-in-95">
        <div class="w-full mb-4">
          <input
            type="text"
            bind:value={newDeckName}
            onkeydown={handleKeyDown}
            class="w-full px-4 py-2 bg-bg-subtle border border-border rounded-xl text-text-primary placeholder:text-text-secondary focus:outline-none focus:ring-2 focus:ring-accent/50 focus:border-accent transition-all"
            placeholder="Deck name..."
          />
        </div>
        <div class="flex gap-3 w-full">
          <button
            onclick={handleCreateDeckSubmit}
            class="flex-1 px-4 py-2 bg-accent text-white rounded-xl hover:bg-accent/90 transition-colors text-sm font-medium cursor-pointer"
          >
            Create
          </button>
          <button
            onclick={handleCancelCreateDeck}
            class="px-4 py-2 bg-bg-subtle text-text-primary rounded-xl hover:bg-bg-subtle/80 transition-colors text-sm font-medium cursor-pointer"
          >
            Cancel
          </button>
        </div>
      </div>
    {/if}
  </div>

  {#if optionsDeckId !== null}
    {@const selectedDeck = decks.find(d => d.id === optionsDeckId)}
    {#if selectedDeck}
      <DeckOptions 
        deckId={optionsDeckId} 
        deckName={selectedDeck.name} 
        onClose={() => optionsDeckId = null} 
      />
    {/if}
  {/if}
</div>

<style>
</style>
