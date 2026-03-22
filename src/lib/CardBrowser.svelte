<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { fly, fade } from "svelte/transition";
  import { prefs } from "./prefs.svelte.ts";
  import { fly_if_enabled } from "./animate.svelte.ts";
  import { addToast } from "./toast";
  import CardDetailPanel from "./CardDetailPanel.svelte";

  // Props
  let { 
    initialQuery = "",
    onClose = () => {}
  }: { 
    initialQuery?: string; 
    onClose?: () => void;
  } = $props();

  // Mode: cards or notes
  type Mode = 'cards' | 'notes';
  let mode: Mode = $state('cards');
  
  // Column definitions
  const CARD_COLUMNS = [
    { key: 'select',    label: '',          width: '40px'  },
    { key: 'front',     label: 'Front',     width: '1fr'   },
    { key: 'deck',      label: 'Deck',      width: '140px' },
    { key: 'due',       label: 'Due',       width: '100px' },
    { key: 'interval',  label: 'Interval',  width: '80px'  },
    { key: 'ease',      label: 'Ease',      width: '70px'  },
    { key: 'lapses',    label: 'Lapses',    width: '70px'  },
    { key: 'tags',      label: 'Tags',      width: '160px' },
    { key: 'flag',      label: '',          width: '32px'  },
  ];
  
  const NOTE_COLUMNS = [
    { key: 'select',    label: '',          width: '40px'  },
    { key: 'front',     label: 'Front',     width: '1fr'   },
    { key: 'back',      label: 'Back',      width: '200px' },
    { key: 'notetype',  label: 'Note Type', width: '120px' },
    { key: 'deck',      label: 'Deck',      width: '120px' },
    { key: 'cards',     label: 'Cards',     width: '60px'  },
    { key: 'tags',      label: 'Tags',      width: '160px' },
    { key: 'created',   label: 'Created',   width: '90px'  },
  ];

  // Search state
  let query = $state(initialQuery);
  let debouncedQuery = $state(initialQuery);
  let searchError = $state<string | null>(null);
  let loading = $state(false);
  let rows: any[] = $state([]);
  let noteRows: any[] = $state([]);

  // Sort order
  let sortOrder = $state('cardDue');

  // Selection
  let selectedCardId: number | null = $state(null);
  let selectedNoteId: number | null = $state(null);
  let cardDetail: any = $state(null);
  
  // Bulk selection
  let selectedIds = $state<Set<number>>(new Set());
  let selectedNoteIds = $state<Set<number>>(new Set());
  let lastClickedIndex = $state<number | null>(null);
  let showMoveDropdown = $state(false);
  let showTagInput = $state(false);
  let showDeleteConfirm = $state(false);
  let newTag = $state('');
  let availableDecks: Array<{id: number, name: string}> = $state([]);
  let headerCheckbox: HTMLInputElement | null = $state(null);
  
  // Derived columns based on mode
  const columns = $derived(mode === 'cards' ? CARD_COLUMNS : NOTE_COLUMNS);
  
  // Derived selection state
  const allSelected = $derived(
    mode === 'cards' 
      ? rows.length > 0 && selectedIds.size === rows.length
      : noteRows.length > 0 && selectedNoteIds.size === noteRows.length
  );
  const someSelected = $derived(
    mode === 'cards'
      ? selectedIds.size > 0 && selectedIds.size < rows.length
      : selectedNoteIds.size > 0 && selectedNoteIds.size < noteRows.length
  );
  const selectionCount = $derived(mode === 'cards' ? selectedIds.size : selectedNoteIds.size);
  
  // Handle indeterminate checkbox state
  $effect(() => {
    if (headerCheckbox) {
      headerCheckbox.indeterminate = someSelected;
    }
  });

  // Filters parsed from query
  let filters: Array<{ key: string; value: string; display: string }> = $state([]);

  // Debounce effect
  $effect(() => {
    const t = setTimeout(() => {
      debouncedQuery = query;
    }, 300);
    return () => clearTimeout(t);
  });

  // Search effect
  $effect(() => {
    if (debouncedQuery !== undefined) {
      performSearch(debouncedQuery);
    }
  });

  // Parse filters from query
  $effect(() => {
    const tokens = query.split(/\s+/).filter(t => t);
    const newFilters: Array<{ key: string; value: string; display: string }> = [];
    
    for (const token of tokens) {
      const colonIndex = token.indexOf(':');
      if (colonIndex > 0) {
        const key = token.substring(0, colonIndex);
        const value = token.substring(colonIndex + 1);
        if (key && value) {
          newFilters.push({ key, value, display: token });
        }
      }
    }
    
    filters = newFilters;
  });

  async function performSearch(q: string) {
    loading = true;
    searchError = null;
    selectedIds = new Set();
    lastClickedIndex = null;
    selectedCardId = null;
    cardDetail = null;
    showMoveDropdown = false;
    showTagInput = false;
    showDeleteConfirm = false;
    
    try {
      if (mode === 'cards') {
        rows = await invoke('search_cards', { query: q || 'deck:*', order: sortOrder || '', limit: 100 });
      } else {
        noteRows = await invoke('search_notes', { query: q || 'deck:*', limit: 100 });
      }
    } catch (e) {
      searchError = e as string;
      if (mode === 'cards') {
        rows = [];
      } else {
        noteRows = [];
      }
    } finally {
      loading = false;
    }
  }

  function switchMode(newMode: Mode) {
    mode = newMode;
    performSearch(debouncedQuery);
  }

  function removeFilter(filter: { display: string }) {
    // Remove this filter from query
    const tokens = query.split(/\s+/).filter(t => t && t !== filter.display);
    query = tokens.join(' ');
    debouncedQuery = query;
  }

  function clearSearch() {
    query = '';
    debouncedQuery = '';
  }

  async function selectCard(cardId: number) {
    selectedCardId = cardId;
    selectedNoteId = null;
    cardDetail = null;
    
    try {
      cardDetail = await invoke('get_card_detail', { cardId });
    } catch (e) {
      addToast('Failed to load card details', 'error');
    }
  }

  async function selectNote(noteId: number, firstCardId: number) {
    selectedNoteId = noteId;
    selectedCardId = firstCardId;
    cardDetail = null;
    
    try {
      cardDetail = await invoke('get_card_detail', { cardId: firstCardId });
    } catch (e) {
      addToast('Failed to load note details', 'error');
    }
  }

  function closeDetail() {
    selectedCardId = null;
    selectedNoteId = null;
    cardDetail = null;
  }

  // Helper to get unique note IDs from selected card IDs
  function getUniqueNoteIds(): number[] {
    if (mode === 'notes') {
      return Array.from(selectedNoteIds);
    }
    const noteIds = new Set<number>();
    for (const cardId of selectedIds) {
      const row = rows.find(r => r.card_id === cardId);
      if (row && row.note_id) {
        noteIds.add(row.note_id);
      }
    }
    return Array.from(noteIds);
  }
  
  // Get card IDs for selected notes (for bulk operations in notes mode)
  async function getCardIdsForSelectedNotes(): Promise<number[]> {
    const noteIds = Array.from(selectedNoteIds);
    if (noteIds.length === 0) return [];
    try {
      return await invoke<number[]>('get_card_ids_for_notes', { noteIds });
    } catch (e) {
      console.error('Failed to get card IDs for notes:', e);
      return [];
    }
  }

  // Toggle selection for all rows
  function toggleSelectAll() {
    if (mode === 'notes') {
      if (allSelected) {
        selectedNoteIds = new Set();
      } else {
        selectedNoteIds = new Set(noteRows.map(r => r.note_id));
      }
    } else {
      if (allSelected) {
        selectedIds = new Set();
      } else {
        selectedIds = new Set(rows.map(r => r.card_id));
      }
    }
  }

  // Handle row checkbox click (cards mode)
  function handleRowSelect(cardId: number, index: number, event: MouseEvent) {
    if (event.shiftKey && lastClickedIndex !== null) {
      // Range selection
      const start = Math.min(lastClickedIndex, index);
      const end = Math.max(lastClickedIndex, index);
      const newSelection = new Set(selectedIds);
      for (let i = start; i <= end; i++) {
        newSelection.add(rows[i].card_id);
      }
      selectedIds = newSelection;
    } else {
      // Toggle single selection
      const newSelection = new Set(selectedIds);
      if (newSelection.has(cardId)) {
        newSelection.delete(cardId);
      } else {
        newSelection.add(cardId);
      }
      selectedIds = newSelection;
    }
    lastClickedIndex = index;
  }

  // Handle row checkbox click (notes mode)
  function handleNoteRowSelect(noteId: number, index: number, event: MouseEvent) {
    if (event.shiftKey && lastClickedIndex !== null) {
      // Range selection
      const start = Math.min(lastClickedIndex, index);
      const end = Math.max(lastClickedIndex, index);
      const newSelection = new Set(selectedNoteIds);
      for (let i = start; i <= end; i++) {
        newSelection.add(noteRows[i].note_id);
      }
      selectedNoteIds = newSelection;
    } else {
      // Toggle single selection
      const newSelection = new Set(selectedNoteIds);
      if (newSelection.has(noteId)) {
        newSelection.delete(noteId);
      } else {
        newSelection.add(noteId);
      }
      selectedNoteIds = newSelection;
    }
    lastClickedIndex = index;
  }

  // Clear selection
  function clearSelection() {
    selectedIds = new Set();
    selectedNoteIds = new Set();
    lastClickedIndex = null;
    showMoveDropdown = false;
    showTagInput = false;
    showDeleteConfirm = false;
    newTag = '';
  }

  // Bulk actions
  async function handleSuspend() {
    let cardIds: number[];
    if (mode === 'notes') {
      cardIds = await getCardIdsForSelectedNotes();
    } else {
      cardIds = Array.from(selectedIds);
    }
    try {
      await invoke('suspend_cards', { cardIds: Array.from(cardIds) });
      addToast(`${cardIds.length} cards suspended`, 'success');
      clearSelection();
      performSearch(debouncedQuery);
    } catch (e) {
      addToast('Failed to suspend cards', 'error');
    }
  }

  async function handleUnsuspend() {
    let cardIds: number[];
    if (mode === 'notes') {
      cardIds = await getCardIdsForSelectedNotes();
    } else {
      cardIds = Array.from(selectedIds);
    }
    try {
      await invoke('unsuspend_cards', { cardIds: Array.from(cardIds) });
      addToast(`${cardIds.length} cards unsuspended`, 'success');
      clearSelection();
      performSearch(debouncedQuery);
    } catch (e) {
      addToast('Failed to unsuspend cards', 'error');
    }
  }

  async function handleBury() {
    let cardIds: number[];
    if (mode === 'notes') {
      cardIds = await getCardIdsForSelectedNotes();
    } else {
      cardIds = Array.from(selectedIds);
    }
    try {
      await invoke('bury_cards', { cardIds: Array.from(cardIds) });
      addToast(`${cardIds.length} cards buried`, 'success');
      clearSelection();
      performSearch(debouncedQuery);
    } catch (e) {
      addToast('Failed to bury cards', 'error');
    }
  }

  async function handleMoveToDeck(deckId: number, deckName: string) {
    let cardIds: number[];
    if (mode === 'notes') {
      // In notes mode, move is disabled - show message
      addToast('Moving notes to deck is not supported. Move individual cards instead.', 'error');
      showMoveDropdown = false;
      return;
    } else {
      cardIds = Array.from(selectedIds);
    }
    try {
      await invoke('move_cards_to_deck', { cardIds: Array.from(cardIds), deckId });
      addToast(`${cardIds.length} cards moved to ${deckName}`, 'success');
      clearSelection();
      performSearch(debouncedQuery);
    } catch (e) {
      addToast('Failed to move cards', 'error');
    }
    showMoveDropdown = false;
  }

  async function handleAddTag() {
    if (!newTag.trim()) return;
    const noteIds = getUniqueNoteIds();
    try {
      await invoke('add_tags_to_notes', { noteIds: Array.from(noteIds), tag: newTag.trim() });
      addToast(`Tag added to ${noteIds.length} notes`, 'success');
      clearSelection();
      performSearch(debouncedQuery);
    } catch (e) {
      addToast('Failed to add tag', 'error');
    }
    showTagInput = false;
  }

  async function handleBulkDelete() {
    const noteIds = getUniqueNoteIds();
    try {
      await invoke('delete_notes', { noteIds: Array.from(noteIds) });
      addToast(`${noteIds.length} notes deleted`, 'success');
      clearSelection();
      performSearch(debouncedQuery);
    } catch (e) {
      addToast('Failed to delete notes', 'error');
    }
    showDeleteConfirm = false;
  }

  // Load decks for move dropdown
  async function loadDecks() {
    try {
      const result = await invoke<Array<{ id: number; name: string; shortName: string; level: number; newCount: number; learnCount: number; reviewCount: number; cardCount: number; isFiltered: boolean }>>('get_all_decks');
      availableDecks = result.map(d => ({ id: d.id, name: d.name }));
    } catch (e) {
      console.error('Failed to load decks:', e);
    }
  }

  // Handle keyboard shortcuts
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      if (showMoveDropdown || showTagInput || showDeleteConfirm) {
        showMoveDropdown = false;
        showTagInput = false;
        showDeleteConfirm = false;
      } else if (selectionCount > 0) {
        clearSelection();
      } else if (selectedCardId || selectedNoteId) {
        closeDetail();
      }
    } else if (event.ctrlKey && event.key === 'a') {
      event.preventDefault();
      if (mode === 'notes') {
        selectedNoteIds = new Set(noteRows.map(r => r.note_id));
      } else {
        selectedIds = new Set(rows.map(r => r.card_id));
      }
    }
  }
</script>

<div
  class="h-full flex flex-col bg-bg-base"
  in:fly={fly_if_enabled({ y: 40, duration: 280 })}
  out:fly={fly_if_enabled({ y: 40, duration: 200 })}
  onkeydown={handleKeydown}
  role="application"
  tabindex="-1"
>
  <!-- Header -->
  <div class="flex items-center gap-4 p-4 border-b border-border/30">
    <!-- Mode Toggle -->
    <div class="flex bg-bg-subtle rounded-xl p-1">
      <button
        onclick={() => switchMode('cards')}
        class="px-4 py-2 rounded-lg text-sm font-medium transition-all {mode === 'cards' ? 'neu-raised text-text-primary' : 'text-text-secondary hover:text-text-primary'}"
      >
        Cards
      </button>
      <button
        onclick={() => switchMode('notes')}
        class="px-4 py-2 rounded-lg text-sm font-medium transition-all {mode === 'notes' ? 'neu-raised text-text-primary' : 'text-text-secondary hover:text-text-primary'}"
      >
        Notes
      </button>
    </div>

    <!-- Search Input -->
    <div class="flex-1 relative">
      <svg 
        class="absolute left-4 top-1/2 -translate-y-1/2 h-4 w-4 text-text-secondary" 
        fill="none" stroke="currentColor" viewBox="0 0 24 24"
      >
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
      </svg>
      <input
        type="text"
        bind:value={query}
        placeholder="Search cards...  Try: deck:French  tag:vocab  is:due"
        class="w-full h-12 pl-12 pr-10 bg-bg-card rounded-2xl text-sm font-mono focus:outline-none focus:ring-3 focus:ring-accent-soft"
      />
      {#if query}
        <button
          onclick={clearSearch}
          class="absolute right-4 top-1/2 -translate-y-1/2 text-text-secondary hover:text-text-primary"
        >
          <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      {/if}
    </div>

    <!-- Close Button -->
    <button
      onclick={onClose}
      class="p-2 rounded-lg text-text-secondary hover:text-text-primary hover:bg-bg-subtle transition-colors"
    >
      <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
      </svg>
    </button>
  </div>

  <!-- Active Filters Bar -->
  {#if filters.length > 0}
    <div class="flex items-center gap-2 px-4 py-2 border-b border-border/30 bg-bg-subtle">
      {#each filters as filter}
        <button
          onclick={() => removeFilter(filter)}
          class="inline-flex items-center gap-1 px-3 py-1 bg-accent-soft text-accent text-sm rounded-full font-medium hover:bg-accent/20 transition-colors"
        >
          {filter.display}
          <svg class="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      {/each}
    </div>
  {/if}

  <!-- Search Error -->
  {#if searchError}
    <div class="px-4 py-3 border-b border-danger/20 bg-danger/5">
      <div class="flex items-center gap-2 text-danger text-sm">
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <span>{typeof searchError === 'string' ? searchError : 'Search failed. Try deck:name or tag:x syntax.'}</span>
      </div>
    </div>
  {/if}

  <!-- Main Content -->
  <div class="flex-1 flex overflow-hidden">
    <!-- Table -->
    <div class="flex-1 overflow-y-auto min-w-0">
      {#key mode}
        <div in:fade={{ duration: 150 }}>
          {#if loading}
        <!-- Loading skeleton -->
        <div class="p-4 space-y-2">
          {#each Array(8) as _, i}
            <div 
              class="h-11 rounded-lg animate-pulse flex gap-4"
              style="background: linear-gradient(90deg, var(--bg-subtle) 25%, var(--bg-card) 50%, var(--bg-subtle) 75%); background-size: 200% 100%; animation: shimmer 1.5s infinite;"
            >
              <div class="w-40 h-full bg-bg-subtle rounded"></div>
              <div class="flex-1 h-full bg-bg-subtle rounded"></div>
              <div class="w-24 h-full bg-bg-subtle rounded"></div>
              <div class="w-20 h-full bg-bg-subtle rounded"></div>
              <div class="w-16 h-full bg-bg-subtle rounded"></div>
              <div class="w-16 h-full bg-bg-subtle rounded"></div>
            </div>
          {/each}
        </div>
      {:else if searchError}
        <div class="flex flex-col items-center justify-center h-full text-text-secondary">
          <svg class="h-12 w-12 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <p>Search error</p>
          <p class="text-sm mt-1">{typeof searchError === 'string' ? searchError : 'Try deck:name or tag:x syntax.'}</p>
        </div>
      {:else if mode === 'cards'}
        {#if rows.length === 0}
          <!-- Empty state -->
          <div class="flex flex-col items-center justify-center h-full text-text-secondary">
            <svg class="h-16 w-16 mb-4 opacity-40" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0zM13 10H7" />
            </svg>
            <p class="text-lg font-medium">No cards found</p>
            <p class="text-sm mt-1">{query ? 'Try a different search query.' : 'Your collection is empty. Add cards in the editor.'}</p>
          </div>
        {:else}
          <!-- Results count -->
          <div class="px-4 py-2 text-xs text-text-secondary text-right border-b border-border/30">
            Showing {rows.length} cards
          </div>
          
          <!-- Cards table -->
          <div class="grid" style="grid-template-columns: 40px 1fr 140px 100px 80px 70px 70px 160px 32px;">
            <!-- Header -->
            <div class="sticky top-0 z-10 bg-bg-subtle text-xs uppercase tracking-wider text-text-secondary font-medium flex items-center px-2">
              <input 
                type="checkbox" 
                class="w-4 h-4 rounded cursor-pointer accent-accent"
                checked={allSelected}
                bind:this={headerCheckbox}
                onclick={toggleSelectAll}
              />
            </div>
            <div class="sticky top-0 z-10 bg-bg-subtle text-xs uppercase tracking-wider text-text-secondary font-medium flex items-center px-4">Front</div>
            <div class="sticky top-0 z-10 bg-bg-subtle text-xs uppercase tracking-wider text-text-secondary font-medium flex items-center px-4">Deck</div>
            <button 
              onclick={() => sortOrder = sortOrder === 'cardDue' ? 'cardDueDesc' : 'cardDue'}
              class="sticky top-0 z-10 bg-bg-subtle text-xs uppercase tracking-wider text-text-secondary font-medium flex items-center justify-between px-4 hover:text-text-primary"
            >
              Due
              {#if sortOrder.includes('Due')}
                <svg class="h-3 w-3 {sortOrder === 'cardDueDesc' ? 'rotate-180' : ''}" fill="currentColor" viewBox="0 0 20 20">
                  <path fill-rule="evenodd" d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z" clip-rule="evenodd" />
                </svg>
              {/if}
            </button>
            <button 
              onclick={() => sortOrder = sortOrder === 'cardInterval' ? 'cardIntervalDesc' : 'cardInterval'}
              class="sticky top-0 z-10 bg-bg-subtle text-xs uppercase tracking-wider text-text-secondary font-medium flex items-center justify-between px-4 hover:text-text-primary"
            >
              Interval
              {#if sortOrder.includes('Interval')}
                <svg class="h-3 w-3 {sortOrder === 'cardIntervalDesc' ? 'rotate-180' : ''}" fill="currentColor" viewBox="0 0 20 20">
                  <path fill-rule="evenodd" d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z" clip-rule="evenodd" />
                </svg>
              {/if}
            </button>
            <button 
              onclick={() => sortOrder = sortOrder === 'cardEase' ? 'cardEaseDesc' : 'cardEase'}
              class="sticky top-0 z-10 bg-bg-subtle text-xs uppercase tracking-wider text-text-secondary font-medium flex items-center justify-between px-4 hover:text-text-primary"
            >
              Ease
              {#if sortOrder.includes('Ease')}
                <svg class="h-3 w-3 {sortOrder === 'cardEaseDesc' ? 'rotate-180' : ''}" fill="currentColor" viewBox="0 0 20 20">
                  <path fill-rule="evenodd" d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z" clip-rule="evenodd" />
                </svg>
              {/if}
            </button>
            <button 
              onclick={() => sortOrder = sortOrder === 'cardLapses' ? 'cardLapsesDesc' : 'cardLapses'}
              class="sticky top-0 z-10 bg-bg-subtle text-xs uppercase tracking-wider text-text-secondary font-medium flex items-center justify-between px-4 hover:text-text-primary"
            >
              Lapses
              {#if sortOrder.includes('Lapses')}
                <svg class="h-3 w-3 {sortOrder === 'cardLapsesDesc' ? 'rotate-180' : ''}" fill="currentColor" viewBox="0 0 20 20">
                  <path fill-rule="evenodd" d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z" clip-rule="evenodd" />
                </svg>
              {/if}
            </button>
            <div class="sticky top-0 z-10 bg-bg-subtle text-xs uppercase tracking-wider text-text-secondary font-medium flex items-center px-4">Tags</div>
            <div class="sticky top-0 z-10 bg-bg-subtle w-8"></div>
            
            <!-- Rows -->
            {#each rows as row, i}
              <div
                onclick={() => { if (!selectedIds.has(row.card_id)) selectCard(row.card_id); }}
                onkeydown={(e) => e.key === 'Enter' && selectCard(row.card_id)}
                role="button"
                tabindex="0"
                class="group cursor-pointer"
                style="display: contents; animation: rowFadeIn 180ms ease forwards; animation-delay: {Math.min(i, 30) * 15}ms; opacity: 0;"
              >
                <div class="h-11 flex items-center justify-center border-b border-border/30 transition-colors group-hover:bg-bg-subtle {selectedCardId === row.card_id ? 'bg-accent-soft border-l-4 border-l-accent' : ''} {selectedIds.has(row.card_id) ? 'bg-accent-soft/50' : ''}" onclick={(e) => e.stopPropagation()}>
                  <input
                    type="checkbox"
                    class="w-4 h-4 rounded cursor-pointer accent-accent"
                    checked={selectedIds.has(row.card_id)}
                    onclick={(e) => handleRowSelect(row.card_id, i, e)}
                  />
                </div>
                <div class="h-11 flex items-center px-4 border-b border-border/30 transition-colors group-hover:bg-bg-subtle {selectedCardId === row.card_id ? 'bg-accent-soft' : ''} {selectedIds.has(row.card_id) ? 'bg-accent-soft/50' : ''} {row.queue === -1 ? 'italic text-text-secondary' : ''} {row.queue < -1 ? 'line-through' : ''}"><span class="text-sm truncate">{row.front_preview}</span></div>
                <div class="h-11 flex items-center px-4 border-b border-border/30 transition-colors group-hover:bg-bg-subtle {selectedCardId === row.card_id ? 'bg-accent-soft' : ''} {selectedIds.has(row.card_id) ? 'bg-accent-soft/50' : ''}"><span class="text-sm text-text-secondary truncate">{row.deck_name}</span></div>
                <div class="h-11 flex items-center px-4 border-b border-border/30 transition-colors group-hover:bg-bg-subtle {selectedCardId === row.card_id ? 'bg-accent-soft' : ''} {selectedIds.has(row.card_id) ? 'bg-accent-soft/50' : ''}"><span class="text-sm {row.due_days < 0 ? 'text-danger font-medium' : row.due_days === 0 ? 'text-accent font-medium' : 'text-text-secondary'}">{row.due_str}</span></div>
                <div class="h-11 flex items-center px-4 border-b border-border/30 transition-colors group-hover:bg-bg-subtle {selectedCardId === row.card_id ? 'bg-accent-soft' : ''} {selectedIds.has(row.card_id) ? 'bg-accent-soft/50' : ''}"><span class="text-sm text-text-secondary">{row.interval > 0 ? row.interval + 'd' : '—'}</span></div>
                <div class="h-11 flex items-center px-4 border-b border-border/30 transition-colors group-hover:bg-bg-subtle {selectedCardId === row.card_id ? 'bg-accent-soft' : ''} {selectedIds.has(row.card_id) ? 'bg-accent-soft/50' : ''}"><span class="text-sm {row.ease < 200 ? 'text-danger' : row.ease > 250 ? 'text-success' : 'text-text-secondary'}">{(row.ease / 10).toFixed(0)}%</span></div>
                <div class="h-11 flex items-center px-4 border-b border-border/30 transition-colors group-hover:bg-bg-subtle {selectedCardId === row.card_id ? 'bg-accent-soft' : ''} {selectedIds.has(row.card_id) ? 'bg-accent-soft/50' : ''}"><span class="text-sm {row.lapses > 7 ? 'text-danger font-medium' : 'text-text-secondary'}">{row.lapses}</span></div>
                <div class="h-11 flex items-center px-4 border-b border-border/30 transition-colors group-hover:bg-bg-subtle {selectedCardId === row.card_id ? 'bg-accent-soft' : ''} {selectedIds.has(row.card_id) ? 'bg-accent-soft/50' : ''}">
                  {#if row.tags && row.tags.length > 0}
                    <div class="flex gap-1">
                      {#each row.tags.slice(0, 2) as tag}
                        <span class="px-2 py-0.5 bg-accent-soft text-accent text-xs rounded-full">{tag}</span>
                      {/each}
                      {#if row.tags.length > 2}
                        <span class="px-2 py-0.5 text-xs text-text-secondary">+{row.tags.length - 2}</span>
                      {/if}
                    </div>
                  {/if}
                </div>
                <div class="h-11 flex items-center justify-center border-b border-border/30 transition-colors group-hover:bg-bg-subtle {selectedCardId === row.card_id ? 'bg-accent-soft' : ''} {selectedIds.has(row.card_id) ? 'bg-accent-soft/50' : ''}">
                  {#if row.flag > 0}
                    <div class="w-3 h-3 rounded-full" style="background-color: {row.flag === 1 ? '#ef4444' : row.flag === 2 ? '#f97316' : row.flag === 3 ? '#eab308' : row.flag === 4 ? '#22c55e' : '#3b82f6'}"></div>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      {:else if noteRows.length === 0}
        <!-- Empty state for notes -->
        <div class="flex flex-col items-center justify-center h-full text-text-secondary">
          <svg class="h-16 w-16 mb-4 opacity-40" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
          <p class="text-lg font-medium">No notes found</p>
          <p class="text-sm mt-1">{query ? 'Try a different search query.' : 'Your collection is empty.'}</p>
        </div>
      {:else}
        <!-- Results count for notes -->
        <div class="px-4 py-2 text-xs text-text-secondary text-right border-b border-border/30">
          Showing {noteRows.length} notes
        </div>
        
        <!-- Notes table -->
        <div class="grid" style="grid-template-columns: 40px 1fr 200px 120px 120px 60px 160px 90px;">
          <!-- Header -->
          <div class="sticky top-0 z-10 bg-bg-subtle text-xs uppercase tracking-wider text-text-secondary font-medium flex items-center px-2">
            <input 
              type="checkbox" 
              class="w-4 h-4 rounded cursor-pointer accent-accent"
              checked={allSelected}
              bind:this={headerCheckbox}
              onclick={toggleSelectAll}
            />
          </div>
          <div class="sticky top-0 z-10 bg-bg-subtle text-xs uppercase tracking-wider text-text-secondary font-medium flex items-center px-4">Front</div>
          <div class="sticky top-0 z-10 bg-bg-subtle text-xs uppercase tracking-wider text-text-secondary font-medium flex items-center px-4">Back</div>
          <div class="sticky top-0 z-10 bg-bg-subtle text-xs uppercase tracking-wider text-text-secondary font-medium flex items-center px-4">Note Type</div>
          <div class="sticky top-0 z-10 bg-bg-subtle text-xs uppercase tracking-wider text-text-secondary font-medium flex items-center px-4">Deck</div>
          <div class="sticky top-0 z-10 bg-bg-subtle text-xs uppercase tracking-wider text-text-secondary font-medium flex items-center px-4">Cards</div>
          <div class="sticky top-0 z-10 bg-bg-subtle text-xs uppercase tracking-wider text-text-secondary font-medium flex items-center px-4">Tags</div>
          <div class="sticky top-0 z-10 bg-bg-subtle text-xs uppercase tracking-wider text-text-secondary font-medium flex items-center px-4">Created</div>
          
          <!-- Rows -->
          {#each noteRows as row, i}
            <div
              onclick={() => { if (!selectedNoteIds.has(row.note_id)) selectNote(row.note_id, row.first_card_id); }}
              onkeydown={(e) => e.key === 'Enter' && selectNote(row.note_id, row.first_card_id)}
              role="button"
              tabindex="0"
              class="group cursor-pointer"
              style="display: contents; animation: rowFadeIn 180ms ease forwards; animation-delay: {Math.min(i, 30) * 15}ms; opacity: 0;"
            >
              <div class="h-11 flex items-center justify-center border-b border-border/30 transition-colors group-hover:bg-bg-subtle {selectedNoteId === row.note_id ? 'bg-accent-soft border-l-4 border-l-accent' : ''} {selectedNoteIds.has(row.note_id) ? 'bg-accent-soft/50' : ''}" onclick={(e) => e.stopPropagation()}>
                <input
                  type="checkbox"
                  class="w-4 h-4 rounded cursor-pointer accent-accent"
                  checked={selectedNoteIds.has(row.note_id)}
                  onclick={(e) => handleNoteRowSelect(row.note_id, i, e)}
                />
              </div>
              <div class="h-11 flex items-center px-4 border-b border-border/30 transition-colors group-hover:bg-bg-subtle {selectedNoteId === row.note_id ? 'bg-accent-soft' : ''} {selectedNoteIds.has(row.note_id) ? 'bg-accent-soft/50' : ''}"><span class="text-sm truncate max-w-xs">{row.front_preview}</span></div>
              <div class="h-11 flex items-center px-4 border-b border-border/30 transition-colors group-hover:bg-bg-subtle {selectedNoteId === row.note_id ? 'bg-accent-soft' : ''} {selectedNoteIds.has(row.note_id) ? 'bg-accent-soft/50' : ''}"><span class="text-sm truncate max-w-xs text-text-secondary">{row.back_preview}</span></div>
              <div class="h-11 flex items-center px-4 border-b border-border/30 transition-colors group-hover:bg-bg-subtle {selectedNoteId === row.note_id ? 'bg-accent-soft' : ''} {selectedNoteIds.has(row.note_id) ? 'bg-accent-soft/50' : ''}"><span class="text-sm text-text-secondary">{row.notetype_name}</span></div>
              <div class="h-11 flex items-center px-4 border-b border-border/30 transition-colors group-hover:bg-bg-subtle {selectedNoteId === row.note_id ? 'bg-accent-soft' : ''} {selectedNoteIds.has(row.note_id) ? 'bg-accent-soft/50' : ''}"><span class="text-sm text-text-secondary truncate">{row.deck_name}</span></div>
              <div class="h-11 flex items-center px-4 border-b border-border/30 transition-colors group-hover:bg-bg-subtle {selectedNoteId === row.note_id ? 'bg-accent-soft' : ''} {selectedNoteIds.has(row.note_id) ? 'bg-accent-soft/50' : ''}"><span class="text-sm text-text-secondary">{row.card_count}</span></div>
              <div class="h-11 flex items-center px-4 border-b border-border/30 transition-colors group-hover:bg-bg-subtle {selectedNoteId === row.note_id ? 'bg-accent-soft' : ''} {selectedNoteIds.has(row.note_id) ? 'bg-accent-soft/50' : ''}">
                {#if row.tags && row.tags.length > 0}
                  <div class="flex gap-1">
                    {#each row.tags.slice(0, 2) as tag}
                      <span class="px-2 py-0.5 bg-accent-soft text-accent text-xs rounded-full">{tag}</span>
                    {/each}
                    {#if row.tags.length > 2}
                      <span class="px-2 py-0.5 text-xs text-text-secondary">+{row.tags.length - 2}</span>
                    {/if}
                  </div>
                {/if}
              </div>
              <div class="h-11 flex items-center px-4 border-b border-border/30 transition-colors group-hover:bg-bg-subtle {selectedNoteId === row.note_id ? 'bg-accent-soft' : ''} {selectedNoteIds.has(row.note_id) ? 'bg-accent-soft/50' : ''}"><span class="text-sm text-text-secondary">{row.created_days_ago === 0 ? 'Today' : row.created_days_ago === 1 ? '1 day ago' : row.created_days_ago + ' days ago'}</span></div>
            </div>
          {/each}
        </div>
      {/if}
        </div>
      {/key}
    </div>

    <!-- Detail Panel -->
    {#if selectedCardId || selectedNoteId}
      <div class="w-[380px] flex-shrink-0 border-l border-border/30 overflow-y-auto">
        <CardDetailPanel
          cardId={selectedCardId}
          noteId={selectedNoteId}
          onClose={closeDetail}
          onFlagChange={(cardId, flag) => {
            // Update the flag in the rows array without re-fetching
            rows = rows.map(row => row.card_id === cardId ? { ...row, flag } : row);
          }}
        />
      </div>
    {/if}
    
    <!-- Floating Action Bar -->
    {#if selectionCount > 0}
      <div 
        class="fixed bottom-6 left-1/2 transform -translate-x-1/2 z-50"
        in:fly={{ y: 16, duration: 180 }}
        out:fly={{ y: 16, duration: 140 }}
      >
        <!-- Dropdown overlays -->
        {#if showMoveDropdown}
          <div class="absolute bottom-full mb-2 left-0 bg-bg-card rounded-xl shadow-xl border border-border/30 py-1 min-w-48">
            {#each availableDecks as deck}
              <button
                onclick={() => handleMoveToDeck(deck.id, deck.name)}
                class="w-full text-left px-4 py-2 text-sm hover:bg-bg-subtle"
              >
                {deck.name}
              </button>
            {/each}
          </div>
        {/if}
        
        {#if showTagInput}
          <div class="absolute bottom-full mb-2 left-0 bg-bg-card rounded-xl shadow-xl border border-border/30 p-3">
            <div class="flex gap-2">
              <input
                type="text"
                bind:value={newTag}
                placeholder="Enter tag..."
                class="px-3 py-1.5 rounded-lg text-sm w-40"
                onkeydown={(e) => e.key === 'Enter' && handleAddTag()}
              />
              <button
                onclick={handleAddTag}
                class="px-3 py-1.5 bg-accent text-white rounded-lg text-sm"
              >
                Add
              </button>
            </div>
          </div>
        {/if}
        
        {#if showDeleteConfirm}
          <div class="absolute bottom-full mb-2 right-0 bg-bg-card rounded-xl shadow-xl border border-border/30 p-3">
            <p class="text-sm mb-2">Delete {getUniqueNoteIds().length} notes and all their cards? This cannot be undone.</p>
            <div class="flex gap-2 justify-end">
              <button
                onclick={() => showDeleteConfirm = false}
                class="px-3 py-1.5 rounded-lg text-sm"
              >
                Cancel
              </button>
              <button
                onclick={handleBulkDelete}
                class="px-3 py-1.5 bg-danger text-white rounded-lg text-sm"
              >
                Delete
              </button>
            </div>
          </div>
        {/if}
        
        <!-- Main action bar -->
        <div class="bg-[#1C1917] text-white rounded-2xl px-5 py-3 shadow-xl flex items-center gap-1">
          <span class="text-sm opacity-70 mr-2">
            {allSelected ? `All ${rows.length} selected` : `${selectionCount} selected`}
          </span>
          
          <div class="w-px h-5 bg-text-secondary/15 mx-2"></div>
          
          <button
            onclick={handleSuspend}
            class="flex items-center gap-2 px-3 py-1.5 rounded-xl hover:bg-text-secondary/10 text-sm"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 9v6m4-6v6m7-3a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            Suspend
          </button>
          
          <button
            onclick={handleUnsuspend}
            class="flex items-center gap-2 px-3 py-1.5 rounded-xl hover:bg-text-secondary/10 text-sm"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            Unsuspend
          </button>
          
          <button
            onclick={handleBury}
            class="flex items-center gap-2 px-3 py-1.5 rounded-xl hover:bg-text-secondary/10 text-sm"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
            </svg>
            Bury
          </button>
          
          <button
            onclick={() => { loadDecks(); showMoveDropdown = !showMoveDropdown; showTagInput = false; showDeleteConfirm = false; }}
            class="flex items-center gap-2 px-3 py-1.5 rounded-xl hover:bg-text-secondary/10 text-sm"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
            </svg>
            Move
          </button>
          
          <button
            onclick={() => { showTagInput = !showTagInput; showMoveDropdown = false; showDeleteConfirm = false; }}
            class="flex items-center gap-2 px-3 py-1.5 rounded-xl hover:bg-text-secondary/10 text-sm"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z" />
            </svg>
            Tag
          </button>
          
          <button
            onclick={() => { showDeleteConfirm = !showDeleteConfirm; showMoveDropdown = false; showTagInput = false; }}
            class="flex items-center gap-2 px-3 py-1.5 rounded-xl hover:bg-text-secondary/10 text-sm text-[#FCA5A5]"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
            </svg>
            Delete
          </button>
          
          <button
            onclick={clearSelection}
            class="p-1.5 rounded-xl hover:bg-text-secondary/10 ml-1"
            aria-label="Clear selection"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>
