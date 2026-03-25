<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { isTauri } from "@tauri-apps/api/core";
  import Chart from "./Chart.svelte";
  import type { ChartData, ChartOptions } from "chart.js";
  import { addToast } from "./toast";
  import { getChartColors, baseBarOptions, baseDoughnutOptions, type ChartColorPalette } from "./chartTheme";

  // Types
  interface ForecastDay {
    day: number;
    new: number;
    review: number;
    learning: number;
  }

  interface DailyReview {
    date: string;
    count: number;
    time_secs: number;
  }

  interface CardTypeStats {
    new: number;
    learning: number;
    young: number;
    mature: number;
  }

  interface RetentionStats {
    young_retention: number;
    mature_retention: number;
    overall: number;
  }

  interface DailyCount {
    date: string;
    count: number;
  }

  interface ReviewStats {
    forecast: ForecastDay[];
    daily_reviews: DailyReview[];
    hourly_breakdown: number[];
    card_types: CardTypeStats;
    retention: RetentionStats;
    cards_added: DailyCount[];
    current_streak: number;
    longest_streak: number;
    total_reviews: number;
    total_cards: number;
    total_notes: number;
    average_ease: number;
    average_interval_days: number;
  }

  interface HeatmapDay {
    date: string;
    count: number;
  }

  interface DifficultCard {
    card_id: number;
    front: string;
    deck_name: string;
    lapses: number;
    reviews: number;
    ease: number;
    retention: number;
    fields_csv: string;
  }

  interface DeckInfo {
    id: number;
    name: string;
    card_count: number;
    is_filtered: boolean;
  }

  // State
  let isLoading = $state(true);
  let hasError = $state(false);
  let errorMessage = $state("");
  let isTauriAvailable = $state(false);
  let scope = $state<"global" | "deck">("global");
  let selectedDeckId = $state<number | null>(null);
  let decks = $state<DeckInfo[]>([]);
  let stats = $state<ReviewStats | null>(null);
  let heatmapData = $state<HeatmapDay[]>([]);
  let difficultCards = $state<DifficultCard[]>([]);
  let chartColors = $state<ChartColorPalette>(getChartColors());

  // Calendar state
  let calYear = $state(new Date().getFullYear());
  let calMonth = $state(new Date().getMonth());

  // Re-derive chart colors on theme changes
  $effect(() => {
    const observer = new MutationObserver(() => {
      chartColors = getChartColors();
    });
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class', 'style'] });
    return () => observer.disconnect();
  });

  // Load data on mount
  onMount(async () => {
    const tauriCheck = await isTauri();
    isTauriAvailable = tauriCheck;
    
    if (!tauriCheck) {
      isLoading = false;
      return;
    }

    await loadData();
  });

  async function loadData() {
    isLoading = true;
    hasError = false;
    errorMessage = "";

    try {
      const [statsResult, heatmapResult, difficultResult, decksResult] = await Promise.all([
        invoke<ReviewStats>("get_review_stats", { deckId: selectedDeckId }),
        invoke<HeatmapDay[]>("get_review_heatmap", { days: 365, deckId: selectedDeckId }),
        invoke<DifficultCard[]>("get_difficult_cards", { deckId: selectedDeckId, limit: 20 }),
        invoke<any[]>("get_all_decks"),
      ]);

      stats = statsResult;
      heatmapData = heatmapResult;
      difficultCards = difficultResult;
      decks = decksResult.map((d: any) => ({
        id: d.id,
        name: d.name,
        card_count: d.card_count,
        is_filtered: d.is_filtered,
      }));
    } catch (error) {
      console.error("Error loading stats:", error);
      hasError = true;
      errorMessage = error instanceof Error ? error.message : "Failed to load statistics";
      addToast(errorMessage, "error");
    } finally {
      isLoading = false;
    }
  }

  function handleScopeChange(newScope: "global" | "deck") {
    scope = newScope;
    if (newScope === "global") {
      selectedDeckId = null;
    } else if (decks.length > 0 && !selectedDeckId) {
      selectedDeckId = decks[0].id;
    }
    loadData();
  }

  function handleDeckChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    selectedDeckId = parseInt(target.value, 10);
    loadData();
  }

  function prevMonth() {
    if (calMonth === 0) {
      calMonth = 11;
      calYear--;
    } else {
      calMonth--;
    }
  }

  function nextMonth() {
    if (calMonth === 11) {
      calMonth = 0;
      calYear++;
    } else {
      calMonth++;
    }
  }

  function formatTime(secs: number): string {
    const hours = Math.floor(secs / 3600);
    const minutes = Math.floor((secs % 3600) / 60);
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes} min`;
  }

  function formatPercentage(value: number): string {
    return `${(value * 100).toFixed(1)}%`;
  }

  function stripHtml(html: string): string {
    return html.replace(/<[^>]*>/g, '').trim();
  }

  function getFirstField(fieldsCsv: string): string {
    const fields = fieldsCsv.split('\x1f');
    return stripHtml(fields[0] || '');
  }

  // Heatmap computation
  let heatmapWeeks = $derived.by(() => {
    if (!heatmapData.length) return [];
    
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    
    const startDate = new Date(today);
    startDate.setDate(startDate.getDate() - 364);
    
    // Adjust to start on Sunday
    const dayOfWeek = startDate.getDay();
    startDate.setDate(startDate.getDate() - dayOfWeek);
    
    const weeks: { date: string; count: number }[][] = [];
    let currentWeek: { date: string; count: number }[] = [];
    
    const dateMap = new Map(heatmapData.map(d => [d.date, d.count]));
    
    const currentDate = new Date(startDate);
    while (currentDate <= today || currentWeek.length > 0) {
      const dateStr = currentDate.toISOString().split('T')[0];
      const count = dateMap.get(dateStr) || 0;
      
      currentWeek.push({ date: dateStr, count });
      
      if (currentWeek.length === 7) {
        weeks.push(currentWeek);
        currentWeek = [];
      }
      
      currentDate.setDate(currentDate.getDate() + 1);
      
      if (currentDate > today && currentWeek.length === 0) break;
    }
    
    return weeks;
  });

  // Calendar cells (always 42)
  let calendarCells = $derived.by(() => {
    const firstDay = new Date(calYear, calMonth, 1).getDay();
    const dim = new Date(calYear, calMonth + 1, 0).getDate();
    const cells: (number | null)[] = [];
    
    for (let i = 0; i < firstDay; i++) cells.push(null);
    for (let d = 1; d <= dim; d++) cells.push(d);
    while (cells.length < 42) cells.push(null);
    
    return cells;
  });

  // Heatmap lookup
  let heatmapLookup = $derived.by(() => {
    const map: Record<string, number> = {};
    for (const day of heatmapData) {
      map[day.date] = day.count;
    }
    return map;
  });

  // Consistency percentage
  let consistency = $derived.by(() => {
    if (!heatmapData.length) return 0;
    const daysStudied = heatmapData.filter(d => d.count > 0).length;
    return daysStudied / 365;
  });

  // Streak data
  let streakData = $derived.by(() => {
    if (!stats) return { current: 0, longest: 0, average: 0, strip: [] as boolean[] };
    
    const strip: boolean[] = [];
    const today = new Date();
    
    for (let i = 13; i >= 0; i--) {
      const date = new Date(today);
      date.setDate(date.getDate() - i);
      const dateStr = date.toISOString().split('T')[0];
      strip.push((heatmapLookup[dateStr] || 0) > 0);
    }
    
    return {
      current: stats.current_streak,
      longest: stats.longest_streak,
      average: stats.total_reviews > 0 ? Math.round(stats.total_reviews / 365 * 10) / 10 : 0,
      strip,
    };
  });

  // Retention by deck
  let retentionByDeck = $derived.by(() => {
    if (!stats || scope === "deck") return [];
    
    // For global view, show overall retention
    return [
      { name: "Overall", retention: stats.retention.overall },
      { name: "Young Cards", retention: stats.retention.young_retention },
      { name: "Mature Cards", retention: stats.retention.mature_retention },
    ];
  });

  // Chart: Card Types (doughnut)
  let cardTypesChartData = $derived.by(() => {
    if (!stats) return { labels: [], datasets: [] };

    const totalCards = stats.card_types.new + stats.card_types.learning + stats.card_types.young + stats.card_types.mature;

    return {
      labels: ['New', 'Learning', 'Young', 'Mature'],
      datasets: [{
        data: [
          stats.card_types.new,
          stats.card_types.learning,
          stats.card_types.young,
          stats.card_types.mature,
        ],
        backgroundColor: [
          chartColors.newCards,
          chartColors.learning,
          chartColors.young,
          chartColors.mature,
        ],
        borderWidth: 0,
      }],
    } as ChartData;
  });

  let cardTypesChartOptions: ChartOptions = {
    ...baseDoughnutOptions(chartColors),
    plugins: {
      ...baseDoughnutOptions(chartColors).plugins,
      legend: {
        display: true,
        position: 'right',
        labels: {
          color: chartColors.textSecondary,
          boxWidth: 12,
          padding: 15,
          font: { family: 'DM Sans' },
        },
      },
      tooltip: {
        ...baseDoughnutOptions(chartColors).plugins?.tooltip,
        callbacks: {
          label: (ctx) => {
            const total = stats ? stats.card_types.new + stats.card_types.learning + stats.card_types.young + stats.card_types.mature : 0;
            const value = ctx.parsed as number;
            return `${ctx.label}: ${value} cards (${((value / total) * 100).toFixed(1)}%)`;
          },
        },
      },
    },
  } as ChartOptions;

  // Chart: Forecast (bar)
  let forecastChartData = $derived.by(() => {
    if (!stats?.forecast.length) return { labels: [], datasets: [] };

    const labels = stats.forecast.slice(0, 14).map(d => {
      const date = new Date();
      date.setDate(date.getDate() + d.day);
      return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
    });

    return {
      labels,
      datasets: [{
        label: 'Due Cards',
        data: stats.forecast.slice(0, 14).map(d => d.new + d.learning + d.review),
        backgroundColor: chartColors.accentAlpha(0.75),
        borderRadius: 4,
      }],
    } as ChartData;
  });

  let forecastChartOptions: ChartOptions = {
    ...baseBarOptions(chartColors),
    scales: {
      ...baseBarOptions(chartColors).scales,
      x: {
        ...baseBarOptions(chartColors).scales?.x,
        ticks: {
          ...((baseBarOptions(chartColors).scales?.x as any)?.ticks),
          maxRotation: 45,
          font: { size: 9, family: 'DM Sans' },
        },
      },
    },
    plugins: {
      ...baseBarOptions(chartColors).plugins,
      tooltip: {
        ...baseBarOptions(chartColors).plugins?.tooltip,
        callbacks: {
          label: (ctx) => `${ctx.parsed.y} cards due`,
        },
      },
    },
  } as ChartOptions;

  // Chart: Daily Reviews (bar)
  let dailyReviewsChartData = $derived.by(() => {
    if (!stats?.daily_reviews.length) return { labels: [], datasets: [] };

    const last30 = stats.daily_reviews.slice(-30);

    return {
      labels: last30.map(d => {
        const date = new Date(d.date);
        return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
      }),
      datasets: [{
        label: 'Reviews',
        data: last30.map(d => d.count),
        backgroundColor: chartColors.accentAlpha(0.75),
        borderRadius: 6,
        borderSkipped: false,
      }],
    } as ChartData;
  });

  let dailyReviewsChartOptions: ChartOptions = {
    ...baseBarOptions(chartColors),
    scales: {
      ...baseBarOptions(chartColors).scales,
      x: {
        ...baseBarOptions(chartColors).scales?.x,
        ticks: {
          ...((baseBarOptions(chartColors).scales?.x as any)?.ticks),
          maxRotation: 45,
        },
      },
    },
    plugins: {
      ...baseBarOptions(chartColors).plugins,
      tooltip: {
        ...baseBarOptions(chartColors).plugins?.tooltip,
        callbacks: {
          title: (ctx) => {
            const idx = ctx[0].dataIndex;
            return new Date(stats!.daily_reviews[idx].date).toLocaleDateString('en-US', { 
              weekday: 'long', 
              month: 'short', 
              day: 'numeric' 
            });
          },
          label: (ctx) => `${ctx.parsed.y} reviews`,
        },
      },
    },
  } as ChartOptions;

  // Chart: Hourly Breakdown (bar)
  let hourlyChartData = $derived.by(() => {
    if (!stats?.hourly_breakdown) return { labels: [], datasets: [] };

    const hours = Array.from({ length: 24 }, (_, i) =>
      i === 0 ? '12am' : i < 12 ? `${i}am` : i === 12 ? '12pm' : `${i - 12}pm`
    );

    return {
      labels: hours,
      datasets: [{
        label: 'Reviews',
        data: stats.hourly_breakdown,
        backgroundColor: chartColors.learning,
        borderRadius: 4,
        borderSkipped: false,
      }],
    } as ChartData;
  });

  let hourlyChartOptions: ChartOptions = {
    ...baseBarOptions(chartColors),
    plugins: {
      ...baseBarOptions(chartColors).plugins,
      tooltip: {
        ...baseBarOptions(chartColors).plugins?.tooltip,
        callbacks: {
          label: (ctx) => `${ctx.parsed.y} reviews at ${ctx.label}`,
        },
      },
    },
  } as ChartOptions;
</script>

<div class="stats-container">
  <!-- Header with Scope Switcher -->
  <div class="stats-header">
    <h1 class="stats-title">Statistics</h1>
    
    {#if isTauriAvailable}
      <div class="scope-switcher">
        <button
          onclick={() => handleScopeChange("global")}
          class="scope-btn {scope === 'global' ? 'neu-pressed' : 'neu-raised'}"
        >
          All Decks
        </button>
        <button
          onclick={() => handleScopeChange("deck")}
          class="scope-btn {scope === 'deck' ? 'neu-pressed' : 'neu-raised'}"
        >
          By Deck
        </button>
      </div>
    {/if}
  </div>

  <!-- Deck Filter (when in deck scope) -->
  {#if isTauriAvailable && scope === "deck"}
    <div class="deck-filter">
      <label for="deck-select" class="filter-label">Deck:</label>
      <div class="filter-select-wrapper neu-pressed">
        <select
          id="deck-select"
          class="filter-select"
          value={selectedDeckId?.toString() ?? ""}
          onchange={handleDeckChange}
        >
          {#each decks.filter(d => !d.is_filtered) as deck}
            <option value={deck.id.toString()}>{deck.name}</option>
          {/each}
        </select>
        <svg class="select-arrow" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </div>
    </div>
  {/if}

  <!-- Not Tauri Message -->
  {#if !isTauriAvailable && !isLoading}
    <div class="stats-card error-card">
      <div class="error-content">
        <svg class="error-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
        </svg>
        <p class="error-title">Desktop app required for statistics.</p>
        <p class="error-message">Statistics require the Tauri desktop environment.</p>
      </div>
    </div>
  {/if}

  <!-- Error State -->
  {#if hasError && !isLoading}
    <div class="stats-card error-card">
      <div class="error-content">
        <svg class="error-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <p class="error-title">Failed to load statistics</p>
        <p class="error-message">{errorMessage}</p>
      </div>
      <button onclick={loadData} class="retry-btn">Retry</button>
    </div>
  {/if}

  <!-- Stats Content -->
  {#if isTauriAvailable && !hasError && !isLoading && stats}
    <!-- Metrics Row -->
    <div class="metrics-row">
      <div class="stats-card metric-card">
        <div class="metric-label">Total Cards</div>
        <div class="metric-value">{stats.total_cards}</div>
      </div>
      <div class="stats-card metric-card">
        <div class="metric-label">Total Notes</div>
        <div class="metric-value">{stats.total_notes}</div>
      </div>
      <div class="stats-card metric-card">
        <div class="metric-label">Total Reviews</div>
        <div class="metric-value">{stats.total_reviews}</div>
      </div>
      <div class="stats-card metric-card">
        <div class="metric-label">Avg. Ease</div>
        <div class="metric-value">{stats.average_ease.toFixed(2)}</div>
      </div>
      <div class="stats-card metric-card">
        <div class="metric-label">Avg. Interval</div>
        <div class="metric-value">{stats.average_interval_days.toFixed(1)}d</div>
      </div>
    </div>

    <!-- Heatmap + Calendar Row -->
    <div class="heatmap-calendar-row">
      <!-- Heatmap -->
      <div class="stats-card heatmap-card">
        <h3 class="card-title">Review Activity</h3>
        <div class="heatmap-container">
          {#each heatmapWeeks as week, weekIdx}
            <div class="heatmap-week">
              {#each week as day, dayIdx}
                <div
                  class="heatmap-cell"
                  style="background: {day.count > 0 ? `rgba(${chartColors.accent.replace('#', '').match(/.{2}/g)?.map(h => parseInt(h, 16)).join(',')}, ${Math.min(0.2 + (day.count / 20) * 0.8, 1)})` : 'var(--bg-subtle)'}"
                  title="{day.date}: {day.count} reviews"
                ></div>
              {/each}
            </div>
          {/each}
        </div>
        <div class="heatmap-legend">
          <span class="heatmap-legend-text">Less</span>
          <div class="heatmap-legend-cells">
            <div class="heatmap-cell" style="background: var(--bg-subtle)"></div>
            <div class="heatmap-cell" style="background: rgba({chartColors.accent.replace('#', '').match(/.{2}/g)?.map(h => parseInt(h, 16)).join(',')}, 0.3)"></div>
            <div class="heatmap-cell" style="background: rgba({chartColors.accent.replace('#', '').match(/.{2}/g)?.map(h => parseInt(h, 16)).join(',')}, 0.5)"></div>
            <div class="heatmap-cell" style="background: rgba({chartColors.accent.replace('#', '').match(/.{2}/g)?.map(h => parseInt(h, 16)).join(',')}, 0.7)"></div>
            <div class="heatmap-cell" style="background: {chartColors.accent}"></div>
          </div>
          <span class="heatmap-legend-text">More</span>
        </div>
        <div class="consistency-footer">
          <span class="consistency-label">Consistency:</span>
          <span class="consistency-value">{(consistency * 100).toFixed(0)}%</span>
        </div>
      </div>

      <!-- Calendar -->
      <div class="stats-card calendar-card">
        <div class="calendar-header">
          <button onclick={prevMonth} class="calendar-nav-btn">&lt;</button>
          <span class="calendar-month">{new Date(calYear, calMonth).toLocaleDateString('en-US', { month: 'long', year: 'numeric' })}</span>
          <button onclick={nextMonth} class="calendar-nav-btn">&gt;</button>
        </div>
        <div class="calendar-grid">
          <div class="calendar-day-header">Su</div>
          <div class="calendar-day-header">Mo</div>
          <div class="calendar-day-header">Tu</div>
          <div class="calendar-day-header">We</div>
          <div class="calendar-day-header">Th</div>
          <div class="calendar-day-header">Fr</div>
          <div class="calendar-day-header">Sa</div>
          {#each calendarCells as cell}
            {#if cell === null}
              <div class="calendar-cell empty"></div>
            {:else}
              {@const dateStr = `${calYear}-${String(calMonth + 1).padStart(2, '0')}-${String(cell).padStart(2, '0')}`}
              {@const count = heatmapLookup[dateStr] || 0}
              <div
                class="calendar-cell {count > 0 ? 'has-reviews' : ''}"
                style="{count > 0 ? `background: rgba(${chartColors.accent.replace('#', '').match(/.{2}/g)?.map(h => parseInt(h, 16)).join(',')}, ${Math.min(0.2 + (count / 10) * 0.8, 1)})` : ''}"
                title="{dateStr}: {count} reviews"
              >
                {cell}
              </div>
            {/if}
          {/each}
        </div>
      </div>
    </div>

    <!-- Charts Row -->
    <div class="charts-row">
      <!-- Card Types -->
      <div class="stats-card chart-card">
        <h3 class="card-title">Card Types</h3>
        <div class="chart-container">
          <Chart type="doughnut" data={cardTypesChartData} options={cardTypesChartOptions} />
        </div>
      </div>

      <!-- Forecast -->
      <div class="stats-card chart-card">
        <h3 class="card-title">Review Forecast (14 days)</h3>
        <div class="chart-container">
          <Chart type="bar" data={forecastChartData} options={forecastChartOptions} />
        </div>
      </div>

      <!-- Streak -->
      <div class="stats-card streak-card">
        <h3 class="card-title">Streak</h3>
        <div class="streak-main">
          <div class="streak-value">{streakData.current}</div>
          <div class="streak-label">days</div>
        </div>
        <div class="streak-details">
          <div class="streak-detail">
            <span class="streak-detail-label">Longest</span>
            <span class="streak-detail-value">{streakData.longest}</span>
          </div>
          <div class="streak-detail">
            <span class="streak-detail-label">Average</span>
            <span class="streak-detail-value">{streakData.average}</span>
          </div>
        </div>
        <div class="streak-strip">
          {#each streakData.strip as active, i}
            <div class="streak-day {active ? 'active' : ''}"></div>
          {/each}
        </div>
      </div>
    </div>

    <!-- Retention + Hourly Row -->
    <div class="retention-hourly-row">
      <!-- Retention by Deck -->
      <div class="stats-card retention-card">
        <h3 class="card-title">Retention</h3>
        <div class="retention-bars">
          {#each retentionByDeck as item}
            <div class="retention-item">
              <div class="retention-header">
                <span class="retention-name">{item.name}</span>
                <span class="retention-value">{formatPercentage(item.retention)}</span>
              </div>
              <div class="retention-bar-bg">
                <div class="retention-bar-fill" style="width: {item.retention * 100}%"></div>
              </div>
            </div>
          {/each}
        </div>
      </div>

      <!-- Hourly Breakdown -->
      <div class="stats-card chart-card">
        <h3 class="card-title">Study Time (Hourly)</h3>
        <div class="chart-container chart-short">
          <Chart type="bar" data={hourlyChartData} options={hourlyChartOptions} />
        </div>
      </div>
    </div>

    <!-- Weak Cards -->
    {#if difficultCards.length > 0}
      <div class="stats-card weak-cards-card">
        <h3 class="card-title">Needs Work</h3>
        <div class="weak-cards-grid">
          {#each difficultCards.slice(0, 12) as card}
            <div class="weak-card-item">
              <div class="weak-card-ring">
                <svg viewBox="0 0 36 36" class="ring-svg">
                  <path
                    class="ring-bg"
                    d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
                  />
                  <path
                    class="ring-fill"
                    stroke-dasharray="{card.retention * 100}, 100"
                    d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
                  />
                </svg>
                <span class="ring-text">{(card.retention * 100).toFixed(0)}%</span>
              </div>
              <div class="weak-card-info">
                <div class="weak-card-front">{getFirstField(card.fields_csv)}</div>
                <div class="weak-card-meta">
                  <span class="weak-card-deck">{card.deck_name}</span>
                  <span class="weak-card-stats">{card.lapses} lapses · {card.reviews} reviews · ease {card.ease.toFixed(2)}</span>
                </div>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Daily Reviews Chart -->
    <div class="stats-card chart-card chart-wide">
      <h3 class="card-title">Reviews per Day (Last 30 Days)</h3>
      <div class="chart-container">
        <Chart type="bar" data={dailyReviewsChartData} options={dailyReviewsChartOptions} />
      </div>
    </div>
  {/if}

  <!-- Loading State -->
  {#if isLoading}
    <div class="stats-card loading-card">
      <div class="loading-spinner"></div>
      <p class="loading-text">Loading statistics...</p>
    </div>
  {/if}
</div>

<style>
  .stats-container {
    max-width: 920px;
    margin: 0 auto;
    padding: 32px 24px;
  }

  .stats-header {
    display: flex;
    flex-direction: column;
    gap: 16px;
    margin-bottom: 24px;
  }

  @media (min-width: 768px) {
    .stats-header {
      flex-direction: row;
      align-items: center;
      justify-content: space-between;
    }
  }

  .stats-title {
    font-family: var(--serif);
    font-size: 28px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
    letter-spacing: -0.02em;
  }

  .scope-switcher {
    display: flex;
    gap: 8px;
  }

  .scope-btn {
    padding: 8px 16px;
    font-family: var(--sans);
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .scope-btn:hover {
    color: var(--text-primary);
  }

  .deck-filter {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 24px;
  }

  .filter-label {
    font-family: var(--sans);
    font-size: 14px;
    color: var(--text-secondary);
  }

  .filter-select-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .filter-select {
    padding: 10px 36px 10px 14px;
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    background: transparent;
    border: none;
    appearance: none;
    cursor: pointer;
    outline: none;
  }

  .select-arrow {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    width: 16px;
    height: 16px;
    color: var(--text-secondary);
    pointer-events: none;
  }

  .stats-card {
    background: var(--bg-card);
    border-radius: var(--radius-md, 14px);
    padding: 20px;
    box-shadow: var(--neu-up);
    border: 1px solid var(--border);
    overflow: hidden;
    min-width: 0;
  }

  .error-card {
    text-align: center;
    padding: 40px 20px;
  }

  .error-content {
    margin-bottom: 20px;
  }

  .error-icon {
    width: 48px;
    height: 48px;
    margin: 0 auto 16px;
    color: var(--danger);
  }

  .error-title {
    font-family: var(--sans);
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 8px 0;
  }

  .error-message {
    font-family: var(--sans);
    font-size: 14px;
    color: var(--text-secondary);
    margin: 0;
  }

  .retry-btn {
    padding: 10px 24px;
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 500;
    color: white;
    background: var(--accent);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .retry-btn:hover {
    opacity: 0.9;
  }

  .loading-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 20px;
  }

  .loading-spinner {
    width: 40px;
    height: 40px;
    border: 3px solid var(--bg-subtle);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .loading-text {
    font-family: var(--sans);
    font-size: 14px;
    color: var(--text-secondary);
    margin: 16px 0 0 0;
  }

  .metrics-row {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 16px;
    margin-bottom: 24px;
  }

  .metric-card {
    padding: 16px;
    text-align: center;
  }

  .metric-label {
    font-family: var(--sans);
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }

  .metric-value {
    font-family: var(--serif);
    font-size: 24px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .heatmap-calendar-row {
    display: grid;
    grid-template-columns: 1fr;
    gap: 24px;
    margin-bottom: 24px;
  }

  @media (min-width: 768px) {
    .heatmap-calendar-row {
      grid-template-columns: 1fr 220px;
    }
  }

  .heatmap-card {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }

  .card-title {
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 16px 0;
  }

  .heatmap-container {
    display: flex;
    gap: 3px;
    flex-wrap: nowrap;
    overflow-x: auto;
    padding-bottom: 8px;
  }

  .heatmap-week {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .heatmap-cell {
    width: 12px;
    height: 12px;
    border-radius: 2px;
    flex-shrink: 0;
    min-width: 0;
  }

  .heatmap-legend {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
    justify-content: flex-end;
  }

  .heatmap-legend-text {
    font-family: var(--sans);
    font-size: 11px;
    color: var(--text-secondary);
  }

  .heatmap-legend-cells {
    display: flex;
    gap: 3px;
  }

  .consistency-footer {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .consistency-label {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-secondary);
  }

  .consistency-value {
    font-family: var(--serif);
    font-size: 18px;
    font-weight: 600;
    color: var(--accent);
  }

  .calendar-card {
    padding: 16px;
  }

  .calendar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }

  .calendar-nav-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-subtle);
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    cursor: pointer;
    font-family: var(--sans);
    font-size: 14px;
  }

  .calendar-nav-btn:hover {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .calendar-month {
    font-family: var(--sans);
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .calendar-grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 4px;
  }

  .calendar-day-header {
    font-family: var(--sans);
    font-size: 10px;
    font-weight: 600;
    color: var(--text-secondary);
    text-align: center;
    padding: 4px 0;
  }

  .calendar-cell {
    aspect-ratio: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--sans);
    font-size: 11px;
    color: var(--text-primary);
    border-radius: 4px;
    background: var(--bg-subtle);
  }

  .calendar-cell.empty {
    background: transparent;
  }

  .calendar-cell.has-reviews {
    font-weight: 600;
    color: white;
  }

  .charts-row {
    display: grid;
    grid-template-columns: 1fr;
    gap: 24px;
    margin-bottom: 24px;
  }

  @media (min-width: 768px) {
    .charts-row {
      grid-template-columns: 190px 1fr;
    }
  }

  @media (min-width: 1024px) {
    .charts-row {
      grid-template-columns: 190px 1fr 170px;
    }
  }

  .chart-card {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }

  .chart-container {
    flex: 1;
    min-height: 150px;
    max-height: 280px;
    position: relative;
    overflow: hidden;
  }

  .chart-short {
    min-height: 150px;
  }

  .chart-wide {
    grid-column: 1 / -1;
  }

  .streak-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
  }

  .streak-main {
    margin-bottom: 16px;
  }

  .streak-value {
    font-family: var(--serif);
    font-size: 48px;
    font-weight: 600;
    color: var(--accent);
    line-height: 1;
  }

  .streak-label {
    font-family: var(--sans);
    font-size: 14px;
    color: var(--text-secondary);
    margin-top: 4px;
  }

  .streak-details {
    display: flex;
    gap: 24px;
    margin-bottom: 16px;
  }

  .streak-detail {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .streak-detail-label {
    font-family: var(--sans);
    font-size: 11px;
    color: var(--text-secondary);
  }

  .streak-detail-value {
    font-family: var(--serif);
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .streak-strip {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
    justify-content: center;
  }

  .streak-day {
    width: 16px;
    height: 16px;
    border-radius: 3px;
    background: var(--bg-subtle);
  }

  .streak-day.active {
    background: var(--accent);
  }

  .retention-hourly-row {
    display: grid;
    grid-template-columns: 1fr;
    gap: 24px;
    margin-bottom: 24px;
  }

  @media (min-width: 768px) {
    .retention-hourly-row {
      grid-template-columns: 1fr 1fr;
    }
  }

  .retention-card {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .retention-bars {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .retention-item {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .retention-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .retention-name {
    font-family: var(--sans);
    font-size: 13px;
    color: var(--text-primary);
  }

  .retention-value {
    font-family: var(--sans);
    font-size: 13px;
    font-weight: 600;
    color: var(--accent);
  }

  .retention-bar-bg {
    height: 8px;
    background: var(--bg-subtle);
    border-radius: 4px;
    overflow: hidden;
  }

  .retention-bar-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 4px;
    transition: width 0.3s ease;
  }

  .weak-cards-card {
    margin-bottom: 24px;
  }

  .weak-cards-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 12px;
  }

  @media (min-width: 640px) {
    .weak-cards-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  .weak-card-item {
    display: flex;
    gap: 12px;
    padding: 12px;
    background: var(--bg-subtle);
    border-radius: var(--radius-sm);
  }

  .weak-card-ring {
    position: relative;
    width: 48px;
    height: 48px;
    flex-shrink: 0;
  }

  .ring-svg {
    width: 100%;
    height: 100%;
    transform: rotate(-90deg);
  }

  .ring-bg {
    fill: none;
    stroke: var(--border);
    stroke-width: 3;
  }

  .ring-fill {
    fill: none;
    stroke: var(--accent);
    stroke-width: 3;
    stroke-linecap: round;
  }

  .ring-text {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .weak-card-info {
    flex: 1;
    min-width: 0;
  }

  .weak-card-front {
    font-family: var(--sans);
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-bottom: 4px;
  }

  .weak-card-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .weak-card-deck {
    font-family: var(--sans);
    font-size: 11px;
    color: var(--text-secondary);
  }

  .weak-card-stats {
    font-family: var(--sans);
    font-size: 10px;
    color: var(--text-muted);
  }
</style>
