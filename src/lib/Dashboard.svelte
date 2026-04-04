<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, tick } from "svelte";
  import { fly } from "svelte/transition";
  import { addToast } from "./toast";
  import { prefs } from "./prefs.svelte";
  import WidgetContainer from "./widgets/WidgetContainer.svelte";
  import DecksGridWidget from "./widgets/DecksGridWidget.svelte";
  import StatsSnapshotWidget from "./widgets/StatsSnapshotWidget.svelte";
  import CalendarWidget from "./widgets/CalendarWidget.svelte";
  import { pluginEngine } from "./pluginEngine";
  import NeuSelect from "./ui/NeuSelect.svelte";
  import DeckSettings from "./DeckSettings.svelte";

  let {
    onStudy = (deckId: number, deckName: string) => {},
    onImport = () => {},
  }: {
    onStudy?: (deckId: number, deckName: string) => void;
    onImport?: () => void;
  } = $props();

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
  };

  let decks: DeckStat[] = $state([]);
  let isLoading = $state(false);
  let lastDeckStatsTime = 0;
  const DECK_STATS_TTL = 3000;

  // Deck search / sort / filter (passed down to widget)
  let deckSearch = $state('');
  let deckSortBy = $state<'name' | 'due' | 'new' | 'total'>('name');
  let deckFilterBy = $state<'all' | 'due' | 'new'>('all');

  // Search expand/collapse
  let searchOpen = $state(false);
  let searchOpenRef = false; // Ref to track state without triggering effects
  let searchInputEl: HTMLInputElement | null = $state(null);
  let searchWrapEl: HTMLDivElement | null = $state(null);

  // Deck settings modal state
  let settingsDeckId: number | null = $state(null);
  let settingsDeckName: string = $state("");

  const sortOptions = [
    { value: 'name'  as const, label: 'A–Z'   },
    { value: 'due'   as const, label: 'Due'   },
    { value: 'new'   as const, label: 'New'   },
    { value: 'total' as const, label: 'Total' },
  ];
  const filterOptions = [
    { value: 'all' as const, label: 'All' },
    { value: 'due' as const, label: 'Due' },
    { value: 'new' as const, label: 'New' },
  ];
  // Abbreviated labels for compact circles
  const sortShort: Record<string, string> = { name: 'A–Z', due: 'Due', new: 'New', total: 'Tot' };
  const filterShort: Record<string, string> = { all: 'All', due: 'Due', new: 'New' };

  async function openSearch() {
    searchOpen = true;
    searchOpenRef = true;
    await tick();
    searchInputEl?.focus();
  }

  function closeSearch() {
    searchOpen = false;
    searchOpenRef = false;
    deckSearch = '';
  }

  function cycleSortBy() {
    const idx = sortOptions.findIndex(o => o.value === deckSortBy);
    deckSortBy = sortOptions[(idx + 1) % sortOptions.length].value;
  }

  function cycleFilterBy() {
    const idx = filterOptions.findIndex(o => o.value === deckFilterBy);
    deckFilterBy = filterOptions[(idx + 1) % filterOptions.length].value;
  }

  // Close search when clicking outside the search wrap
  $effect(() => {
    if (!searchOpen) return;
    function onDocClick(e: MouseEvent) {
      if (searchOpenRef && searchWrapEl && !searchWrapEl.contains(e.target as Node)) {
        closeSearch();
      }
    }
    document.addEventListener('click', onDocClick);
    return () => document.removeEventListener('click', onDocClick);
  });

  const totalDue = $derived.by(() =>
    decks.reduce((sum, d) => sum + d.new_count + d.learn_count + d.review_count, 0)
  );

  let loadingPromise: Promise<void> | null = null;

  async function loadDeckStats(force = false) {
    const now = Date.now();
    if (!force && now - lastDeckStatsTime < DECK_STATS_TTL && decks.length > 0) return;
    if (loadingPromise) return loadingPromise;
    isLoading = true;
    loadingPromise = (async () => {
      try {
        decks = await invoke<DeckStat[]>("get_all_decks");
        if (import.meta.env.DEV && decks.length === 0) {
          decks = [
            { id: 1, name: 'Japanese Core 2000',               short_name: 'Japanese Core 2000', level: 1, new_count: 25, learn_count:  8, review_count: 42, card_count: 2000, is_filtered: false },
            { id: 2, name: 'Spanish Vocab',                    short_name: 'Spanish Vocab',       level: 1, new_count: 15, learn_count:  3, review_count: 28, card_count:  500, is_filtered: false },
            { id: 3, name: 'Medical Terminology',              short_name: 'Medical Terminology', level: 1, new_count: 50, learn_count: 12, review_count:  0, card_count:  800, is_filtered: false },
            { id: 4, name: 'CS Fundamentals::Data Structures', short_name: 'Data Structures',     level: 2, new_count: 10, learn_count:  5, review_count: 15, card_count:  200, is_filtered: false },
            { id: 5, name: 'CS Fundamentals::Algorithms',      short_name: 'Algorithms',          level: 2, new_count:  8, learn_count:  0, review_count: 22, card_count:  150, is_filtered: false },
          ];
        }
        lastDeckStatsTime = Date.now();
      } catch (error) {
        addToast(error instanceof Error ? error.message : "Failed to load decks", "error");
        decks = [];
      } finally {
        isLoading = false;
        loadingPromise = null;
      }
    })();
    return loadingPromise;
  }

  onMount(async () => {
    await loadDeckStats();
    const refresh = () => loadDeckStats(true);
    window.addEventListener('refresh-decks', refresh);
    return () => window.removeEventListener('refresh-decks', refresh);
  });

  // Build widgets array
  let dashboardWidgets = $derived.by(() => {
    const widgets = [];
    
    // Add built-in widgets based on widget_order
    for (const widgetId of prefs.widget_order) {
      if (widgetId === 'decks' || widgetId === 'upcoming') {
        continue;
      } else if (widgetId === 'stats') {
        widgets.push({
          id: 'stats',
          type: 'stats',
          title: 'Stats Snapshot',
          component: StatsSnapshotWidget,
          props: {},
          order: widgets.length,
          gridHeight: 1
        });
      } else if (widgetId === 'schedule') {
        widgets.push({
          id: 'calendar',
          type: 'calendar',
          title: 'Study Calendar',
          component: CalendarWidget,
          props: {},
          order: widgets.length,
          gridHeight: 3
        });
      }
    }
    
    // Add plugin widgets
    const pluginWidgets = pluginEngine.getWidgets('dashboard');
    for (const pluginWidget of pluginWidgets) {
      widgets.push({
        id: pluginWidget.id,
        type: 'plugin',
        title: pluginWidget.title,
        component: { render: (props: any) => pluginWidget.render(props.container) },
        props: {},
        order: pluginWidget.defaultOrder || widgets.length,
        gridHeight: pluginWidget.gridHeight || 1
      });
    }
    
    return widgets;
  });
</script>

<div class="max-w-[1100px] mx-auto px-9 pt-5 pb-10" style="animation: fadeUp 0.4s ease-out;">
  <!-- Two-column dashboard layout -->
  <div class="dashboard-layout">
    <!-- Left: Decks column -->
    <div class="decks-column">
      <div class="decks-card">
        <!-- Title row -->
        <div style="display: flex; align-items: flex-start; justify-content: space-between; margin-bottom: 16px;">
          <div>
            <h1 style="font-family: var(--serif); font-size: 36px; font-weight: 600; color: var(--text-primary); letter-spacing: -0.02em; margin-bottom: 4px;">Your Decks</h1>
            <p style="font-family: var(--sans); font-size: 14px; color: var(--text-secondary);">{totalDue} cards due today across {decks.length} {decks.length === 1 ? 'deck' : 'decks'}</p>
          </div>
          <button
            onclick={onImport}
            class="neu-subtle neu-btn px-4 py-2 rounded-lg cursor-pointer"
            style="background: var(--bg-card); flex-shrink: 0; margin-top: 4px;"
          >
            <span style="font-family: var(--sans); font-size: 13px; font-weight: 500; color: var(--text-secondary);">Import</span>
          </button>
        </div>

        <!-- Search + controls row -->
        <div class="controls-row">
          {#if searchOpen}
            <!-- Expanded: search input + compact circles -->
            <div class="search-wrap" bind:this={searchWrapEl} transition:fly={{ x: -8, duration: 150, opacity: 0 }}>
              <svg class="search-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0" />
              </svg>
              <input
                bind:this={searchInputEl}
                type="search"
                bind:value={deckSearch}
                placeholder="Search decks…"
                class="search-input"
                onkeydown={(e) => e.key === 'Escape' && closeSearch()}
              />
              <button class="search-close" onclick={() => deckSearch = ''} aria-label="Clear search">
                <svg width="9" height="9" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2.5" stroke-linecap="round">
                  <path d="M18 6L6 18M6 6l12 12" />
                </svg>
              </button>
            </div>
            <button
              class="ctrl-circle"
              class:ctrl-active={deckSortBy !== 'name'}
              onclick={cycleSortBy}
              title="Sort: {sortOptions.find(o => o.value === deckSortBy)?.label}"
            >
              <span class="ctrl-label">{sortShort[deckSortBy]}</span>
            </button>
            <button
              class="ctrl-circle"
              class:ctrl-active={deckFilterBy !== 'all'}
              onclick={cycleFilterBy}
              title="Filter: {filterOptions.find(o => o.value === deckFilterBy)?.label}"
            >
              <span class="ctrl-label">{filterShort[deckFilterBy]}</span>
            </button>
          {:else}
            <!-- Collapsed: search circle + full dropdowns -->
            <button class="ctrl-circle" onclick={(e) => { e.stopPropagation(); openSearch(); }} aria-label="Search decks">
              <svg width="13" height="13" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2" stroke-linecap="round">
                <path d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0" />
              </svg>
            </button>
            <div style="flex: 1; min-width: 0;">
              <NeuSelect
                size="sm"
                options={sortOptions}
                bind:value={deckSortBy}
              />
            </div>
            <div style="flex: 1; min-width: 0;">
              <NeuSelect
                size="sm"
                options={filterOptions}
                bind:value={deckFilterBy}
              />
            </div>
          {/if}
        </div>

        <!-- Deck list -->
        <div class="decks-scroll">
          <DecksGridWidget
            {onStudy}
            onDeckSettings={(deckId, deckName) => {
              settingsDeckId = deckId;
              settingsDeckName = deckName;
            }}
            onDeckDelete={(deckId, deckName) => {
              // For now, just show a toast. In a full implementation,
              // you might want to show a confirmation dialog here.
              addToast(`Delete "${deckName}" - use Deck Settings for delete options`, "info");
            }}
            compact={true}
            search={deckSearch}
            sortBy={deckSortBy}
            filterBy={deckFilterBy}
          />
        </div>
      </div>
    </div>

    <!-- Right: Other widgets -->
    <div class="widgets-column">
      <WidgetContainer widgets={dashboardWidgets} />
    </div>
  </div>
</div>

<!-- Deck Settings Modal -->
{#if settingsDeckId !== null}
  <DeckSettings
    deckId={settingsDeckId}
    deckName={settingsDeckName}
    isOpen={true}
    onClose={() => settingsDeckId = null}
    onRenamed={(newName) => {
      settingsDeckName = newName;
      loadDeckStats(true);
    }}
    onDeleted={() => {
      settingsDeckId = null;
      loadDeckStats(true);
    }}
    onOpenOptions={() => {
      // Navigate to deck overview to open options
      onStudy(settingsDeckId!, settingsDeckName);
      settingsDeckId = null;
    }}
  />
{/if}

<style>
  .dashboard-layout {
    display: flex;
    gap: 32px;
    align-items: flex-start;
  }

  .decks-column {
    width: 300px;
    min-width: 300px;
    position: sticky;
    top: 24px;
  }

  .decks-card {
    max-height: calc(100vh - 180px);
    display: flex;
    flex-direction: column;
    gap: 0;
    padding: 0;
    background: transparent;
  }

  .decks-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 16px 16px;
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }

  .decks-scroll::-webkit-scrollbar {
    width: 4px;
  }
  .decks-scroll::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 2px;
  }

  .widgets-column {
    flex: 1;
    min-width: 0;
  }

  .controls-row {
    display: flex;
    gap: 6px;
    margin-bottom: 14px;
    align-items: center;
  }

  .ctrl-circle {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    border: none;
    background: var(--bg-card);
    box-shadow: var(--neu-up);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: var(--text-secondary);
    transition: box-shadow 0.12s, color 0.12s;
  }

  .ctrl-circle:hover {
    color: var(--text-primary);
  }

  .ctrl-active {
    color: var(--accent);
    box-shadow: var(--neu-down);
  }

  .ctrl-label {
    font-size: 10px;
    font-weight: 700;
    font-family: var(--sans);
    letter-spacing: 0.01em;
  }

  .search-wrap {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 9px;
    width: 13px;
    height: 13px;
    color: var(--text-secondary);
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: 7px 28px 7px 28px;
    background: var(--bg-subtle);
    box-shadow: var(--neu-down);
    border-radius: var(--radius-md);
    border: none;
    font-family: var(--sans);
    font-size: 12px;
    color: var(--text-primary);
    outline: none;
    box-sizing: border-box;
  }

  .search-close {
    position: absolute;
    right: 7px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: none;
    background: transparent;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    padding: 0;
    transition: color 0.1s;
  }

  .search-close:hover {
    color: var(--text-primary);
  }

  @media (max-width: 768px) {
    .dashboard-layout {
      flex-direction: column;
    }
    .decks-column {
      width: 100%;
      min-width: 100%;
      position: static;
    }
    .decks-card {
      max-height: none;
    }
  }
</style>
