<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { addToast } from "./toast";
  import { prefs } from "./prefs.svelte";
  import WidgetContainer from "./widgets/WidgetContainer.svelte";
  import DecksGridWidget from "./widgets/DecksGridWidget.svelte";
  import UpcomingSessionsWidget from "./widgets/UpcomingSessionsWidget.svelte";
  import StatsSnapshotWidget from "./widgets/StatsSnapshotWidget.svelte";
  import StudyScheduleWidget from "./widgets/StudyScheduleWidget.svelte";
  import { pluginEngine } from "./pluginEngine";
  import NeuSelect from "./ui/NeuSelect.svelte";

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
      if (widgetId === 'decks') {
        continue; // Decks are rendered directly in the left column, not through WidgetContainer
      } else if (widgetId === 'upcoming') {
        widgets.push({
          id: 'upcoming',
          type: 'upcoming',
          title: 'Upcoming Sessions',
          component: UpcomingSessionsWidget,
          props: {},
          order: widgets.length,
          gridHeight: 1
        });
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
          id: 'schedule',
          type: 'schedule',
          title: 'Study Schedule',
          component: StudyScheduleWidget,
          props: {},
          order: widgets.length,
          gridHeight: 2
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
  <!-- Header Row -->
  <div class="flex items-center justify-between mb-8">
    <div>
      <h1 style="font-family: var(--serif); font-size: 36px; font-weight: 600; color: var(--text-primary); letter-spacing: -0.02em; margin-bottom: 4px;">Your Decks</h1>
      <p style="font-family: var(--sans); font-size: 14px; color: var(--text-secondary);">{totalDue} cards due today across {decks.length} {decks.length === 1 ? 'deck' : 'decks'}</p>
    </div>
    <button
      onclick={onImport}
      class="neu-subtle neu-btn px-4 py-2 rounded-lg cursor-pointer"
      style="background: var(--bg-card);"
    >
      <span style="font-family: var(--sans); font-size: 13px; font-weight: 500; color: var(--text-secondary);">Import</span>
    </button>
  </div>

  <!-- Two-column dashboard layout -->
  <div class="dashboard-layout">
    <!-- Left: Decks column -->
    <div class="decks-column">
      <div class="decks-card">
        <!-- Search + controls row -->
        <div style="display: flex; gap: 8px; margin-bottom: 14px; align-items: center;">
          <div style="flex: 1; position: relative;">
            <svg style="position: absolute; left: 10px; top: 50%; transform: translateY(-50%); width: 14px; height: 14px; color: var(--text-secondary);" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0" />
            </svg>
            <input
              type="search"
              bind:value={deckSearch}
              placeholder="Search decks…"
              style="
                width: 100%;
                padding: 7px 10px 7px 30px;
                background: var(--bg-subtle);
                box-shadow: var(--neu-down);
                border-radius: var(--radius-md);
                font-family: var(--sans);
                font-size: 12px;
                color: var(--text-primary);
                outline: none;
                box-sizing: border-box;
              "
            />
          </div>
          <NeuSelect
            size="sm"
            options={[
              { value: 'name',  label: 'A–Z'   },
              { value: 'due',   label: 'Due'   },
              { value: 'new',   label: 'New'   },
              { value: 'total', label: 'Total' },
            ]}
            bind:value={deckSortBy}
          />
          <NeuSelect
            size="sm"
            options={[
              { value: 'all', label: 'All'  },
              { value: 'due', label: 'Due'  },
              { value: 'new', label: 'New'  },
            ]}
            bind:value={deckFilterBy}
          />
        </div>

        <!-- Deck list -->
        <div class="decks-scroll">
          <DecksGridWidget
            {onStudy}
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
    overflow: hidden;
    display: flex;
    flex-direction: column;
    gap: 0;
    padding: 0;
    background: transparent;
  }

  .decks-scroll {
    flex: 1;
    overflow-y: auto;
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
